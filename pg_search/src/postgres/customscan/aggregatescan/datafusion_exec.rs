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

//! DataFusion plan builder for aggregate-on-join queries.
//!
//! Builds a DataFusion logical plan from a [`RelNode`] join tree and a
//! [`JoinAggregateTargetList`], producing: scan(s) → join → aggregate.
//!
//! Key difference from JoinScan's plan builder: no CTID columns, no late
//! materialization, no SegmentedTopK — aggregates run entirely on fast fields
//! and the result is aggregate rows, not individual tuples.

use super::join_targetlist::AggOrderByEntry;
use crate::index::fast_fields_helper::WhichFastField;
use crate::postgres::customscan::aggregatescan::join_targetlist::{
    AggKind, JoinAggregateEntry, JoinAggregateTargetList,
};
use crate::postgres::customscan::aggregatescan::privdat::{CompareOp, DataFusionTopK, FilterExpr};
use crate::postgres::customscan::datafusion::translator::{
    apply_join_level_filter, build_join_df, make_col, make_source_col, ColumnMapper,
    JoinTypeAllowList, PredicateTranslator,
};
use crate::postgres::customscan::joinscan::build::{
    JoinLevelSearchPredicate, JoinSource, RelNode, RelationAlias,
};
use crate::postgres::customscan::joinscan::scan_state::{
    create_datafusion_session_context, create_datafusion_session_context_mpp,
    register_source_table, SessionContextProfile,
};
use crate::scan::info::RowEstimate;
use crate::scan::PgSearchTableProvider;
use datafusion::common::{DataFusionError, Result};
use datafusion::functions_aggregate::array_agg::array_agg_udaf;
use datafusion::functions_aggregate::count::count_udaf;
use datafusion::functions_aggregate::expr_fn::{
    array_agg, avg, bool_and, bool_or, count, max, min, stddev, stddev_pop, sum, var_pop,
    var_sample,
};
use datafusion::functions_aggregate::string_agg::string_agg_udaf;
use datafusion::logical_expr::expr::{AggregateFunction, Sort};
use datafusion::logical_expr::{lit, Expr};
use datafusion::prelude::{DataFrame, SessionContext};
use futures::future::{FutureExt, LocalBoxFuture};
use pgrx::pg_sys;

/// Creates a DataFusion [`SessionContext`] for aggregate-on-join workloads.
///
/// Thin wrapper around the shared
/// [`create_datafusion_session_context`] with the
/// [`SessionContextProfile::Aggregate`] profile. Kept as a named function so
/// the call sites in `aggregatescan/mod.rs` remain stable; if more aggregate-
/// specific session setup ever appears, this is the place to put it.
pub fn create_aggregate_session_context() -> SessionContext {
    create_datafusion_session_context(SessionContextProfile::Aggregate)
}

/// MPP-aware variant. Applies the join-layer bug fixes required when the
/// aggregate path runs as part of an MPP pipeline: skip `SortMergeJoinEnforcer`
/// and disable `enable_join_dynamic_filter_pushdown`. Without this, the
/// probe-side scan races with the exchange producer and silently drops rows
/// before the dynamic filter stabilizes.
pub fn create_aggregate_session_context_mpp(
    mpp: &crate::postgres::customscan::mpp::MppParticipantConfig,
) -> SessionContext {
    create_datafusion_session_context_mpp(SessionContextProfile::Aggregate, mpp)
}

/// Build the complete DataFusion logical plan for an aggregate-on-join query:
/// scan(s) → join → aggregate [→ sort → limit].
#[allow(clippy::too_many_arguments)]
pub async fn build_join_aggregate_plan(
    plan: &RelNode,
    targetlist: &JoinAggregateTargetList,
    topk: Option<&DataFusionTopK>,
    join_level_predicates: &[JoinLevelSearchPredicate],
    custom_exprs: *mut pg_sys::List,
    custom_scan_tlist: *mut pg_sys::List,
    having_filter: Option<&FilterExpr>,
    ctx: &SessionContext,
) -> Result<datafusion::logical_expr::LogicalPlan> {
    // Step 1: Build the join DataFrame from the RelNode tree
    let df = build_relnode_df(
        ctx,
        plan,
        join_level_predicates,
        custom_exprs,
        custom_scan_tlist,
    )
    .await?;

    // Step 2: Build GROUP BY expressions
    let group_exprs: Vec<Expr> = targetlist
        .group_columns
        .iter()
        .map(|gc| make_rti_col(plan, gc.rti, &gc.field_name))
        .collect();

    // Step 3: Build aggregate expressions
    let agg_exprs: Vec<Expr> = targetlist
        .aggregates
        .iter()
        .enumerate()
        .map(|(i, agg)| {
            let agg_expr = match agg.agg_kind {
                AggKind::CountStar => Ok(count(lit(1))),
                AggKind::Count => agg_field_col(agg, plan).map(count),
                AggKind::CountDistinct => {
                    let col_exprs = agg_field_cols(agg, plan)?;
                    Ok(Expr::AggregateFunction(AggregateFunction::new_udf(
                        count_udaf(),
                        col_exprs,
                        true,   // distinct
                        None,   // filter
                        vec![], // order_by
                        None,   // null_treatment
                    )))
                }
                AggKind::Sum => agg_field_col(agg, plan).map(sum),
                AggKind::Avg => agg_field_col(agg, plan).map(avg),
                AggKind::Min => agg_field_col(agg, plan).map(min),
                AggKind::Max => agg_field_col(agg, plan).map(max),
                AggKind::StddevSamp => agg_field_col(agg, plan).map(stddev),
                AggKind::StddevPop => agg_field_col(agg, plan).map(stddev_pop),
                AggKind::VarSamp => agg_field_col(agg, plan).map(var_sample),
                AggKind::VarPop => agg_field_col(agg, plan).map(var_pop),
                AggKind::BoolAnd => agg_field_col(agg, plan).map(bool_and),
                AggKind::BoolOr => agg_field_col(agg, plan).map(bool_or),
                AggKind::ArrayAgg => {
                    let col_expr = agg_field_col(agg, plan)?;
                    if agg.order_by.is_empty() {
                        Ok(array_agg(col_expr))
                    } else {
                        Ok(Expr::AggregateFunction(AggregateFunction::new_udf(
                            array_agg_udaf(),
                            vec![col_expr],
                            false,
                            None,
                            agg_order_by_exprs(&agg.order_by, plan),
                            None,
                        )))
                    }
                }
                AggKind::StringAgg(ref sep) => {
                    let col_expr = agg_field_col(agg, plan)?;
                    let sep_lit = lit(sep.clone());
                    if agg.order_by.is_empty() {
                        Ok(datafusion::functions_aggregate::string_agg::string_agg(
                            col_expr, sep_lit,
                        ))
                    } else {
                        Ok(Expr::AggregateFunction(AggregateFunction::new_udf(
                            string_agg_udaf(),
                            vec![col_expr, sep_lit],
                            false,
                            None,
                            agg_order_by_exprs(&agg.order_by, plan),
                            None,
                        )))
                    }
                }
            }?;
            // Apply DISTINCT flag for non-CountDistinct aggregates.
            // CountDistinct already sets distinct=true via new_udf above.
            let agg_expr = if agg.distinct
                && !matches!(agg.agg_kind, AggKind::CountDistinct | AggKind::CountStar)
            {
                with_distinct(agg_expr)
            } else {
                agg_expr
            };
            // Apply per-aggregate FILTER clause if present.
            let agg_expr = if let Some(ref filter_expr) = agg.filter {
                let filter_ctx = FilterExprExecContext {
                    targetlist: None,
                    plan: Some(plan),
                };
                let df_filter = filter_expr.to_datafusion(&filter_ctx).ok_or_else(|| {
                    DataFusionError::Internal(
                        "Failed to translate aggregate FILTER clause to DataFusion".to_string(),
                    )
                })?;
                with_filter(agg_expr, df_filter)
            } else {
                agg_expr
            };
            // Alias for stable reference
            Ok(agg_expr.alias(format!("agg_{}", i)))
        })
        .collect::<Result<Vec<Expr>>>()?;

    // Step 4: Apply aggregate
    let mut df = df.aggregate(group_exprs, agg_exprs)?;

    // Step 4.5: Apply HAVING filter (post-aggregate)
    if let Some(having) = having_filter {
        let having_ctx = FilterExprExecContext {
            targetlist: Some(targetlist),
            plan: None,
        };
        let expr = having.to_datafusion(&having_ctx).ok_or_else(|| {
            DataFusionError::Internal(
                "Failed to translate HAVING clause to DataFusion expression".to_string(),
            )
        })?;
        df = df.filter(expr)?;
    }

    // Step 5: If TopK is requested, add sort + limit so DataFusion handles
    // it internally. DataFusion's built-in TopKAggregation optimizer rule
    // can then push the limit into AggregateExec for group-key and MIN/MAX
    // ordering. For COUNT/SUM/AVG ordering, SortExec(fetch=K) uses a
    // bounded TopK heap.
    if let Some(topk) = topk {
        let sort_col_name = topk.sort_target.resolve_sort_col_name(targetlist, plan);
        let sort_expr = datafusion::prelude::col(&sort_col_name)
            .sort(topk.direction.is_asc(), topk.direction.is_nulls_first());
        let df = df.sort(vec![sort_expr])?;
        let df = df.limit(0, Some(topk.k))?;
        return df.into_optimized_plan();
    }

    df.into_optimized_plan()
}

/// Recursively lower a [`RelNode`] tree into a DataFusion [`DataFrame`].
///
/// Unlike JoinScan's `build_relnode_df`, this version:
/// - Does NOT include CTID columns (no heap fetch needed for aggregates)
/// - Does NOT handle LIMIT, ORDER BY, DISTINCT, or output projection
///   (those are handled by the aggregate layer above)
/// - Is single-threaded (no partitioning logic)
fn build_relnode_df<'a>(
    ctx: &'a SessionContext,
    node: &'a RelNode,
    join_level_predicates: &'a [JoinLevelSearchPredicate],
    custom_exprs: *mut pg_sys::List,
    custom_scan_tlist: *mut pg_sys::List,
) -> LocalBoxFuture<'a, Result<DataFrame>> {
    async move {
        match node {
            RelNode::Scan(source) => {
                let plan_position = source.plan_position;
                let df = build_source_df(ctx, source, plan_position).await?;
                let alias =
                    RelationAlias::new(source.scan_info.alias.as_deref()).execution(plan_position);
                Ok(df.alias(&alias)?)
            }
            RelNode::Join(join) => {
                let left_df = build_relnode_df(
                    ctx,
                    &join.left,
                    join_level_predicates,
                    custom_exprs,
                    custom_scan_tlist,
                )
                .await?;
                let right_df = build_relnode_df(
                    ctx,
                    &join.right,
                    join_level_predicates,
                    custom_exprs,
                    custom_scan_tlist,
                )
                .await?;

                build_join_df(left_df, right_df, join, JoinTypeAllowList::EquiOnly)
            }
            RelNode::Filter(filter) => {
                let df = build_relnode_df(
                    ctx,
                    &filter.input,
                    join_level_predicates,
                    custom_exprs,
                    custom_scan_tlist,
                )
                .await?;

                let has_predicates = !join_level_predicates.is_empty() || !custom_exprs.is_null();

                if !has_predicates {
                    // No predicates to apply — pass through
                    return Ok(df);
                }

                // Build a ctid_map: plan_position → ctid column expression.
                // In the aggregate path, ctid columns are real (not deferred),
                // and the ctid field is named "ctid" (from WhichFastField::Ctid)
                // in the table provider schema. After aliasing, it's accessible
                // as `<alias>.ctid`.
                let sources = filter.input.sources();
                let ctid_map: crate::api::HashMap<pg_sys::Index, Expr> = sources
                    .iter()
                    .map(|s| (s.plan_position as pg_sys::Index, make_source_col(s, "ctid")))
                    .collect();

                // No deferred positions in aggregate path (no VisibilityFilterExec)
                let deferred_positions = crate::api::HashSet::default();

                // Translate custom_exprs (non-@@@ cross-table predicates) using
                // PredicateTranslator, mirroring JoinScan's scan_state.rs:562-576.
                // After setrefs, Vars in custom_exprs are INDEX_VAR references
                // that index into custom_scan_tlist. We need a mapper to resolve
                // them back to the correct DataFusion column names.
                let mut translated_exprs = Vec::new();
                if !custom_exprs.is_null() {
                    let mapper = AggregateIndexVarMapper {
                        sources: &sources,
                        custom_scan_tlist,
                    };
                    let translator =
                        PredicateTranslator::new(&sources).with_mapper(Box::new(mapper));
                    unsafe {
                        let expr_list = pgrx::PgList::<pg_sys::Node>::from_pg(custom_exprs);
                        for (i, expr_node) in expr_list.iter_ptr().enumerate() {
                            let expr = translator.translate(expr_node).ok_or_else(|| {
                                DataFusionError::Internal(format!(
                                    "Failed to translate aggregate custom expression at index {}",
                                    i
                                ))
                            })?;
                            translated_exprs.push(expr);
                        }
                    }
                }

                apply_join_level_filter(
                    df,
                    &filter.predicate,
                    &translated_exprs,
                    &ctid_map,
                    join_level_predicates,
                    &deferred_positions,
                    &sources,
                    /* handle_mark = */ false,
                )
            }
        }
    }
    .boxed_local()
}

/// Maps INDEX_VAR references (from setrefs-transformed custom_exprs) back to
/// DataFusion column names. In the aggregate scan, custom_scan_tlist mirrors
/// the plan's targetlist (plus any Vars we added for predicates), and INDEX_VAR
/// varattno indexes into it. We resolve each Var by looking up the original
/// (rti, attno) from custom_scan_tlist and finding the corresponding source.
struct AggregateIndexVarMapper<'a> {
    sources: &'a [&'a JoinSource],
    custom_scan_tlist: *mut pg_sys::List,
}

impl<'a> ColumnMapper for AggregateIndexVarMapper<'a> {
    fn map_var(&self, varno: pg_sys::Index, varattno: pg_sys::AttrNumber) -> Option<Expr> {
        let (rti, attno) = if varno == pg_sys::INDEX_VAR as pg_sys::Index {
            // INDEX_VAR: look up the original Var from custom_scan_tlist.
            // varattno is 1-indexed into the target list.
            unsafe {
                let tlist = pgrx::PgList::<pg_sys::TargetEntry>::from_pg(self.custom_scan_tlist);
                let idx = (varattno - 1) as usize;
                let te = tlist.get_ptr(idx)?;
                if (*(*te).expr).type_ != pg_sys::NodeTag::T_Var {
                    return None;
                }
                let var = (*te).expr as *mut pg_sys::Var;
                ((*var).varno as pg_sys::Index, (*var).varattno)
            }
        } else {
            (varno, varattno)
        };

        let source = self.sources.iter().find(|s| s.contains_rti(rti))?;
        let field_name = source.column_name(attno)?;
        Some(make_source_col(source, &field_name))
    }
}

/// Context for the **exec phase** — translating a [`FilterExpr`] IR into a
/// DataFusion [`Expr`].
///
/// HAVING provides `targetlist` for resolving `AggRef`/`GroupRef`;
/// FILTER provides `plan` (a `RelNode` tree) for resolving `ColumnRef`.
///
/// This is distinct from the build-phase context in `datafusion_build.rs`,
/// which carries raw planner `JoinAggSource`s instead of a `RelNode` tree.
struct FilterExprExecContext<'a> {
    targetlist: Option<&'a JoinAggregateTargetList>,
    plan: Option<&'a RelNode>,
}

impl FilterExpr {
    /// Translate this expression to a DataFusion `Expr`.
    ///
    /// Used for both HAVING (pass `targetlist`) and per-aggregate FILTER (pass `plan`).
    fn to_datafusion(&self, ctx: &FilterExprExecContext<'_>) -> Option<Expr> {
        use datafusion::logical_expr::Operator;

        match self {
            FilterExpr::AggRef(idx) => {
                let tl = ctx.targetlist?;
                if *idx < tl.aggregates.len() {
                    Some(datafusion::prelude::col(format!("agg_{}", idx)))
                } else {
                    None
                }
            }
            FilterExpr::GroupRef(field_name) => Some(datafusion::prelude::col(field_name.as_str())),
            FilterExpr::ColumnRef { rti, field_name } => {
                let plan = ctx.plan?;
                Some(make_rti_col(plan, *rti, field_name))
            }
            FilterExpr::LitInt(v) => Some(lit(*v)),
            FilterExpr::LitFloat(v) => Some(lit(*v)),
            FilterExpr::LitBool(v) => Some(lit(*v)),
            FilterExpr::LitString(v) => Some(lit(v.clone())),
            FilterExpr::BinOp { left, op, right } => {
                let l = left.to_datafusion(ctx)?;
                let r = right.to_datafusion(ctx)?;
                let df_op = match op {
                    CompareOp::Eq => Operator::Eq,
                    CompareOp::NotEq => Operator::NotEq,
                    CompareOp::Lt => Operator::Lt,
                    CompareOp::LtEq => Operator::LtEq,
                    CompareOp::Gt => Operator::Gt,
                    CompareOp::GtEq => Operator::GtEq,
                };
                Some(Expr::BinaryExpr(datafusion::logical_expr::BinaryExpr::new(
                    Box::new(l),
                    df_op,
                    Box::new(r),
                )))
            }
            FilterExpr::And(children) => {
                let exprs: Vec<Expr> = children
                    .iter()
                    .map(|c| c.to_datafusion(ctx))
                    .collect::<Option<Vec<Expr>>>()?;
                let mut result = exprs.into_iter();
                let first = result.next()?;
                Some(result.fold(first, |acc, e| acc.and(e)))
            }
            FilterExpr::Or(children) => {
                let exprs: Vec<Expr> = children
                    .iter()
                    .map(|c| c.to_datafusion(ctx))
                    .collect::<Option<Vec<Expr>>>()?;
                let mut result = exprs.into_iter();
                let first = result.next()?;
                Some(result.fold(first, |acc, e| acc.or(e)))
            }
            FilterExpr::Not(inner) => {
                let e = inner.to_datafusion(ctx)?;
                Some(Expr::Not(Box::new(e)))
            }
            FilterExpr::IsNull(inner) => {
                let e = inner.to_datafusion(ctx)?;
                Some(e.is_null())
            }
            FilterExpr::IsNotNull(inner) => {
                let e = inner.to_datafusion(ctx)?;
                Some(e.is_not_null())
            }
        }
    }
}

/// Build a DataFusion [`DataFrame`] for a single scan source.
///
/// Unlike JoinScan's `build_source_df`, this version:
/// - Does NOT include CTID or Score columns
/// - Is always single-threaded (no partitioning)
/// - Does NOT set up late materialization
async fn build_source_df(
    ctx: &SessionContext,
    source: &JoinSource,
    plan_position: usize,
) -> Result<DataFrame> {
    let mut scan_info = source.scan_info.clone();

    // Set estimated_rows_per_worker for the table provider. In M1 we're
    // single-threaded, so the per-worker estimate equals the total estimate.
    if scan_info.estimated_rows_per_worker.is_none() {
        scan_info.estimated_rows_per_worker = Some(match scan_info.estimate {
            RowEstimate::Known(n) => n,
            RowEstimate::Unknown => 1000, // conservative fallback
        });
    }

    let alias = RelationAlias::new(scan_info.alias.as_deref()).execution(plan_position);

    // Use all fast fields from the source (the provider exposes them to DataFusion).
    // Include Named fields plus Ctid as a sentinel — DataFusion needs at least one
    // column to produce RecordBatches with row counts (important for COUNT(*)).
    let mut fields: Vec<WhichFastField> = source
        .scan_info
        .fields
        .iter()
        .filter_map(|f| match &f.field {
            WhichFastField::Named(..) | WhichFastField::Deferred(..) => Some(f.field.clone()),
            WhichFastField::Ctid
            | WhichFastField::Score
            | WhichFastField::Junk(_)
            | WhichFastField::TableOid
            | WhichFastField::DeferredCtid(_) => None,
        })
        .collect();

    // Always include Ctid so the provider schema is never empty
    if fields.is_empty() || !fields.iter().any(|f| matches!(f, WhichFastField::Ctid)) {
        fields.push(WhichFastField::Ctid);
    }

    let provider = PgSearchTableProvider::new(scan_info, fields, false);
    let df = register_source_table(ctx, alias.as_str(), provider).await?;

    // Select all fields from the provider schema using their qualified names.
    // This mirrors JoinScan's pattern and ensures column names are accessible
    // via make_col(alias, field_name) in join keys and aggregate expressions.
    let exprs: Vec<Expr> = df
        .schema()
        .fields()
        .iter()
        .map(|f| make_col(alias.as_str(), f.name()))
        .collect();

    if exprs.is_empty() {
        // No fields at all — this can happen for COUNT(*) where no columns are
        // referenced from this source. Return the raw DataFrame.
        Ok(df)
    } else {
        df.select(exprs)
    }
}

/// Build a DataFusion column expression for a `(rti, field_name)` reference
/// against the given plan tree.
///
/// Thin wrapper around the shared [`make_source_col`] that walks the plan to
/// find the source claiming `rti` first. Use this instead of resolving the
/// source manually at every aggregate-on-join call site.
fn make_rti_col(plan: &RelNode, rti: pgrx::pg_sys::Index, field_name: &str) -> Expr {
    let source = plan
        .source_for_rti_in_subtree(rti)
        .unwrap_or_else(|| panic!("make_rti_col: RTI {rti} not found in plan sources"));
    make_source_col(source, field_name)
}

/// Replace an `Expr::AggregateFunction` with the same call but `distinct=true`.
/// Non-aggregate-function expressions are returned unchanged.
fn with_distinct(expr: Expr) -> Expr {
    match expr {
        Expr::AggregateFunction(af) => Expr::AggregateFunction(AggregateFunction::new_udf(
            af.func,
            af.params.args,
            true,
            af.params.filter,
            af.params.order_by,
            af.params.null_treatment,
        )),
        other => other,
    }
}

/// Replace an `Expr::AggregateFunction` with the same call but `filter=Some(...)`.
/// Non-aggregate-function expressions are returned unchanged.
fn with_filter(expr: Expr, filter: Expr) -> Expr {
    match expr {
        Expr::AggregateFunction(af) => Expr::AggregateFunction(AggregateFunction::new_udf(
            af.func,
            af.params.args,
            af.params.distinct,
            Some(Box::new(filter)),
            af.params.order_by,
            af.params.null_treatment,
        )),
        other => other,
    }
}

/// Build a DataFusion column expression for an aggregate's first field reference.
fn agg_field_col(agg: &JoinAggregateEntry, plan: &RelNode) -> Result<Expr> {
    let (rti, _attno, ref field_name) = agg.field_refs.first().ok_or_else(|| {
        DataFusionError::Internal("non-COUNT(*) aggregate must have a field reference".to_string())
    })?;
    Ok(make_rti_col(plan, *rti, field_name))
}

/// Convert aggregate ORDER BY entries to DataFusion `Sort` expressions.
fn agg_order_by_exprs(order_by: &[AggOrderByEntry], plan: &RelNode) -> Vec<Sort> {
    order_by
        .iter()
        .map(|entry| {
            Sort::new(
                make_rti_col(plan, entry.rti, &entry.field_name),
                entry.direction.is_asc(),
                entry.direction.is_nulls_first(),
            )
        })
        .collect()
}

/// Build DataFusion column expressions for all of an aggregate's field references.
/// Used for multi-column DISTINCT (e.g. `COUNT(DISTINCT col1, col2)`).
fn agg_field_cols(agg: &JoinAggregateEntry, plan: &RelNode) -> Result<Vec<Expr>> {
    agg.field_refs
        .iter()
        .map(|(rti, _attno, field_name)| Ok(make_rti_col(plan, *rti, field_name)))
        .collect()
}
