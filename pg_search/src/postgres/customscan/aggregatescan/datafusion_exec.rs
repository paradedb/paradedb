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

use std::sync::Arc;

use crate::index::fast_fields_helper::WhichFastField;
use crate::postgres::customscan::aggregatescan::join_targetlist::{
    AggKind, JoinAggregateEntry, JoinAggregateTargetList,
};
use crate::postgres::customscan::aggregatescan::privdat::DataFusionTopK;
use crate::postgres::customscan::joinscan::build::{JoinSource, RelNode, RelationAlias};
use crate::postgres::customscan::joinscan::scan_state::build_base_session;
use crate::postgres::customscan::joinscan::translator::{build_equi_join_exprs, make_col};
use crate::scan::info::RowEstimate;
use crate::scan::PgSearchTableProvider;
use datafusion::common::{DataFusionError, JoinType, Result};
use datafusion::functions_aggregate::count::count_udaf;
use datafusion::functions_aggregate::expr_fn::{
    avg, count, max, min, stddev, stddev_pop, sum, var_pop, var_sample,
};
use datafusion::logical_expr::{lit, Expr};
use datafusion::physical_optimizer::filter_pushdown::FilterPushdown;
use datafusion::prelude::{DataFrame, SessionConfig, SessionContext};
use futures::future::{FutureExt, LocalBoxFuture};

/// Creates a DataFusion [`SessionContext`] for aggregate workloads.
///
/// Shares the base session setup with JoinScan (visibility, late
/// materialization, sort-merge join) via [`build_base_session`].
/// Unlike JoinScan, this does not include `SegmentedTopKRule` (row-level
/// TopK doesn't apply to aggregates). DataFusion's built-in
/// `SortExec(fetch=K)` already uses a bounded TopK heap internally.
pub fn create_aggregate_session_context() -> SessionContext {
    let config = SessionConfig::new().with_target_partitions(1);
    let builder = build_base_session(config)
        // FilterPushdown: push filters to PgSearchTableProvider
        .with_physical_optimizer_rule(Arc::new(FilterPushdown::new_post_optimization()));

    SessionContext::new_with_state(builder.build())
}

/// Build the complete DataFusion logical plan for an aggregate-on-join query:
/// scan(s) → join → aggregate [→ sort → limit].
pub async fn build_join_aggregate_plan(
    plan: &RelNode,
    targetlist: &JoinAggregateTargetList,
    topk: Option<&DataFusionTopK>,
    post_join_filters: &[crate::postgres::customscan::aggregatescan::privdat::PostJoinFilter],
    having_filter: Option<&crate::postgres::customscan::aggregatescan::privdat::HavingExpr>,
    ctx: &SessionContext,
) -> Result<datafusion::logical_expr::LogicalPlan> {
    // Step 1: Build the join DataFrame from the RelNode tree
    let mut df = build_relnode_df(ctx, plan).await?;

    // Step 1.5: Apply post-join filters (non-equi quals from joinrestrictinfo)
    for filter in post_join_filters {
        if let Some(expr) = filter_expr_to_datafusion(&filter.expr, plan) {
            df = df.filter(expr)?;
        }
    }

    // Step 2: Build GROUP BY expressions
    let group_exprs: Vec<Expr> = targetlist
        .group_columns
        .iter()
        .map(|gc| {
            let source = plan.source_for_rti_in_subtree(gc.rti);
            let (alias, _col_name) = resolve_source_column(source, gc.rti, &gc.field_name, plan);
            make_col(&alias, &gc.field_name)
        })
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
                    let col_expr = agg_field_col(agg, plan)?;
                    Ok(Expr::AggregateFunction(
                        datafusion::logical_expr::expr::AggregateFunction::new_udf(
                            count_udaf(),
                            vec![col_expr],
                            true,   // distinct
                            None,   // filter
                            vec![], // order_by
                            None,   // null_treatment
                        ),
                    ))
                }
                AggKind::Sum => agg_field_col(agg, plan).map(sum),
                AggKind::Avg => agg_field_col(agg, plan).map(avg),
                AggKind::Min => agg_field_col(agg, plan).map(min),
                AggKind::Max => agg_field_col(agg, plan).map(max),
                AggKind::StddevSamp => agg_field_col(agg, plan).map(stddev),
                AggKind::StddevPop => agg_field_col(agg, plan).map(stddev_pop),
                AggKind::VarSamp => agg_field_col(agg, plan).map(var_sample),
                AggKind::VarPop => agg_field_col(agg, plan).map(var_pop),
            }?;
            // Alias for stable reference
            Ok(agg_expr.alias(format!("agg_{}", i)))
        })
        .collect::<Result<Vec<Expr>>>()?;

    // Step 4: Apply aggregate
    let mut df = df.aggregate(group_exprs, agg_exprs)?;

    // Step 4.5: Apply HAVING filter (post-aggregate)
    if let Some(having) = having_filter {
        if let Some(expr) = having_expr_to_datafusion(having, targetlist) {
            df = df.filter(expr)?;
        }
    }

    // Step 5: If TopK is requested, add sort + limit so DataFusion handles it internally
    if let Some(topk) = topk {
        let sort_col_name = format!("agg_{}", topk.sort_agg_idx);
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
/// Translate a serialized `FilterExpr` to a DataFusion `Expr`.
fn filter_expr_to_datafusion(
    filter: &crate::postgres::customscan::aggregatescan::privdat::FilterExpr,
    plan: &RelNode,
) -> Option<Expr> {
    use crate::postgres::customscan::aggregatescan::privdat::{FilterExpr, FilterOp};
    use datafusion::logical_expr::Operator;

    match filter {
        FilterExpr::Column(source_idx, field_name) => {
            let sources = plan.sources();
            let source = sources.get(*source_idx)?;
            let alias = RelationAlias::new(source.scan_info.alias.as_deref())
                .execution(source.plan_position);
            Some(make_col(&alias, field_name))
        }
        FilterExpr::LitInt(v) => Some(lit(*v)),
        FilterExpr::LitFloat(v) => Some(lit(*v)),
        FilterExpr::LitString(v) => Some(lit(v.clone())),
        FilterExpr::LitBool(v) => Some(lit(*v)),
        FilterExpr::LitNull => Some(lit(datafusion::scalar::ScalarValue::Null)),
        FilterExpr::BinOp { left, op, right } => {
            let l = filter_expr_to_datafusion(left, plan)?;
            let r = filter_expr_to_datafusion(right, plan)?;
            let df_op = match op {
                FilterOp::Eq => Operator::Eq,
                FilterOp::NotEq => Operator::NotEq,
                FilterOp::Lt => Operator::Lt,
                FilterOp::LtEq => Operator::LtEq,
                FilterOp::Gt => Operator::Gt,
                FilterOp::GtEq => Operator::GtEq,
            };
            Some(Expr::BinaryExpr(datafusion::logical_expr::BinaryExpr::new(
                Box::new(l),
                df_op,
                Box::new(r),
            )))
        }
        FilterExpr::And(children) => {
            let mut exprs: Vec<Expr> = children
                .iter()
                .filter_map(|c| filter_expr_to_datafusion(c, plan))
                .collect();
            if exprs.is_empty() {
                return None;
            }
            let mut result = exprs.remove(0);
            for e in exprs {
                result = result.and(e);
            }
            Some(result)
        }
        FilterExpr::Or(children) => {
            let mut exprs: Vec<Expr> = children
                .iter()
                .filter_map(|c| filter_expr_to_datafusion(c, plan))
                .collect();
            if exprs.is_empty() {
                return None;
            }
            let mut result = exprs.remove(0);
            for e in exprs {
                result = result.or(e);
            }
            Some(result)
        }
        FilterExpr::Not(inner) => {
            let e = filter_expr_to_datafusion(inner, plan)?;
            Some(Expr::Not(Box::new(e)))
        }
    }
}

/// Translate a serialized `HavingExpr` to a DataFusion `Expr`.
/// Aggregate references use the `agg_{idx}` aliases from the aggregate step.
fn having_expr_to_datafusion(
    expr: &crate::postgres::customscan::aggregatescan::privdat::HavingExpr,
    targetlist: &JoinAggregateTargetList,
) -> Option<Expr> {
    use crate::postgres::customscan::aggregatescan::privdat::HavingExpr;
    use datafusion::logical_expr::Operator;

    match expr {
        HavingExpr::AggRef(idx) => {
            if *idx < targetlist.aggregates.len() {
                Some(datafusion::prelude::col(format!("agg_{}", idx)))
            } else {
                None
            }
        }
        HavingExpr::GroupRef(idx) => {
            if *idx < targetlist.group_columns.len() {
                Some(datafusion::prelude::col(
                    &targetlist.group_columns[*idx].field_name,
                ))
            } else {
                None
            }
        }
        HavingExpr::LitInt(v) => Some(lit(*v)),
        HavingExpr::LitFloat(v) => Some(lit(*v)),
        HavingExpr::LitBool(v) => Some(lit(*v)),
        HavingExpr::LitNull => Some(lit(datafusion::scalar::ScalarValue::Null)),
        HavingExpr::BinOp { left, op, right } => {
            use crate::postgres::customscan::aggregatescan::privdat::FilterOp;
            let l = having_expr_to_datafusion(left, targetlist)?;
            let r = having_expr_to_datafusion(right, targetlist)?;
            let df_op = match op {
                FilterOp::Eq => Operator::Eq,
                FilterOp::NotEq => Operator::NotEq,
                FilterOp::Lt => Operator::Lt,
                FilterOp::LtEq => Operator::LtEq,
                FilterOp::Gt => Operator::Gt,
                FilterOp::GtEq => Operator::GtEq,
            };
            Some(Expr::BinaryExpr(datafusion::logical_expr::BinaryExpr::new(
                Box::new(l),
                df_op,
                Box::new(r),
            )))
        }
        HavingExpr::And(children) => {
            let mut exprs: Vec<Expr> = children
                .iter()
                .filter_map(|c| having_expr_to_datafusion(c, targetlist))
                .collect();
            if exprs.is_empty() {
                return None;
            }
            let mut result = exprs.remove(0);
            for e in exprs {
                result = result.and(e);
            }
            Some(result)
        }
        HavingExpr::Or(children) => {
            let mut exprs: Vec<Expr> = children
                .iter()
                .filter_map(|c| having_expr_to_datafusion(c, targetlist))
                .collect();
            if exprs.is_empty() {
                return None;
            }
            let mut result = exprs.remove(0);
            for e in exprs {
                result = result.or(e);
            }
            Some(result)
        }
        HavingExpr::Not(inner) => {
            let e = having_expr_to_datafusion(inner, targetlist)?;
            Some(Expr::Not(Box::new(e)))
        }
    }
}

/// - Does NOT include CTID columns (no heap fetch needed for aggregates)
/// - Does NOT handle LIMIT, ORDER BY, DISTINCT, or output projection
///   (those are handled by the aggregate layer above)
/// - Is single-threaded (no partitioning logic)
fn build_relnode_df<'a>(
    ctx: &'a SessionContext,
    node: &'a RelNode,
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
                let left_df = build_relnode_df(ctx, &join.left).await?;
                let right_df = build_relnode_df(ctx, &join.right).await?;

                let on = build_equi_join_exprs(join)?;

                let df_join_type = match join.join_type {
                    crate::postgres::customscan::joinscan::build::JoinType::Inner => {
                        JoinType::Inner
                    }
                    crate::postgres::customscan::joinscan::build::JoinType::Left => JoinType::Left,
                    crate::postgres::customscan::joinscan::build::JoinType::Right => {
                        JoinType::Right
                    }
                    unsupported => {
                        return Err(DataFusionError::NotImplemented(format!(
                            "Aggregate-on-join does not support {} JOIN",
                            unsupported
                        )));
                    }
                };

                if on.is_empty() {
                    left_df.join(right_df, df_join_type, &[], &[], None)
                } else {
                    left_df.join_on(right_df, df_join_type, on)
                }
            }
            RelNode::Filter(filter) => {
                // For now, filters in the RelNode tree are not expected for aggregate queries.
                // The search predicates are pushed into PgSearchTableProvider at scan level.
                let df = build_relnode_df(ctx, &filter.input).await?;
                // TODO: translate filter.predicate to DataFusion Expr if needed
                Ok(df)
            }
        }
    }
    .boxed_local()
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
    let provider = Arc::new(provider);
    ctx.register_table(alias.as_str(), provider)?;

    let df = ctx.table(alias.as_str()).await?;

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

/// Resolve a source column to its DataFusion alias and column name.
fn resolve_source_column(
    source: Option<&JoinSource>,
    rti: pgrx::pg_sys::Index,
    field_name: &str,
    plan: &RelNode,
) -> (String, String) {
    if let Some(src) = source {
        let alias = RelationAlias::new(src.scan_info.alias.as_deref()).execution(src.plan_position);
        (alias, field_name.to_string())
    } else {
        // Fallback: find the source by walking sources list
        for src in plan.sources() {
            if src.contains_rti(rti) {
                let alias =
                    RelationAlias::new(src.scan_info.alias.as_deref()).execution(src.plan_position);
                return (alias, field_name.to_string());
            }
        }
        // Should not happen — the planner validated all RTIs
        pgrx::warning!(
            "resolve_source_column: RTI {} not found in plan sources",
            rti
        );
        (format!("unknown_rti_{}", rti), field_name.to_string())
    }
}

/// Build a DataFusion column expression for an aggregate's field reference.
fn agg_field_col(agg: &JoinAggregateEntry, plan: &RelNode) -> Result<Expr> {
    let (rti, _attno, ref field_name) = agg.field_ref.as_ref().ok_or_else(|| {
        DataFusionError::Internal("non-COUNT(*) aggregate must have a field reference".to_string())
    })?;

    let source = plan.source_for_rti_in_subtree(*rti);
    let (alias, _) = resolve_source_column(source, *rti, field_name, plan);
    Ok(make_col(&alias, field_name))
}
