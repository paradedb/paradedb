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

//! MPP plan-shape classifier + per-shape physical-plan builders.
//!
//! Different query shapes want different MPP topologies — trying to force one
//! plan structure across all of them breaks correctness (the earlier
//! `build_mpp_aggregate_plan` baked in a post-Partial-shuffle topology that
//! produced spurious rows for scalar aggregation). This module classifies the
//! query, dispatches to the right builder, and each builder composes only the
//! nodes it needs.
//!
//! # Supported shapes
//!
//! [`MppPlanShape::ScalarAggOnBinaryJoin`] — e.g., `SELECT COUNT(*) FROM f
//! JOIN p WHERE ...`. Per worker: pre-join shuffle of each side → HashJoin →
//! AggregateExec(Partial). Partial rows are shipped to PG's Gather and
//! finalized by PG's native `Finalize Aggregate` above. No post-Partial
//! shuffle is inserted (it would produce spurious all-NULL rows for scalar
//! aggregation on non-hit workers).
//!
//! [`MppPlanShape::GroupByAggOnBinaryJoin`] — e.g., `SELECT k, COUNT(*) FROM
//! f JOIN p GROUP BY k`. Per worker: pre-join shuffle → HashJoin →
//! AggregateExec(Partial) → post-Partial shuffle on group keys →
//! AggregateExec(FinalPartitioned). Each worker emits final rows for its
//! group-key hash partition; PG's Gather concatenates — no double-count
//! because every group lives on exactly one worker.
//!
//! [`MppPlanShape::GroupByAggSingleTable`] — `SELECT k, COUNT(*) FROM t
//! GROUP BY k`. No join; Partial → post-Partial shuffle → FinalPartitioned.
//!
//! [`MppPlanShape::JoinOnly`] — `SELECT ... FROM f JOIN p`. Pre-join shuffle
//! → HashJoin → emit rows (no aggregate).
//!
//! [`MppPlanShape::Ineligible`] — fall back to the non-MPP serial path.
//!
//! # What this module *does not* do
//!
//! The DSM hook layer allocates the mesh wirings. A single shuffle =
//! one mesh. Binary join under `ScalarAggOnBinaryJoin` or
//! `GroupByAggOnBinaryJoin` needs TWO meshes (one per join input).
//! `GroupByAggOnBinaryJoin` adds a THIRD mesh for the post-Partial
//! shuffle. Multi-mesh DSM allocation is a separate piece — this module
//! takes pre-allocated wirings as parameters.

// caller lands once create_custom_path flips parallel-safe
// `CoalesceBatchesExec` is deprecated in favor of arrow-rs's `BatchCoalescer`,
// but DataFusion 52 still ships it as an `ExecutionPlan` node — the replacement
// is a streaming coalescer, not a plan node, so we cannot drop into the same
// slot. Remove this allow when the wrapper migrates.
#![allow(deprecated)]

use std::sync::Arc;

use datafusion::arrow::datatypes::SchemaRef;
use datafusion::common::Result;
use datafusion::logical_expr::JoinType;
use datafusion::physical_expr::aggregate::AggregateFunctionExpr;
use datafusion::physical_expr::PhysicalExpr;
use datafusion::physical_plan::aggregates::{AggregateExec, AggregateMode, PhysicalGroupBy};
use datafusion::physical_plan::coalesce_batches::CoalesceBatchesExec;
use datafusion::physical_plan::filter::FilterExec;
use datafusion::physical_plan::joins::utils::JoinOn;
use datafusion::physical_plan::joins::{HashJoinExec, PartitionMode};
use datafusion::physical_plan::ExecutionPlan;

use crate::postgres::customscan::mpp::plan_build::{wrap_with_mpp_shuffle, MppShuffleInputs};
use crate::postgres::customscan::mpp::shuffle::ShuffleWiring;
use crate::postgres::customscan::mpp::transport::DrainHandle;

/// Classify a query so the dispatcher picks the right topology.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MppPlanShape {
    ScalarAggOnBinaryJoin,
    GroupByAggOnBinaryJoin,
    GroupByAggSingleTable,
    JoinOnly,
    Ineligible,
}

/// Shape-classification inputs. Kept as plain fields rather than a reference
/// to a larger state so callers can construct it from whatever they have
/// (RelNode walk, AggregateCSClause inspection, test synthetic data).
pub struct ClassifyInputs {
    /// Number of tables in the join. 0 = no join, 1 = single table,
    /// 2 = binary join, >=3 = multi-table join.
    pub n_join_tables: usize,
    /// True if the query has a GROUP BY clause with at least one column.
    pub has_group_by: bool,
    /// True if every aggregate function in the targetlist is one of
    /// COUNT/SUM/MIN/MAX/AVG/BOOL_*/STDDEV_*/VAR_* — the set with a safe
    /// Partial/Final split. `false` for COUNT(DISTINCT), ARRAY_AGG,
    /// STRING_AGG, ordered-set, hypothetical-set aggregates.
    pub all_aggregates_splittable: bool,
    /// True if the query has at least one aggregate (COUNT, SUM, …). When
    /// `false`, we're classifying a join-only shape.
    pub has_aggregate: bool,
}

/// Classify the query into an [`MppPlanShape`].
///
/// Rules (all implicitly `AND`-ed):
///   * Two tables + has_aggregate + splittable: binary-join aggregate.
///     Split further on `has_group_by` to pick scalar vs group-by topology.
///   * One table + has_aggregate + group_by + splittable:
///     single-table group-by aggregate (post-Partial shuffle helps when
///     cardinality is high).
///   * Two tables + no_aggregate: join-only.
///   * Otherwise: Ineligible.
pub fn classify(inputs: &ClassifyInputs) -> MppPlanShape {
    if !inputs.all_aggregates_splittable && inputs.has_aggregate {
        return MppPlanShape::Ineligible;
    }
    match (
        inputs.n_join_tables,
        inputs.has_aggregate,
        inputs.has_group_by,
    ) {
        (2, true, false) => MppPlanShape::ScalarAggOnBinaryJoin,
        (2, true, true) => MppPlanShape::GroupByAggOnBinaryJoin,
        (1, true, true) => MppPlanShape::GroupByAggSingleTable,
        (2, false, _) => MppPlanShape::JoinOnly,
        _ => MppPlanShape::Ineligible,
    }
}

// ============================================================================
// Per-shape plan builders
// ============================================================================

/// Inputs to [`build_scalar_agg_on_binary_join`]. Both `left_child` and
/// `right_child` are the pre-shuffle scans (scan → filter) for each join
/// input. The wirings + drains are the caller-allocated halves of three
/// independent meshes: one per join input plus a final-gather mesh that
/// routes every worker's Partial row to the leader seat so a single
/// `AggregateExec(FinalPartitioned)` on the leader produces the one-row
/// scalar result.
pub struct ScalarAggOnBinaryJoinInputs {
    pub left_child: Arc<dyn ExecutionPlan>,
    pub right_child: Arc<dyn ExecutionPlan>,
    pub left_shuffle: ShuffleWiring,
    pub left_drain: std::sync::Arc<DrainHandle>,
    pub left_schema: SchemaRef,
    pub right_shuffle: ShuffleWiring,
    pub right_drain: std::sync::Arc<DrainHandle>,
    pub right_schema: SchemaRef,
    pub aggr_expr: Vec<Arc<AggregateFunctionExpr>>,
    pub filter_expr: Vec<Option<Arc<dyn PhysicalExpr>>>,
    /// Wiring for the final-gather mesh: every participant ships its
    /// Partial row to one fixed target seat (the leader, seat 0) via a
    /// [`FixedTargetPartitioner`](super::shuffle::FixedTargetPartitioner).
    /// The caller must build this with `target == 0` so PG's Gather sees
    /// exactly one row (from the leader) per query.
    pub final_shuffle: ShuffleWiring,
    pub final_drain: std::sync::Arc<DrainHandle>,
    /// The original `HashJoinExec` from the standard plan. Rebuilt here via
    /// `builder().with_new_children(...)` so its `dynamic_filter` Arc (populated
    /// by `SharedBuildAccumulator` once the local build side completes) is
    /// preserved across the MPP rewrite. The bridge strips the Arc from the
    /// probe-side `PgSearchScanPlan` and this module re-applies it as a
    /// [`FilterExec`] on top of the post-shuffle probe stream (see below).
    pub original_hash_join: Arc<HashJoinExec>,
}

/// Build the MPP plan for a scalar aggregate over a binary join.
///
/// # Leader / worker asymmetry
///
/// For a scalar aggregate, DataFusion's `AggregateExec(FinalPartitioned)`
/// emits *one* row even when its input is empty (the SQL semantics for
/// `SELECT COUNT(*) FROM empty` is `0`, not "no row"). If every
/// participant ran the full plan, PG's Gather above would concatenate N
/// rows (one per worker + leader), breaking the scalar contract.
///
/// Instead, the leader and workers build asymmetric plans driven by
/// `is_leader`:
///
/// **Leader** (is_leader = true):
/// ```text
///     AggregateExec(FinalPartitioned)
///       wrap_with_mpp_shuffle(final_mesh, all-to-seat-0)
///         AggregateExec(Partial)
///           HashJoinExec(PartitionMode::Partitioned)
///             wrap_with_mpp_shuffle(left_child,  left_wiring, left_drain)
///             wrap_with_mpp_shuffle(right_child, right_wiring, right_drain)
/// ```
/// Leader's own Partial row stays local (self-partition == target 0);
/// its drain receives Partial rows from every worker. `FinalPartitioned`
/// sums N partials → emits one row.
///
/// **Worker** (is_leader = false):
/// ```text
///     wrap_with_mpp_shuffle(final_mesh, all-to-seat-0)   ← no FinalPartitioned
///       AggregateExec(Partial)
///         HashJoinExec(PartitionMode::Partitioned)
///           wrap_with_mpp_shuffle(left_child,  …)
///           wrap_with_mpp_shuffle(right_child, …)
/// ```
/// Worker's ShuffleExec ships its Partial row to seat 0 via the final
/// mesh (self != target so its self-partition is empty). Worker's
/// DrainGatherExec reads from the worker's inbound receivers on the
/// final mesh, which are empty by construction (everyone ships *to*
/// seat 0, nobody ships *to* the worker). The worker's stream therefore
/// emits zero rows — PG's Gather sees exactly one row per query (from
/// the leader).
pub fn build_scalar_agg_on_binary_join(
    inputs: ScalarAggOnBinaryJoinInputs,
    is_leader: bool,
) -> Result<Arc<dyn ExecutionPlan>> {
    let left_shuffled = wrap_with_mpp_shuffle(MppShuffleInputs {
        child: inputs.left_child,
        wiring: inputs.left_shuffle,
        drain_handle: inputs.left_drain,
        wrapped_schema: inputs.left_schema,
        tag: "scalar_left",
    })?;
    let right_shuffled = wrap_with_mpp_shuffle(MppShuffleInputs {
        child: inputs.right_child,
        wiring: inputs.right_shuffle,
        drain_handle: inputs.right_drain,
        wrapped_schema: inputs.right_schema,
        tag: "scalar_right",
    })?;

    // Re-apply the HashJoin's dynamic filter above the post-shuffle probe
    // stream. The filter was stripped from the probe-side scan by the bridge
    // so it isn't applied before shuffle routing (which would drop rows
    // destined for peer participants); re-attaching it here lets each
    // participant's local `SharedBuildAccumulator` populate the Arc once its
    // local build side finishes, and the `FilterExec` then prunes probe rows
    // that cannot match any local build key.
    let right_probe: Arc<dyn ExecutionPlan> =
        match inputs.original_hash_join.dynamic_filter_for_test().cloned() {
            Some(df) => Arc::new(FilterExec::try_new(df, right_shuffled)?),
            None => right_shuffled,
        };

    // Rebuild through the builder so the `dynamic_filter` field survives. A
    // plain `HashJoinExec::try_new` would clear it (the builder initializes
    // `dynamic_filter: None` and `try_new` never calls `with_dynamic_filter`),
    // orphaning the Arc held by the `FilterExec` above.
    let join = inputs
        .original_hash_join
        .builder()
        .with_new_children(vec![left_shuffled, right_probe])?
        .build_exec()?;
    let join_schema = join.schema();

    let partial: Arc<dyn ExecutionPlan> = Arc::new(AggregateExec::try_new(
        AggregateMode::Partial,
        PhysicalGroupBy::new_single(vec![]),
        inputs.aggr_expr.clone(),
        inputs.filter_expr.clone(),
        join,
        join_schema,
    )?);
    let partial_schema = partial.schema();

    let gathered = wrap_with_mpp_shuffle(MppShuffleInputs {
        child: partial,
        wiring: inputs.final_shuffle,
        drain_handle: inputs.final_drain,
        wrapped_schema: partial_schema.clone(),
        tag: "scalar_final",
    })?;

    if !is_leader {
        // Worker plan: stream drives the shuffle (shipping Partial to
        // the leader) and returns zero rows locally.
        return Ok(gathered);
    }

    let final_agg = AggregateExec::try_new(
        AggregateMode::FinalPartitioned,
        PhysicalGroupBy::new_single(vec![]),
        inputs.aggr_expr,
        inputs.filter_expr,
        gathered,
        partial_schema,
    )?;
    Ok(Arc::new(final_agg))
}

/// Inputs to [`build_groupby_agg_on_binary_join`]. Adds a third mesh
/// (`postagg_shuffle` + `postagg_drain`) on top of
/// [`ScalarAggOnBinaryJoinInputs`] for the Partial → FinalPartitioned
/// shuffle on group keys. Caller supplies group-by expressions +
/// per-partial schema.
pub struct GroupByAggOnBinaryJoinInputs {
    pub left_child: Arc<dyn ExecutionPlan>,
    pub right_child: Arc<dyn ExecutionPlan>,
    pub left_shuffle: ShuffleWiring,
    pub left_drain: std::sync::Arc<DrainHandle>,
    pub left_schema: SchemaRef,
    pub right_shuffle: ShuffleWiring,
    pub right_drain: std::sync::Arc<DrainHandle>,
    pub right_schema: SchemaRef,
    pub group_by: PhysicalGroupBy,
    pub aggr_expr: Vec<Arc<AggregateFunctionExpr>>,
    pub filter_expr: Vec<Option<Arc<dyn PhysicalExpr>>>,
    pub postagg_shuffle: ShuffleWiring,
    pub postagg_drain: std::sync::Arc<DrainHandle>,
    /// See [`ScalarAggOnBinaryJoinInputs::original_hash_join`].
    pub original_hash_join: Arc<HashJoinExec>,
}

/// Build the MPP plan for a GROUP BY aggregate over a binary join.
///
/// Symmetric across all seats — no leader/worker asymmetry. Each seat
/// runs the same plan; rows land on the seat that owns their group-by
/// hash partition, so `FinalPartitioned` on each seat emits a disjoint
/// set of groups and PG's Gather concatenates.
///
/// Shape (every seat):
/// ```text
///     AggregateExec(FinalPartitioned, group_by)
///       wrap_with_mpp_shuffle(gb_postagg, HashPartitioner(group_keys))
///         CoalesceBatchesExec(target = 64 Ki rows)
///           AggregateExec(Partial, group_by)
///             HashJoinExec(Partitioned)
///               wrap_with_mpp_shuffle(gb_left)
///               wrap_with_mpp_shuffle(gb_right)
/// ```
///
/// # Why `CoalesceBatchesExec` before the post-agg shuffle
///
/// `AggregateExec(Partial)` emits DataFusion's default 8 Ki-row batches.
/// At 25 M input / 3.12 M distinct groups, that's ~191 batches per seat
/// going over shm_mq via Arrow-IPC. Per-batch overhead (schema/dictionary
/// preamble + per-iteration dispatch in `ShuffleStream::process_batch`)
/// dominates at small batch sizes. Coalescing to 64 Ki rows collapses
/// that to ~24 batches, amortizing the fixed per-batch cost ~8× and
/// keeping payload under the 64 MiB `shm_mq` queue capacity so
/// backpressure stays near zero.
///
/// # Why hash-partition (not gather-to-leader) once coalesce lands
///
/// Hash-partitioning by group keys lets each seat finalize 1/N of the
/// distinct groups in parallel; the gather-to-leader variant
/// (earlier `FixedTargetPartitioner(0)`) serialized `Final` on the
/// leader. Gather was a win only while per-batch encode cost was the
/// binding constraint — with batches coalesced, encode is cheap enough
/// that parallel `Final` dominates.
pub fn build_groupby_agg_on_binary_join(
    inputs: GroupByAggOnBinaryJoinInputs,
) -> Result<Arc<dyn ExecutionPlan>> {
    let left_shuffled = wrap_with_mpp_shuffle(MppShuffleInputs {
        child: inputs.left_child,
        wiring: inputs.left_shuffle,
        drain_handle: inputs.left_drain,
        wrapped_schema: inputs.left_schema,
        tag: "gb_left",
    })?;
    let right_shuffled = wrap_with_mpp_shuffle(MppShuffleInputs {
        child: inputs.right_child,
        wiring: inputs.right_shuffle,
        drain_handle: inputs.right_drain,
        wrapped_schema: inputs.right_schema,
        tag: "gb_right",
    })?;

    // See `build_scalar_agg_on_binary_join` for the rationale behind this
    // post-shuffle `FilterExec` + `builder()` rebuild.
    let right_probe: Arc<dyn ExecutionPlan> =
        match inputs.original_hash_join.dynamic_filter_for_test().cloned() {
            Some(df) => Arc::new(FilterExec::try_new(df, right_shuffled)?),
            None => right_shuffled,
        };

    let join = inputs
        .original_hash_join
        .builder()
        .with_new_children(vec![left_shuffled, right_probe])?
        .build_exec()?;
    let join_schema = join.schema();

    let partial: Arc<dyn ExecutionPlan> = Arc::new(AggregateExec::try_new(
        AggregateMode::Partial,
        inputs.group_by.clone(),
        inputs.aggr_expr.clone(),
        inputs.filter_expr.clone(),
        join,
        join_schema,
    )?);
    // Coalesce Partial's default 8 Ki-row batches into 64 Ki-row batches
    // before the post-agg shuffle so Arrow-IPC encode amortizes over
    // ~8× fewer batches. See the function-level doc for why.
    let coalesced_partial: Arc<dyn ExecutionPlan> =
        Arc::new(CoalesceBatchesExec::new(partial, 65_536));
    let partial_schema = coalesced_partial.schema();

    let repartitioned = wrap_with_mpp_shuffle(MppShuffleInputs {
        child: coalesced_partial,
        wiring: inputs.postagg_shuffle,
        drain_handle: inputs.postagg_drain,
        wrapped_schema: partial_schema.clone(),
        tag: "gb_postagg",
    })?;

    let final_agg = AggregateExec::try_new(
        AggregateMode::FinalPartitioned,
        inputs.group_by,
        inputs.aggr_expr,
        inputs.filter_expr,
        repartitioned,
        partial_schema,
    )?;
    Ok(Arc::new(final_agg))
}

/// Inputs to [`build_join_only`]. Two meshes (one per join input).
pub struct JoinOnlyInputs {
    pub left_child: Arc<dyn ExecutionPlan>,
    pub right_child: Arc<dyn ExecutionPlan>,
    pub left_shuffle: ShuffleWiring,
    pub left_drain: std::sync::Arc<DrainHandle>,
    pub left_schema: SchemaRef,
    pub right_shuffle: ShuffleWiring,
    pub right_drain: std::sync::Arc<DrainHandle>,
    pub right_schema: SchemaRef,
    pub join_on: JoinOn,
    /// Column projection applied to the `HashJoinExec` output. DataFusion
    /// may prune the raw `left_schema ++ right_schema` down to only the
    /// columns the caller needs; forwarding the same projection here keeps
    /// downstream column indices valid.
    pub join_projection: Option<Vec<usize>>,
    pub join_type: JoinType,
}

/// Build the MPP plan for a join without an aggregate on top. Emits the
/// join output rows directly.
///
/// Shape:
/// ```text
///     HashJoinExec(PartitionMode::Partitioned)
///       wrap_with_mpp_shuffle(left_child,  hash=[left_key])
///       wrap_with_mpp_shuffle(right_child, hash=[right_key])
/// ```
pub fn build_join_only(inputs: JoinOnlyInputs) -> Result<Arc<dyn ExecutionPlan>> {
    let left_shuffled = wrap_with_mpp_shuffle(MppShuffleInputs {
        child: inputs.left_child,
        wiring: inputs.left_shuffle,
        drain_handle: inputs.left_drain,
        wrapped_schema: inputs.left_schema,
        tag: "join_left",
    })?;
    let right_shuffled = wrap_with_mpp_shuffle(MppShuffleInputs {
        child: inputs.right_child,
        wiring: inputs.right_shuffle,
        drain_handle: inputs.right_drain,
        wrapped_schema: inputs.right_schema,
        tag: "join_right",
    })?;

    let join = HashJoinExec::try_new(
        left_shuffled,
        right_shuffled,
        inputs.join_on,
        None,
        &inputs.join_type,
        inputs.join_projection,
        PartitionMode::Partitioned,
        datafusion::common::NullEquality::NullEqualsNothing,
        false,
    )?;
    Ok(Arc::new(join))
}

// ============================================================================
// Counting meshes needed per shape — useful for the DSM hook sizing pass
// ============================================================================

/// How many independent shuffle meshes does this shape need?
///
/// - `ScalarAggOnBinaryJoin`: 3 (left + right pre-join + final-gather-to-leader)
/// - `GroupByAggOnBinaryJoin`: 3 (left + right pre-join + post-Partial)
/// - `GroupByAggSingleTable`: 1 (post-Partial)
/// - `JoinOnly`: 2 (left + right pre-join)
/// - `Ineligible`: 0 — not reachable through an MPP path
pub fn shuffles_required(shape: MppPlanShape) -> u32 {
    match shape {
        MppPlanShape::ScalarAggOnBinaryJoin => 3,
        MppPlanShape::GroupByAggOnBinaryJoin => 3,
        MppPlanShape::GroupByAggSingleTable => 1,
        MppPlanShape::JoinOnly => 2,
        MppPlanShape::Ineligible => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs(n: usize, group: bool, agg: bool, splittable: bool) -> ClassifyInputs {
        ClassifyInputs {
            n_join_tables: n,
            has_group_by: group,
            has_aggregate: agg,
            all_aggregates_splittable: splittable,
        }
    }

    #[test]
    fn classify_scalar_agg_on_binary_join() {
        // COUNT(*) FROM f JOIN p WHERE ...
        assert_eq!(
            classify(&inputs(2, false, true, true)),
            MppPlanShape::ScalarAggOnBinaryJoin
        );
    }

    #[test]
    fn classify_groupby_agg_on_binary_join() {
        // SELECT k, COUNT(*) FROM f JOIN p GROUP BY k
        assert_eq!(
            classify(&inputs(2, true, true, true)),
            MppPlanShape::GroupByAggOnBinaryJoin
        );
    }

    #[test]
    fn classify_groupby_agg_single_table() {
        // SELECT k, COUNT(*) FROM t GROUP BY k
        assert_eq!(
            classify(&inputs(1, true, true, true)),
            MppPlanShape::GroupByAggSingleTable
        );
    }

    #[test]
    fn classify_join_only() {
        // SELECT * FROM f JOIN p — no aggregate at all.
        assert_eq!(
            classify(&inputs(2, false, false, true)),
            MppPlanShape::JoinOnly
        );
        // A CLASSIFY_OUTPUT_BY with no aggregate is still join-only; GROUP BY
        // without aggregates is unusual SQL but should not elevate to group-by
        // aggregate shape because there's no partial state to shuffle.
        assert_eq!(
            classify(&inputs(2, true, false, true)),
            MppPlanShape::JoinOnly
        );
    }

    #[test]
    fn classify_rejects_unsplittable_aggregates() {
        // ARRAY_AGG, COUNT(DISTINCT), etc. aren't safe to Partial/Final split.
        assert_eq!(
            classify(&inputs(2, false, true, false)),
            MppPlanShape::Ineligible
        );
        assert_eq!(
            classify(&inputs(2, true, true, false)),
            MppPlanShape::Ineligible
        );
    }

    #[test]
    fn classify_rejects_single_table_scalar() {
        // Single-table scalar aggregates don't benefit from MPP — the
        // aggregate is already O(rows/workers) via PG's native parallel
        // scan. No shuffle needed.
        assert_eq!(
            classify(&inputs(1, false, true, true)),
            MppPlanShape::Ineligible
        );
    }

    #[test]
    fn classify_rejects_no_tables() {
        assert_eq!(
            classify(&inputs(0, false, false, true)),
            MppPlanShape::Ineligible
        );
    }

    #[test]
    fn classify_rejects_multi_table_join() {
        // 3+ table joins aren't wired yet (scope = milestone 1 binary join).
        // Extend `classify` when milestone-2 adds them.
        assert_eq!(
            classify(&inputs(3, false, true, true)),
            MppPlanShape::Ineligible
        );
    }

    #[test]
    fn shuffles_required_matches_shape_definition() {
        assert_eq!(shuffles_required(MppPlanShape::ScalarAggOnBinaryJoin), 3);
        assert_eq!(shuffles_required(MppPlanShape::GroupByAggOnBinaryJoin), 3);
        assert_eq!(shuffles_required(MppPlanShape::GroupByAggSingleTable), 1);
        assert_eq!(shuffles_required(MppPlanShape::JoinOnly), 2);
        assert_eq!(shuffles_required(MppPlanShape::Ineligible), 0);
    }
}
