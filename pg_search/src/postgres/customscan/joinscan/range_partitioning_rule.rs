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

use std::collections::HashMap;
use std::sync::Arc;

use datafusion::catalog::default_table_source::DefaultTableSource;
use datafusion::common::config::ConfigOptions;
use datafusion::common::tree_node::{Transformed, TreeNode, TreeNodeRecursion};
use datafusion::common::{Column, DataFusionError, Result};
use datafusion::logical_expr::{Expr, Join, LogicalPlan, TableScan};
use datafusion::optimizer::{OptimizerConfig, OptimizerRule, optimizer::ApplyOrder};
use datafusion::physical_optimizer::PhysicalOptimizerRule;
use datafusion::physical_plan::joins::{HashJoinExec, PartitionMode};
use datafusion::physical_plan::{ExecutionPlan, ExecutionPlanProperties, Partitioning};

use crate::api::FieldName;
use crate::index::fast_fields_helper::WhichFastField;
use crate::index::stats::persisted_split_points;
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::postgres::rel::PgSearchRelation;
use crate::scan::info::RowEstimate;
use crate::scan::range_partitioning::RangeSplitPoints;
use crate::scan::table_provider::PgSearchTableProvider;

/// Optimizer rule that coordinates range partitioning across joins in MPP execution.
///
/// # Background & Motivation
/// In distributed execution, joining partitioned tables without co-partitioning requires
/// either an expensive network shuffle (`NetworkShuffleExec`) of both inputs or broadcasting
/// (`NetworkBroadcastExec`) the build side to all worker tasks. When tables share identical
/// range split points on their join keys, the join can execute task-locally in `mode=Partitioned`
/// with zero network transfer.
///
/// # Tree-Oriented Bottom-Up Strategy
/// In relational query planning, joins execute bottom-up according to the join tree chosen by
/// the query planner. A base table's disk segments can only physically co-partition with whatever
/// table it joins *first* at the leaf level.
///
/// This rule traverses joins bottom-up (post-order), evaluating candidate join edges in execution order:
///
/// 1. When neither table is assigned:
///    - Symmetrically partitioned pairs: if one side is broadcast-eligible and its partner has
///      another non-broadcast co-partition candidate downstream, committing is deferred so the
///      partner can co-partition downstream while the smaller side broadcasts (`CollectLeft`),
///      achieving 0 network shuffles. Otherwise, both tables commit to shared split points
///      (0-shuffle task-local co-partitioning).
///    - Asymmetric pairs: stamp the partitioned table only if strictly larger than its partner
///      (`rows > partner_rows`). If smaller, it is not stamped to avoid forcing the larger partner
///      to shuffle (Scenario 5).
///
/// 2. When one table is already assigned and the other is unassigned:
///    - If the assigned table is partitioned on this join key: the unassigned table adopts the
///      existing split points (0-shuffle co-partitioning).
///    - If the assigned table was committed to a different key: the incoming stream must repartition
///      across the network anyway. If the partner table declared this key in `partition_by` or if
///      the unassigned table is strictly larger, the unassigned table is stamped with native split
///      points so it stays task-local (0 shuffles) while the stream range-adapts (1 shuffle total).
///
/// 3. When both tables are already assigned:
///    - Neither table can change; preserved as-is.
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

fn unwrap_column(expr: &Expr) -> Option<&Column> {
    match expr {
        Expr::Column(c) => Some(c),
        Expr::Cast(cast) => unwrap_column(&cast.expr),
        _ => None,
    }
}

/// Recursively searches the logical plan tree to find the underlying `PgSearchTableProvider`
/// and, if the referenced column matches one of the provider's declared partition keys, that
/// partition field name.
/// Resolves aliases, subqueries, projections, and intermediate joins.
fn resolve_column_to_provider<'a>(
    plan: &'a LogicalPlan,
    col: &Column,
) -> Option<(&'a PgSearchTableProvider, Option<FieldName>)> {
    match plan {
        LogicalPlan::TableScan(scan) => {
            if scan.projected_schema.has_column(col) {
                let (_, sf) = scan
                    .projected_schema
                    .qualified_field_from_column(col)
                    .ok()?;
                let provider = pg_search_provider_from_scan(scan)?;
                let partition_field = provider.scan_info.partition_by.iter().find_map(|field| {
                    if sf.name() == field.as_ref() {
                        Some(field.clone())
                    } else {
                        None
                    }
                });
                return Some((provider, partition_field));
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
            if let Some(c) = unwrap_column(unaliased) {
                resolve_column_to_provider(proj.input.as_ref(), c)
            } else {
                None
            }
        }
        LogicalPlan::Filter(filter) => resolve_column_to_provider(filter.input.as_ref(), col),
        LogicalPlan::Sort(sort) => resolve_column_to_provider(sort.input.as_ref(), col),
        LogicalPlan::Limit(limit) => resolve_column_to_provider(limit.input.as_ref(), col),
        LogicalPlan::SubqueryAlias(alias) => {
            let idx = alias.schema.index_of_column(col).ok()?;
            let (q, f) = alias.input.schema().qualified_field(idx);
            resolve_column_to_provider(alias.input.as_ref(), &Column::new(q.cloned(), f.name()))
        }
        LogicalPlan::Join(join) => {
            if join.left.schema().has_column(col) {
                resolve_column_to_provider(join.left.as_ref(), col)
            } else if join.right.schema().has_column(col) {
                resolve_column_to_provider(join.right.as_ref(), col)
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
                    resolve_column_to_provider(inputs[0], &Column::new(q.cloned(), f.name()))
                } else {
                    None
                }
            } else {
                None
            }
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

/// Returns whether the plan contains at least one `PgSearchTableProvider` configured for MPP
/// (`source_idx.is_some()`). Serial scans (`source_idx.is_none()`) never use range partitioning.
fn plan_has_mpp_provider(plan: &LogicalPlan) -> bool {
    let mut has_mpp = false;
    let _ = plan.apply(|node| {
        if let LogicalPlan::TableScan(scan) = node
            && let Some(provider) = pg_search_provider_from_scan(scan)
            && provider.source_idx().is_some()
        {
            has_mpp = true;
            return Ok(TreeNodeRecursion::Stop);
        }
        Ok(TreeNodeRecursion::Continue)
    });
    has_mpp
}

/// Returns whether the provider's row estimate is below the threshold for broadcast join
/// (`PartitionMode::CollectLeft`), matching DataFusion's `JoinSelection` rule.
fn is_broadcast_eligible(provider: &PgSearchTableProvider) -> bool {
    let threshold_rows = crate::gucs::hash_join_single_partition_threshold_rows();
    if threshold_rows <= 0 {
        return false;
    }
    match provider.scan_info.estimate {
        RowEstimate::Known(n) => n < threshold_rows as u64,
        RowEstimate::Unknown => false,
    }
}

/// Represents one side of an equi-join condition.
struct JoinSide<'a> {
    source_idx: usize,
    prov: &'a PgSearchTableProvider,
    field: Option<FieldName>,
}

/// An equi-join condition between two table providers in the join tree.
struct JoinEdge<'a> {
    join_idx: usize,
    left: JoinSide<'a>,
    right: JoinSide<'a>,
}

/// Collects all equi-join edges from the bottom-up join list.
fn collect_join_edges<'a>(joins: &[&'a Join]) -> Vec<JoinEdge<'a>> {
    let mut edges = Vec::new();
    for (join_idx, join) in joins.iter().enumerate() {
        for (l_expr, r_expr) in &join.on {
            let (Some(l_col), Some(r_col)) = (unwrap_column(l_expr), unwrap_column(r_expr)) else {
                continue;
            };
            let (Some((l_prov, l_field)), Some((r_prov, r_field))) = (
                resolve_column_to_provider(&join.left, l_col),
                resolve_column_to_provider(&join.right, r_col),
            ) else {
                continue;
            };
            let (Some(l_idx), Some(r_idx)) = (l_prov.source_idx(), r_prov.source_idx()) else {
                continue;
            };
            if l_idx == r_idx {
                continue;
            }
            edges.push(JoinEdge {
                join_idx,
                left: JoinSide {
                    source_idx: l_idx,
                    prov: l_prov,
                    field: l_field,
                },
                right: JoinSide {
                    source_idx: r_idx,
                    prov: r_prov,
                    field: r_field,
                },
            });
        }
    }
    edges
}

/// Returns whether the table identified by `source_idx` participates in another join edge (outside `current_join_idx`)
/// with a partner table that:
/// 1. Joins on columns declared in both tables' `partition_by` with matching Arrow types;
/// 2. Has persisted split points on at least one side;
/// 3. Is NOT broadcast-eligible (`!is_broadcast_eligible`).
fn has_non_broadcast_copartition_candidate(
    source_idx: usize,
    edges: &[JoinEdge<'_>],
    current_join_idx: usize,
) -> Result<bool> {
    for edge in edges {
        if edge.join_idx == current_join_idx {
            continue;
        }

        let (my_side, partner_side) = if edge.left.source_idx == source_idx {
            (&edge.left, &edge.right)
        } else if edge.right.source_idx == source_idx {
            (&edge.right, &edge.left)
        } else {
            continue;
        };

        let (Some(my_field), Some(partner_field)) = (&my_side.field, &partner_side.field) else {
            continue;
        };

        if is_broadcast_eligible(partner_side.prov) {
            continue;
        }

        let (Some(my_type), Some(partner_type)) = (
            named_field_arrow_type(my_side.prov, my_field),
            named_field_arrow_type(partner_side.prov, partner_field),
        ) else {
            continue;
        };
        if my_type != partner_type {
            continue;
        }

        if side_split_points(my_side.prov, my_field)?.is_none()
            && side_split_points(partner_side.prov, partner_field)?.is_none()
        {
            continue;
        }

        return Ok(true);
    }
    Ok(false)
}

/// Handles the case where one side of a join is already assigned split points, and the other is unassigned.
fn handle_one_assigned(
    assigned_side: &JoinSide<'_>,
    assigned_pts: &RangeSplitPoints,
    unassigned_side: &JoinSide<'_>,
    assigned: &mut HashMap<usize, RangeSplitPoints>,
) -> Result<()> {
    let Some(unassigned_field) = &unassigned_side.field else {
        return Ok(());
    };
    let unassigned_idx = unassigned_side.source_idx;

    if let Some(assigned_field) = &assigned_side.field
        && assigned_pts.partition_by == *assigned_field
        && let (Some(a_type), Some(u_type)) = (
            named_field_arrow_type(assigned_side.prov, assigned_field),
            named_field_arrow_type(unassigned_side.prov, unassigned_field),
        )
        && a_type == u_type
    {
        // Adopt split points from already partitioned partner (0 shuffles)
        assigned.insert(
            unassigned_idx,
            RangeSplitPoints {
                partition_by: unassigned_field.clone(),
                points: assigned_pts.points.clone(),
            },
        );
    } else {
        // Partner is partitioned on a different key (intermediate stream must repartition across the network),
        // or partner did not declare this key in partition_by.
        // Stamping the unassigned table allows it to remain task-local (0 shuffles) while the stream range-adapts.
        let partner_declared_key = assigned_side.field.is_some();
        let u_rows = unassigned_side
            .prov
            .scan_info
            .estimate
            .as_planner_estimate();
        let a_rows = assigned_side.prov.scan_info.estimate.as_planner_estimate();
        if (partner_declared_key || u_rows > a_rows)
            && let Some(points) = side_split_points(unassigned_side.prov, unassigned_field)?
        {
            assigned.insert(
                unassigned_idx,
                RangeSplitPoints {
                    partition_by: unassigned_field.clone(),
                    points,
                },
            );
        }
    }
    Ok(())
}

/// Processes a candidate join edge bottom-up, assigning split points to unassigned tables.
fn process_join_edge(
    edge: &JoinEdge<'_>,
    edges: &[JoinEdge<'_>],
    assigned: &mut HashMap<usize, RangeSplitPoints>,
) -> Result<()> {
    let l_idx = edge.left.source_idx;
    let r_idx = edge.right.source_idx;

    match (assigned.get(&l_idx), assigned.get(&r_idx)) {
        (None, None) => {
            let l_rows = edge.left.prov.scan_info.estimate.as_planner_estimate();
            let r_rows = edge.right.prov.scan_info.estimate.as_planner_estimate();

            match (&edge.left.field, &edge.right.field) {
                (Some(l_field), Some(r_field)) => {
                    // Symmetrically partitioned join:
                    // If one side is broadcast-eligible and its partner has a non-broadcast co-partition
                    // candidate downstream, defer committing so the partner can co-partition downstream
                    // while the smaller side broadcasts (CollectLeft) for 0 network shuffles.
                    let l_defer = is_broadcast_eligible(edge.left.prov)
                        && has_non_broadcast_copartition_candidate(r_idx, edges, edge.join_idx)?;
                    let r_defer = is_broadcast_eligible(edge.right.prov)
                        && has_non_broadcast_copartition_candidate(l_idx, edges, edge.join_idx)?;

                    if l_defer || r_defer {
                        return Ok(());
                    }

                    if let Some(points) =
                        compute_shared_points(edge.left.prov, l_field, edge.right.prov, r_field)?
                    {
                        assigned.insert(
                            l_idx,
                            RangeSplitPoints {
                                partition_by: l_field.clone(),
                                points: points.clone(),
                            },
                        );
                        assigned.insert(
                            r_idx,
                            RangeSplitPoints {
                                partition_by: r_field.clone(),
                                points,
                            },
                        );
                    }
                }
                (Some(l_field), None) => {
                    // Asymmetric join: stamp partitioned table if strictly larger (Scenario 4 vs 5)
                    if l_rows > r_rows
                        && let Some(points) = side_split_points(edge.left.prov, l_field)?
                    {
                        assigned.insert(
                            l_idx,
                            RangeSplitPoints {
                                partition_by: l_field.clone(),
                                points,
                            },
                        );
                    }
                }
                (None, Some(r_field)) => {
                    // Asymmetric join: stamp partitioned table if strictly larger (Scenario 4 vs 5)
                    if r_rows > l_rows
                        && let Some(points) = side_split_points(edge.right.prov, r_field)?
                    {
                        assigned.insert(
                            r_idx,
                            RangeSplitPoints {
                                partition_by: r_field.clone(),
                                points,
                            },
                        );
                    }
                }
                (None, None) => {}
            }
        }
        (Some(l_pts), None) => {
            let l_pts = l_pts.clone();
            handle_one_assigned(&edge.left, &l_pts, &edge.right, assigned)?;
        }
        (None, Some(r_pts)) => {
            let r_pts = r_pts.clone();
            handle_one_assigned(&edge.right, &r_pts, &edge.left, assigned)?;
        }
        (Some(_), Some(_)) => {}
    }
    Ok(())
}

/// Collects all `LogicalPlan::Join` nodes in post-order (bottom-up), ensuring that inner joins
/// are processed before outer joins.
fn collect_joins_bottom_up<'a>(plan: &'a LogicalPlan, joins: &mut Vec<&'a Join>) {
    for input in plan.inputs() {
        collect_joins_bottom_up(input, joins);
    }
    if let LogicalPlan::Join(join) = plan {
        joins.push(join);
    }
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
            || !plan_has_mpp_provider(&plan)
        {
            return Ok(Transformed::no(plan));
        }

        let mut joins = Vec::new();
        collect_joins_bottom_up(&plan, &mut joins);

        let edges = collect_join_edges(&joins);
        let mut assigned: HashMap<usize, RangeSplitPoints> = HashMap::new();

        for edge in &edges {
            process_join_edge(edge, &edges, &mut assigned)?;
        }

        if assigned.is_empty() {
            return Ok(Transformed::no(plan));
        }

        // Apply assigned range split points to the leaf TableScans
        plan.transform_up(|node| match node {
            LogicalPlan::TableScan(scan) => {
                let Some(provider) = pg_search_provider_from_scan(&scan) else {
                    return Ok(Transformed::no(LogicalPlan::TableScan(scan)));
                };
                let Some(source_idx) = provider.source_idx() else {
                    return Ok(Transformed::no(LogicalPlan::TableScan(scan)));
                };
                if let Some(points) = assigned.get(&source_idx) {
                    apply_split_points_to_scan(scan, points)
                } else {
                    Ok(Transformed::no(LogicalPlan::TableScan(scan)))
                }
            }
            _ => Ok(Transformed::no(node)),
        })
    }
}

/// Returns the arrow type of `field` when the provider exposes it as a named fast field.
fn named_field_arrow_type(
    provider: &PgSearchTableProvider,
    field: &FieldName,
) -> Option<arrow_schema::DataType> {
    provider.fields.iter().find_map(|f| match f {
        WhichFastField::Named { name, .. } if name == field.as_ref() => Some(f.arrow_data_type()),
        _ => None,
    })
}

/// Computes shared split points for two tables joining on matching types.
/// Returns `None` if types don't match or neither side has split points.
fn compute_shared_points(
    l_provider: &PgSearchTableProvider,
    l_field: &FieldName,
    r_provider: &PgSearchTableProvider,
    r_field: &FieldName,
) -> Result<Option<Vec<PdbOwnedValue>>> {
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
    Ok(Some(points))
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

/// Physical optimizer rule that converts a `CollectLeft` hash join to
/// `Partitioned` mode when both inputs declare compatible `Partitioning::Range`
/// layouts on the join keys.
///
/// A `CollectLeft` join materializes the entire build side in every consumer,
/// which the distributed planner satisfies by broadcasting it across tasks. When
/// both sides are range partitioned with identical split points, build-side
/// partition `i` can only ever match probe-side partition `i`, so `Partitioned`
/// mode joins each pair task-locally and the broadcast disappears.
///
/// That holds for every join type: whether a row matches (inner, semi, mark) or
/// has no match (outer, anti) is decided within its own partition. A null-aware
/// anti join (`NOT IN`) is the exception: one NULL key anywhere on the build side
/// must empty every task's result, and no task can see the other partitions.
///
/// A separate rule because `JoinSelection` picks `CollectLeft` from the build
/// side's row and byte statistics alone. It never consults `output_partitioning`,
/// so it can't see that these inputs are already co-partitioned and that the
/// repartition it's avoiding would cost nothing here. Declaring
/// `Partitioning::Range` on the scans only helps a join that is already
/// `Partitioned`, so the mode has to be revisited after the fact.
///
/// TODO: `JoinSelection` converts `PartitionMode::Auto` to `PartitionMode::CollectLeft` (broadcast)
/// purely from the build side's row count and byte statistics
/// (`hash_join_single_partition_threshold_rows`), without checking `output_partitioning`.
/// It assumes that `PartitionMode::Partitioned` always incurs 2 network shuffles. But when inputs
/// are physically co-partitioned (e.g. via `Partitioning::Range` with identical split points),
/// `PartitionMode::Partitioned` has 0 network cost (task-local join), whereas `CollectLeft` forces
/// an expensive broadcast across all worker tasks. Upstream `JoinSelection` should check whether
/// children already satisfy co-partitioning before degrading to `CollectLeft`.
/// See <https://github.com/apache/datafusion/issues/25301>
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
        if !crate::gucs::enable_range_partitioned_join()
            || !crate::postgres::customscan::mpp::glue::mpp_is_active()
        {
            return Ok(plan);
        }

        plan.transform_up(|node| {
            let Some(join) = node.downcast_ref::<HashJoinExec>() else {
                return Ok(Transformed::no(node));
            };
            if join.null_aware {
                return Ok(Transformed::no(node));
            }

            if join.partition_mode() == &PartitionMode::Partitioned {
                return Ok(Transformed::no(node));
            }

            let range_partitioned = |input: &Arc<dyn ExecutionPlan>| {
                matches!(input.output_partitioning(), Partitioning::Range(_))
                    && input.output_partitioning().partition_count() > 1
            };
            if !range_partitioned(join.left()) || !range_partitioned(join.right()) {
                return Ok(Transformed::no(node));
            }

            // Deliberately not `reset_state()`: it would drop the join's handle on the
            // dynamic filter that `FilterPushdown` already pushed into the probe scan,
            // leaving the scan holding a filter that nothing ever narrows. Switching the
            // mode invalidates the cached properties on its own.
            let candidate = join
                .builder()
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
