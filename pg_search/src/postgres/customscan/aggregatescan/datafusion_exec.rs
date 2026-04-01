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
use crate::postgres::customscan::joinscan::build::{JoinSource, RelNode, RelationAlias};
use crate::postgres::customscan::joinscan::translator::{build_equi_join_exprs, make_col};
use crate::scan::info::RowEstimate;
use crate::scan::PgSearchTableProvider;
use datafusion::common::{DataFusionError, JoinType, Result};
use datafusion::execution::context::{QueryPlanner, SessionState};
use datafusion::execution::session_state::SessionStateBuilder;
use datafusion::functions_aggregate::expr_fn::{avg, count, max, min, sum};
use datafusion::logical_expr::{lit, Expr};
use datafusion::physical_plan::coalesce_partitions::CoalescePartitionsExec;
use datafusion::physical_plan::{ExecutionPlan, ExecutionPlanProperties};
use datafusion::physical_planner::{DefaultPhysicalPlanner, PhysicalPlanner};
use datafusion::prelude::{DataFrame, SessionConfig, SessionContext};
use futures::future::{FutureExt, LocalBoxFuture};

use async_trait::async_trait;
use datafusion::physical_optimizer::filter_pushdown::FilterPushdown;

use crate::postgres::customscan::joinscan::planner::SortMergeJoinEnforcer;

/// Custom query planner for aggregate-on-join workloads.
///
/// Includes both `LateMaterializePlanner` and `VisibilityExtensionPlanner`
/// (same as JoinScan's `PgSearchQueryPlanner`). Visibility filtering is
/// required because `PgSearchTableProvider` scans return raw rows including
/// invisible (deleted-but-not-vacuumed) tuples — without the visibility
/// filter, aggregates like COUNT(*) would include dead rows.
#[derive(Debug)]
struct AggQueryPlanner;

#[async_trait]
impl QueryPlanner for AggQueryPlanner {
    async fn create_physical_plan(
        &self,
        logical_plan: &datafusion::logical_expr::LogicalPlan,
        session_state: &SessionState,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let extension_planners: Vec<
            Arc<dyn datafusion::physical_planner::ExtensionPlanner + Send + Sync>,
        > = vec![
            Arc::new(crate::scan::late_materialization::LateMaterializePlanner {}),
            Arc::new(
                crate::postgres::customscan::joinscan::visibility_filter::VisibilityExtensionPlanner::new(),
            ),
        ];
        let physical_planner = DefaultPhysicalPlanner::with_extension_planners(extension_planners);
        physical_planner
            .create_physical_plan(logical_plan, session_state)
            .await
    }
}

/// Create a DataFusion [`SessionContext`] for aggregate workloads.
///
/// Similar to JoinScan's `create_session_context()` but replaces
/// `SegmentedTopKRule` (row-level TopK) with `TopKAggregateRule`
/// (aggregate-level TopK). All other rules — including MVCC visibility
/// filtering — are preserved.
pub fn create_aggregate_session_context() -> SessionContext {
    use crate::postgres::customscan::joinscan::visibility_filter::VisibilityFilterOptimizerRule;
    use crate::scan::visibility_ctid_resolver_rule::VisibilityCtidResolverRule;

    let config = SessionConfig::new().with_target_partitions(1);

    let mut builder = SessionStateBuilder::new().with_config(config);

    // Inject visibility before late materialization so ctid lineage is analyzed
    // while DeferredCtid columns are still present in the logical plan.
    builder = builder
        .with_optimizer_rule(Arc::new(VisibilityFilterOptimizerRule::new()))
        .with_optimizer_rule(Arc::new(
            crate::scan::late_materialization::LateMaterializationRule,
        ));

    // SortMergeJoinEnforcer: converts HashJoinExec → SortMergeJoinExec when inputs are sorted
    if crate::gucs::is_columnar_sort_enabled() {
        builder = builder.with_physical_optimizer_rule(Arc::new(SortMergeJoinEnforcer::new()));
    }

    // Our custom query planner (includes VisibilityExtensionPlanner)
    builder = builder.with_query_planner(Arc::new(AggQueryPlanner {}));

    // VisibilityCtidResolverRule: wire ctid resolvers for visibility checks
    builder = builder.with_physical_optimizer_rule(Arc::new(VisibilityCtidResolverRule));

    // TopKAggregateRule: fuse sort + limit into TopK selection for aggregate output
    builder = builder.with_physical_optimizer_rule(Arc::new(
        crate::scan::topk_aggregate_rule::TopKAggregateRule,
    ));

    // FilterPushdown: push filters to PgSearchTableProvider
    builder =
        builder.with_physical_optimizer_rule(Arc::new(FilterPushdown::new_post_optimization()));

    SessionContext::new_with_state(builder.build())
}

/// Build the complete DataFusion logical plan for an aggregate-on-join query:
/// scan(s) → join → aggregate [→ sort → limit].
pub async fn build_join_aggregate_plan(
    plan: &RelNode,
    targetlist: &JoinAggregateTargetList,
    topk: Option<&crate::postgres::customscan::aggregatescan::privdat::DataFusionTopK>,
    ctx: &SessionContext,
) -> Result<datafusion::logical_expr::LogicalPlan> {
    // Step 1: Build the join DataFrame from the RelNode tree
    let df = build_relnode_df(ctx, plan).await?;

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
                AggKind::Sum => agg_field_col(agg, plan).map(sum),
                AggKind::Avg => agg_field_col(agg, plan).map(avg),
                AggKind::Min => agg_field_col(agg, plan).map(min),
                AggKind::Max => agg_field_col(agg, plan).map(max),
            }?;
            // Alias for stable reference
            Ok(agg_expr.alias(format!("agg_{}", i)))
        })
        .collect::<Result<Vec<Expr>>>()?;

    // Step 4: Apply aggregate
    let df = df.aggregate(group_exprs, agg_exprs)?;

    // Step 5: If TopK is requested, add sort + limit so DataFusion handles it internally
    if let Some(topk) = topk {
        let sort_col_name = format!("agg_{}", topk.sort_agg_idx);
        let sort_expr = datafusion::prelude::col(&sort_col_name).sort(!topk.descending, true);
        let df = df.sort(vec![sort_expr])?;
        let df = df.limit(0, Some(topk.k))?;
        return df.into_optimized_plan();
    }

    df.into_optimized_plan()
}

/// Build a DataFusion physical plan from a logical plan.
pub async fn build_aggregate_physical_plan(
    ctx: &SessionContext,
    logical_plan: datafusion::logical_expr::LogicalPlan,
) -> Result<Arc<dyn ExecutionPlan>> {
    let state = ctx.state();
    let plan = state
        .query_planner()
        .create_physical_plan(&logical_plan, &state)
        .await?;

    if plan.output_partitioning().partition_count() > 1 {
        Ok(Arc::new(CoalescePartitionsExec::new(plan)) as Arc<dyn ExecutionPlan>)
    } else {
        Ok(plan)
    }
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
                    unsupported => {
                        return Err(DataFusionError::NotImplemented(format!(
                            "Aggregate-on-join only supports INNER JOIN, got {}",
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
