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

use std::sync::Arc;

use datafusion::catalog::default_table_source::DefaultTableSource;
use datafusion::common::config::ConfigOptions;
use datafusion::common::tree_node::{Transformed, TreeNode};
use datafusion::common::{Column, DataFusionError, JoinType, Result};
use datafusion::logical_expr::{Expr, LogicalPlan, TableScan};
use datafusion::optimizer::{OptimizerConfig, OptimizerRule, optimizer::ApplyOrder};
use datafusion::physical_optimizer::PhysicalOptimizerRule;
use datafusion::physical_plan::coalesce_partitions::CoalescePartitionsExec;
use datafusion::physical_plan::joins::{HashJoinExec, PartitionMode};
use datafusion::physical_plan::repartition::RepartitionExec;
use datafusion::physical_plan::{ExecutionPlan, ExecutionPlanProperties, Partitioning};

use crate::api::FieldName;
use crate::index::fast_fields_helper::{FieldDelivery, WhichFastField};
use crate::index::stats::persisted_split_points;
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::postgres::rel::PgSearchRelation;
use crate::scan::range_partitioning::RangeSplitPoints;
use crate::scan::table_provider::PgSearchTableProvider;

/// Optimizer rule that coordinates range partitioning across a join.
///
/// It detects when both sides of an equi-join are partitioned on matching column types,
/// takes the split points a partitioned build stamped on either side, and injects them
/// into the `PgSearchTableProvider`s of both sides. This guarantees that both sides of
/// the join produce identical partition boundaries during MPP execution.
#[derive(Debug, Default)]
pub struct RangePartitioningRule;

impl RangePartitioningRule {
    pub fn new() -> Self {
        Self
    }
}

fn pg_search_provider_from_scan(scan: &TableScan) -> Option<&PgSearchTableProvider> {
    let source = scan.source.as_ref();
    if let Some(default_source) = source.downcast_ref::<DefaultTableSource>() {
        default_source
            .table_provider
            .downcast_ref::<PgSearchTableProvider>()
    } else {
        None
    }
}

/// Recursively searches the logical plan tree to find the underlying `PgSearchTableProvider`
/// and the original index field name that corresponds to a given `Column` expression at the top level
/// of the plan. This resolves aliases, subqueries, and projections to trace a join key back to its source.
fn find_provider_for_column<'a>(
    plan: &'a LogicalPlan,
    col: &Column,
) -> Option<(&'a PgSearchTableProvider, FieldName)> {
    match plan {
        LogicalPlan::TableScan(scan) => {
            if scan.projected_schema.has_column(col) {
                let (_, sf) = scan
                    .projected_schema
                    .qualified_field_from_column(col)
                    .ok()?;
                let provider = pg_search_provider_from_scan(scan)?;
                for field in &provider.scan_info.partition_by {
                    if sf.name() == field.as_ref() {
                        return Some((provider, field.clone()));
                    }
                }
            }
            None
        }
        LogicalPlan::Projection(proj) => {
            let idx = proj.schema.index_of_column(col).ok()?;
            let expr = &proj.expr[idx];
            let unaliased = match expr {
                Expr::Alias(alias) => alias.expr.as_ref(),
                e => e,
            };
            if let Expr::Column(c) = unaliased {
                find_provider_for_column(proj.input.as_ref(), c)
            } else {
                None
            }
        }
        LogicalPlan::Filter(filter) => find_provider_for_column(filter.input.as_ref(), col),
        LogicalPlan::Sort(sort) => find_provider_for_column(sort.input.as_ref(), col),
        LogicalPlan::Limit(limit) => find_provider_for_column(limit.input.as_ref(), col),
        LogicalPlan::SubqueryAlias(alias) => {
            let idx = alias.schema.index_of_column(col).ok()?;
            let (q, f) = alias.input.schema().qualified_field(idx);
            find_provider_for_column(alias.input.as_ref(), &Column::new(q.cloned(), f.name()))
        }
        LogicalPlan::Join(join) => {
            if join.left.schema().has_column(col) {
                find_provider_for_column(join.left.as_ref(), col)
            } else if join.right.schema().has_column(col) {
                find_provider_for_column(join.right.as_ref(), col)
            } else {
                None
            }
        }
        _ => {
            let inputs = plan.inputs();
            if inputs.len() == 1 {
                let idx = plan.schema().index_of_column(col).ok()?;
                let input_schema = inputs[0].schema();
                if idx < input_schema.fields().len() {
                    let (q, f) = input_schema.qualified_field(idx);
                    find_provider_for_column(inputs[0], &Column::new(q.cloned(), f.name()))
                } else {
                    None
                }
            } else {
                None
            }
        }
    }
}

/// Recursively traverses the logical plan to locate the specific `TableScan` node
/// that produces the given `Column`. Applies the mutating function `f` to that scan
/// and rebuilds the plan tree to reflect the transformation.
fn map_scan_for_column<F>(
    plan: LogicalPlan,
    col: &Column,
    f: &mut F,
) -> Result<Transformed<LogicalPlan>>
where
    F: FnMut(TableScan) -> Result<Transformed<LogicalPlan>>,
{
    match plan {
        LogicalPlan::TableScan(scan) => {
            if scan.projected_schema.has_column(col) {
                f(scan)
            } else {
                Ok(Transformed::no(LogicalPlan::TableScan(scan)))
            }
        }
        LogicalPlan::Projection(mut proj) => {
            if let Ok(idx) = proj.schema.index_of_column(col) {
                let expr = &proj.expr[idx];
                let unaliased = match expr {
                    Expr::Alias(alias) => alias.expr.as_ref(),
                    e => e,
                };
                if let Expr::Column(c) = unaliased {
                    let res = map_scan_for_column(Arc::unwrap_or_clone(proj.input.clone()), c, f)?;
                    if res.transformed {
                        proj.input = Arc::new(res.data);
                        return Ok(Transformed::yes(LogicalPlan::Projection(proj)));
                    }
                }
            }
            Ok(Transformed::no(LogicalPlan::Projection(proj)))
        }
        LogicalPlan::Filter(mut filter) => {
            let res = map_scan_for_column(Arc::unwrap_or_clone(filter.input.clone()), col, f)?;
            if res.transformed {
                filter.input = Arc::new(res.data);
                Ok(Transformed::yes(LogicalPlan::Filter(filter)))
            } else {
                Ok(Transformed::no(LogicalPlan::Filter(filter)))
            }
        }
        LogicalPlan::Sort(mut sort) => {
            let res = map_scan_for_column(Arc::unwrap_or_clone(sort.input.clone()), col, f)?;
            if res.transformed {
                sort.input = Arc::new(res.data);
                Ok(Transformed::yes(LogicalPlan::Sort(sort)))
            } else {
                Ok(Transformed::no(LogicalPlan::Sort(sort)))
            }
        }
        LogicalPlan::Limit(mut limit) => {
            let res = map_scan_for_column(Arc::unwrap_or_clone(limit.input.clone()), col, f)?;
            if res.transformed {
                limit.input = Arc::new(res.data);
                Ok(Transformed::yes(LogicalPlan::Limit(limit)))
            } else {
                Ok(Transformed::no(LogicalPlan::Limit(limit)))
            }
        }
        LogicalPlan::SubqueryAlias(mut alias) => {
            if let Ok(idx) = alias.schema.index_of_column(col) {
                let (q, field) = alias.input.schema().qualified_field(idx);
                let inner_col = Column::new(q.cloned(), field.name());
                let res =
                    map_scan_for_column(Arc::unwrap_or_clone(alias.input.clone()), &inner_col, f)?;
                if res.transformed {
                    alias.input = Arc::new(res.data);
                    return Ok(Transformed::yes(LogicalPlan::SubqueryAlias(alias)));
                }
            }
            Ok(Transformed::no(LogicalPlan::SubqueryAlias(alias)))
        }
        LogicalPlan::Join(mut join) => {
            if join.left.schema().has_column(col) {
                let res = map_scan_for_column(Arc::unwrap_or_clone(join.left.clone()), col, f)?;
                if res.transformed {
                    join.left = Arc::new(res.data);
                    return Ok(Transformed::yes(LogicalPlan::Join(join)));
                }
            } else if join.right.schema().has_column(col) {
                let res = map_scan_for_column(Arc::unwrap_or_clone(join.right.clone()), col, f)?;
                if res.transformed {
                    join.right = Arc::new(res.data);
                    return Ok(Transformed::yes(LogicalPlan::Join(join)));
                }
            }
            Ok(Transformed::no(LogicalPlan::Join(join)))
        }
        _ => {
            let inputs = plan.inputs();
            if inputs.len() == 1
                && let Ok(idx) = plan.schema().index_of_column(col)
            {
                let input_schema = inputs[0].schema();
                if idx < input_schema.fields().len() {
                    let (q, field) = input_schema.qualified_field(idx);
                    let inner_col = Column::new(q.cloned(), field.name());

                    return plan.map_children(|child| map_scan_for_column(child, &inner_col, f));
                }
            }
            Ok(Transformed::no(plan))
        }
    }
}

fn apply_split_points_to_scan(
    mut scan: TableScan,
    points: &RangeSplitPoints,
) -> Result<Transformed<LogicalPlan>> {
    let Some(provider) = pg_search_provider_from_scan(&scan) else {
        return Ok(Transformed::no(LogicalPlan::TableScan(scan)));
    };

    if provider.range_split_points() == Some(points) {
        return Ok(Transformed::no(LogicalPlan::TableScan(scan)));
    }

    let mut new_provider = provider.clone();
    new_provider.with_range_partitioning(Some(points.clone()));

    let new_source = Arc::new(DefaultTableSource::new(Arc::new(new_provider)))
        as Arc<dyn datafusion::logical_expr::TableSource>;

    scan.source = new_source;
    Ok(Transformed::yes(LogicalPlan::TableScan(scan)))
}

impl OptimizerRule for RangePartitioningRule {
    fn name(&self) -> &str {
        "RangePartitioningRule"
    }

    fn apply_order(&self) -> Option<ApplyOrder> {
        None
    }

    fn rewrite(
        &self,
        plan: LogicalPlan,
        _config: &dyn OptimizerConfig,
    ) -> Result<Transformed<LogicalPlan>> {
        if !crate::gucs::enable_range_partitioned_join()
            || !crate::postgres::customscan::mpp::glue::mpp_is_active()
        {
            return Ok(Transformed::no(plan));
        }

        plan.transform_up(|node| {
            if let LogicalPlan::Join(mut join) = node {
                let mut transformed = false;
                for (l, r) in &join.on {
                    if let (Expr::Column(l_col), Expr::Column(r_col)) = (l, r) {
                        let l_res = find_provider_for_column(&join.left, l_col);
                        let r_res = find_provider_for_column(&join.right, r_col);
                        if let (
                            Some((l_provider, l_field_name)),
                            Some((r_provider, r_field_name)),
                        ) = (l_res, r_res)
                        {
                            let points = merged_split_points(
                                l_provider,
                                &l_field_name,
                                r_provider,
                                &r_field_name,
                            )?;

                            if let Some(mut shared_points) = points {
                                shared_points.partition_by = l_field_name;
                                let left_res = map_scan_for_column(
                                    Arc::unwrap_or_clone(join.left.clone()),
                                    l_col,
                                    &mut |scan| apply_split_points_to_scan(scan, &shared_points),
                                )?;
                                if left_res.transformed {
                                    join.left = Arc::new(left_res.data);
                                }

                                shared_points.partition_by = r_field_name;
                                let right_res = map_scan_for_column(
                                    Arc::unwrap_or_clone(join.right.clone()),
                                    r_col,
                                    &mut |scan| apply_split_points_to_scan(scan, &shared_points),
                                )?;
                                if right_res.transformed {
                                    join.right = Arc::new(right_res.data);
                                }
                                transformed =
                                    transformed || left_res.transformed || right_res.transformed;
                            }
                        }
                    }
                }
                if transformed {
                    return Ok(Transformed::yes(LogicalPlan::Join(join)));
                } else {
                    return Ok(Transformed::no(LogicalPlan::Join(join)));
                }
            }
            Ok(Transformed::no(node))
        })
    }
}

/// Returns the arrow type of `field` when the provider exposes it as a named fast field.
fn named_field_arrow_type(
    provider: &PgSearchTableProvider,
    field: &FieldName,
) -> Option<arrow_schema::DataType> {
    provider.fields.iter().find_map(|f| match f {
        WhichFastField::Named {
            name,
            delivery: FieldDelivery::Eager,
            ..
        } if name == field.as_ref() => Some(f.arrow_data_type()),
        _ => None,
    })
}

/// The split points both sides of the join cut on.
///
/// Returns `None` when neither side has any, or when the two key columns expose different
/// arrow types and one set could not describe both sides faithfully. Any split points
/// partition a side correctly, and its segments are placed by their own statistics, so one
/// side's points serve both: the larger side's, since that is the scan alignment saves the
/// most on. A union of both sides' points would turn every nearly shared edge into a sliver
/// partition.
fn merged_split_points(
    l_provider: &PgSearchTableProvider,
    l_field: &FieldName,
    r_provider: &PgSearchTableProvider,
    r_field: &FieldName,
) -> Result<Option<RangeSplitPoints>> {
    let (Some(l_type), Some(r_type)) = (
        named_field_arrow_type(l_provider, l_field),
        named_field_arrow_type(r_provider, r_field),
    ) else {
        return Ok(None);
    };
    if l_type != r_type {
        return Ok(None);
    }

    let points = match (
        side_split_points(l_provider, l_field)?,
        side_split_points(r_provider, r_field)?,
    ) {
        (Some(l_points), Some(r_points)) => {
            let l_rows = l_provider.scan_info.estimate.as_planner_estimate();
            let r_rows = r_provider.scan_info.estimate.as_planner_estimate();
            if r_rows > l_rows { r_points } else { l_points }
        }
        (Some(points), None) | (None, Some(points)) => points,
        (None, None) => return Ok(None),
    };
    Ok(Some(RangeSplitPoints {
        partition_by: l_field.clone(),
        points,
    }))
}

/// The split points a partitioned build stamped on the side's segments, sorted ascending, or
/// `None` for an index without any.
fn side_split_points(
    provider: &PgSearchTableProvider,
    partition_by: &FieldName,
) -> Result<Option<Vec<PdbOwnedValue>>> {
    let index_rel = PgSearchRelation::open(provider.scan_info.indexrelid);
    persisted_split_points(&index_rel, partition_by.as_ref())
        .map_err(|e| DataFusionError::Internal(format!("Failed to read segment statistics: {e}")))
}

/// The input of a round-robin `RepartitionExec` over a range-partitioned plan, or `plan`
/// itself.
fn peel_round_robin(plan: Arc<dyn ExecutionPlan>) -> Arc<dyn ExecutionPlan> {
    let Some(repartition) = plan.downcast_ref::<RepartitionExec>() else {
        return plan;
    };
    let lifts_range = matches!(repartition.partitioning(), Partitioning::RoundRobinBatch(_))
        && matches!(
            repartition.input().output_partitioning(),
            Partitioning::Range(_)
        );
    if lifts_range {
        Arc::clone(repartition.input())
    } else {
        plan
    }
}

/// Physical optimizer rule that converts a `CollectLeft` inner hash join to
/// `Partitioned` mode when both inputs declare compatible `Partitioning::Range`
/// layouts on the join keys.
///
/// A `CollectLeft` join materializes the entire build side in every consumer,
/// which the distributed planner satisfies by broadcasting it across tasks. When
/// both sides are range partitioned with identical split points, build-side
/// partition `i` can only ever match probe-side partition `i`, so `Partitioned`
/// mode joins each pair task-locally and the broadcast disappears.
///
/// A separate rule because `JoinSelection` picks `CollectLeft` from the build
/// side's row and byte statistics alone. It never consults `output_partitioning`,
/// so it can't see that these inputs are already co-partitioned and that the
/// repartition it's avoiding would cost nothing here. Declaring
/// `Partitioning::Range` on the scans only helps a join that is already
/// `Partitioned`, so the mode has to be revisited after the fact. Runs after
/// `EnsureRequirements` has resolved `PartitionMode::Auto` and inserted the build
/// side's `CoalescePartitionsExec`.
#[derive(Debug, Default)]
pub struct RangeCoPartitionedJoinRule;

impl PhysicalOptimizerRule for RangeCoPartitionedJoinRule {
    fn name(&self) -> &str {
        "RangeCoPartitionedJoinRule"
    }

    fn schema_check(&self) -> bool {
        true
    }

    fn optimize(
        &self,
        plan: Arc<dyn ExecutionPlan>,
        _config: &ConfigOptions,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        if !crate::gucs::enable_range_partitioned_join() {
            return Ok(plan);
        }

        plan.transform_up(|node| {
            let Some(join) = node.downcast_ref::<HashJoinExec>() else {
                return Ok(Transformed::no(node));
            };
            // `HashJoinExec` enables `allow_range_satisfaction_for_key_partitioning`
            // only for Partitioned inner joins; flipping any other shape would make
            // `EnforceDistribution` re-introduce hash repartitions on both sides.
            if join.partition_mode() != &PartitionMode::CollectLeft
                || join.join_type() != &JoinType::Inner
            {
                return Ok(Transformed::no(node));
            }

            // EnforceDistribution collapses the build side to a single partition for
            // CollectLeft; peel that off to recover the source's declared partitioning.
            let left = match join.left().downcast_ref::<CoalescePartitionsExec>() {
                Some(coalesce) if coalesce.fetch().is_none() => Arc::clone(coalesce.input()),
                _ => Arc::clone(join.left()),
            };
            // A range-partitioned scan seats only as many partitions as it has split points.
            // When the session asks for more, EnforceDistribution lifts the scan with a
            // round-robin repartition, which throws the range layout away. Peel it too: a
            // co-partitioned join on fewer tasks beats a broadcast on more.
            let left = peel_round_robin(left);
            let right = peel_round_robin(Arc::clone(join.right()));

            let range_partitioned = |input: &Arc<dyn ExecutionPlan>| {
                matches!(input.output_partitioning(), Partitioning::Range(_))
                    && input.output_partitioning().partition_count() > 1
            };
            if !range_partitioned(&left) || !range_partitioned(&right) {
                return Ok(Transformed::no(node));
            }

            // Deliberately not `reset_state()`: it would drop the join's handle on the
            // dynamic filter that `FilterPushdown` already pushed into the probe scan,
            // leaving the scan holding a filter that nothing ever narrows. Switching the
            // mode invalidates the cached properties on its own.
            let candidate = join
                .builder()
                .with_new_children(vec![left, right])?
                .with_partition_mode(PartitionMode::Partitioned)
                .build_exec()?;

            // Keep the flip only when DataFusion agrees the inputs are co-partitioned:
            // the join keys match the range keys through equivalences and the split
            // points are identical on both sides. Anything else keeps the CollectLeft
            // join (and its broadcast) untouched.
            let children: Vec<&dyn ExecutionPlan> = candidate
                .children()
                .into_iter()
                .map(|child| child.as_ref())
                .collect();
            let co_partitioned = candidate
                .input_distribution_requirements()
                .unsatisfied_co_partitioned_children(candidate.name(), &children)?
                .is_empty();
            if co_partitioned {
                Ok(Transformed::yes(candidate))
            } else {
                Ok(Transformed::no(node))
            }
        })
        .map(|transformed| transformed.data)
    }
}
