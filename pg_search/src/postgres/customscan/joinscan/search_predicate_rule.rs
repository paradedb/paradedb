// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

//! Logical optimizer rule (`JoinSearchPredicateRule`) that lowers join-level
//! single-table search predicates (`SearchPredicateUdf`) into scan execution modes.
//!
//! During initial plan construction (`build_clause_df`), join-level single-table
//! search predicates (`JoinLevelExpr::SingleTablePredicate`) are emitted as
//! `SearchPredicateUdf` scalar function calls taking the relation's `ctid_<n>`
//! column as an argument. That column dependency allows DataFusion's standard
//! `PushDownFilter` and `OuterJoinToInner` rules to push search predicates to the
//! appropriate `Join.filter` or `Filter` nodes before any scan mode commitment is made.
//!
//! `JoinSearchPredicateRule` then runs immediately before `OptimizeProjections` and
//! converts `SearchPredicateUdf` calls into `ScanMode::Tagged` match-tag columns
//! (`__{alias}_tag_{i}`) on the leaf `PgSearchTableProvider`s.

use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use arrow_array::BooleanArray;
use arrow_schema::{DataType, Field, SortOptions};
use datafusion::catalog::default_table_source::DefaultTableSource;
use datafusion::common::config::ConfigOptions;
use datafusion::common::tree_node::{Transformed, TreeNode, TreeNodeRecursion};
use datafusion::common::{Column, DFSchema, DataFusionError, Result, ScalarValue};
use datafusion::logical_expr::expr::{ScalarFunction, Sort as SortExpr};
use datafusion::logical_expr::logical_plan::{Join, Sort};
use datafusion::logical_expr::{
    BinaryExpr as LogicalBinaryExpr, ColumnarValue, Expr, Filter, JoinType, LogicalPlan, Operator,
    Projection, ScalarFunctionArgs, ScalarUDF, ScalarUDFImpl, Signature, SubqueryAlias, TableScan,
    TableSource, Volatility,
};
use datafusion::optimizer::optimizer::ApplyOrder;
use datafusion::optimizer::{OptimizerConfig, OptimizerRule};
use datafusion::physical_expr::expressions::{
    BinaryExpr as PhysicalBinaryExpr, CastExpr, Column as PhysicalColumn, Literal, TryCastExpr,
};
use datafusion::physical_expr::{PhysicalExpr, PhysicalSortExpr, ScalarFunctionExpr};

use serde::{Deserialize, Serialize};

use crate::postgres::customscan::datafusion::translator::make_source_col;
use crate::postgres::customscan::joinscan::CtidColumn;
use crate::postgres::customscan::joinscan::build::{JoinSource, RelationAlias};
use crate::query::SearchQueryInput;
use crate::scan::late_materialization::{get_deferred_fields, trace_column};
use crate::scan::rank_join_rule::{
    MAX_RANK_JOIN_K, SubtreeScoreStats, analyze_join_score_expr, is_score_column_name,
};
use crate::scan::table_provider::pg_search_provider_from_scan;
use crate::scan::{GlobalPredicateIndex, ScanMode, TagIndex, TaggedQuery};

pub const SEARCH_PREDICATE_UDF_PREFIX: &str = "pdb_search_predicate_";

static NEXT_PREDICATE_ORDINAL: AtomicUsize = AtomicUsize::new(0);

fn default_search_predicate_signature() -> Signature {
    Signature::any(1, Volatility::Immutable)
}

/// Placeholder scalar UDF representing a join-level single-table search predicate
/// (`JoinLevelExpr::SingleTablePredicate`) in a DataFusion `LogicalPlan` prior to
/// [`JoinSearchPredicateRule`].
///
/// Takes the target source's `ctid_<plan_position>` column as its single argument
/// so DataFusion's filter pushdown and join simplification rules preserve relation
/// attribution. Each instance carries a distinct `name` (`pdb_search_predicate_{ordinal}`)
/// because DataFusion's `NormalizeEq` (used by `CommonSubexprEliminate`) compares
/// `ScalarFunction` expressions by `func.name()` and arguments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPredicateUdf {
    name: String,
    ordinal: usize,
    plan_position: usize,
    alias: String,
    query: Box<SearchQueryInput>,
    #[serde(skip, default = "default_search_predicate_signature")]
    signature: Signature,
}

impl SearchPredicateUdf {
    pub fn new(plan_position: usize, alias: String, query: Box<SearchQueryInput>) -> Self {
        let ordinal = NEXT_PREDICATE_ORDINAL.fetch_add(1, Ordering::Relaxed);
        let name = format!("{SEARCH_PREDICATE_UDF_PREFIX}{ordinal}");
        Self {
            name,
            ordinal,
            plan_position,
            alias,
            query,
            signature: default_search_predicate_signature(),
        }
    }
}

impl PartialEq for SearchPredicateUdf {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.plan_position == other.plan_position
            && self.alias == other.alias
            && self.query == other.query
    }
}

impl Eq for SearchPredicateUdf {}

impl Hash for SearchPredicateUdf {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.plan_position.hash(state);
        self.alias.hash(state);
    }
}

impl ScalarUDFImpl for SearchPredicateUdf {
    fn name(&self) -> &str {
        &self.name
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> Result<DataType> {
        Ok(DataType::Boolean)
    }

    fn invoke_with_args(&self, args: ScalarFunctionArgs) -> Result<ColumnarValue> {
        // DataFusion's `OuterJoinToInner` rule probes whether a filter is null-rejecting
        // by substituting NULL for nullable outer-join columns (including `ctid_<n>`) and
        // running the constant evaluator. For a null-extended row, the match tag is NULL.
        if let Some(arg) = args.args.first() {
            match arg {
                ColumnarValue::Scalar(s) if s.is_null() => {
                    return Ok(ColumnarValue::Scalar(ScalarValue::Boolean(None)));
                }
                ColumnarValue::Array(arr) if arr.null_count() == arr.len() => {
                    return Ok(ColumnarValue::Array(Arc::new(BooleanArray::new_null(
                        arr.len(),
                    ))));
                }
                _ => {}
            }
        }
        Err(DataFusionError::Internal(format!(
            "{} must be rewritten by JoinSearchPredicateRule before execution",
            self.name
        )))
    }
}

/// Builds a `SearchPredicateUdf` call expression referencing `source`'s `ctid_<plan_position>` column.
pub fn make_search_predicate_expr(source: &JoinSource, query: &SearchQueryInput) -> Expr {
    let plan_position = source.plan_position;
    let alias = RelationAlias::new(source.scan_info.alias.as_deref()).execution(plan_position);
    let ctid_col = make_source_col(source, &CtidColumn::new(plan_position).to_string());
    let udf = ScalarUDF::new_from_impl(SearchPredicateUdf::new(
        plan_position,
        alias,
        Box::new(query.clone()),
    ));
    udf.call(vec![ctid_col])
}

/// Logical optimizer rule that resolves `SearchPredicateUdf` expressions in a `LogicalPlan`.
#[derive(Debug, Default)]
pub struct JoinSearchPredicateRule {
    /// Optional override for `paradedb.rank_join_min_rows` in unit tests.
    min_rows_override: Option<usize>,
}

impl JoinSearchPredicateRule {
    pub fn new() -> Self {
        Self {
            min_rows_override: None,
        }
    }

    #[cfg(any(test, feature = "pg_test"))]
    pub fn new_with_min_rows(min_rows: usize) -> Self {
        Self {
            min_rows_override: Some(min_rows),
        }
    }
}

impl OptimizerRule for JoinSearchPredicateRule {
    fn name(&self) -> &str {
        "join_search_predicate_rule"
    }

    fn apply_order(&self) -> Option<ApplyOrder> {
        None
    }

    fn rewrite(
        &self,
        plan: LogicalPlan,
        _config: &dyn OptimizerConfig,
    ) -> Result<Transformed<LogicalPlan>> {
        let collected = collect_search_predicates(&plan);
        if collected.is_empty() {
            return Ok(Transformed::no(plan));
        }

        let tags_by_position = build_tagged_queries(collected.clone());
        let elided_positions = if self.min_rows_override.is_some()
            || (crate::gucs::enable_rank_join()
                && !crate::postgres::customscan::mpp::glue::mpp_is_active())
        {
            let min_rows = self
                .min_rows_override
                .unwrap_or_else(crate::gucs::rank_join_min_rows);
            find_elidable_rank_join_positions(&plan, &tags_by_position, &collected, min_rows)
        } else {
            BTreeSet::new()
        };

        let original_schema = Arc::clone(plan.schema());
        let rewritten = rewrite_plan(plan, &tags_by_position, &elided_positions, &BTreeSet::new())?;
        if !rewritten.transformed {
            return Ok(rewritten);
        }

        // Preserve the root plan schema invariant if the root node was not a Projection.
        let final_plan = if rewritten.data.schema().fields().len() != original_schema.fields().len()
        {
            let proj_exprs: Vec<Expr> = original_schema
                .iter()
                .map(|(q, f)| Expr::Column(Column::new(q.cloned(), f.name())))
                .collect();
            LogicalPlan::Projection(Projection::try_new(proj_exprs, Arc::new(rewritten.data))?)
        } else {
            rewritten.data
        };

        Ok(Transformed::yes(final_plan))
    }
}

fn as_search_predicate_udf(expr: &Expr) -> Option<&SearchPredicateUdf> {
    if let Expr::ScalarFunction(ScalarFunction { func, .. }) = expr {
        func.inner().downcast_ref::<SearchPredicateUdf>()
    } else {
        None
    }
}

/// Collects all `SearchPredicateUdf` instances appearing in `plan` expressions.
fn collect_search_predicates(plan: &LogicalPlan) -> Vec<SearchPredicateUdf> {
    let mut predicates = Vec::new();
    let _ = plan.apply(|node| {
        let _ = node.apply_expressions(|expr| {
            let _ = expr.apply(|subexpr| {
                if let Some(udf) = as_search_predicate_udf(subexpr) {
                    predicates.push(udf.clone());
                }
                Ok(TreeNodeRecursion::Continue)
            });
            Ok(TreeNodeRecursion::Continue)
        });
        Ok(TreeNodeRecursion::Continue)
    });
    predicates
}

/// Groups collected `SearchPredicateUdf`s by `plan_position` in creation-ordinal order,
/// deduplicating identical `(plan_position, query)` pairs and assigning `TaggedQuery` metadata.
fn build_tagged_queries(
    mut collected: Vec<SearchPredicateUdf>,
) -> BTreeMap<usize, Vec<TaggedQuery>> {
    collected.sort_by_key(|u| u.ordinal);

    let mut unique: Vec<SearchPredicateUdf> = Vec::new();
    for udf in collected {
        if !unique.iter().any(|existing| {
            existing.plan_position == udf.plan_position && existing.query == udf.query
        }) {
            unique.push(udf);
        }
    }

    let mut by_position: BTreeMap<usize, Vec<TaggedQuery>> = BTreeMap::new();
    for (global_idx, udf) in unique.into_iter().enumerate() {
        let entry = by_position.entry(udf.plan_position).or_default();
        let local_idx = entry.len();
        let tag_name = format!("__{}_tag_{local_idx}", udf.alias);
        entry.push(TaggedQuery {
            tag_name,
            tag_idx: TagIndex(local_idx),
            predicate_idx: GlobalPredicateIndex(global_idx),
            query: udf.query,
        });
    }
    by_position
}

/// Identifies `plan_position`s whose `SearchPredicateUdf`s sit in the root `Join.filter` of an
/// eligible `RankJoin` subtree and can therefore be executed via `SA` + `RA` without materializing
/// boolean match-tag columns or a post-join `OR` filter.
fn find_elidable_rank_join_positions(
    plan: &LogicalPlan,
    tags_by_position: &BTreeMap<usize, Vec<TaggedQuery>>,
    all_predicates: &[SearchPredicateUdf],
    min_rows: usize,
) -> BTreeSet<usize> {
    let mut elided = BTreeSet::new();
    let _ = plan.apply(|node| {
        if let LogicalPlan::Sort(sort) = node
            && let Some(positions) =
                try_elide_under_sort(sort, tags_by_position, all_predicates, min_rows)
        {
            elided.extend(positions);
            return Ok(TreeNodeRecursion::Jump);
        }
        Ok(TreeNodeRecursion::Continue)
    });
    elided
}

fn try_elide_under_sort(
    sort: &Sort,
    tags_by_position: &BTreeMap<usize, Vec<TaggedQuery>>,
    all_predicates: &[SearchPredicateUdf],
    min_rows: usize,
) -> Option<BTreeSet<usize>> {
    let k = sort.fetch?;
    if k == 0 || k > MAX_RANK_JOIN_K || sort.expr.is_empty() {
        return None;
    }

    // If any sort expression (including secondary tie-breaker keys) references a deferred
    // column, `LateMaterializationRule` will anchor a `LateMaterializeNode` (`TantivyDecodeExec`
    // or `TantivyFetchExec`) directly below `Sort`, which changes that column's `DataType`
    // from `UInt64` to `Utf8View`/`BinaryView` and causes `RankJoinRule::map_ordering_through_passthrough`
    // to decline the plan.
    if sort_references_deferred_column(sort) {
        return None;
    }

    let phys_sort = logical_sort_to_physical(&sort.expr[0], sort.input.schema())?;
    let (root_join, join_sort_expr) = find_root_join_under_sort(sort.input.as_ref(), phys_sort)?;

    let filter_expr = root_join.filter.as_ref()?;
    let mut filter_udfs = Vec::new();
    if !extract_pure_or_search_predicates(filter_expr, &mut filter_udfs) || filter_udfs.is_empty() {
        return None;
    }

    // Collect all leaf `TableScan` `plan_position`s in `root_join`'s subtree.
    let mut leaf_positions = BTreeSet::new();
    let mut num_leaves = 0usize;
    let mut all_leaves_valid = true;
    let _ = LogicalPlan::Join(root_join.clone()).apply(|n| {
        if let LogicalPlan::TableScan(scan) = n {
            num_leaves += 1;
            if let Some(pos) = resolve_scan_plan_position(scan) {
                leaf_positions.insert(pos);
            } else {
                all_leaves_valid = false;
                return Ok(TreeNodeRecursion::Stop);
            }
        }
        Ok(TreeNodeRecursion::Continue)
    });
    if !all_leaves_valid || num_leaves < 2 || leaf_positions.len() != num_leaves {
        return None;
    }

    // Every leaf `TableScan` in `root_join`'s subtree must be covered by the pure `OR` in
    // `root_join.filter` so that every leaf `SA` stream produces only rows satisfying the disjunction.
    let filter_positions: BTreeSet<usize> = filter_udfs.iter().map(|u| u.plan_position).collect();
    if filter_positions != leaf_positions {
        return None;
    }

    // None of the leaf `plan_position`s in `root_join`'s subtree may have other `SearchPredicateUdf`s
    // outside `root_join.filter`.
    let filter_ordinals: BTreeSet<usize> = filter_udfs.iter().map(|u| u.ordinal).collect();
    for udf in all_predicates {
        if leaf_positions.contains(&udf.plan_position) && !filter_ordinals.contains(&udf.ordinal) {
            return None;
        }
    }

    // Verify Gates 1, 2, and 3 across the entire `root_join` subtree.
    inspect_logical_join(root_join, &join_sort_expr, tags_by_position, min_rows, true)?;

    Some(leaf_positions)
}

fn sort_references_deferred_column(sort: &Sort) -> bool {
    let deferred = get_deferred_fields(sort.input.as_ref());
    if deferred.is_empty() {
        return false;
    }
    let mut found_deferred = false;
    for sort_expr in &sort.expr {
        let _ = sort_expr.expr.apply(|e| {
            if let Expr::Column(col) = e
                && let Some(base) = trace_column(sort.input.as_ref(), col)
                && deferred.iter().any(|d| base.is(d))
            {
                found_deferred = true;
                return Ok(TreeNodeRecursion::Stop);
            }
            Ok(TreeNodeRecursion::Continue)
        });
        if found_deferred {
            return true;
        }
    }
    false
}

/// Walks down from `Sort.input` through pass-through nodes (`Projection`, `SubqueryAlias`)
/// to locate the root `LogicalPlan::Join` and map `required_sort` into `join.schema`.
fn find_root_join_under_sort(
    node: &LogicalPlan,
    required_sort: PhysicalSortExpr,
) -> Option<(&Join, PhysicalSortExpr)> {
    match node {
        LogicalPlan::Projection(proj) => {
            let mapped = map_sort_through_logical_projection(proj, &required_sort)?;
            find_root_join_under_sort(proj.input.as_ref(), mapped)
        }
        LogicalPlan::SubqueryAlias(alias) => {
            find_root_join_under_sort(alias.input.as_ref(), required_sort)
        }
        LogicalPlan::Join(join) => Some((join, required_sort)),
        _ => None,
    }
}

fn map_sort_through_logical_projection(
    proj: &Projection,
    sort_expr: &PhysicalSortExpr,
) -> Option<PhysicalSortExpr> {
    let input_schema = proj.input.schema();
    let mut valid = true;
    let rewritten = Arc::clone(&sort_expr.expr)
        .transform_down(|expr| {
            if let Some(col) = expr.downcast_ref::<PhysicalColumn>() {
                if col.index() >= proj.expr.len() {
                    valid = false;
                    return Ok(Transformed::no(expr));
                }
                let Some(mapped_expr) =
                    logical_expr_to_physical(&proj.expr[col.index()], input_schema)
                else {
                    valid = false;
                    return Ok(Transformed::no(expr));
                };
                return Ok(Transformed::new(mapped_expr, true, TreeNodeRecursion::Jump));
            }
            Ok(Transformed::no(expr))
        })
        .ok()?
        .data;
    if !valid {
        return None;
    }
    Some(PhysicalSortExpr {
        expr: rewritten,
        options: sort_expr.options,
    })
}

fn inspect_logical_join(
    join: &Join,
    required_sort: &PhysicalSortExpr,
    tags_by_position: &BTreeMap<usize, Vec<TaggedQuery>>,
    min_rows: usize,
    allow_filter: bool,
) -> Option<SubtreeScoreStats> {
    if join.join_type != JoinType::Inner || join.null_aware || join.on.is_empty() {
        return None;
    }
    if !allow_filter && join.filter.is_some() {
        return None;
    }

    let left_schema = join.left.schema().inner();
    let right_schema = join.right.schema().inner();
    let decomposed = analyze_join_score_expr(required_sort, left_schema, right_schema)?;

    // Disjunctive RankJoin requires two-sided scoring (`uses_left && uses_right`) at every join
    // in the subtree, because single-sided (`NRJN`) joins disable `SA` on the unscored child.
    if !decomposed.combiner.uses_left() || !decomposed.combiner.uses_right() {
        return None;
    }

    let left_stats = inspect_logical_child_branch(
        join.left.as_ref(),
        &decomposed.left_sort_expr,
        tags_by_position,
        min_rows,
    )?;
    let right_stats = inspect_logical_child_branch(
        join.right.as_ref(),
        &decomposed.right_sort_expr,
        tags_by_position,
        min_rows,
    )?;

    SubtreeScoreStats::combine(
        &decomposed.combiner,
        left_stats,
        right_stats,
        min_rows,
        false,
    )
}

fn inspect_logical_child_branch(
    node: &LogicalPlan,
    required_sort: &PhysicalSortExpr,
    tags_by_position: &BTreeMap<usize, Vec<TaggedQuery>>,
    min_rows: usize,
) -> Option<SubtreeScoreStats> {
    match node {
        LogicalPlan::Projection(proj) => {
            let mapped = map_sort_through_logical_projection(proj, required_sort)?;
            inspect_logical_child_branch(proj.input.as_ref(), &mapped, tags_by_position, min_rows)
        }
        LogicalPlan::SubqueryAlias(alias) => inspect_logical_child_branch(
            alias.input.as_ref(),
            required_sort,
            tags_by_position,
            min_rows,
        ),
        LogicalPlan::Join(join) => {
            inspect_logical_join(join, required_sort, tags_by_position, min_rows, false)
        }
        LogicalPlan::TableScan(scan) => {
            inspect_logical_leaf_scan(scan, required_sort, tags_by_position)
        }
        _ => None,
    }
}

fn inspect_logical_leaf_scan(
    scan: &TableScan,
    required_sort: &PhysicalSortExpr,
    tags_by_position: &BTreeMap<usize, Vec<TaggedQuery>>,
) -> Option<SubtreeScoreStats> {
    let provider = pg_search_provider_from_scan(scan)?;
    let plan_pos = resolve_scan_plan_position(scan)?;

    // Multi-partition MPP scans are not eligible for `RankJoinExec`.
    if provider.source_idx().is_some() || provider.range_split_points().is_some() {
        return None;
    }

    let col = required_sort.expr.downcast_ref::<PhysicalColumn>()?;
    if col.index() >= scan.projected_schema.fields().len() {
        return None;
    }
    let field_name = scan.projected_schema.field(col.index()).name();
    if provider
        .deferred_fields()
        .iter()
        .any(|d| d.name == *field_name)
    {
        return None;
    }

    let is_score = is_score_column_name(field_name);
    let base_query =
        provider.combine_query_with_filters(provider.scan_info.mode.query().clone(), &scan.filters);
    let disjunct_queries: Vec<SearchQueryInput> = tags_by_position
        .get(&plan_pos)
        .map(|tqs| tqs.iter().map(|tq| (*tq.query).clone()).collect())
        .unwrap_or_default();

    SubtreeScoreStats::from_leaf_queries(
        is_score,
        &base_query,
        &disjunct_queries,
        provider.scan_info.estimate.as_planner_estimate(),
    )
}

fn logical_sort_to_physical(sort_expr: &SortExpr, schema: &DFSchema) -> Option<PhysicalSortExpr> {
    let expr = logical_expr_to_physical(&sort_expr.expr, schema)?;
    Some(PhysicalSortExpr {
        expr,
        options: SortOptions {
            descending: !sort_expr.asc,
            nulls_first: sort_expr.nulls_first,
        },
    })
}

fn logical_expr_to_physical(expr: &Expr, schema: &DFSchema) -> Option<Arc<dyn PhysicalExpr>> {
    match expr {
        Expr::Alias(alias) => logical_expr_to_physical(&alias.expr, schema),
        Expr::Column(col) => {
            let idx = schema.index_of_column(col).ok()?;
            let name = schema.field(idx).name();
            Some(Arc::new(PhysicalColumn::new(name, idx)))
        }
        Expr::Literal(scalar, _) => Some(Arc::new(Literal::new(scalar.clone()))),
        Expr::Negative(inner) => {
            let right = logical_expr_to_physical(inner, schema)?;
            let zero = Arc::new(Literal::new(ScalarValue::Float64(Some(0.0))));
            Some(Arc::new(PhysicalBinaryExpr::new(
                zero,
                Operator::Minus,
                right,
            )))
        }
        Expr::BinaryExpr(bin) => {
            let left = logical_expr_to_physical(&bin.left, schema)?;
            let right = logical_expr_to_physical(&bin.right, schema)?;
            Some(Arc::new(PhysicalBinaryExpr::new(left, bin.op, right)))
        }
        Expr::Cast(cast) => {
            let inner = logical_expr_to_physical(&cast.expr, schema)?;
            Some(Arc::new(CastExpr::new(
                inner,
                cast.field.data_type().clone(),
                None,
            )))
        }
        Expr::TryCast(try_cast) => {
            let inner = logical_expr_to_physical(&try_cast.expr, schema)?;
            Some(Arc::new(TryCastExpr::new(
                inner,
                try_cast.field.data_type().clone(),
            )))
        }
        Expr::ScalarFunction(sf) => {
            let args: Option<Vec<Arc<dyn PhysicalExpr>>> = sf
                .args
                .iter()
                .map(|a| logical_expr_to_physical(a, schema))
                .collect();
            let args = args?;
            let return_field = Arc::new(Field::new(sf.func.name(), DataType::Float64, true));
            Some(Arc::new(ScalarFunctionExpr::new(
                sf.func.name(),
                Arc::clone(&sf.func),
                args,
                return_field,
                Arc::new(ConfigOptions::default()),
            )))
        }
        _ => None,
    }
}

/// Extracts all leaf `SearchPredicateUdf`s from `expr` if and only if `expr` is a pure
/// disjunction (`OR` tree) of `SearchPredicateUdf` calls.
fn extract_pure_or_search_predicates(expr: &Expr, out: &mut Vec<SearchPredicateUdf>) -> bool {
    match expr {
        Expr::Alias(alias) => extract_pure_or_search_predicates(&alias.expr, out),
        Expr::BinaryExpr(LogicalBinaryExpr {
            left,
            op: Operator::Or,
            right,
        }) => {
            extract_pure_or_search_predicates(left, out)
                && extract_pure_or_search_predicates(right, out)
        }
        _ => {
            if let Some(udf) = as_search_predicate_udf(expr) {
                out.push(udf.clone());
                true
            } else {
                false
            }
        }
    }
}

fn is_elided_join_filter(expr: &Expr, elided_positions: &BTreeSet<usize>) -> bool {
    if elided_positions.is_empty() {
        return false;
    }
    let mut udfs = Vec::new();
    extract_pure_or_search_predicates(expr, &mut udfs)
        && !udfs.is_empty()
        && udfs
            .iter()
            .all(|u| elided_positions.contains(&u.plan_position))
}

fn lookup_tag_name<'a>(
    udf: &SearchPredicateUdf,
    tags_by_position: &'a BTreeMap<usize, Vec<TaggedQuery>>,
) -> Result<&'a str> {
    tags_by_position
        .get(&udf.plan_position)
        .and_then(|list| list.iter().find(|tq| tq.query == udf.query))
        .map(|tq| tq.tag_name.as_str())
        .ok_or_else(|| {
            DataFusionError::Internal(format!(
                "JoinSearchPredicateRule: missing tag for plan_position {}",
                udf.plan_position
            ))
        })
}

/// Collects the tag column names referenced by any non-elided `SearchPredicateUdf` in `node`'s own expressions.
fn collect_node_referenced_tags(
    node: &LogicalPlan,
    tags_by_position: &BTreeMap<usize, Vec<TaggedQuery>>,
    elided_positions: &BTreeSet<usize>,
    out: &mut BTreeSet<String>,
) -> Result<()> {
    node.apply_expressions(|expr| {
        expr.apply(|subexpr| {
            if let Some(udf) = as_search_predicate_udf(subexpr)
                && !elided_positions.contains(&udf.plan_position)
            {
                let tag_name = lookup_tag_name(udf, tags_by_position)?;
                out.insert(tag_name.to_string());
            }
            Ok(TreeNodeRecursion::Continue)
        })
    })?;
    Ok(())
}

/// Rewrites any `SearchPredicateUdf` subexpressions in `expr` to `Expr::Column(tag_name)`.
fn rewrite_search_predicate_expr(
    expr: Expr,
    tags_by_position: &BTreeMap<usize, Vec<TaggedQuery>>,
) -> Result<Transformed<Expr>> {
    expr.transform_up(|subexpr| {
        if let Some(udf) = as_search_predicate_udf(&subexpr) {
            let tag_name = lookup_tag_name(udf, tags_by_position)?;
            return Ok(Transformed::yes(Expr::Column(Column::from_name(tag_name))));
        }
        Ok(Transformed::no(subexpr))
    })
}

fn resolve_scan_plan_position(scan: &TableScan) -> Option<usize> {
    let provider = pg_search_provider_from_scan(scan)?;
    if let Some(pos) = provider.configured_deferred_ctid_plan_position() {
        return Some(pos);
    }
    if let Some(pos) = provider.source_idx() {
        return Some(pos);
    }
    // Fallback: execution alias format `__{alias}_{plan_position}`
    scan.table_name
        .table()
        .rsplit_once('_')
        .and_then(|(_, suffix)| suffix.parse::<usize>().ok())
}

fn apply_tagging_to_scan(
    mut scan: TableScan,
    tags_by_position: &BTreeMap<usize, Vec<TaggedQuery>>,
    elided_positions: &BTreeSet<usize>,
    tags_needed_above: &BTreeSet<String>,
) -> Result<Transformed<LogicalPlan>> {
    let Some(provider) = pg_search_provider_from_scan(&scan) else {
        return Ok(Transformed::no(LogicalPlan::TableScan(scan)));
    };
    let Some(plan_pos) = resolve_scan_plan_position(&scan) else {
        return Ok(Transformed::no(LogicalPlan::TableScan(scan)));
    };

    let local_queries = tags_by_position.get(&plan_pos).cloned().unwrap_or_default();
    let is_elided = elided_positions.contains(&plan_pos);

    let mut new_provider = provider.clone();
    let base_query = new_provider.scan_info.mode.query().clone();
    if !local_queries.is_empty() {
        new_provider.scan_info.has_search_predicate = true;
    }
    if is_elided {
        let disjunct_queries = local_queries.iter().map(|tq| (*tq.query).clone()).collect();
        new_provider.scan_info.mode = ScanMode::disjunctive(base_query, disjunct_queries);
        scan.source =
            Arc::new(DefaultTableSource::new(Arc::new(new_provider))) as Arc<dyn TableSource>;
        return Ok(Transformed::yes(LogicalPlan::TableScan(scan)));
    }

    for tq in &local_queries {
        new_provider.add_match_tag_column(&tq.tag_name);
    }
    new_provider.scan_info.mode = ScanMode::tagged(base_query, local_queries.clone());

    let new_source =
        Arc::new(DefaultTableSource::new(Arc::new(new_provider))) as Arc<dyn TableSource>;

    let source_schema = new_source.schema();

    // Project any tag columns needed by filters above this scan.
    let mut projected_indices: Vec<usize> = match &scan.projection {
        Some(indices) => indices.clone(),
        None => (0..scan.projected_schema.fields().len()).collect(),
    };
    for tq in &local_queries {
        if tags_needed_above.contains(&tq.tag_name)
            && scan
                .projected_schema
                .index_of_column_by_name(None, &tq.tag_name)
                .is_none()
            && let Ok(tag_idx) = source_schema.index_of(&tq.tag_name)
        {
            projected_indices.push(tag_idx);
        }
    }

    let projected_arrow_schema = source_schema.project(&projected_indices)?;
    let mut new_qualified_fields = Vec::with_capacity(projected_arrow_schema.fields().len());
    for (i, field) in projected_arrow_schema.fields().iter().enumerate() {
        let qualifier = if i < scan.projected_schema.fields().len() {
            let (q, _) = scan.projected_schema.qualified_field(i);
            q.cloned()
        } else if !scan.projected_schema.fields().is_empty() {
            let (q, _) = scan.projected_schema.qualified_field(0);
            q.cloned()
        } else {
            Some(scan.table_name.clone())
        };
        new_qualified_fields.push((qualifier, Arc::clone(field)));
    }

    scan.source = new_source;
    if scan.projection.is_some() || projected_indices.len() != source_schema.fields().len() {
        scan.projection = Some(projected_indices);
    }
    scan.projected_schema = Arc::new(DFSchema::new_with_metadata(
        new_qualified_fields,
        scan.projected_schema.metadata().clone(),
    )?);

    Ok(Transformed::yes(LogicalPlan::TableScan(scan)))
}

fn rewrite_plan(
    plan: LogicalPlan,
    tags_by_position: &BTreeMap<usize, Vec<TaggedQuery>>,
    elided_positions: &BTreeSet<usize>,
    tags_needed_above: &BTreeSet<String>,
) -> Result<Transformed<LogicalPlan>> {
    if let LogicalPlan::TableScan(scan) = plan {
        return apply_tagging_to_scan(scan, tags_by_position, elided_positions, tags_needed_above);
    }

    let mut children_needed = tags_needed_above.clone();
    collect_node_referenced_tags(
        &plan,
        tags_by_position,
        elided_positions,
        &mut children_needed,
    )?;

    let children: Vec<LogicalPlan> = plan.inputs().into_iter().cloned().collect();
    let mut new_children = Vec::with_capacity(children.len());
    let mut any_child_modified = false;
    for child in children {
        let res = rewrite_plan(child, tags_by_position, elided_positions, &children_needed)?;
        any_child_modified |= res.transformed;
        new_children.push(res.data);
    }

    match plan {
        LogicalPlan::Projection(proj) => {
            let input = new_children.into_iter().next().unwrap();
            let mut new_exprs = Vec::with_capacity(proj.expr.len());
            let mut expr_modified = false;
            for expr in proj.expr {
                let res = rewrite_search_predicate_expr(expr, tags_by_position)?;
                expr_modified |= res.transformed;
                new_exprs.push(res.data);
            }

            // Forward any tag columns produced by `input` that are required by an ancestor
            // Filter or Join above this Projection.
            let mut added_tag = false;
            if !tags_needed_above.is_empty() {
                for (i, field) in input.schema().fields().iter().enumerate() {
                    if tags_needed_above.contains(field.name())
                        && proj
                            .schema
                            .index_of_column_by_name(None, field.name())
                            .is_none()
                    {
                        let (qualifier, _) = input.schema().qualified_field(i);
                        new_exprs.push(Expr::Column(Column::new(
                            qualifier.cloned(),
                            field.name().clone(),
                        )));
                        added_tag = true;
                    }
                }
            }

            let new_proj = Projection::try_new(new_exprs, Arc::new(input))?;
            if any_child_modified || expr_modified || added_tag {
                Ok(Transformed::yes(LogicalPlan::Projection(new_proj)))
            } else {
                Ok(Transformed::no(LogicalPlan::Projection(new_proj)))
            }
        }
        LogicalPlan::SubqueryAlias(alias) => {
            let input = new_children.into_iter().next().unwrap();
            if any_child_modified {
                let new_alias = SubqueryAlias::try_new(Arc::new(input), alias.alias)?;
                Ok(Transformed::yes(LogicalPlan::SubqueryAlias(new_alias)))
            } else {
                Ok(Transformed::no(LogicalPlan::SubqueryAlias(alias)))
            }
        }
        LogicalPlan::Filter(filter) => {
            let input = new_children.into_iter().next().unwrap();
            let pred_res = rewrite_search_predicate_expr(filter.predicate, tags_by_position)?;
            if any_child_modified || pred_res.transformed {
                let new_filter = Filter::try_new(pred_res.data, Arc::new(input))?;
                Ok(Transformed::yes(LogicalPlan::Filter(new_filter)))
            } else {
                Ok(Transformed::no(LogicalPlan::Filter(Filter::try_new(
                    pred_res.data,
                    Arc::new(input),
                )?)))
            }
        }
        LogicalPlan::Join(join) => {
            let mut iter = new_children.into_iter();
            let left = iter.next().unwrap();
            let right = iter.next().unwrap();
            let mut filter_modified = false;
            let new_filter = match join.filter {
                Some(f) if is_elided_join_filter(&f, elided_positions) => {
                    filter_modified = true;
                    None
                }
                Some(f) => {
                    let res = rewrite_search_predicate_expr(f, tags_by_position)?;
                    filter_modified = res.transformed;
                    Some(res.data)
                }
                None => None,
            };
            if any_child_modified || filter_modified {
                let new_join = Join::try_new(
                    Arc::new(left),
                    Arc::new(right),
                    join.on,
                    new_filter,
                    join.join_type,
                    join.join_constraint,
                    join.null_equality,
                    join.null_aware,
                )?;
                Ok(Transformed::yes(LogicalPlan::Join(new_join)))
            } else {
                Ok(Transformed::no(LogicalPlan::Join(Join::try_new(
                    Arc::new(left),
                    Arc::new(right),
                    join.on,
                    new_filter,
                    join.join_type,
                    join.join_constraint,
                    join.null_equality,
                    join.null_aware,
                )?)))
            }
        }
        other => {
            let mut expr_modified = false;
            let mut new_exprs = Vec::new();
            for expr in other.expressions() {
                let res = rewrite_search_predicate_expr(expr, tags_by_position)?;
                expr_modified |= res.transformed;
                new_exprs.push(res.data);
            }
            if any_child_modified || expr_modified {
                let rebuilt = other
                    .with_new_exprs(new_exprs, new_children)?
                    .recompute_schema()?;
                Ok(Transformed::yes(rebuilt))
            } else {
                Ok(Transformed::no(other))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::fast_fields_helper::WhichFastField;
    use crate::scan::codec::{deserialize_logical_plan_for_explain, serialize_logical_plan};
    use crate::scan::info::ScanInfo;
    use crate::scan::table_provider::PgSearchTableProvider;
    use crate::schema::SearchFieldType;
    use datafusion::logical_expr::{JoinConstraint, JoinType, LogicalPlanBuilder, col};
    use datafusion::optimizer::OptimizerContext;
    use datafusion::prelude::SessionContext;
    use pgrx::pg_sys;

    fn test_scan_provider(plan_position: usize, alias: &str) -> Arc<PgSearchTableProvider> {
        let scan_info = ScanInfo::new(
            (plan_position + 1) as u32,
            pg_sys::Oid::from_u32(1000 + plan_position as u32),
            pg_sys::Oid::from_u32(2000 + plan_position as u32),
            ScanMode::standard(SearchQueryInput::All),
        )
        .with_alias(alias);

        let fields = vec![
            WhichFastField::DeferredCtid(format!("ctid_{plan_position}")),
            WhichFastField::eager("id", SearchFieldType::I64(pg_sys::INT8OID)),
        ];

        Arc::new(PgSearchTableProvider::new(
            scan_info,
            fields,
            Some(plan_position),
        ))
    }

    fn make_udf_expr(plan_position: usize, alias: &str, query: SearchQueryInput) -> Expr {
        let exec_alias = format!("__{alias}_{plan_position}");
        let udf = ScalarUDF::new_from_impl(SearchPredicateUdf::new(
            plan_position,
            exec_alias.clone(),
            Box::new(query),
        ));
        udf.call(vec![Expr::Column(Column::new(
            Some(exec_alias),
            format!("ctid_{plan_position}"),
        ))])
    }

    #[test]
    fn search_predicate_udf_roundtrips_through_logical_plan_codec() {
        let provider = test_scan_provider(0, "p");
        let pred = make_udf_expr(0, "p", SearchQueryInput::All);
        let plan =
            LogicalPlanBuilder::scan("__p_0", Arc::new(DefaultTableSource::new(provider)), None)
                .unwrap()
                .filter(pred)
                .unwrap()
                .build()
                .unwrap();

        let bytes = serialize_logical_plan(&plan).unwrap();
        let ctx = SessionContext::new();
        let decoded =
            deserialize_logical_plan_for_explain(&bytes, &ctx.task_ctx(), std::ptr::null_mut())
                .unwrap();
        assert_eq!(collect_search_predicates(&decoded).len(), 1);
    }

    #[test]
    fn rule_rewrites_predicates_to_tags_and_updates_providers() {
        let left_provider = test_scan_provider(0, "p");
        let right_provider = test_scan_provider(1, "s");

        let left_scan = LogicalPlanBuilder::scan(
            "__p_0",
            Arc::new(DefaultTableSource::new(left_provider)),
            None,
        )
        .unwrap()
        .project(vec![col("__p_0.ctid_0"), col("__p_0.id")])
        .unwrap()
        .alias("__p_0")
        .unwrap()
        .build()
        .unwrap();

        let right_scan = LogicalPlanBuilder::scan(
            "__s_1",
            Arc::new(DefaultTableSource::new(right_provider)),
            None,
        )
        .unwrap()
        .project(vec![col("__s_1.ctid_1"), col("__s_1.id")])
        .unwrap()
        .alias("__s_1")
        .unwrap()
        .build()
        .unwrap();

        let pred_left = make_udf_expr(0, "p", SearchQueryInput::All);
        let pred_right = make_udf_expr(
            1,
            "s",
            SearchQueryInput::Boolean {
                must: vec![SearchQueryInput::All],
                should: vec![],
                must_not: vec![],
                minimum_should_match: None,
            },
        );
        let join_filter = pred_left.or(pred_right);

        let join_plan = LogicalPlan::Join(
            datafusion::logical_expr::logical_plan::Join::try_new(
                Arc::new(left_scan),
                Arc::new(right_scan),
                vec![(col("__p_0.id"), col("__s_1.id"))],
                Some(join_filter),
                JoinType::Inner,
                JoinConstraint::On,
                datafusion::common::NullEquality::NullEqualsNothing,
                false,
            )
            .unwrap(),
        );

        let root_plan = LogicalPlanBuilder::from(join_plan)
            .project(vec![col("__p_0.id"), col("__s_1.id")])
            .unwrap()
            .build()
            .unwrap();

        let rule = JoinSearchPredicateRule::new();
        let rewritten = rule
            .rewrite(root_plan.clone(), &OptimizerContext::new())
            .unwrap();
        assert!(rewritten.transformed);
        assert_eq!(
            rewritten.data.schema().fields().len(),
            root_plan.schema().fields().len()
        );

        // No SearchPredicateUdf should remain in the rewritten plan.
        assert!(collect_search_predicates(&rewritten.data).is_empty());

        // Both leaf TableScans should now have ScanMode::Tagged with 1 local query.
        let mut tagged_count = 0;
        let _ = rewritten.data.apply(|node| {
            if let LogicalPlan::TableScan(scan) = node
                && let Some(provider) = pg_search_provider_from_scan(scan)
                && let ScanMode::Tagged { local_queries, .. } = &provider.scan_info.mode
            {
                assert_eq!(local_queries.len(), 1);
                tagged_count += 1;
            }
            Ok(TreeNodeRecursion::Continue)
        });
        assert_eq!(tagged_count, 2);
    }

    #[test]
    fn distinct_predicates_on_same_table_are_not_conflated_by_cse() {
        let left_provider = test_scan_provider(0, "p");
        let right_provider = test_scan_provider(1, "s");

        let left_scan = LogicalPlanBuilder::scan(
            "__p_0",
            Arc::new(DefaultTableSource::new(left_provider)),
            None,
        )
        .unwrap()
        .project(vec![col("__p_0.ctid_0"), col("__p_0.id")])
        .unwrap()
        .alias("__p_0")
        .unwrap()
        .build()
        .unwrap();

        let right_scan = LogicalPlanBuilder::scan(
            "__s_1",
            Arc::new(DefaultTableSource::new(right_provider)),
            None,
        )
        .unwrap()
        .project(vec![col("__s_1.ctid_1"), col("__s_1.id")])
        .unwrap()
        .alias("__s_1")
        .unwrap()
        .build()
        .unwrap();

        let p_pred_0 = make_udf_expr(0, "p", SearchQueryInput::All);
        let p_pred_1 = make_udf_expr(
            0,
            "p",
            SearchQueryInput::Boolean {
                must: vec![SearchQueryInput::All],
                should: vec![],
                must_not: vec![SearchQueryInput::All],
                minimum_should_match: None,
            },
        );
        let s_pred_0 = make_udf_expr(
            1,
            "s",
            SearchQueryInput::Boolean {
                must: vec![SearchQueryInput::All],
                should: vec![],
                must_not: vec![],
                minimum_should_match: None,
            },
        );

        // `(s_pred_0 AND p_pred_0) OR p_pred_1` triggers `PushDownFilter`'s
        // `extract_or_clauses_for_join` to push `Filter(p_pred_0 OR p_pred_1)` to `p`.
        let join_plan = LogicalPlanBuilder::from(left_scan)
            .join_on(
                right_scan,
                JoinType::Inner,
                vec![
                    col("__p_0.id").eq(col("__s_1.id")),
                    s_pred_0.and(p_pred_0).or(p_pred_1),
                ],
            )
            .unwrap()
            .project(vec![col("__p_0.id"), col("__s_1.id")])
            .unwrap()
            .build()
            .unwrap();

        let mut rules = datafusion::optimizer::optimizer::Optimizer::default().rules;
        let pos = rules
            .iter()
            .position(|r| r.name() == "optimize_projections")
            .unwrap();
        rules.insert(pos, Arc::new(JoinSearchPredicateRule::new()));
        let optimizer = datafusion::optimizer::optimizer::Optimizer::with_rules(rules);

        let optimized = optimizer
            .optimize(join_plan, &OptimizerContext::new(), |_, _| {})
            .unwrap();

        let mut filter_preds = Vec::new();
        let _ = optimized.apply(|node| {
            if let LogicalPlan::Filter(f) = node {
                filter_preds.push(f.predicate.to_string());
            }
            Ok(TreeNodeRecursion::Continue)
        });
        assert_eq!(filter_preds, vec!["__p_0_tag_0 OR __p_0_tag_1"]);
    }

    #[test]
    fn rank_join_elides_tags_and_join_filter_and_falls_back_when_ineligible() {
        let make_scored_provider = |plan_position: usize, alias: &str, rows: u64| {
            let mut scan_info = ScanInfo::new(
                (plan_position + 1) as u32,
                pg_sys::Oid::from_u32(1000 + plan_position as u32),
                pg_sys::Oid::from_u32(2000 + plan_position as u32),
                ScanMode::standard(SearchQueryInput::All),
            )
            .with_alias(alias);
            scan_info.estimate = crate::scan::info::RowEstimate::Known(rows);

            let fields = vec![
                WhichFastField::DeferredCtid(format!("ctid_{plan_position}")),
                WhichFastField::eager("id", SearchFieldType::I64(pg_sys::INT8OID)),
                WhichFastField::Score(Some(format!("__pdb_score_{plan_position}"))),
            ];

            Arc::new(PgSearchTableProvider::new(
                scan_info,
                fields,
                Some(plan_position),
            ))
        };

        let build_disjunctive_top_k_plan =
            |left_q: SearchQueryInput, right_q: SearchQueryInput, rows: u64, fetch: usize| {
                let left_provider = make_scored_provider(0, "p", rows);
                let right_provider = make_scored_provider(1, "s", rows);

                let left_scan = LogicalPlanBuilder::scan(
                    "__p_0",
                    Arc::new(DefaultTableSource::new(left_provider)),
                    None,
                )
                .unwrap()
                .build()
                .unwrap();

                let right_scan = LogicalPlanBuilder::scan(
                    "__s_1",
                    Arc::new(DefaultTableSource::new(right_provider)),
                    None,
                )
                .unwrap()
                .build()
                .unwrap();

                let pred_left = make_udf_expr(0, "p", left_q);
                let pred_right = make_udf_expr(1, "s", right_q);
                let join_plan = LogicalPlan::Join(
                    datafusion::logical_expr::logical_plan::Join::try_new(
                        Arc::new(left_scan),
                        Arc::new(right_scan),
                        vec![(col("__p_0.id"), col("__s_1.id"))],
                        Some(pred_left.or(pred_right)),
                        JoinType::Inner,
                        JoinConstraint::On,
                        datafusion::common::NullEquality::NullEqualsNothing,
                        false,
                    )
                    .unwrap(),
                );

                let score_sum = col("__p_0.__pdb_score_0") + col("__s_1.__pdb_score_1");
                LogicalPlanBuilder::from(join_plan)
                    .project(vec![
                        col("__p_0.id"),
                        col("__s_1.id"),
                        score_sum.alias("total_score"),
                    ])
                    .unwrap()
                    .sort_with_limit(vec![col("total_score").sort(false, false)], Some(fetch))
                    .unwrap()
                    .build()
                    .unwrap()
            };

        let bm25_left = SearchQueryInput::Parse {
            query_string: "running".into(),
            lenient: None,
            conjunction_mode: None,
        };
        let bm25_right = SearchQueryInput::Parse {
            query_string: "shoes".into(),
            lenient: None,
            conjunction_mode: None,
        };

        // Case 1: Eligible RankJoin (BM25 queries, 50k rows >= 10k min_rows, fetch = 10 <= 1000).
        // Tags and `Join.filter` should be elided, while `ScanMode::Disjunctive.disjunct_queries` is populated.
        let eligible_plan =
            build_disjunctive_top_k_plan(bm25_left.clone(), bm25_right.clone(), 50_000, 10);
        let rule = JoinSearchPredicateRule::new_with_min_rows(10_000);
        let rewritten = rule
            .rewrite(eligible_plan, &OptimizerContext::new())
            .unwrap()
            .data;

        let mut join_filters = Vec::new();
        let mut scan_tag_cols = Vec::new();
        let mut disjunct_query_counts = Vec::new();
        let _ = rewritten.apply(|node| {
            match node {
                LogicalPlan::Join(j) => join_filters.push(j.filter.clone()),
                LogicalPlan::TableScan(scan) => {
                    let has_tag_col = scan
                        .projected_schema
                        .fields()
                        .iter()
                        .any(|f| f.name().contains("_tag_"));
                    scan_tag_cols.push(has_tag_col);
                    if let Some(provider) = pg_search_provider_from_scan(scan)
                        && let ScanMode::Disjunctive {
                            disjunct_queries, ..
                        } = &provider.scan_info.mode
                    {
                        disjunct_query_counts.push(disjunct_queries.len());
                    }
                }
                _ => {}
            }
            Ok(TreeNodeRecursion::Continue)
        });
        assert_eq!(join_filters, vec![None]);
        assert_eq!(scan_tag_cols, vec![false, false]);
        assert_eq!(disjunct_query_counts, vec![1, 1]);

        // Case 2: Ineligible due to row estimate below `min_rows` (1k < 10k) -> falls back to tagging.
        let small_plan = build_disjunctive_top_k_plan(bm25_left, bm25_right, 1_000, 10);
        let small_rewritten = rule
            .rewrite(small_plan, &OptimizerContext::new())
            .unwrap()
            .data;
        let mut small_join_filters = Vec::new();
        let mut small_scan_tag_cols = Vec::new();
        let _ = small_rewritten.apply(|node| {
            match node {
                LogicalPlan::Join(j) => {
                    small_join_filters.push(j.filter.as_ref().map(|e| e.to_string()));
                }
                LogicalPlan::TableScan(scan) => {
                    let has_tag_col = scan
                        .projected_schema
                        .fields()
                        .iter()
                        .any(|f| f.name().contains("_tag_"));
                    small_scan_tag_cols.push(has_tag_col);
                }
                _ => {}
            }
            Ok(TreeNodeRecursion::Continue)
        });
        assert_eq!(
            small_join_filters,
            vec![Some("__p_0.__p_0_tag_0 OR __s_1.__s_1_tag_0".to_string())]
        );
        assert_eq!(small_scan_tag_cols, vec![true, true]);
    }
}
