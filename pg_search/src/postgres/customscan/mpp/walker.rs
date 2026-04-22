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

//! Generic cut walker (`distribute_plan`).
//!
//! Ported in spirit from datafusion-contrib/datafusion-distributed's
//! `src/distributed_planner/distribute_plan.rs`. Walks a DataFusion physical
//! plan once, identifies the cut points where the plan needs to cross a
//! network boundary, and rewrites those cuts to emit
//! [`ShuffleExec`](crate::postgres::customscan::mpp::shuffle::ShuffleExec) +
//! [`DrainGatherExec`](crate::postgres::customscan::mpp::shuffle::DrainGatherExec)
//! pairs whose [`MppNetworkBoundary::input_stage`] is stamped by the walker.
//!
//! # What this module does
//!
//! [`distribute_plan`] is the single entry point callers invoke to turn a
//! standard DataFusion physical plan into its MPP-partitioned equivalent.
//! Cut detection is derived from plan structure — the walker looks for a
//! `HashJoinExec` and any `AggregateExec(Partial|Single)` above it, and
//! picks the right topology (scalar-agg, group-by-agg, or bare-join)
//! from that in situ rather than trusting an out-of-band shape enum.
//! The caller's [`MppPlanShape`] is kept around as a sanity cross-check
//! against the derivation, and its only other role is sizing the DSM
//! region at plan time via [`cut_count_for_shape`].
//!
//! # Visibility correctness (invariant enforced here)
//!
//! `VisibilityFilterExec` resolves per-segment packed DocAddress keys to
//! heap TIDs using segment-local Tantivy state plus a ctid-resolver table
//! keyed by `(plan_position, seg_ord)` that lists segments **local to this
//! seat**. Every [`ShuffleExec`] cut must therefore sit **inside** the subtree
//! of every [`VisibilityFilterExec`] the plan contains — i.e. no
//! `VisibilityFilterExec` may be a descendant of a `ShuffleExec`. Inverting
//! that placement means a row from seat A would reach seat B's resolver with
//! a `seg_ord` that addresses A's segment catalog; the lookup would return
//! the wrong heap TID (or panic in `heap_fetch` if the slot is absent).
//! [`assert_visibility_invariant`] walks the finished MPP plan and rejects it
//! with `DataFusionError::Plan` if that invariant is violated.
//!
//! The same constraint applies to other segment-local scan adornments
//! (`SegmentedTopKExec` pre-merge, `TantivyLookupExec`, etc.): they must
//! remain on the scan side of every cut. Today only
//! `VisibilityFilterExec` is asserted because it's the one that causes
//! `heap_fetch` panics; extending the assert to the other node types is
//! mechanical but out of scope until one of them is re-introduced as a
//! cross-shuffle risk.
//!
//! # Mesh allocation timing
//!
//! The walker exposes [`cut_count_for_shape`] so the DSM-estimate hook can
//! size the region at **plan time** without needing to run the full walker
//! inside the hook. Overestimating is cheap (one unused `shm_mq` region per
//! edge, dropped on tear-down) — the hook is free to round up via
//! [`worst_case_cut_count`] when the shape is not yet classified.
//!
//! # Leader/worker mesh-index contract
//!
//! Every topology consumes a contiguous slice of `MppExecutionState::meshes`
//! in a fixed order shared by every participant:
//!
//! - `meshes[0]` is wired into the **left** join input's shuffle.
//! - `meshes[1]` is wired into the **right** join input's shuffle.
//! - `meshes[2]` (when present) is the post-join mesh — either
//!   **final-gather-to-leader** (scalar-agg, `FixedTargetPartitioner(0)`)
//!   or **hash-partition on group keys** (group-by-agg, `HashPartitioner`).
//!
//! `JoinOnly` uses only meshes 0 and 1. Mesh ordering must be identical on
//! every seat — the meshes themselves are symmetric, but a mismatch would
//! route left rows to the right drain and break correctness.

#![allow(deprecated)] // `CoalesceBatchesExec` is deprecated in favor of
                      // arrow-rs's streaming `BatchCoalescer`, but DataFusion
                      // 52 still emits it as a plan node and we must recognize
                      // + reuse it.

use std::sync::Arc;

use datafusion::arrow::datatypes::SchemaRef;
use datafusion::common::{DataFusionError, Result as DfResult};
use datafusion::physical_expr::expressions::Column;
use datafusion::physical_expr::PhysicalExpr;
use datafusion::physical_plan::aggregates::{AggregateExec, AggregateMode, PhysicalGroupBy};
use datafusion::physical_plan::coalesce_batches::CoalesceBatchesExec;
use datafusion::physical_plan::coalesce_partitions::CoalescePartitionsExec;
use datafusion::physical_plan::filter::FilterExec;
use datafusion::physical_plan::joins::{HashJoinExec, PartitionMode};
use datafusion::physical_plan::repartition::RepartitionExec;
use datafusion::physical_plan::ExecutionPlan;

use super::customscan_glue::{MppExecutionState, DEFAULT_MPP_QUEUE_BYTES};
use super::plan_build::{wrap_with_mpp_shuffle, MppShuffleInputs};
use super::shape::MppPlanShape;
use super::shuffle::{FixedTargetPartitioner, HashPartitioner, RowPartitioner, ShuffleWiring};
use super::stage::{MppStage, MppTaskKey};
use super::transport::{DrainBuffer, DrainHandle, MppReceiver, MppSender};
use super::worker::{LeaderMesh, WorkerMesh};
use crate::scan::execution_plan::PgSearchScanPlan;

/// How many shuffle cuts the walker will insert for `shape`. Called by the
/// DSM-estimate hooks in `aggregatescan/mod.rs` and `joinscan/mod.rs` at
/// plan time to size the shared-memory region before the walker has a
/// concrete plan to inspect.
pub fn cut_count_for_shape(shape: MppPlanShape) -> u32 {
    match shape {
        MppPlanShape::ScalarAggOnBinaryJoin => 3,
        MppPlanShape::GroupByAggOnBinaryJoin => 3,
        MppPlanShape::JoinOnly => 2,
        MppPlanShape::GroupByAggSingleTable => 1,
        MppPlanShape::Ineligible => 0,
    }
}

/// Upper bound on the number of cuts the walker can produce for any
/// supported shape. Used by the DSM estimate hook when the walker hasn't
/// been run yet (or when we don't want to risk running DataFusion physical
/// planning from inside the hook).
///
/// Safe to overestimate: an unused mesh wiring costs one `shm_mq` region
/// per edge and is dropped during `attach_cooperative_drain` / `take_meshes`.
#[allow(dead_code)]
pub fn worst_case_cut_count() -> u32 {
    cut_count_for_shape(MppPlanShape::ScalarAggOnBinaryJoin)
        .max(cut_count_for_shape(MppPlanShape::GroupByAggOnBinaryJoin))
        .max(cut_count_for_shape(MppPlanShape::GroupByAggSingleTable))
        .max(cut_count_for_shape(MppPlanShape::JoinOnly))
}

/// Walk `standard` and rewrite it into an MPP-partitioned plan by
/// identifying cut points and inserting `ShuffleExec` / `DrainGatherExec`
/// pairs stamped with an [`MppStage`].
///
/// Topology is derived from plan structure — the walker locates
/// `HashJoinExec` and any `AggregateExec(Partial|Single)` above it, and
/// picks the right assembler from there. `shape` is kept as an explicit
/// parameter so the pre-classified shape from `aggregatescan` / `joinscan`
/// can be cross-checked against the derivation; a disagreement aborts the
/// query via `DataFusionError::Plan` rather than silently executing the
/// wrong topology.
///
/// Before returning, the built plan is validated against the visibility
/// invariant (see module doc). A violation aborts the query with
/// `DataFusionError::Plan` rather than risk a `heap_fetch` panic at
/// execution.
pub fn distribute_plan(
    standard: Arc<dyn ExecutionPlan>,
    mpp_state: &mut MppExecutionState,
    shape: MppPlanShape,
) -> DfResult<Arc<dyn ExecutionPlan>> {
    let total = mpp_state.participant_config().total_participants;

    crate::mpp_log!(
        "mpp: distribute_plan walker dispatching shape={:?} total={} cuts={}",
        shape,
        total,
        cut_count_for_shape(shape)
    );

    // Derive topology from plan structure. `find_partial_agg` walks through
    // `AggregateExec(Final*)` / `CoalescePartitionsExec` / `CoalesceBatchesExec`
    // layers to reach a `Partial|Single` aggregate; `find_hash_join` does the
    // same to reach a `HashJoinExec`. Combined presence of the two + whether
    // the partial agg has group-by keys picks one of three assemblers.
    let partial_agg_opt = find_partial_agg(standard.as_ref());
    let hash_join_opt = find_hash_join(standard.as_ref());

    let built = match (partial_agg_opt, hash_join_opt) {
        // Partial agg with empty group keys on top of a HashJoin — scalar agg.
        (Some(agg), Some(_)) if agg.group_expr().expr().is_empty() => {
            validate_shape_matches(shape, MppPlanShape::ScalarAggOnBinaryJoin)?;
            build_scalar_agg_topology(standard, mpp_state)?
        }
        // Partial agg with group-by keys on top of a HashJoin — group-by agg.
        (Some(_), Some(_)) => {
            validate_shape_matches(shape, MppPlanShape::GroupByAggOnBinaryJoin)?;
            build_groupby_agg_topology(standard, mpp_state)?
        }
        // HashJoin without a Partial agg above — bare join.
        (None, Some(_)) => {
            validate_shape_matches(shape, MppPlanShape::JoinOnly)?;
            build_join_only_topology(standard, mpp_state)?
        }
        // Partial agg without HashJoin — e.g. GroupByAggSingleTable, not yet
        // wired through the assembler code.
        (Some(_), None) => {
            return Err(DataFusionError::Plan(
                "mpp: distribute_plan: aggregate-on-single-table shape not yet implemented \
                 (no HashJoinExec in plan)"
                    .into(),
            ));
        }
        // Neither — the caller's classifier should have routed to serial.
        (None, None) => {
            return Err(DataFusionError::Plan(
                "mpp: distribute_plan invoked on an ineligible plan — no HashJoinExec \
                 found; caller should have routed to the non-MPP serial path"
                    .into(),
            ));
        }
    };

    assert_visibility_invariant(&built)?;
    Ok(built)
}

/// Cross-check the pre-classified shape (from the planner) against the
/// walker's structural derivation. Disagreement means either the planner
/// mis-classified the query or the standard physical plan diverged from
/// what the classifier saw — either way, executing anyway would produce
/// wrong results for the shape we actually have.
fn validate_shape_matches(classified: MppPlanShape, derived: MppPlanShape) -> DfResult<()> {
    if classified == derived {
        Ok(())
    } else {
        Err(DataFusionError::Plan(format!(
            "mpp: walker shape derivation mismatch — classifier said {classified:?} \
             but plan structure implies {derived:?}"
        )))
    }
}

// ============================================================================
// Topology assemblers — one per supported shape. Each one owns the full flow:
// plan walk + mesh allocation + shuffle wiring + final DF operator composition.
// ============================================================================

/// `ScalarAggOnBinaryJoin`: `COUNT(*) FROM a JOIN b WHERE …`.
///
/// Topology (per seat):
/// ```text
///     [Leader only] AggregateExec(FinalPartitioned)
///       wrap_with_mpp_shuffle(FixedTargetPartitioner(0), mesh 2)
///         AggregateExec(Partial, COUNT(*))
///           HashJoinExec(Partitioned)
///             wrap_with_mpp_shuffle(HashPartitioner(left_key),  mesh 0)
///             wrap_with_mpp_shuffle(HashPartitioner(right_key), mesh 1)
/// ```
///
/// Leader/worker asymmetry: `AggregateExec(FinalPartitioned)` on a scalar
/// aggregate emits one row even when empty (SQL `COUNT(*) FROM empty` = 0).
/// Workers skip `FinalPartitioned` entirely and let `ShuffleExec` ship the
/// sole Partial row to seat 0; PG's Gather above sees exactly one row per
/// query (from the leader).
fn build_scalar_agg_topology(
    standard: Arc<dyn ExecutionPlan>,
    mpp_state: &mut MppExecutionState,
) -> DfResult<Arc<dyn ExecutionPlan>> {
    let (partial_agg, hash_join) = find_partial_agg_and_join(standard.as_ref())?;

    let participant_index = mpp_state.participant_config().participant_index;
    let total_participants = mpp_state.participant_config().total_participants;

    // Strip any RepartitionExec / CoalesceBatchesExec layers DataFusion
    // inserted for single-process hash partitioning — the MPP shuffle
    // replaces them. Segment sharding already happened in
    // `PgSearchTableProvider::scan` via `MppShardConfig`.
    let left_child = strip_repartition_layers(Arc::clone(hash_join.left()));
    // Probe side: strip the dynamic-filter Arc so rows aren't filtered
    // *before* the shuffle routes them to peer participants — we re-apply
    // the same Arc as a `FilterExec` above the post-shuffle output where
    // it's safe (each seat's local build covers exactly the keys that
    // hash-route to its own probe).
    let right_child =
        strip_dynamic_filters_in_subtree(strip_repartition_layers(Arc::clone(hash_join.right())))?;

    // Clone via the builder so the `dynamic_filter` Arc survives — a plain
    // `HashJoinExec::try_new` leaves `dynamic_filter: None`, orphaning the
    // Arc that `SharedBuildAccumulator` populates when the local build
    // side completes.
    let original_hash_join = Arc::new(hash_join.builder().build()?);
    let join_on = hash_join.on().to_vec();
    let (left_keys, right_keys) = extract_key_col_indices(&join_on)?;

    // mesh[0]=left, mesh[1]=right, mesh[2]=final-gather-to-leader.
    let mut meshes = take_meshes(mpp_state, 3);
    let final_mesh = meshes.pop().expect("len checked in take_meshes");
    let right_mesh = meshes.pop().expect("len checked in take_meshes");
    let left_mesh = meshes.pop().expect("len checked in take_meshes");

    let left_drain = spawn_drain(left_mesh.inbound);
    let right_drain = spawn_drain(right_mesh.inbound);
    let final_drain = spawn_drain(final_mesh.inbound);

    let query_id = mpp_state.query_id();
    let left_stage = MppStage::new(query_id, 0, total_participants);
    let right_stage = MppStage::new(query_id, 1, total_participants);
    let final_stage = MppStage::new(query_id, 2, total_participants);

    let left_outbound = attach_cooperative_drain(
        stamp_frame_ids(left_mesh.outbound, task_key_for(mpp_state, left_stage)),
        &left_drain,
    );
    let right_outbound = attach_cooperative_drain(
        stamp_frame_ids(right_mesh.outbound, task_key_for(mpp_state, right_stage)),
        &right_drain,
    );
    let final_outbound = attach_cooperative_drain(
        stamp_frame_ids(final_mesh.outbound, task_key_for(mpp_state, final_stage)),
        &final_drain,
    );

    let left_shuffle = build_shuffle_wiring(
        left_keys,
        total_participants,
        participant_index,
        left_outbound,
        Arc::clone(&left_drain),
    );
    let right_shuffle = build_shuffle_wiring(
        right_keys,
        total_participants,
        participant_index,
        right_outbound,
        Arc::clone(&right_drain),
    );
    // Final-gather mesh: every seat routes its Partial row to seat 0.
    let final_shuffle = ShuffleWiring {
        partitioner: Arc::new(FixedTargetPartitioner::new(0, total_participants))
            as Arc<dyn RowPartitioner>,
        outbound_senders: final_outbound,
        participant_index,
        cooperative_drain: Some(Arc::clone(&final_drain)),
    };

    let left_schema: SchemaRef = left_child.schema();
    let right_schema: SchemaRef = right_child.schema();
    let aggr_expr = partial_agg.aggr_expr().to_vec();
    let filter_expr = partial_agg.filter_expr().to_vec();

    crate::mpp_log!(
        "mpp: assembling ScalarAggOnBinaryJoin plan (participant={}, total={}, \
         aggr_count={}, join_keys={})",
        participant_index,
        total_participants,
        aggr_expr.len(),
        join_on.len()
    );

    let is_leader = mpp_state.is_leader();

    let left_shuffled = wrap_with_mpp_shuffle(MppShuffleInputs {
        child: left_child,
        wiring: left_shuffle,
        drain_handle: left_drain,
        wrapped_schema: left_schema,
        tag: "scalar_left",
        stage: Some(left_stage),
    })?;
    let right_shuffled = wrap_with_mpp_shuffle(MppShuffleInputs {
        child: right_child,
        wiring: right_shuffle,
        drain_handle: right_drain,
        wrapped_schema: right_schema,
        tag: "scalar_right",
        stage: Some(right_stage),
    })?;

    // Re-apply the HashJoin's dynamic filter above the post-shuffle probe
    // stream — see `strip_dynamic_filters_in_subtree` for why we stripped it.
    let right_probe: Arc<dyn ExecutionPlan> =
        match original_hash_join.dynamic_filter_for_test().cloned() {
            Some(df) => Arc::new(FilterExec::try_new(df, right_shuffled)?),
            None => right_shuffled,
        };

    // Rebuild through the builder so `dynamic_filter` survives.
    let join = original_hash_join
        .builder()
        .with_new_children(vec![left_shuffled, right_probe])?
        .build_exec()?;
    let join_schema = join.schema();

    let partial: Arc<dyn ExecutionPlan> = Arc::new(AggregateExec::try_new(
        AggregateMode::Partial,
        PhysicalGroupBy::new_single(vec![]),
        aggr_expr.clone(),
        filter_expr.clone(),
        join,
        join_schema,
    )?);
    let partial_schema = partial.schema();

    let gathered = wrap_with_mpp_shuffle(MppShuffleInputs {
        child: partial,
        wiring: final_shuffle,
        drain_handle: final_drain,
        wrapped_schema: partial_schema.clone(),
        tag: "scalar_final",
        stage: Some(final_stage),
    })?;

    if !is_leader {
        // Worker plan: the ShuffleExec below drives the Partial→final-mesh
        // ship-to-seat-0; DrainGatherExec reads nothing (every peer ships
        // *to* the leader, not to workers) so the worker stream emits zero
        // rows. PG's Gather therefore sees exactly one row per query.
        return Ok(gathered);
    }

    let final_agg = AggregateExec::try_new(
        AggregateMode::FinalPartitioned,
        PhysicalGroupBy::new_single(vec![]),
        aggr_expr,
        filter_expr,
        gathered,
        partial_schema,
    )?;
    Ok(Arc::new(final_agg))
}

/// `GroupByAggOnBinaryJoin`: `SELECT k, COUNT(*) FROM a JOIN b GROUP BY k`.
///
/// Topology (every seat — symmetric):
/// ```text
///     AggregateExec(FinalPartitioned, group_by)
///       wrap_with_mpp_shuffle(HashPartitioner(group_keys), mesh 2)
///         CoalesceBatchesExec(target = 64 Ki rows)
///           AggregateExec(Partial, group_by)
///             HashJoinExec(Partitioned)
///               wrap_with_mpp_shuffle(HashPartitioner(left_key),  mesh 0)
///               wrap_with_mpp_shuffle(HashPartitioner(right_key), mesh 1)
/// ```
///
/// Each group lands on exactly one seat via the group-key hash shuffle,
/// so every seat's `FinalPartitioned` emits a disjoint subset and PG's
/// Gather concatenates without double-counting.
///
/// The `CoalesceBatchesExec(64 Ki)` before the post-aggregate shuffle
/// amortizes Arrow-IPC per-batch overhead — on the 25 M row benchmark
/// it collapses ~191 batches per seat to ~24, keeping payload under the
/// 64 MiB shm_mq queue capacity so backpressure stays near zero while
/// `FinalPartitioned` runs in parallel on every seat.
fn build_groupby_agg_topology(
    standard: Arc<dyn ExecutionPlan>,
    mpp_state: &mut MppExecutionState,
) -> DfResult<Arc<dyn ExecutionPlan>> {
    let (partial_agg, hash_join) = find_partial_agg_and_join(standard.as_ref())?;

    let left_child = strip_repartition_layers(Arc::clone(hash_join.left()));
    let right_child =
        strip_dynamic_filters_in_subtree(strip_repartition_layers(Arc::clone(hash_join.right())))?;

    let original_hash_join = Arc::new(hash_join.builder().build()?);
    let join_on = hash_join.on().to_vec();
    let (left_keys, right_keys) = extract_key_col_indices(&join_on)?;

    let group_by = partial_agg.group_expr().clone();
    let num_group_keys = group_by.expr().len();
    if num_group_keys == 0 {
        return Err(DataFusionError::Plan(
            "mpp: GroupByAggOnBinaryJoin shape requires >= 1 group-by key".into(),
        ));
    }

    let mut meshes = take_meshes(mpp_state, 3);
    let postagg_mesh = meshes.pop().expect("len checked in take_meshes");
    let right_mesh = meshes.pop().expect("len checked in take_meshes");
    let left_mesh = meshes.pop().expect("len checked in take_meshes");

    let participant_index = mpp_state.participant_config().participant_index;
    let total_participants = mpp_state.participant_config().total_participants;

    let left_drain = spawn_drain(left_mesh.inbound);
    let right_drain = spawn_drain(right_mesh.inbound);
    let postagg_drain = spawn_drain(postagg_mesh.inbound);

    let query_id = mpp_state.query_id();
    let left_stage = MppStage::new(query_id, 0, total_participants);
    let right_stage = MppStage::new(query_id, 1, total_participants);
    let postagg_stage = MppStage::new(query_id, 2, total_participants);

    let left_outbound = attach_cooperative_drain(
        stamp_frame_ids(left_mesh.outbound, task_key_for(mpp_state, left_stage)),
        &left_drain,
    );
    let right_outbound = attach_cooperative_drain(
        stamp_frame_ids(right_mesh.outbound, task_key_for(mpp_state, right_stage)),
        &right_drain,
    );
    let postagg_outbound = attach_cooperative_drain(
        stamp_frame_ids(
            postagg_mesh.outbound,
            task_key_for(mpp_state, postagg_stage),
        ),
        &postagg_drain,
    );

    let left_shuffle = build_shuffle_wiring(
        left_keys,
        total_participants,
        participant_index,
        left_outbound,
        Arc::clone(&left_drain),
    );
    let right_shuffle = build_shuffle_wiring(
        right_keys,
        total_participants,
        participant_index,
        right_outbound,
        Arc::clone(&right_drain),
    );
    // Group-by columns become the partitioning keys. In
    // `AggregateExec(Partial)`'s output schema, group-by columns come
    // first (indices `0..num_group_keys`), followed by partial-aggregate
    // state columns.
    let postagg_keys: Vec<usize> = (0..num_group_keys).collect();
    let postagg_shuffle = build_shuffle_wiring(
        postagg_keys,
        total_participants,
        participant_index,
        postagg_outbound,
        Arc::clone(&postagg_drain),
    );

    let left_schema: SchemaRef = left_child.schema();
    let right_schema: SchemaRef = right_child.schema();
    let aggr_expr = partial_agg.aggr_expr().to_vec();
    let filter_expr = partial_agg.filter_expr().to_vec();

    crate::mpp_log!(
        "mpp: assembling GroupByAggOnBinaryJoin plan (participant={}, total={}, \
         aggr_count={}, group_keys={}, join_keys={})",
        participant_index,
        total_participants,
        aggr_expr.len(),
        num_group_keys,
        join_on.len()
    );

    let left_shuffled = wrap_with_mpp_shuffle(MppShuffleInputs {
        child: left_child,
        wiring: left_shuffle,
        drain_handle: left_drain,
        wrapped_schema: left_schema,
        tag: "gb_left",
        stage: Some(left_stage),
    })?;
    let right_shuffled = wrap_with_mpp_shuffle(MppShuffleInputs {
        child: right_child,
        wiring: right_shuffle,
        drain_handle: right_drain,
        wrapped_schema: right_schema,
        tag: "gb_right",
        stage: Some(right_stage),
    })?;

    let right_probe: Arc<dyn ExecutionPlan> =
        match original_hash_join.dynamic_filter_for_test().cloned() {
            Some(df) => Arc::new(FilterExec::try_new(df, right_shuffled)?),
            None => right_shuffled,
        };

    let join = original_hash_join
        .builder()
        .with_new_children(vec![left_shuffled, right_probe])?
        .build_exec()?;
    let join_schema = join.schema();

    let partial: Arc<dyn ExecutionPlan> = Arc::new(AggregateExec::try_new(
        AggregateMode::Partial,
        group_by.clone(),
        aggr_expr.clone(),
        filter_expr.clone(),
        join,
        join_schema,
    )?);
    // See function-level doc for why the coalesce at 64 Ki matters.
    let coalesced_partial: Arc<dyn ExecutionPlan> =
        Arc::new(CoalesceBatchesExec::new(partial, 65_536));
    let partial_schema = coalesced_partial.schema();

    let repartitioned = wrap_with_mpp_shuffle(MppShuffleInputs {
        child: coalesced_partial,
        wiring: postagg_shuffle,
        drain_handle: postagg_drain,
        wrapped_schema: partial_schema.clone(),
        tag: "gb_postagg",
        stage: Some(postagg_stage),
    })?;

    let final_agg = AggregateExec::try_new(
        AggregateMode::FinalPartitioned,
        group_by,
        aggr_expr,
        filter_expr,
        repartitioned,
        partial_schema,
    )?;
    Ok(Arc::new(final_agg))
}

/// `JoinOnly`: bare join without an aggregate above.
///
/// Topology:
/// ```text
///     ...outer wrappers (VisibilityFilterExec, SegmentedTopKExec, ...)
///       HashJoinExec(Partitioned)
///         wrap_with_mpp_shuffle(HashPartitioner(left_key),  mesh 0)
///         wrap_with_mpp_shuffle(HashPartitioner(right_key), mesh 1)
/// ```
///
/// Outer wrappers above the join in the standard plan are preserved — for
/// deferred-visibility queries, `VisibilityFilterExec` is what turns
/// packed DocAddresses in the `ctid` column into real heap TIDs, and
/// without it `JoinScan::build_result_tuple` passes unpacked DocAddresses
/// to `heap_fetch`, tripping `ItemPointerIsValid`. The MPP-shuffled
/// `HashJoinExec` is grafted in via `replace_first_hash_join`; the
/// surrounding `with_new_children` rebuild resets per-node state such as
/// `VisibilityFilterExec::ctid_resolvers`, so we re-run the resolver
/// rule against the grafted tree to re-wire those.
fn build_join_only_topology(
    standard: Arc<dyn ExecutionPlan>,
    mpp_state: &mut MppExecutionState,
) -> DfResult<Arc<dyn ExecutionPlan>> {
    let hash_join = find_hash_join(standard.as_ref()).ok_or_else(|| {
        DataFusionError::Plan(
            "mpp: could not locate HashJoinExec in standard physical plan for JoinOnly shape"
                .into(),
        )
    })?;

    let participant_index = mpp_state.participant_config().participant_index;
    let total_participants = mpp_state.participant_config().total_participants;

    let left_child = strip_repartition_layers(Arc::clone(hash_join.left()));
    // The HashJoin's own dynamic filter is preserved by `builder().build()` in
    // the assembler further down; no row-reducing aggregate sits above the
    // join so we don't re-apply via a `FilterExec` the way the aggregate
    // shapes do. Strip it from the probe scan so peer-destined rows aren't
    // dropped before the shuffle.
    let right_child =
        strip_dynamic_filters_in_subtree(strip_repartition_layers(Arc::clone(hash_join.right())))?;

    let join_on = hash_join.on().to_vec();
    let (left_keys, right_keys) = extract_key_col_indices(&join_on)?;

    let mut meshes = take_meshes(mpp_state, 2);
    let right_mesh = meshes.pop().expect("len checked in take_meshes");
    let left_mesh = meshes.pop().expect("len checked in take_meshes");

    let left_drain = spawn_drain(left_mesh.inbound);
    let right_drain = spawn_drain(right_mesh.inbound);

    let query_id = mpp_state.query_id();
    let left_stage = MppStage::new(query_id, 0, total_participants);
    let right_stage = MppStage::new(query_id, 1, total_participants);

    let left_outbound = attach_cooperative_drain(
        stamp_frame_ids(left_mesh.outbound, task_key_for(mpp_state, left_stage)),
        &left_drain,
    );
    let right_outbound = attach_cooperative_drain(
        stamp_frame_ids(right_mesh.outbound, task_key_for(mpp_state, right_stage)),
        &right_drain,
    );

    let left_shuffle = build_shuffle_wiring(
        left_keys,
        total_participants,
        participant_index,
        left_outbound,
        Arc::clone(&left_drain),
    );
    let right_shuffle = build_shuffle_wiring(
        right_keys,
        total_participants,
        participant_index,
        right_outbound,
        Arc::clone(&right_drain),
    );

    let left_schema: SchemaRef = left_child.schema();
    let right_schema: SchemaRef = right_child.schema();
    let join_type = *hash_join.join_type();
    let join_projection = hash_join.projection.as_deref().map(|s| s.to_vec());

    crate::mpp_log!(
        "mpp: assembling JoinOnly plan (participant={}, total={}, join_keys={}, join_type={:?})",
        participant_index,
        total_participants,
        join_on.len(),
        join_type,
    );

    let left_shuffled = wrap_with_mpp_shuffle(MppShuffleInputs {
        child: left_child,
        wiring: left_shuffle,
        drain_handle: left_drain,
        wrapped_schema: left_schema,
        tag: "join_left",
        stage: Some(left_stage),
    })?;
    let right_shuffled = wrap_with_mpp_shuffle(MppShuffleInputs {
        child: right_child,
        wiring: right_shuffle,
        drain_handle: right_drain,
        wrapped_schema: right_schema,
        tag: "join_right",
        stage: Some(right_stage),
    })?;

    let mpp_hash_join: Arc<dyn ExecutionPlan> = Arc::new(HashJoinExec::try_new(
        left_shuffled,
        right_shuffled,
        join_on,
        None,
        &join_type,
        join_projection,
        PartitionMode::Partitioned,
        datafusion::common::NullEquality::NullEqualsNothing,
        false,
    )?);

    // Graft the MPP-shuffled `HashJoinExec` back into the standard plan tree
    // in place of the original. Outer wrappers (`VisibilityFilterExec`,
    // `SegmentedTopKExec`, `TantivyLookupExec`, `ProjectionExec`, ...) are
    // preserved by `with_new_children` so ctid resolution, top-K, and late
    // column materialization still happen.
    let grafted =
        replace_first_hash_join(Arc::clone(&standard), mpp_hash_join)?.ok_or_else(|| {
            DataFusionError::Internal("mpp: HashJoinExec replacement failed to find target".into())
        })?;

    // `with_new_children` rebuilt any `VisibilityFilterExec` above the join
    // via `VisibilityFilterExec::new`, which resets `ctid_resolvers` to
    // empty. Re-run the resolver rule so the new exec is wired to the scans
    // in its fresh subtree.
    use datafusion::common::config::ConfigOptions;
    use datafusion::physical_optimizer::PhysicalOptimizerRule;
    let config = ConfigOptions::default();
    crate::scan::visibility_ctid_resolver_rule::VisibilityCtidResolverRule
        .optimize(grafted, &config)
}

// ============================================================================
// Mesh / shuffle primitives. Each supported topology needs to take N mesh
// slots out of `MppExecutionState`, spin up a drain per mesh, attach the
// cooperative-drain + frame-id stamps, and build the outbound `ShuffleWiring`.
// Shared across every topology; fully private to this module.
// ============================================================================

/// Single directed mesh extracted from the per-scan MPP state. A
/// participant-agnostic adapter over `LeaderMesh` / `WorkerMesh`, whose
/// shapes are identical but whose type-level variants force otherwise-
/// generic code to branch for no reason.
struct MeshHalves {
    outbound: Vec<Option<MppSender>>,
    inbound: Vec<Option<MppReceiver>>,
}

impl From<LeaderMesh> for MeshHalves {
    fn from(m: LeaderMesh) -> Self {
        MeshHalves {
            outbound: m.outbound,
            inbound: m.inbound,
        }
    }
}

impl From<WorkerMesh> for MeshHalves {
    fn from(m: WorkerMesh) -> Self {
        MeshHalves {
            outbound: m.outbound,
            inbound: m.inbound,
        }
    }
}

/// Take the per-mesh wirings out of `mpp_state.meshes`, replacing them with
/// empty `Vec`s so the borrow checker is satisfied. Drops the participant's
/// side-specific variant on the floor — only the generic `MeshHalves` survive.
///
/// Panics (via `pgrx::error!`) if `meshes.len() != expected`. This is a
/// leader/worker contract mismatch and must surface loudly, not silently.
fn take_meshes(state: &mut MppExecutionState, expected: usize) -> Vec<MeshHalves> {
    match state {
        MppExecutionState::Leader(ctx) => {
            let taken = std::mem::take(&mut ctx.meshes);
            if taken.len() != expected {
                pgrx::error!(
                    "mpp: leader meshes.len()={} but shape expected {}",
                    taken.len(),
                    expected
                );
            }
            taken.into_iter().map(MeshHalves::from).collect()
        }
        MppExecutionState::Worker(ctx) => {
            let taken = std::mem::take(&mut ctx.meshes);
            if taken.len() != expected {
                pgrx::error!(
                    "mpp: worker meshes.len()={} but shape expected {}",
                    taken.len(),
                    expected
                );
            }
            taken.into_iter().map(MeshHalves::from).collect()
        }
    }
}

fn build_shuffle_wiring(
    key_columns: Vec<usize>,
    total_participants: u32,
    participant_index: u32,
    outbound_senders: Vec<Option<MppSender>>,
    cooperative_drain: Arc<DrainHandle>,
) -> ShuffleWiring {
    ShuffleWiring {
        partitioner: Arc::new(HashPartitioner::new(key_columns, total_participants)),
        outbound_senders,
        participant_index,
        cooperative_drain: Some(cooperative_drain),
    }
}

fn spawn_drain(inbound: Vec<Option<MppReceiver>>) -> Arc<DrainHandle> {
    // `inbound[participant_index]` is always `None`; flatten drops it. Every
    // other peer contributes exactly one receiver.
    let receivers: Vec<_> = inbound.into_iter().flatten().collect();
    let num_sources = receivers.len();
    // `DrainBuffer::new(0)` would flip to EOF immediately; give it at least 1.
    let buffer = DrainBuffer::new(num_sources.max(1) as u32);
    let _ = DEFAULT_MPP_QUEUE_BYTES; // referenced only for docs consistency

    // Cooperative (not thread-backed) drain: pgrx's `check_active_thread`
    // panics any pg FFI call from non-backend threads, so spawning a
    // `std::thread` to read from `shm_mq` would die on its first
    // `shm_mq_receive`. The drain work runs inline from
    // `DrainGatherStream::poll_next` on the backend thread; the returned
    // `Arc` is also held by each same-mesh `MppSender` so `send_batch` can
    // cooperatively poll the inbound during would-block retries, breaking
    // the symmetric-send deadlock on a single-threaded runtime.
    Arc::new(DrainHandle::cooperative(receivers, buffer))
}

/// Inject `drain` into each outbound sender so its `send_batch` can
/// cooperatively poll our inbound during would-block retries — breaks
/// the symmetric-send deadlock on a single-threaded runtime.
fn attach_cooperative_drain(
    senders: Vec<Option<MppSender>>,
    drain: &Arc<DrainHandle>,
) -> Vec<Option<MppSender>> {
    senders
        .into_iter()
        .map(|opt| opt.map(|s| s.with_cooperative_drain(Arc::clone(drain))))
        .collect()
}

/// Stamp every outbound sender in `senders` with a `FrameId` computed from
/// the shared `task_key` (one per mesh — query_id + stage_id + our
/// participant_index as `task_number`) plus the sender's position in the
/// `Vec` as `partition`. Position == destination seat index today: the
/// outbound vec is already built that way by `take_meshes`. P5b will
/// decouple the two when multiple logical streams share one shm_mq.
fn stamp_frame_ids(
    senders: Vec<Option<MppSender>>,
    task_key: MppTaskKey,
) -> Vec<Option<MppSender>> {
    senders
        .into_iter()
        .enumerate()
        .map(|(partition, opt)| opt.map(|s| s.with_frame_id(task_key, partition as u32)))
        .collect()
}

/// Convenience: build an `MppTaskKey` identifying the local seat as the
/// producer at a given stage. `task_number == participant_index` because
/// the MPP mesh has one task per seat in every stage today.
fn task_key_for(mpp_state: &MppExecutionState, stage: MppStage) -> MppTaskKey {
    MppTaskKey {
        query_id: stage.query_id,
        stage_id: stage.stage_id,
        task_number: mpp_state.participant_config().participant_index,
    }
}

// ============================================================================
// Plan-walking primitives. Find the Partial agg / HashJoin in a standard
// DataFusion plan, skipping the coalesce/final wrapper layers the planner
// may have inserted.
// ============================================================================

/// Walk a standard physical plan to find the top-most `AggregateExec(Partial)`
/// whose transitive child (skipping `CoalescePartitionsExec` /
/// `CoalesceBatchesExec`) is a `HashJoinExec`. Returns references into the
/// original plan; the assembler then clones the pieces it needs.
fn find_partial_agg_and_join(
    plan: &dyn ExecutionPlan,
) -> DfResult<(&AggregateExec, &HashJoinExec)> {
    let partial = find_partial_agg(plan).ok_or_else(|| {
        DataFusionError::Plan(
            "mpp: could not locate AggregateExec(Partial) in standard physical plan".into(),
        )
    })?;
    let join = find_hash_join(partial.input().as_ref()).ok_or_else(|| {
        DataFusionError::Plan(
            "mpp: AggregateExec(Partial) child is not a HashJoinExec (through coalesce \
             layers) — plan shape unexpected for binary-join aggregate"
                .into(),
        )
    })?;
    Ok((partial, join))
}

/// Recursively walk the plan skipping `AggregateExec(Final)`,
/// `CoalescePartitionsExec`, `CoalesceBatchesExec`, and other pass-through
/// nodes to find an `AggregateExec` in `Partial` or `Single` mode.
///
/// DataFusion's planner picks `AggregateMode::Single` when the input
/// produces exactly one partition (our usual serial build), and
/// `Partial + FinalPartitioned` when it produces multiple partitions —
/// both are equally valid as the "aggregate atop the join" we want to
/// rebuild from. The assembler always emits `Partial` on top of the
/// shuffled join, regardless of which mode the serial plan used.
fn find_partial_agg(plan: &dyn ExecutionPlan) -> Option<&AggregateExec> {
    if let Some(agg) = plan.as_any().downcast_ref::<AggregateExec>() {
        if matches!(agg.mode(), AggregateMode::Partial | AggregateMode::Single) {
            return Some(agg);
        }
        // Final / FinalPartitioned / SinglePartitioned: descend into its child.
        return find_partial_agg(agg.input().as_ref());
    }
    if let Some(cp) = plan.as_any().downcast_ref::<CoalescePartitionsExec>() {
        return find_partial_agg(cp.input().as_ref());
    }
    if let Some(cb) = plan.as_any().downcast_ref::<CoalesceBatchesExec>() {
        return find_partial_agg(cb.input().as_ref());
    }
    // Generic single-child pass-through fallback: only if the node has exactly
    // one child, to avoid descending into a join or union.
    let children = plan.children();
    if children.len() == 1 {
        return find_partial_agg(children[0].as_ref());
    }
    None
}

/// Find a `HashJoinExec` underneath a Partial aggregate's input (or
/// under the `standard` root when the plan is a bare join), tolerating
/// `CoalescePartitionsExec` / `CoalesceBatchesExec` layers the planner
/// may insert between them.
fn find_hash_join(plan: &dyn ExecutionPlan) -> Option<&HashJoinExec> {
    if let Some(hj) = plan.as_any().downcast_ref::<HashJoinExec>() {
        return Some(hj);
    }
    if let Some(cp) = plan.as_any().downcast_ref::<CoalescePartitionsExec>() {
        return find_hash_join(cp.input().as_ref());
    }
    if let Some(cb) = plan.as_any().downcast_ref::<CoalesceBatchesExec>() {
        return find_hash_join(cb.input().as_ref());
    }
    let children = plan.children();
    if children.len() == 1 {
        return find_hash_join(children[0].as_ref());
    }
    None
}

/// Walk a join-input subtree and replace every `PgSearchScanPlan` with a
/// copy whose `dynamic_filters` Vec is empty. The `FilterPushdown` physical
/// optimizer pushed the HashJoin's dynamic-filter Arc into the probe-side
/// scan at planning time; MPP rewires so the same Arc is applied by a
/// `FilterExec` above the post-shuffle output instead.
fn strip_dynamic_filters_in_subtree(
    node: Arc<dyn ExecutionPlan>,
) -> DfResult<Arc<dyn ExecutionPlan>> {
    if node.as_any().downcast_ref::<PgSearchScanPlan>().is_some() {
        return PgSearchScanPlan::strip_dynamic_filters_from_dyn(node);
    }

    let children = node.children();
    if children.is_empty() {
        return Ok(node);
    }

    let new_children = children
        .iter()
        .map(|c| strip_dynamic_filters_in_subtree(Arc::clone(c)))
        .collect::<Result<Vec<_>, _>>()?;
    node.with_new_children(new_children)
}

/// Strip `RepartitionExec` and `CoalesceBatchesExec` layers off the top of a
/// plan, returning the underlying child. DataFusion inserts these between a
/// `HashJoinExec(Partitioned)` and its inputs to hash-repartition; in the MPP
/// plan we replace them with our own shuffle.
fn strip_repartition_layers(plan: Arc<dyn ExecutionPlan>) -> Arc<dyn ExecutionPlan> {
    if let Some(rp) = plan.as_any().downcast_ref::<RepartitionExec>() {
        return strip_repartition_layers(Arc::clone(rp.input()));
    }
    if let Some(cb) = plan.as_any().downcast_ref::<CoalesceBatchesExec>() {
        return strip_repartition_layers(Arc::clone(cb.input()));
    }
    plan
}

/// Extract `(left_col_idx, right_col_idx)` pairs from a `HashJoinExec::on()`
/// list, asserting every expression is a plain `Column` reference. Any other
/// expression (e.g., `CAST(col)`, `col + 0`) is rejected — the MPP
/// `HashPartitioner` only supports column keys today, and silently falling
/// through to DataFusion's own hash would diverge routing across workers.
#[allow(clippy::type_complexity)]
fn extract_key_col_indices(
    on: &[(Arc<dyn PhysicalExpr>, Arc<dyn PhysicalExpr>)],
) -> DfResult<(Vec<usize>, Vec<usize>)> {
    let mut left = Vec::with_capacity(on.len());
    let mut right = Vec::with_capacity(on.len());
    for (li, ri) in on {
        left.push(col_index(li)?);
        right.push(col_index(ri)?);
    }
    Ok((left, right))
}

fn col_index(expr: &Arc<dyn PhysicalExpr>) -> DfResult<usize> {
    expr.as_any()
        .downcast_ref::<Column>()
        .map(|c| c.index())
        .ok_or_else(|| {
            DataFusionError::Plan(format!(
                "mpp: join key expression {expr} is not a plain Column — MPP shuffle only \
                 supports column-ref keys in milestone 1"
            ))
        })
}

/// Walk `root` top-down, replacing the first `HashJoinExec` found with
/// `replacement`. Outer wrappers are rebuilt via `with_new_children` so their
/// identities (and per-node state) refresh, which is required for nodes like
/// `VisibilityFilterExec` whose resolver table must be re-wired against the
/// new subtree. Returns `Ok(None)` if no `HashJoinExec` is present.
fn replace_first_hash_join(
    root: Arc<dyn ExecutionPlan>,
    replacement: Arc<dyn ExecutionPlan>,
) -> DfResult<Option<Arc<dyn ExecutionPlan>>> {
    if root.as_any().downcast_ref::<HashJoinExec>().is_some() {
        return Ok(Some(replacement));
    }
    let children = root.children();
    if children.is_empty() {
        return Ok(None);
    }
    let mut new_children: Vec<Arc<dyn ExecutionPlan>> = Vec::with_capacity(children.len());
    let mut replaced = false;
    for child in children {
        if !replaced {
            if let Some(new_child) =
                replace_first_hash_join(Arc::clone(child), Arc::clone(&replacement))?
            {
                new_children.push(new_child);
                replaced = true;
                continue;
            }
        }
        new_children.push(Arc::clone(child));
    }
    if replaced {
        Ok(Some(root.with_new_children(new_children)?))
    } else {
        Ok(None)
    }
}

// ============================================================================
// Post-build validation
// ============================================================================

/// Post-build validation: no `VisibilityFilterExec` may be a descendant of a
/// `ShuffleExec` in the produced MPP plan.
///
/// Rationale (see module doc): `VisibilityFilterExec` resolves packed
/// DocAddress → heap TID via a ctid-resolver table populated with segments
/// local to this seat. If a shuffle sits above a visibility filter, the
/// filter operates on rows originating locally only (fine), but if a
/// visibility filter sits below a shuffle we run the filter on rows that
/// came in over shm_mq from a peer seat, whose `seg_ord` addresses the
/// peer's segment catalog rather than ours — `heap_fetch` then returns
/// garbage or panics.
///
/// The walk is cheap (O(nodes)) and runs once per query, so there is no
/// perf reason to gate this behind a GUC.
pub fn assert_visibility_invariant(plan: &Arc<dyn ExecutionPlan>) -> DfResult<()> {
    walk_checking_visibility(plan.as_ref(), /*inside_shuffle=*/ false)
}

fn walk_checking_visibility(node: &dyn ExecutionPlan, inside_shuffle: bool) -> DfResult<()> {
    use super::shuffle::ShuffleExec;
    use crate::postgres::customscan::joinscan::visibility_filter::VisibilityFilterExec;

    let is_shuffle = node.as_any().downcast_ref::<ShuffleExec>().is_some();
    let is_visibility = node
        .as_any()
        .downcast_ref::<VisibilityFilterExec>()
        .is_some();

    if inside_shuffle && is_visibility {
        return Err(DataFusionError::Plan(
            "mpp: visibility invariant violated — VisibilityFilterExec appears below a \
             ShuffleExec. Segment-local ctid resolution cannot run on rows that crossed \
             an shm_mq boundary from a peer seat (peer seg_ord addresses its own segment \
             catalog, not ours). Place every shuffle cut above any VisibilityFilterExec \
             in the standard plan."
                .into(),
        ));
    }

    let next_inside_shuffle = inside_shuffle || is_shuffle;
    for child in node.children() {
        walk_checking_visibility(child.as_ref(), next_inside_shuffle)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres::customscan::mpp::transport::{in_proc_channel, MppSender};
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::physical_expr::expressions::{BinaryExpr, Literal};
    use datafusion::scalar::ScalarValue;

    #[test]
    fn cut_count_matches_topology() {
        assert_eq!(cut_count_for_shape(MppPlanShape::ScalarAggOnBinaryJoin), 3);
        assert_eq!(cut_count_for_shape(MppPlanShape::GroupByAggOnBinaryJoin), 3);
        assert_eq!(cut_count_for_shape(MppPlanShape::GroupByAggSingleTable), 1);
        assert_eq!(cut_count_for_shape(MppPlanShape::JoinOnly), 2);
        assert_eq!(cut_count_for_shape(MppPlanShape::Ineligible), 0);
    }

    #[test]
    fn worst_case_cut_count_matches_widest_shape() {
        // Scalar-agg-on-join and groupby-agg-on-join are tied at 3 cuts.
        // If someone adds a shape with more cuts, this assertion surfaces
        // the mismatch.
        assert_eq!(worst_case_cut_count(), 3);
    }

    #[test]
    fn stamp_frame_ids_assigns_destination_partition_per_slot() {
        // The vec slot index equals the destination seat today. If
        // `stamp_frame_ids` ever stops using the slot index as partition,
        // the wire format's routing guarantee breaks.
        let (tx1, _rx1) = in_proc_channel(1);
        let (tx3, _rx3) = in_proc_channel(1);
        let senders: Vec<Option<MppSender>> = vec![
            None, // seat 0 (self)
            Some(MppSender::new(Box::new(tx1))),
            None, // gap — simulates a partially-built mesh on a larger cluster
            Some(MppSender::new(Box::new(tx3))),
        ];

        let stage = MppStage::new(0xa5a5, 2, 4);
        let stamped = stamp_frame_ids(
            senders,
            MppTaskKey {
                query_id: stage.query_id,
                stage_id: stage.stage_id,
                task_number: 0, // pretend we're seat 0
            },
        );

        assert!(stamped[0].is_none());
        let f1 = stamped[1].as_ref().unwrap().frame_id().unwrap();
        assert_eq!(f1.partition, 1);
        assert_eq!(f1.task_key.stage_id, 2);
        assert_eq!(f1.task_key.task_number, 0);
        assert_eq!(f1.task_key.query_id, 0xa5a5);

        assert!(stamped[2].is_none());
        let f3 = stamped[3].as_ref().unwrap().frame_id().unwrap();
        assert_eq!(f3.partition, 3);
    }

    #[test]
    fn col_index_plain_column() {
        let col: Arc<dyn PhysicalExpr> = Arc::new(Column::new("id", 3));
        assert_eq!(col_index(&col).unwrap(), 3);
    }

    #[test]
    fn col_index_rejects_non_column() {
        let left: Arc<dyn PhysicalExpr> = Arc::new(Column::new("id", 0));
        let right: Arc<dyn PhysicalExpr> = Arc::new(Literal::new(ScalarValue::Int32(Some(0))));
        let expr: Arc<dyn PhysicalExpr> = Arc::new(BinaryExpr::new(
            left,
            datafusion::logical_expr::Operator::Plus,
            right,
        ));
        let err = col_index(&expr).unwrap_err();
        assert!(
            format!("{err}").contains("not a plain Column"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn extract_key_col_indices_round_trip() {
        let l0: Arc<dyn PhysicalExpr> = Arc::new(Column::new("a", 0));
        let r0: Arc<dyn PhysicalExpr> = Arc::new(Column::new("b", 5));
        let l1: Arc<dyn PhysicalExpr> = Arc::new(Column::new("c", 2));
        let r1: Arc<dyn PhysicalExpr> = Arc::new(Column::new("d", 7));
        let on = vec![(l0, r0), (l1, r1)];
        let (l, r) = extract_key_col_indices(&on).unwrap();
        assert_eq!(l, vec![0, 2]);
        assert_eq!(r, vec![5, 7]);
    }

    #[test]
    fn find_partial_agg_walks_through_coalesce_and_final() {
        use datafusion::arrow::array::{Int32Array, RecordBatch};
        use datafusion::datasource::memory::MemorySourceConfig;

        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![Arc::new(Int32Array::from(vec![1, 2, 3]))],
        )
        .unwrap();
        let input = MemorySourceConfig::try_new_from_batches(schema.clone(), vec![batch]).unwrap();

        // Build a Partial -> CoalescePartitions -> Final stack.
        let partial = Arc::new(
            AggregateExec::try_new(
                AggregateMode::Partial,
                PhysicalGroupBy::new_single(vec![]),
                vec![],
                vec![],
                input,
                schema.clone(),
            )
            .unwrap(),
        );
        let partial_schema = partial.schema();
        let coalesced: Arc<dyn ExecutionPlan> = Arc::new(CoalescePartitionsExec::new(partial));
        let final_agg = Arc::new(
            AggregateExec::try_new(
                AggregateMode::Final,
                PhysicalGroupBy::new_single(vec![]),
                vec![],
                vec![],
                coalesced,
                partial_schema,
            )
            .unwrap(),
        );

        let plan: Arc<dyn ExecutionPlan> = final_agg;
        let partial_ref = find_partial_agg(plan.as_ref()).expect("partial found");
        assert!(matches!(partial_ref.mode(), AggregateMode::Partial));
    }

    // The visibility-invariant walker is exercised by the regression tests
    // (mpp_join, mpp_exec) against real plans. A lightweight unit test on
    // synthetic `ExecutionPlan`s would require wiring up enough of
    // `VisibilityFilterExec` + `ShuffleExec` to make the downcast fire,
    // which duplicates the fixture the bridges already exercise. Skipped.
}
