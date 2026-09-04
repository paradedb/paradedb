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

use datafusion::common::{Column, JoinType, NullEquality, Result, TableReference};
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{col, lit, BinaryExpr, Expr, LogicalPlanBuilder, Operator};
use datafusion::prelude::DataFrame;
use pgrx::pg_sys;

use crate::postgres::customscan::joinscan::build::{
    JoinLevelExpr, JoinNode, JoinSource, JoinType as PgJoinType, LateralUnnestInfo, RelNode,
    RelationAlias, UnnestNode,
};
use crate::postgres::customscan::joinscan::privdat::{OutputColumnInfo, SCORE_COL_NAME};
use crate::scan::ScanMode;

pub trait ColumnMapper {
    /// Map a PostgreSQL variable to a DataFusion Column expression
    fn map_var(&self, varno: pg_sys::Index, varattno: pg_sys::AttrNumber) -> Option<Expr>;
}

/// Human-readable PG type name (e.g. `"integer"`, `"jsonb"`) for debug
/// logging. Falls back to `"OID {n}"` if the OID can't be resolved.
pub(crate) unsafe fn type_name(oid: pg_sys::Oid) -> String {
    let c_str = pg_sys::format_type_be(oid);
    if c_str.is_null() {
        return format!("OID {}", oid);
    }
    std::ffi::CStr::from_ptr(c_str)
        .to_string_lossy()
        .into_owned()
}

/// Short label for a node tag (strips the `T_` prefix). Used in debug
/// logs so the offending node type is immediately scannable. Separate from
/// `expr_translators::node_tag_label`, which covers only the subset of
/// tags that reach the UDF-naming path.
pub(crate) unsafe fn node_tag_debug(node: *mut pg_sys::Node) -> String {
    if node.is_null() {
        return "null".to_string();
    }
    let dbg = format!("{:?}", (*node).type_);
    dbg.strip_prefix("T_").unwrap_or(&dbg).to_string()
}

/// Helper struct for translating PostgreSQL expression trees into DataFusion `Expr`s.
pub struct PredicateTranslator<'a> {
    pub sources: &'a [&'a JoinSource],
    pub root: Option<*mut pg_sys::PlannerInfo>,
    pub(crate) mapper: Option<Box<dyn ColumnMapper + 'a>>,
}

impl<'a> PredicateTranslator<'a> {
    pub fn new(sources: &'a [&'a JoinSource]) -> Self {
        Self {
            sources,
            root: None,
            mapper: None,
        }
    }

    pub fn with_planner_info(mut self, root: *mut pg_sys::PlannerInfo) -> Self {
        self.root = if root.is_null() { None } else { Some(root) };
        self
    }

    pub fn with_mapper(mut self, mapper: Box<dyn ColumnMapper + 'a>) -> Self {
        self.mapper = Some(mapper);
        self
    }

    /// Format a PostgreSQL expression node into a debug string for logs,
    /// using `deparse_planner_expr` if planning context is available.
    pub(crate) unsafe fn deparse_for_debug(&self, node: *mut pg_sys::Node) -> String {
        if let Some(root) = self.root {
            if let Some(deparsed) = crate::postgres::deparse::deparse_planner_expr(root, node) {
                return deparsed;
            }
        }
        crate::postgres::deparse::node_to_string_without_context(node)
    }

    /// Translate a `JoinLevelExpr` tree to a DataFusion `Expr`.
    ///
    /// Single-table search predicates map to synthetic match tag columns
    /// (`__{alias}_{idx}_tag_{local_idx}`), while multi-table expressions reference
    /// their corresponding translated custom expressions.
    pub unsafe fn translate_join_level_expr(
        expr: &JoinLevelExpr,
        custom_exprs: &[Expr],
        custom_expr_idx: &mut usize,
        sources: &[&JoinSource],
    ) -> Option<Expr> {
        match expr {
            JoinLevelExpr::SingleTablePredicate {
                plan_position,
                predicate,
            } => {
                let source = sources.iter().find(|s| s.plan_position == *plan_position)?;
                let tag_name = match &source.scan_info.mode {
                    ScanMode::Tagged { local_queries, .. } => {
                        &local_queries
                            .iter()
                            .find(|tq| tq.query.as_ref() == &predicate.query)?
                            .tag_name
                    }
                    ScanMode::Standard { .. } => return None,
                };
                Some(col(tag_name))
            }
            JoinLevelExpr::MultiTablePredicate { .. } => {
                let expr = custom_exprs.get(*custom_expr_idx).cloned();
                *custom_expr_idx += 1;
                expr
            }
            JoinLevelExpr::And(children) => {
                if children.is_empty() {
                    return None;
                }
                let mut result = Self::translate_join_level_expr(
                    &children[0],
                    custom_exprs,
                    custom_expr_idx,
                    sources,
                )?;
                for child in &children[1..] {
                    let right = Self::translate_join_level_expr(
                        child,
                        custom_exprs,
                        custom_expr_idx,
                        sources,
                    )?;
                    result = Expr::BinaryExpr(BinaryExpr::new(
                        Box::new(result),
                        Operator::And,
                        Box::new(right),
                    ));
                }
                Some(result)
            }
            JoinLevelExpr::Or(children) => {
                if children.is_empty() {
                    return None;
                }
                let mut result = Self::translate_join_level_expr(
                    &children[0],
                    custom_exprs,
                    custom_expr_idx,
                    sources,
                )?;
                for child in &children[1..] {
                    let right = Self::translate_join_level_expr(
                        child,
                        custom_exprs,
                        custom_expr_idx,
                        sources,
                    )?;
                    result = Expr::BinaryExpr(BinaryExpr::new(
                        Box::new(result),
                        Operator::Or,
                        Box::new(right),
                    ));
                }
                Some(result)
            }
            JoinLevelExpr::Not(child) => {
                let inner =
                    Self::translate_join_level_expr(child, custom_exprs, custom_expr_idx, sources)?;
                Some(Expr::Not(Box::new(inner)))
            }
            JoinLevelExpr::MarkOrNull {
                is_anti,
                null_test_varno,
                null_test_attno,
            } => {
                // Resolve the outer column name from source metadata.
                let source = sources.iter().find(|s| s.contains_rti(*null_test_varno));
                let col_name = source.and_then(|s| s.column_name(*null_test_attno))?;

                // Build: mark = true OR col IS NULL  (for IN)
                //        mark = false OR col IS NULL  (for NOT IN)
                let mark_check = if *is_anti {
                    col("mark").eq(lit(false))
                } else {
                    col("mark").eq(lit(true))
                };
                let null_check = Expr::IsNull(Box::new(col(col_name)));
                Some(mark_check.or(null_check))
            }
            // `PgExpression` is only produced as a top-level `JoinNode.filter`
            // for Semi/Anti joins, translated in [`build_join_df_with_filter`]
            // via `stringToNode` + [`PredicateTranslator::translate`] with a
            // `CombinedMapper` against the join's sources. It should never
            // appear inside a `RelNode::Filter` predicate tree.
            JoinLevelExpr::PgExpression { .. } => None,
        }
    }

    /// Check whether an expression can be translated to DataFusion.
    /// Validates both node-type support and column resolution against
    /// the provided sources, without requiring plan_position or
    /// output_columns.
    pub unsafe fn can_translate(
        root: Option<*mut pg_sys::PlannerInfo>,
        sources: &'a [&'a JoinSource],
        node: *mut pg_sys::Node,
        plan: Option<&'a RelNode>,
    ) -> bool {
        struct ValidationMapper<'a> {
            sources: &'a [&'a JoinSource],
            plan: Option<&'a RelNode>,
        }

        impl<'a> ColumnMapper for ValidationMapper<'a> {
            /// Just confirm the source exists — field registration hasn't
            /// happened yet at this planning stage. Column validity is
            /// covered by all_vars_are_fast_fields_recursive which runs first.
            fn map_var(&self, varno: pg_sys::Index, _varattno: pg_sys::AttrNumber) -> Option<Expr> {
                if let Some(_source) = self.sources.iter().find(|s| s.contains_rti(varno)) {
                    return Some(col("placeholder"));
                }
                if let Some(plan) = self.plan {
                    if let Some(unnest_info) = plan.find_lateral_unnest(varno) {
                        if self
                            .sources
                            .iter()
                            .any(|s| s.contains_rti(unnest_info.source_rti.0))
                        {
                            return Some(col("placeholder"));
                        }
                    }
                }
                None
            }
        }

        let mapper = ValidationMapper { sources, plan };
        let mut translator = Self::new(sources).with_mapper(Box::new(mapper));
        if let Some(r) = root {
            translator = translator.with_planner_info(r);
        }
        translator.translate(node).is_some()
    }

    /// Translate a PostgreSQL expression to a DataFusion `Expr`.
    ///
    /// Returns `None` if the expression cannot be translated.
    ///
    /// IMPORTANT: This translator is used to check if a predicate CAN be translated,
    /// but the actual predicate evaluation happens via heap fetch + PostgreSQL evaluation.
    /// Cross-type comparisons (e.g., INT < NUMERIC) involve type casts that change value
    /// semantics - we cannot simply look through them because the underlying storage
    /// representations differ (e.g., INT 95 vs Numeric64 5225 for 52.25).
    ///
    /// For predicates involving type casts, we return None to indicate that the predicate
    /// cannot be evaluated purely in DataFusion and must fall back to PostgreSQL evaluation.
    pub unsafe fn translate(&self, node: *mut pg_sys::Node) -> Option<Expr> {
        if node.is_null() {
            pgrx::debug1!("PredicateTranslator: null node pointer");
            return None;
        }

        // A List is an implicit conjunction container, so reaching here means a
        // caller skipped normalization. `translate` doubles as a capability
        // probe, so decline rather than abort an otherwise valid query.
        if (*node).type_ == pg_sys::NodeTag::T_List {
            pgrx::debug1!(
                "PredicateTranslator received an unnormalized PostgreSQL List; implicit AND conjuncts must be normalized before translation"
            );
            return None;
        }

        let native = match (*node).type_ {
            pg_sys::NodeTag::T_OpExpr => self.translate_op_expr(node as *mut pg_sys::OpExpr),
            pg_sys::NodeTag::T_Var => self.translate_var(node as *mut pg_sys::Var),
            pg_sys::NodeTag::T_Const => self.translate_const(node as *mut pg_sys::Const),
            pg_sys::NodeTag::T_BoolExpr => self.translate_bool_expr(node as *mut pg_sys::BoolExpr),
            pg_sys::NodeTag::T_RelabelType => {
                // Binary-compatible cast (e.g. varchar → text). The underlying
                // datum is identical — just recurse into the inner expression.
                let relabel = node as *mut pg_sys::RelabelType;
                self.translate((*relabel).arg.cast())
            }
            pg_sys::NodeTag::T_FuncExpr => self.translate_func_expr(node),
            pg_sys::NodeTag::T_NullTest => self.translate_null_test(node),
            pg_sys::NodeTag::T_BooleanTest => self.translate_boolean_test(node),
            pg_sys::NodeTag::T_CaseExpr => self.translate_case_expr(node),
            pg_sys::NodeTag::T_CoalesceExpr => self.translate_coalesce_expr(node),
            pg_sys::NodeTag::T_NullIfExpr => self.translate_nullif_expr(node),
            pg_sys::NodeTag::T_MinMaxExpr => self.translate_min_max_expr(node),
            pg_sys::NodeTag::T_ScalarArrayOpExpr => self.translate_scalar_array_op_expr(node),
            pg_sys::NodeTag::T_CoerceViaIO => self.translate_coerce_via_io(node),
            _ => None,
        };

        // Fallback runs per recursive call, so the innermost failing subtree
        // gets wrapped, and its parent can still evaluate natively.
        native.or_else(|| {
            let wrapped = self.try_wrap_as_udf(node);
            if wrapped.is_none() {
                pgrx::debug1!(
                    "PredicateTranslator: UDF fallback failed [{}] | {}",
                    node_tag_debug(node),
                    self.deparse_for_debug(node)
                );
            }
            wrapped
        })
    }

    // `translate_op_expr`, `translate_var`, `translate_const`,
    // `translate_bool_expr` — plus the extended node-type translators and
    // the UDF fallback — are defined in `expr_translators.rs` on a split
    // `impl PredicateTranslator` block.
}

/// Translate a `JoinLevelExpr` predicate into a DataFusion filter expression
/// and apply it to `df`. This is the shared spine of the `RelNode::Filter`
/// arm in JoinScan's and AggregateScan's `build_relnode_df` recursions —
/// both modules previously inlined the same translate-and-filter sequence.
///
/// `handle_mark` controls JoinScan-specific cleanup: when `true`, a
/// `MarkOrNull` predicate triggers a post-filter projection that drops the
/// synthetic `mark` column from the schema. AggregateScan never produces
/// `MarkOrNull` predicates and passes `false`.
pub fn apply_join_level_filter(
    mut df: DataFrame,
    predicate: &JoinLevelExpr,
    translated_exprs: &[Expr],
    sources: &[&JoinSource],
    handle_mark: bool,
) -> Result<DataFrame> {
    let mut custom_expr_idx = 0;
    let filter_expr = unsafe {
        PredicateTranslator::translate_join_level_expr(
            predicate,
            translated_exprs,
            &mut custom_expr_idx,
            sources,
        )
    }
    .ok_or_else(|| {
        DataFusionError::Internal(format!(
            "Failed to translate join level expression tree: {:?}",
            predicate
        ))
    })?;

    df = df.filter(filter_expr)?;

    // For MarkOrNull filters in JoinScan, drop the synthetic "mark" column
    // post-filter so it doesn't leak into the projection.
    if handle_mark && matches!(predicate, JoinLevelExpr::MarkOrNull { .. }) {
        let schema = df.schema().clone();
        let proj_cols: Vec<Expr> = schema
            .columns()
            .into_iter()
            .filter(|c| c.name != "mark")
            .map(col)
            .collect();
        if !proj_cols.is_empty() {
            df = df.select(proj_cols)?;
        }
    }

    Ok(df)
}

/// Creates a DataFusion column expression with a bare table reference.
/// This is preferred over `datafusion::logical_expr::col()` because `col()` parses the input string,
pub fn make_col(relation: &str, name: &str) -> Expr {
    Expr::Column(Column::new(
        Some(TableReference::Bare {
            table: relation.into(),
        }),
        name,
    ))
}

/// Lower a `JoinNode` over already-built left/right `DataFrame`s.
///
/// Builds equi-join keys with [`build_equi_join_exprs`], maps the
/// `super::build::JoinType` into DataFusion's `JoinType`, and dispatches
/// to `join_on` (when there are equi keys) or `join` (cross join). Caller
/// is responsible for any post-join filter handling - `JoinNode::filter`
/// is not consulted here.
///
/// Returns `Err(NotImplemented)` if the join type is unsupported.
pub fn build_join_df(left: DataFrame, right: DataFrame, join: &JoinNode) -> Result<DataFrame> {
    let on = build_equi_join_exprs(join)?;
    let df_join_type = map_join_type(join.join_type)?;

    if matches!(join.join_type, PgJoinType::Anti { null_aware: true }) {
        return build_null_aware_anti_join(left, right, &on, df_join_type);
    }

    if on.is_empty() {
        left.join(right, df_join_type, &[], &[], None)
    } else {
        left.join_on(right, df_join_type, on)
    }
}

/// Construct a `LeftAnti` join with `null_aware=true` so SQL `NOT IN`
/// three-valued NULL semantics are preserved. DataFusion's null-aware mode
/// requires single-column equi-key + LeftAnti; we pass the single equi
/// predicate as a join filter and rely on `ExtractEquijoinPredicate`
/// (`datafusion-optimizer/src/extract_equijoin_predicate.rs`) to pull it
/// into the on-keys list at logical-plan-build time. The unit test
/// `null_aware_anti_lifts_to_hash_join_with_null_equals_nothing` guards
/// against regressions in that pipeline.
fn build_null_aware_anti_join(
    left: DataFrame,
    right: DataFrame,
    on: &[Expr],
    df_join_type: JoinType,
) -> Result<DataFrame> {
    // The (df_join_type == LeftAnti) invariant is enforced structurally by
    // `JoinType::Anti { null_aware: true }` only being lowered by `map_join_type`
    // to `JoinType::LeftAnti`. The single-key invariant is still a runtime
    // check because `equi_keys.len() == 1` is only enforced by
    // `wrap_with_semi_anti`'s lift-time guard, not by the type system.
    debug_assert_eq!(df_join_type, JoinType::LeftAnti);
    if on.len() != 1 {
        return Err(DataFusionError::NotImplemented(format!(
            "null_aware NOT IN supports a single equi-key only, got {}",
            on.len()
        )));
    }
    let filter = Some(on[0].clone());

    let (session_state, left_plan) = left.into_parts();
    let right_plan = right.into_unoptimized_plan();
    let plan = LogicalPlanBuilder::from(left_plan)
        .join_detailed_with_options(
            right_plan,
            df_join_type,
            (Vec::<Column>::new(), Vec::<Column>::new()),
            filter,
            NullEquality::NullEqualsNothing,
            true,
        )?
        .build()?;
    Ok(DataFrame::new(session_state, plan))
}

/// Map a `super::build::JoinType` to DataFusion's `JoinType`. Returns
/// `Err(NotImplemented)` for variants the translator does not lower -
/// today only `UniqueOuter` / `UniqueInner`, which are Postgres-side
/// optimizer artifacts that should never reach the DataFusion executor.
fn map_join_type(jt: PgJoinType) -> Result<JoinType> {
    match jt {
        PgJoinType::Inner => Ok(JoinType::Inner),
        PgJoinType::Left => Ok(JoinType::Left),
        PgJoinType::Right => Ok(JoinType::Right),
        PgJoinType::Full => Ok(JoinType::Full),
        PgJoinType::Semi => Ok(JoinType::LeftSemi),
        PgJoinType::Anti { .. } => Ok(JoinType::LeftAnti),
        PgJoinType::LeftMark => Ok(JoinType::LeftMark),
        PgJoinType::RightMark => Ok(JoinType::RightMark),
        PgJoinType::RightSemi => Ok(JoinType::RightSemi),
        PgJoinType::RightAnti => Ok(JoinType::RightAnti),
        PgJoinType::UniqueOuter | PgJoinType::UniqueInner => Err(DataFusionError::NotImplemented(
            format!("{jt} JOIN is not supported in DataFusion execution"),
        )),
    }
}

/// Like [`build_join_df`], but translates [`JoinNode::filter`] and attaches it
/// to the join (passed to `DataFrame::join`'s filter parameter, which
/// DataFusion lowers to `NestedLoopJoinExec` when there are no equi keys).
///
/// Required for Semi/Anti joins whose condition cannot be expressed as equi
/// keys (e.g. `a.col = b.x OR a.col = b.y`): a post-join filter would mean
/// "cross-join then filter", which drops every left row under LeftAnti
/// semantics. Placing the predicate on the join itself evaluates it per
/// (left, right) pair and preserves correctness.
///
/// The filter expression is built by deserializing the PostgreSQL expression
/// tree (stored in [`JoinLevelExpr::PgExpression`]) with `stringToNode` and
/// translating it via [`PredicateTranslator::translate`] + [`CombinedMapper`]
/// — the same pipeline used for regular join-level conditions, just without
/// the custom_exprs / setrefs round-trip that fails under Semi/Anti tlist
/// pruning.
pub fn build_join_df_with_filter(
    left: DataFrame,
    right: DataFrame,
    join: &JoinNode,
    sources: &[&JoinSource],
    output_columns: &[OutputColumnInfo],
    lateral_unnests: &[LateralUnnestInfo],
) -> Result<DataFrame> {
    let df_join_type = map_join_type(join.join_type)?;

    let filter_expr = match &join.filter {
        Some(JoinLevelExpr::PgExpression { pg_node_string, .. }) => unsafe {
            translate_pg_expression_filter(
                pg_node_string,
                sources,
                output_columns,
                lateral_unnests,
            )?
        },
        Some(other) => {
            return Err(DataFusionError::NotImplemented(format!(
                "JoinNode.filter variant {:?} not yet supported in build_join_df_with_filter",
                other
            )));
        }
        None => {
            return build_join_df(left, right, join);
        }
    };

    if matches!(join.join_type, PgJoinType::Anti { null_aware: true }) {
        return Err(DataFusionError::NotImplemented(
            "null_aware NOT IN anti join does not support non-equi join filters".into(),
        ));
    }

    if join.equi_keys.is_empty() {
        return left.join(right, df_join_type, &[], &[], Some(filter_expr));
    }

    let mut on = build_equi_join_exprs(join)?;
    on.push(filter_expr);
    left.join_on(right, df_join_type, on)
}

/// Apply a lateral unnest operation onto an existing DataFusion DataFrame.
///
/// Resolves the source table's execution alias, applies DataFusion's `unnest_columns_with_options`
/// (preserving empty arrays for LEFT joins if requested), and re-qualifies the unnested column.
pub fn apply_relnode_unnest(df: DataFrame, unnest: &UnnestNode) -> Result<DataFrame> {
    use datafusion::common::{NullHandling, UnnestOptions};

    let source = unnest
        .input
        .sources()
        .into_iter()
        .find(|s| s.contains_rti(unnest.unnest_info.source_rti.0))
        .ok_or_else(|| {
            DataFusionError::Internal(format!(
                "source RTI {} not found for lateral unnest",
                unnest.unnest_info.source_rti.0
            ))
        })?;
    let alias =
        RelationAlias::new(source.scan_info.alias.as_deref()).execution(source.plan_position);
    let unnested_col_name = format!("{}_{}", alias, unnest.unnest_info.field_name);

    let select_exprs = df
        .schema()
        .iter()
        .map(|(qualifier, field)| {
            if qualifier.as_ref().map(|q| q.to_string()) == Some(alias.clone())
                && field.name() == &unnest.unnest_info.field_name
            {
                Expr::Column(datafusion::common::Column::new(
                    qualifier.cloned(),
                    field.name(),
                ))
                .alias(&unnested_col_name)
            } else {
                Expr::Column(datafusion::common::Column::new(
                    qualifier.cloned(),
                    field.name(),
                ))
            }
        })
        .collect::<Vec<_>>();
    let df = df.select(select_exprs)?;

    let unnest_options = if unnest.unnest_info.is_left_join {
        UnnestOptions::new().with_null_handling(NullHandling::PreserveAndExpandEmpty)
    } else {
        UnnestOptions::new().with_null_handling(NullHandling::Drop)
    };
    df.unnest_columns_with_options(&[&unnested_col_name], unnest_options)
}

/// Deserialize a PostgreSQL expression from its `nodeToString` representation
/// and translate it to a DataFusion `Expr` against `sources`, using the
/// caller-supplied `mapper` to resolve Var nodes. `context` is used only in
/// error messages to disambiguate call sites.
pub unsafe fn translate_pg_node_string<'a>(
    pg_node_string: &str,
    sources: &'a [&'a JoinSource],
    mapper: Box<dyn ColumnMapper + 'a>,
    context: &'static str,
) -> Result<Expr> {
    let c_str = std::ffi::CString::new(pg_node_string).map_err(|e| {
        DataFusionError::Internal(format!("CString conversion failed for {context}: {e}"))
    })?;
    let node = pg_sys::stringToNode(c_str.as_ptr().cast_mut());
    if node.is_null() {
        return Err(DataFusionError::Internal(format!(
            "stringToNode returned null for {context}"
        )));
    }

    let translator = PredicateTranslator::new(sources).with_mapper(mapper);
    translator.translate(node.cast()).ok_or_else(|| {
        DataFusionError::Internal(format!(
            "PredicateTranslator failed to translate deserialized {context}"
        ))
    })
}

/// Translate a `PgExpression` join filter using a [`CombinedMapper`] so Var
/// nodes (carrying their original `(varno, varattno)`) resolve to qualified
/// DataFusion columns.
unsafe fn translate_pg_expression_filter(
    pg_node_string: &str,
    sources: &[&JoinSource],
    output_columns: &[OutputColumnInfo],
    lateral_unnests: &[LateralUnnestInfo],
) -> Result<Expr> {
    let mapper = CombinedMapper {
        sources,
        output_columns,
        lateral_unnests,
    };
    translate_pg_node_string(pg_node_string, sources, Box::new(mapper), "PgExpression")
}

/// Build equi-join filter expressions from a [`JoinNode`]'s key pairs.
///
/// Shared between JoinScan and AggregateScan `build_relnode_df` implementations.
/// Each key pair is resolved against the join's left/right subtrees and converted
/// to a `left_col = right_col` DataFusion expression.
pub fn build_equi_join_exprs(join: &JoinNode) -> Result<Vec<Expr>> {
    let mut on = Vec::with_capacity(join.equi_keys.len());
    for jk in &join.equi_keys {
        let ((left_source, left_attno), (right_source, right_attno)) =
            jk.resolve_against(&join.left, &join.right).ok_or_else(|| {
                DataFusionError::Internal(format!(
                    "Failed to resolve join key to current join sides: outer_rti={}, inner_rti={}",
                    jk.outer_rti, jk.inner_rti
                ))
            })?;

        let left_col_name = left_source
            .column_name(left_attno)
            .ok_or_else(|| DataFusionError::Internal("Missing left join-key column".into()))?;
        let right_col_name = right_source
            .column_name(right_attno)
            .ok_or_else(|| DataFusionError::Internal("Missing right join-key column".into()))?;

        let left_expr = make_source_col(left_source, &left_col_name);
        let right_expr = make_source_col(right_source, &right_col_name);
        on.push(left_expr.eq(right_expr));
    }
    Ok(on)
}

/// Build a DataFusion column expression for a `(source, field_name)` pair.
///
/// The execution alias is built from the source's optional alias and its
/// `plan_position` (the DFS index assigned by `JoinCSClause::new`). Both
/// JoinScan and AggregateScan use this exact pattern; sharing it keeps the
/// alias-construction policy in one place.
pub fn make_source_col(source: &JoinSource, field_name: &str) -> Expr {
    let alias =
        RelationAlias::new(source.scan_info.alias.as_deref()).execution(source.plan_position);
    make_col(&alias, field_name)
}

/// Build a DataFusion column expression for the synthetic score column on the
/// given source. Equivalent to `make_source_col(source, SCORE_COL_NAME)`.
pub fn make_source_score_col(source: &JoinSource) -> Expr {
    make_source_col(source, SCORE_COL_NAME)
}

/// Build a DataFusion column expression for an unnested field on the given
/// source, matching the `{alias}_{field_name}` naming convention established
/// by `apply_relnode_unnest`.
pub fn make_source_unnested_col(source: &JoinSource, field_name: &str) -> Expr {
    let alias =
        RelationAlias::new(source.scan_info.alias.as_deref()).execution(source.plan_position);
    col(format!("{}_{}", alias, field_name))
}

pub struct CombinedMapper<'a> {
    pub sources: &'a [&'a JoinSource],
    pub output_columns: &'a [OutputColumnInfo],
    pub lateral_unnests: &'a [LateralUnnestInfo],
}

impl<'a> ColumnMapper for CombinedMapper<'a> {
    fn map_var(&self, varno: pg_sys::Index, varattno: pg_sys::AttrNumber) -> Option<Expr> {
        let (rti, attno, is_score, unnested_info) = if varno == pg_sys::INDEX_VAR as pg_sys::Index {
            let idx = (varattno - 1) as usize;
            match self.output_columns.get(idx)? {
                OutputColumnInfo::Var {
                    rti,
                    original_attno,
                    ..
                } => (*rti, *original_attno, false, None),
                OutputColumnInfo::Score { rti, .. } => (*rti, 0, true, None),
                OutputColumnInfo::Unnested {
                    function_rti,
                    source_rti,
                    field_name,
                } => (
                    function_rti.0,
                    1,
                    false,
                    Some((source_rti.0, field_name.clone())),
                ),
                OutputColumnInfo::Pruned => return None,
            }
        } else {
            (varno, varattno, false, None)
        };

        if let Some((source_rti, field_name)) = unnested_info {
            let source = self.sources.iter().find(|s| s.contains_rti(source_rti))?;
            return Some(make_source_unnested_col(source, &field_name));
        }

        if let Some(source) = self.sources.iter().find(|s| s.contains_rti(rti)) {
            if is_score {
                if let Some(col_idx) = source.map_var(rti, 0) {
                    if let Some(name) = source.column_name(col_idx) {
                        return Some(make_source_col(source, &name));
                    }
                }
                return Some(make_source_score_col(source));
            }

            let mapped_attno = source.map_var(rti, attno)?;
            let col_name = source.column_name(mapped_attno)?;
            if self
                .lateral_unnests
                .iter()
                .any(|u| source.contains_rti(u.source_rti.0) && u.field_name == col_name)
            {
                Some(make_source_unnested_col(source, &col_name))
            } else {
                Some(make_source_col(source, &col_name))
            }
        } else {
            let unnest_info = self
                .lateral_unnests
                .iter()
                .find(|u| u.function_rti.0 == rti)?;
            let s = self
                .sources
                .iter()
                .find(|s| s.contains_rti(unnest_info.source_rti.0))?;
            Some(make_source_unnested_col(s, &unnest_info.field_name))
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::build_null_aware_anti_join;
    use datafusion::arrow::array::Int64Array;
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::arrow::record_batch::RecordBatch;
    use datafusion::common::{JoinType, NullEquality};
    use datafusion::datasource::MemTable;
    use datafusion::logical_expr::col;
    use datafusion::physical_plan::joins::HashJoinExec;
    use datafusion::physical_plan::ExecutionPlan;
    use datafusion::prelude::SessionContext;
    use std::sync::Arc;

    /// First `HashJoinExec`'s `(null_equality, join_type)`, or `None`.
    fn find_hash_join_attrs(plan: &dyn ExecutionPlan) -> Option<(NullEquality, JoinType)> {
        if let Some(hj) = plan.downcast_ref::<HashJoinExec>() {
            return Some((hj.null_equality(), *hj.join_type()));
        }
        plan.children()
            .iter()
            .find_map(|child| find_hash_join_attrs(child.as_ref()))
    }

    /// Register a single-column `Int64` table named `name` with column `id`
    /// holding `rows` (nullable).
    fn register_int_table(ctx: &SessionContext, name: &str, rows: Vec<Option<i64>>) {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int64, true)]));
        let batch = RecordBatch::try_new(schema.clone(), vec![Arc::new(Int64Array::from(rows))])
            .expect("build batch");
        ctx.register_table(
            name,
            Arc::new(MemTable::try_new(schema, vec![vec![batch]]).expect("memtable")),
        )
        .expect("register");
    }

    /// Empty-on-keys + filter must lower to `HashJoinExec` with
    /// `NullEqualsNothing` (via the `ExtractEquijoinPredicate` rule). A DF
    /// bump that breaks that pipeline would silently degrade us to
    /// `NestedLoopJoinExec`; this test catches that.
    #[tokio::test]
    async fn null_aware_anti_lifts_to_hash_join_with_null_equals_nothing() {
        let ctx = SessionContext::new();
        register_int_table(&ctx, "l", vec![Some(1), Some(2), Some(3)]);
        register_int_table(&ctx, "r", vec![Some(2), None, Some(4)]);

        let left = ctx.table("l").await.expect("table l");
        let right = ctx.table("r").await.expect("table r");

        let on = vec![col("l.id").eq(col("r.id"))];

        let result = build_null_aware_anti_join(left, right, &on, JoinType::LeftAnti)
            .expect("build_null_aware_anti_join");
        let physical = result
            .create_physical_plan()
            .await
            .expect("create_physical_plan");

        let (null_equality, join_type) = find_hash_join_attrs(physical.as_ref())
            .expect("expected HashJoinExec in physical plan; got NestedLoopJoin or other");
        assert_eq!(
            null_equality,
            NullEquality::NullEqualsNothing,
            "HashJoinExec must use NullEqualsNothing for SQL NOT IN three-valued semantics"
        );
        assert_eq!(
            join_type,
            JoinType::LeftAnti,
            "join type must remain LeftAnti after physical planning"
        );
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::PredicateTranslator;
    use pgrx::prelude::*;
    use pgrx::PgList;

    #[pg_test]
    fn unnormalized_list_is_a_non_throwing_capability_miss() {
        unsafe {
            let mut list = PgList::<pg_sys::Node>::new();
            list.push(pg_sys::makeBoolConst(true, false).cast());

            assert!(!PredicateTranslator::can_translate(
                None,
                &[],
                list.into_pg().cast(),
                None,
            ));
        }
    }
}
