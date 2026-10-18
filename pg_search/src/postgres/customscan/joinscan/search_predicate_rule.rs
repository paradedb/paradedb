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
use arrow_schema::DataType;
use datafusion::catalog::default_table_source::DefaultTableSource;
use datafusion::common::tree_node::{Transformed, TreeNode, TreeNodeRecursion};
use datafusion::common::{Column, DFSchema, DataFusionError, Result, ScalarValue};
use datafusion::logical_expr::expr::ScalarFunction;
use datafusion::logical_expr::{
    ColumnarValue, Expr, Filter, LogicalPlan, Projection, ScalarFunctionArgs, ScalarUDF,
    ScalarUDFImpl, Signature, SubqueryAlias, TableScan, TableSource, Volatility,
};
use datafusion::optimizer::optimizer::ApplyOrder;
use datafusion::optimizer::{OptimizerConfig, OptimizerRule};

use serde::{Deserialize, Serialize};

use crate::postgres::customscan::datafusion::translator::make_source_col;
use crate::postgres::customscan::joinscan::CtidColumn;
use crate::postgres::customscan::joinscan::build::{JoinSource, RelationAlias};
use crate::query::SearchQueryInput;
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
pub struct JoinSearchPredicateRule;

impl JoinSearchPredicateRule {
    pub fn new() -> Self {
        Self
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

        let tags_by_position = build_tagged_queries(collected);
        let original_schema = Arc::clone(plan.schema());
        let rewritten = rewrite_plan(plan, &tags_by_position, &BTreeSet::new())?;
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

/// Collects the tag column names referenced by any `SearchPredicateUdf` in `node`'s own expressions.
fn collect_node_referenced_tags(
    node: &LogicalPlan,
    tags_by_position: &BTreeMap<usize, Vec<TaggedQuery>>,
    out: &mut BTreeSet<String>,
) -> Result<()> {
    node.apply_expressions(|expr| {
        expr.apply(|subexpr| {
            if let Some(udf) = as_search_predicate_udf(subexpr) {
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
    tags_needed_above: &BTreeSet<String>,
) -> Result<Transformed<LogicalPlan>> {
    let Some(provider) = pg_search_provider_from_scan(&scan) else {
        return Ok(Transformed::no(LogicalPlan::TableScan(scan)));
    };
    let Some(plan_pos) = resolve_scan_plan_position(&scan) else {
        return Ok(Transformed::no(LogicalPlan::TableScan(scan)));
    };

    let local_queries = tags_by_position.get(&plan_pos).cloned().unwrap_or_default();

    let mut new_provider = provider.clone();
    let base_query = new_provider.scan_info.mode.query().clone();
    if !local_queries.is_empty() {
        new_provider.scan_info.has_search_predicate = true;
        for tq in &local_queries {
            new_provider.add_match_tag_column(&tq.tag_name);
        }
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
    tags_needed_above: &BTreeSet<String>,
) -> Result<Transformed<LogicalPlan>> {
    if let LogicalPlan::TableScan(scan) = plan {
        return apply_tagging_to_scan(scan, tags_by_position, tags_needed_above);
    }

    let mut children_needed = tags_needed_above.clone();
    collect_node_referenced_tags(&plan, tags_by_position, &mut children_needed)?;

    let children: Vec<LogicalPlan> = plan.inputs().into_iter().cloned().collect();
    let mut new_children = Vec::with_capacity(children.len());
    let mut any_child_modified = false;
    for child in children {
        let res = rewrite_plan(child, tags_by_position, &children_needed)?;
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
                Some(f) => {
                    let res = rewrite_search_predicate_expr(f, tags_by_position)?;
                    filter_modified = res.transformed;
                    Some(res.data)
                }
                None => None,
            };
            if any_child_modified || filter_modified {
                let new_join = datafusion::logical_expr::logical_plan::Join::try_new(
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
                Ok(Transformed::no(LogicalPlan::Join(
                    datafusion::logical_expr::logical_plan::Join::try_new(
                        Arc::new(left),
                        Arc::new(right),
                        join.on,
                        new_filter,
                        join.join_type,
                        join.join_constraint,
                        join.null_equality,
                        join.null_aware,
                    )?,
                )))
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
}
