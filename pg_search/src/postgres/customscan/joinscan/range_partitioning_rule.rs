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
use std::collections::hash_map::Entry;
use std::sync::Arc;

use datafusion::catalog::default_table_source::DefaultTableSource;
use datafusion::common::config::ConfigOptions;
use datafusion::common::tree_node::{Transformed, TreeNode, TreeNodeRecursion};
use datafusion::common::{Column, DataFusionError, JoinType, Result};
use datafusion::logical_expr::{Expr, LogicalPlan, TableScan};
use datafusion::optimizer::{OptimizerConfig, OptimizerRule, optimizer::ApplyOrder};
use datafusion::physical_optimizer::PhysicalOptimizerRule;
use datafusion::physical_plan::joins::{HashJoinExec, PartitionMode};
use datafusion::physical_plan::{ExecutionPlan, ExecutionPlanProperties, Partitioning};

use pgrx::pg_sys;

use crate::api::FieldName;
use crate::index::fast_fields_helper::WhichFastField;
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::scan::range_partitioning::RangeSplitPoints;
use crate::scan::table_provider::PgSearchTableProvider;

/// Estimated row count of a table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TableRowCount(u64);

/// Priority of an equi-join edge between two tables, based on their estimated row counts.
///
/// Edges involving larger tables are prioritized so that co-partitioning avoids
/// expensive network shuffles or broadcasts on the largest data volumes.
///
/// - Primary criterion: `min(left_rows, right_rows)` (the size of the side that would
///   otherwise have to be broadcast or shuffled across tasks).
/// - Secondary criterion: `max(left_rows, right_rows)` (tie-breaker favoring the larger partner).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct EdgePriority {
    min_rows: TableRowCount,
    max_rows: TableRowCount,
}

impl EdgePriority {
    fn new(left_rows: u64, right_rows: u64) -> Self {
        let (min, max) = if left_rows < right_rows {
            (left_rows, right_rows)
        } else {
            (right_rows, left_rows)
        };
        Self {
            min_rows: TableRowCount(min),
            max_rows: TableRowCount(max),
        }
    }
}

/// A candidate equi-join edge between two `PgSearchTableProvider` leaf scans.
struct JoinEdge {
    priority: EdgePriority,
    l_rti: pg_sys::Index,
    l_field: FieldName,
    r_rti: pg_sys::Index,
    r_field: FieldName,
}

/// An asymmetric candidate where a table participates in an equi-join on one of its declared
/// partition keys, but its join partner does not share that partition key.
///
/// In distributed execution, DataFusion (via PR #24600) can adapt an unpartitioned input to match
/// a range-partitioned reference child. Stamping the LARGER table allows the large table to remain
/// local (0 shuffles) while the smaller partner is shuffled (1 shuffle total).
///
/// Conversely, stamping a SMALLER table when joining a larger unpartitioned table is counter-productive:
/// DataFusion would force the large table to repartition and shuffle across the network to match the
/// small table's split points.
struct AsymmetricCandidate {
    anchor_rti: pg_sys::Index,
    anchor_field: FieldName,
    anchor_rows: u64,
    partner_rti: pg_sys::Index,
    partner_rows: u64,
}

/// Optimizer rule that coordinates range partitioning across joins in MPP execution.
///
/// # Background & Motivation
/// In distributed execution, joining partitioned tables without co-partitioning requires
/// either an expensive network shuffle (`NetworkShuffleExec`) of both inputs or broadcasting
/// (`NetworkBroadcastExec`) the build side to all worker tasks. When tables share identical
/// range split points on their join keys, the join can execute task-locally in `mode=Partitioned`
/// with zero network transfer.
///
/// # The Multi-Key Challenge
/// Tables in ParadeDB can declare multiple partition keys in their index configuration
/// (e.g., a bridge table like `posts` with `partition_by = 'user_id,topic_id'`). Because each
/// index segment is physically range-partitioned along one dimension, a table scan can only
/// declare one `Partitioning::Range` layout at runtime.
///
/// If partition keys were assigned purely on a first-come-first-served basis during a bottom-up
/// join tree traversal, a small dimension join (e.g., `posts JOIN topics` where `topics` has 10 rows)
/// encountered early in the plan could claim `posts`'s partition key on `topic_id`. This would
/// lock `posts` out of co-partitioning with a large table (e.g., `users` with millions of rows)
/// on `user_id`, forcing the query to broadcast or shuffle the massive tables while saving almost
/// nothing on the tiny dimension table.
///
/// # High-Level Strategy
/// To guarantee that partitioning is preserved for the largest tables regardless of join tree
/// shape or query order (without modifying the join tree structure itself):
///
/// 1. **Collection**: Traverses the `LogicalPlan` to identify candidate equi-join edges `(T1.k1 = T2.k2)`:
///    - Symmetrically partitioned pairs (both tables declare matching keys in `partition_by`).
///    - Asymmetric pairs where only one table has a declared partition key.
/// 2. **Prioritization**: Ranks candidate join edges by data volume:
///    - Primary key: `min(T1.rows, T2.rows)`, representing the data volume saved from network
///      broadcast/shuffle.
///    - Secondary key: `max(T1.rows, T2.rows)`, tie-breaking in favor of the larger partner.
/// 3. **Greedy Assignment**: In descending priority order, commits table scans to partition keys
///    and split points:
///    - When an edge `(T1, T2)` is selected, both tables adopt shared split points derived from
///      the larger of the two tables (minimizing segment skew on the heavier side).
///    - If a table joins multiple peers on the same key (e.g., `T1 JOIN T2 ON x JOIN T3 ON x`),
///      subsequent tables adopt the already-established split points.
///    - A table committed to a key on a higher-priority edge cannot be overwritten by a lower-priority
///      edge on a different key.
/// 4. **Asymmetric Anchor Stamping**: For joins that cannot be co-partitioned, unassigned tables
///    that participate on a declared partition key are stamped if they are strictly larger
///    than their join partner (`anchor_rows > partner_rows`), OR if the partner is already committed
///    to a different partition key (guaranteeing that the partner stream must be repartitioned anyway).
///    This designates the anchor table as the range anchor, allowing DataFusion (via PR #24600 / #24766)
///    to keep the anchor table local (0 shuffles) while shuffling only the partner (1 shuffle total).
/// 5. **Rewriting**: Directly applies the assigned split points to the leaf `TableScan`s in a single
///    pass, leaving the join tree order and node hierarchy intact.
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

        let mut join_edges: Vec<JoinEdge> = Vec::new();
        let mut asymmetric_candidates: Vec<AsymmetricCandidate> = Vec::new();
        let mut providers: HashMap<pg_sys::Index, PgSearchTableProvider> = HashMap::new();

        plan.apply(|node| {
            if let LogicalPlan::Join(join) = node {
                for (l_expr, r_expr) in &join.on {
                    if let (Some(l_col), Some(r_col)) =
                        (unwrap_column(l_expr), unwrap_column(r_expr))
                    {
                        let l_res = resolve_column_to_provider(&join.left, l_col);
                        let r_res = resolve_column_to_provider(&join.right, r_col);
                        if let (Some((l_prov, l_field_opt)), Some((r_prov, r_field_opt))) =
                            (l_res, r_res)
                        {
                            if l_prov.source_idx().is_none() || r_prov.source_idx().is_none() {
                                return Ok(TreeNodeRecursion::Continue);
                            }
                            let l_rti = l_prov.scan_info.heap_rti;
                            let r_rti = r_prov.scan_info.heap_rti;
                            providers.entry(l_rti).or_insert_with(|| l_prov.clone());
                            providers.entry(r_rti).or_insert_with(|| r_prov.clone());
                            let l_rows = l_prov.scan_info.estimate.as_planner_estimate();
                            let r_rows = r_prov.scan_info.estimate.as_planner_estimate();

                            match (l_field_opt, r_field_opt) {
                                (Some(l_field), Some(r_field)) => {
                                    if let (Some(l_type), Some(r_type)) = (
                                        named_field_arrow_type(l_prov, &l_field),
                                        named_field_arrow_type(r_prov, &r_field),
                                    ) && l_type == r_type
                                        && l_rti != r_rti
                                    {
                                        join_edges.push(JoinEdge {
                                            priority: EdgePriority::new(l_rows, r_rows),
                                            l_rti,
                                            l_field: l_field.clone(),
                                            r_rti,
                                            r_field: r_field.clone(),
                                        });
                                    }

                                    asymmetric_candidates.push(AsymmetricCandidate {
                                        anchor_rti: l_rti,
                                        anchor_field: l_field,
                                        anchor_rows: l_rows,
                                        partner_rti: r_rti,
                                        partner_rows: r_rows,
                                    });
                                    asymmetric_candidates.push(AsymmetricCandidate {
                                        anchor_rti: r_rti,
                                        anchor_field: r_field,
                                        anchor_rows: r_rows,
                                        partner_rti: l_rti,
                                        partner_rows: l_rows,
                                    });
                                }
                                (Some(l_field), None) => {
                                    asymmetric_candidates.push(AsymmetricCandidate {
                                        anchor_rti: l_rti,
                                        anchor_field: l_field,
                                        anchor_rows: l_rows,
                                        partner_rti: r_rti,
                                        partner_rows: r_rows,
                                    });
                                }
                                (None, Some(r_field)) => {
                                    asymmetric_candidates.push(AsymmetricCandidate {
                                        anchor_rti: r_rti,
                                        anchor_field: r_field,
                                        anchor_rows: r_rows,
                                        partner_rti: l_rti,
                                        partner_rows: l_rows,
                                    });
                                }
                                (None, None) => {}
                            }
                        }
                    }
                }
            }
            Ok(TreeNodeRecursion::Continue)
        })?;

        if join_edges.is_empty() && asymmetric_candidates.is_empty() {
            return Ok(Transformed::no(plan));
        }

        // Sort candidate edges in descending order of data volume
        join_edges.sort_by_key(|b| std::cmp::Reverse(b.priority));

        let mut assigned: HashMap<pg_sys::Index, RangeSplitPoints> = HashMap::new();

        for edge in join_edges {
            let l_assigned = assigned.get(&edge.l_rti);
            let r_assigned = assigned.get(&edge.r_rti);

            match (l_assigned, r_assigned) {
                (None, None) => {
                    let l_prov = &providers[&edge.l_rti];
                    let r_prov = &providers[&edge.r_rti];
                    if let Some(points) =
                        compute_shared_points(l_prov, &edge.l_field, r_prov, &edge.r_field)?
                    {
                        assigned.insert(
                            edge.l_rti,
                            RangeSplitPoints {
                                partition_by: edge.l_field,
                                points: points.clone(),
                            },
                        );
                        assigned.insert(
                            edge.r_rti,
                            RangeSplitPoints {
                                partition_by: edge.r_field,
                                points,
                            },
                        );
                    }
                }
                (Some(l_pts), None) => {
                    if l_pts.partition_by == edge.l_field {
                        assigned.insert(
                            edge.r_rti,
                            RangeSplitPoints {
                                partition_by: edge.r_field,
                                points: l_pts.points.clone(),
                            },
                        );
                    }
                }
                (None, Some(r_pts)) => {
                    if r_pts.partition_by == edge.r_field {
                        assigned.insert(
                            edge.l_rti,
                            RangeSplitPoints {
                                partition_by: edge.l_field,
                                points: r_pts.points.clone(),
                            },
                        );
                    }
                }
                (Some(_), Some(_)) => {}
            }
        }

        // Tier 2: Asymmetric anchor stamping.
        //
        // TODO(upstream DataFusion / PR #24600): In DataFusion's `enforce_distribution_relationships`,
        // when `satisfied_children.len() == 1`, that single satisfied child is chosen as the reference
        // partitioning regardless of size (`PlanSize` tie-breaking only runs when `satisfied_children.len() > 1`).
        // Consequently, if a tiny dimension table (e.g. 10 rows) is range-partitioned while its large join
        // partner (e.g. 10M rows) is unpartitioned on that key, DataFusion treats the tiny table as the reference
        // and forces the massive partner to be repartitioned and shuffled across the network to match the tiny table's
        // split points. Until upstream DataFusion considers the size of unsatisfied children before adapting them
        // to a satisfied reference child, we must never range-partition the smaller side of an asymmetric join.
        // See DataFusion issue #25302: https://github.com/apache/datafusion/issues/25302
        //
        // For joins that cannot be co-partitioned, we range-partition an anchor table if:
        // 1. The anchor is strictly larger than its partner (anchor_rows > partner_rows), OR
        // 2. The partner is already committed to a DIFFERENT partition key in `assigned`. In this
        //    case, the partner's stream cannot be partition-aligned on this join key anyway and
        //    will inevitably be repartitioned/shuffled across the network. Stamping the anchor
        //    allows the anchor to stay local in its native range partitions (0 shuffles) while the
        //    partner's stream adapts to it (1 shuffle total), avoiding a 2-sided hash shuffle.
        asymmetric_candidates.sort_by_key(|c| {
            (
                std::cmp::Reverse(c.anchor_rows),
                std::cmp::Reverse(c.partner_rows),
            )
        });
        for cand in asymmetric_candidates {
            let partner_committed_elsewhere = assigned
                .get(&cand.partner_rti)
                .is_some_and(|pts| pts.partition_by != cand.anchor_field);

            if (cand.anchor_rows > cand.partner_rows || partner_committed_elsewhere)
                && let Entry::Vacant(e) = assigned.entry(cand.anchor_rti)
                && let Some(prov) = providers.get(&cand.anchor_rti)
                && let Some(points) = side_split_points(prov, &cand.anchor_field)?
            {
                e.insert(RangeSplitPoints {
                    partition_by: cand.anchor_field,
                    points,
                });
            }
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
                if let Some(points) = assigned.get(&provider.scan_info.heap_rti) {
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
        WhichFastField::Named(name, sft) if name == field.as_ref() => Some(sft.arrow_data_type()),
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
    provider
        .persisted_split_points(partition_by.as_ref())
        .map_err(|e| DataFusionError::Internal(format!("Failed to read segment statistics: {e}")))
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
            if join.join_type() != &JoinType::Inner {
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
