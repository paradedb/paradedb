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

//! Bridge between DataFusion's "standard" physical plan and the MPP
//! shape-specific physical plan.
//!
//! At exec time we let DataFusion's planner do its normal work to produce a
//! physical plan like:
//!
//! ```text
//!     AggregateExec(Final, ...)
//!       CoalescePartitionsExec
//!         AggregateExec(Partial, ...)
//!           HashJoinExec(Partitioned)
//!             RepartitionExec(Hash([left_key]))
//!               <left scan + filter>
//!             RepartitionExec(Hash([right_key]))
//!               <right scan>
//! ```
//!
//! [`build_mpp_physical_plan`] walks that plan, extracts the pieces it needs
//! (the `AggregateExec(Partial)`, the `HashJoinExec`, each side's scan+filter
//! child), and re-assembles them using the MPP shape builders in
//! [`super::shape`]. The resulting plan emits *partial* aggregate rows per
//! worker — PG's `Gather` + `Finalize Aggregate` finalizes across workers.
//!
//! # Leader/worker mesh-index contract
//!
//! For the `ScalarAggOnBinaryJoin` shape we consume **three** meshes out of
//! the per-side `MppExecutionState::meshes` vector. The contract —
//! consistent across every participant — is:
//!
//! - `meshes[0]` is wired into the **left** join input's shuffle.
//! - `meshes[1]` is wired into the **right** join input's shuffle.
//! - `meshes[2]` is the **final-gather-to-leader** shuffle: every
//!   participant ships its Partial aggregate row to seat 0 (the leader)
//!   via a `FixedTargetPartitioner`. The leader's
//!   `AggregateExec(FinalPartitioned)` then sums all N partials and emits
//!   one row; workers' `FinalPartitioned` sees zero rows and emits zero.
//!   This is what makes PG's `Gather` see exactly one row per query
//!   instead of N (one per worker).
//!
//! This is the first consumer of `mpp_state.meshes`, so it owns the
//! convention. Future callers (e.g., the `GroupByAggOnBinaryJoin` post-
//! Partial shuffle on mesh 2) must keep mesh-index ordering identical on
//! leader and workers — the meshes themselves are symmetric, but a
//! mismatch would route left rows to the right drain and break
//! correctness.

// caller lands in aggregatescan/mod.rs in this same change.
// `CoalesceBatchesExec` is deprecated in favor of arrow-rs's `BatchCoalescer`, but
// DataFusion 52's planner still emits `CoalesceBatchesExec` nodes in the physical
// plan we receive. We must be able to recognize and skip them when walking to find
// the `HashJoinExec` / `AggregateExec(Partial)`. Drop the allow once DataFusion
// stops emitting them.
#![allow(deprecated)]

use std::sync::Arc;

use datafusion::arrow::datatypes::SchemaRef;
use datafusion::common::DataFusionError;
use datafusion::physical_expr::expressions::Column;
use datafusion::physical_expr::PhysicalExpr;
use datafusion::physical_plan::aggregates::{AggregateExec, AggregateMode};
use datafusion::physical_plan::coalesce_batches::CoalesceBatchesExec;
use datafusion::physical_plan::coalesce_partitions::CoalescePartitionsExec;
use datafusion::physical_plan::joins::HashJoinExec;
use datafusion::physical_plan::repartition::RepartitionExec;
use datafusion::physical_plan::ExecutionPlan;

use crate::postgres::customscan::mpp::customscan_glue::{
    MppExecutionState, DEFAULT_MPP_QUEUE_BYTES,
};
use crate::postgres::customscan::mpp::shape::{
    build_groupby_agg_on_binary_join, build_join_only, build_scalar_agg_on_binary_join,
    GroupByAggOnBinaryJoinInputs, JoinOnlyInputs, MppPlanShape, ScalarAggOnBinaryJoinInputs,
};
use crate::postgres::customscan::mpp::shuffle::{
    FixedTargetPartitioner, HashPartitioner, RowPartitioner, ShuffleWiring,
};
use crate::postgres::customscan::mpp::stage::MppStage;
use crate::postgres::customscan::mpp::transport::{DrainBuffer, DrainHandle};
use crate::postgres::customscan::mpp::worker::{LeaderMesh, WorkerMesh};
use crate::scan::execution_plan::PgSearchScanPlan;

/// Single directed mesh extracted from the per-scan MPP state. A
/// participant-agnostic adapter over `LeaderMesh` / `WorkerMesh`, whose
/// shapes are identical but whose type-level variants force the rest of the
/// bridge to branch for no reason.
struct MeshHalves {
    outbound: Vec<Option<crate::postgres::customscan::mpp::transport::MppSender>>,
    inbound: Vec<Option<crate::postgres::customscan::mpp::transport::MppReceiver>>,
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

/// Transform a DataFusion standard physical plan into an MPP physical plan
/// for the given shape.
///
/// Consumes the `meshes` field of `mpp_state` (replacing it with an empty
/// `Vec`). Safe to call at most once per scan lifecycle.
///
/// Only [`MppPlanShape::ScalarAggOnBinaryJoin`] is implemented in milestone 1;
/// other shapes `pgrx::error!` with a clear "not yet implemented in M1"
/// message. Callers must pre-filter on shape before entering the MPP branch.
pub fn build_mpp_physical_plan(
    standard: Arc<dyn ExecutionPlan>,
    mpp_state: &mut MppExecutionState,
    shape: MppPlanShape,
) -> Result<Arc<dyn ExecutionPlan>, DataFusionError> {
    // Dark-launched dispatch: when `paradedb.mpp_use_generic_walker` is on,
    // route through the P3 cut-walker instead of the shape-specific bridges.
    // The walker currently returns `AnnotateError::NotYetImplemented`, so
    // flipping the GUC in A/B testing fails loudly rather than silently
    // falling back — that's intentional. Leave the GUC off in production.
    if crate::gucs::mpp_use_generic_walker() {
        return crate::postgres::customscan::mpp::walker::annotate_plan(standard, mpp_state, shape);
    }

    match shape {
        MppPlanShape::ScalarAggOnBinaryJoin => {
            build_scalar_agg_on_binary_join_bridge(standard, mpp_state)
        }
        MppPlanShape::GroupByAggOnBinaryJoin => {
            build_groupby_agg_on_binary_join_bridge(standard, mpp_state)
        }
        MppPlanShape::JoinOnly => build_join_only_bridge(standard, mpp_state),
        MppPlanShape::GroupByAggSingleTable => {
            pgrx::error!("mpp: shape {:?} not yet implemented in M1", shape);
        }
        MppPlanShape::Ineligible => {
            pgrx::error!(
                "mpp: build_mpp_physical_plan invoked with Ineligible shape — \
                 caller should have routed to the non-MPP serial path"
            );
        }
    }
}

fn build_scalar_agg_on_binary_join_bridge(
    standard: Arc<dyn ExecutionPlan>,
    mpp_state: &mut MppExecutionState,
) -> Result<Arc<dyn ExecutionPlan>, DataFusionError> {
    // 1) Walk the standard plan to pull out the Partial aggregate and the
    //    HashJoin underneath it.
    let (partial_agg, hash_join) = find_partial_agg_and_join(standard.as_ref())?;

    // 2) Strip any RepartitionExec / CoalesceBatchesExec layers on each join
    //    input. Those are DataFusion's single-process repartition inserted
    //    by the planner; we replace them with MPP shuffles.
    //
    //    Segment sharding already happened one layer up: the exec path sets
    //    `MppShardConfig` as a DataFusion SessionConfig extension before the
    //    logical plan is built; `PgSearchTableProvider::scan` reads it and
    //    emits a `PgSearchScanPlan` whose single `ScanState` wraps only
    //    this participant's slice of segments (lazy path — the aggregate
    //    benchmark's path). Don't re-shard here; the earlier version that
    //    did this at the `states` vec level would drop the (already-
    //    sharded) single ScanState on workers because its enumerate index
    //    is 0 and `0 % total != participant_index` for workers,
    //    effectively reverting to leader-only scanning.

    // Read both values from the leader-stamped `participant_config` — it's the
    // single source of truth for the shape the DSM mesh was allocated against.
    // Reading `total_participants` from the GUC at exec time could disagree if
    // the GUC was changed after planning (workers would still be wired as if
    // the old N was in effect, so the partitioner must match).
    let participant_index = mpp_state.participant_config().participant_index;
    let total_participants = mpp_state.participant_config().total_participants;

    let left_child = strip_repartition_layers(Arc::clone(hash_join.left()));
    // Strip any dynamic filter Arc from the probe-side scan so it is not
    // applied before the MPP shuffle routes rows to peer participants. The
    // shape builder re-attaches the same Arc as a `FilterExec` above the
    // post-shuffle output where it is safe to filter: a participant's local
    // build side covers exactly the keys that hash-route to its own probe.
    let right_child =
        strip_dynamic_filters_in_subtree(strip_repartition_layers(Arc::clone(hash_join.right())))?;

    // Clone the HashJoinExec via its builder (the exec isn't `Clone` — the
    // build-side `OnceFut` forbids it). The `From<&HashJoinExec>` impl copies
    // `dynamic_filter`, so `.builder().build()` hands us a standalone copy
    // ready to be re-children'd in the shape builder.
    let original_hash_join = Arc::new(hash_join.builder().build()?);

    // 3) Extract per-side key column indices from the join `on` list.
    let join_on = hash_join.on().to_vec();
    let (left_keys, right_keys) = extract_key_col_indices(&join_on)?;

    // 4) Pull mesh wirings out of mpp_state. Mesh 0 = left, mesh 1 = right,
    //    mesh 2 = final-gather-to-leader (see module-level contract).
    let mut meshes = take_meshes(mpp_state, 3);
    let final_mesh = meshes.pop().expect("len checked in take_meshes");
    let right_mesh = meshes.pop().expect("len checked in take_meshes");
    let left_mesh = meshes.pop().expect("len checked in take_meshes");

    // Build drains before shuffle wirings so each mesh's senders carry a
    // cooperative-drain share — needed to break the symmetric-send
    // deadlock on a single-threaded runtime (see `MppSender::send_batch`).
    let left_drain = spawn_drain(left_mesh.inbound);
    let right_drain = spawn_drain(right_mesh.inbound);
    let final_drain = spawn_drain(final_mesh.inbound);

    let left_outbound = attach_cooperative_drain(left_mesh.outbound, &left_drain);
    let right_outbound = attach_cooperative_drain(right_mesh.outbound, &right_drain);
    let final_outbound = attach_cooperative_drain(final_mesh.outbound, &final_drain);

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

    // Final-gather mesh: route every Partial row to seat 0 (leader).
    let final_shuffle = ShuffleWiring {
        partitioner: Arc::new(FixedTargetPartitioner::new(0, total_participants))
            as Arc<dyn RowPartitioner>,
        outbound_senders: final_outbound,
        participant_index,
        cooperative_drain: Some(Arc::clone(&final_drain)),
    };

    let left_schema: SchemaRef = left_child.schema();
    let right_schema: SchemaRef = right_child.schema();

    // 5) Extract aggregate expressions.
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
    let query_id = mpp_state.query_id();
    // Mesh-index / stage-id contract (see module-level doc):
    //   mesh 0 = left pre-join, stage_id = 0
    //   mesh 1 = right pre-join, stage_id = 1
    //   mesh 2 = final-gather-to-leader, stage_id = 2
    let left_stage = MppStage::new(query_id, 0, total_participants);
    let right_stage = MppStage::new(query_id, 1, total_participants);
    let final_stage = MppStage::new(query_id, 2, total_participants);
    build_scalar_agg_on_binary_join(
        ScalarAggOnBinaryJoinInputs {
            left_child,
            right_child,
            left_shuffle,
            left_drain,
            left_schema,
            right_shuffle,
            right_drain,
            right_schema,
            aggr_expr,
            filter_expr,
            final_shuffle,
            final_drain,
            original_hash_join,
            left_stage,
            right_stage,
            final_stage,
        },
        is_leader,
    )
}

fn build_groupby_agg_on_binary_join_bridge(
    standard: Arc<dyn ExecutionPlan>,
    mpp_state: &mut MppExecutionState,
) -> Result<Arc<dyn ExecutionPlan>, DataFusionError> {
    // Same plan-walking as the scalar case: Partial aggregate sits atop
    // HashJoin with sharded scans below each side.
    let (partial_agg, hash_join) = find_partial_agg_and_join(standard.as_ref())?;

    let left_child = strip_repartition_layers(Arc::clone(hash_join.left()));
    // See `build_scalar_agg_on_binary_join_bridge` for why we strip dynamic
    // filters from the probe side and clone the HashJoinExec via its builder.
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

    // Build drains before wirings so each mesh's senders carry a cooperative
    // drain share — see `MppSender::send_batch`.
    let left_drain = spawn_drain(left_mesh.inbound);
    let right_drain = spawn_drain(right_mesh.inbound);
    let postagg_drain = spawn_drain(postagg_mesh.inbound);

    let left_outbound = attach_cooperative_drain(left_mesh.outbound, &left_drain);
    let right_outbound = attach_cooperative_drain(right_mesh.outbound, &right_drain);
    let postagg_outbound = attach_cooperative_drain(postagg_mesh.outbound, &postagg_drain);

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
    // Postagg mesh hash-partitions Partial rows by group key. Each group
    // lives on exactly one seat and `AggregateExec(FinalPartitioned)` on
    // that seat emits one row per distinct group; PG's Gather concatenates
    // across seats. `shape::build_groupby_agg_on_binary_join` coalesces
    // the Partial output into 64 Ki-row batches before this shuffle so
    // per-batch Arrow-IPC encode cost stays amortized.
    //
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

    // Symmetric plan on every seat. Hash partitioning ensures each group
    // lands on exactly one seat's Final, so PG's Gather concatenates
    // without double-count.
    let query_id = mpp_state.query_id();
    // Mesh-index / stage-id contract: mesh 0=left, 1=right, 2=postagg.
    let left_stage = MppStage::new(query_id, 0, total_participants);
    let right_stage = MppStage::new(query_id, 1, total_participants);
    let postagg_stage = MppStage::new(query_id, 2, total_participants);
    build_groupby_agg_on_binary_join(GroupByAggOnBinaryJoinInputs {
        left_child,
        right_child,
        left_shuffle,
        left_drain,
        left_schema,
        right_shuffle,
        right_drain,
        right_schema,
        group_by,
        aggr_expr,
        filter_expr,
        postagg_shuffle,
        postagg_drain,
        original_hash_join,
        left_stage,
        right_stage,
        postagg_stage,
    })
}

/// Build the MPP physical plan for the `JoinOnly` shape: a bare join without
/// an aggregate on top. Mirrors `build_scalar_agg_on_binary_join_bridge`
/// without the Partial aggregate walk and the final-gather mesh — two meshes
/// (left + right pre-join), no asymmetric leader/worker plan.
fn build_join_only_bridge(
    standard: Arc<dyn ExecutionPlan>,
    mpp_state: &mut MppExecutionState,
) -> Result<Arc<dyn ExecutionPlan>, DataFusionError> {
    // For the `JoinOnly` shape there's no Partial aggregate above the join —
    // the `HashJoinExec` sits below zero or more wrapper layers such as
    // `VisibilityFilterExec`, `SegmentedTopKExec`, `TantivyLookupExec`,
    // `ProjectionExec`, etc. We must preserve those wrappers on the MPP path:
    // for deferred-visibility queries, `VisibilityFilterExec` is what turns
    // packed DocAddresses in the `ctid` column into real heap TIDs — without
    // it, `JoinScan::build_result_tuple` passes an unpacked DocAddress to
    // `heap_fetch`, tripping `ItemPointerIsValid`.
    let hash_join = find_hash_join(standard.as_ref()).ok_or_else(|| {
        DataFusionError::Plan(
            "mpp: could not locate HashJoinExec in standard physical plan for JoinOnly shape"
                .into(),
        )
    })?;

    let participant_index = mpp_state.participant_config().participant_index;
    let total_participants = mpp_state.participant_config().total_participants;

    let left_child = strip_repartition_layers(Arc::clone(hash_join.left()));
    // Same dynamic-filter stripping as the aggregate bridges — the probe-side
    // scan would otherwise filter before the shuffle and drop rows destined
    // for peer participants. Unlike the aggregate shapes, `build_join_only`
    // does not re-attach the filter above the shuffle; the HashJoin's own
    // dynamic filter (carried across via `builder().build()`) suffices because
    // no row-reducing aggregate sits above the join.
    let right_child =
        strip_dynamic_filters_in_subtree(strip_repartition_layers(Arc::clone(hash_join.right())))?;

    let join_on = hash_join.on().to_vec();
    let (left_keys, right_keys) = extract_key_col_indices(&join_on)?;

    // Two meshes: left + right pre-join shuffles. No post-Partial or
    // final-gather mesh for the JoinOnly shape.
    let mut meshes = take_meshes(mpp_state, 2);
    let right_mesh = meshes.pop().expect("len checked in take_meshes");
    let left_mesh = meshes.pop().expect("len checked in take_meshes");

    let left_drain = spawn_drain(left_mesh.inbound);
    let right_drain = spawn_drain(right_mesh.inbound);

    let left_outbound = attach_cooperative_drain(left_mesh.outbound, &left_drain);
    let right_outbound = attach_cooperative_drain(right_mesh.outbound, &right_drain);

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

    let query_id = mpp_state.query_id();
    // Mesh-index / stage-id contract: mesh 0=left pre-join, 1=right pre-join.
    let left_stage = MppStage::new(query_id, 0, total_participants);
    let right_stage = MppStage::new(query_id, 1, total_participants);
    let mpp_hash_join = build_join_only(JoinOnlyInputs {
        left_child,
        right_child,
        left_shuffle,
        left_drain,
        left_schema,
        right_shuffle,
        right_drain,
        right_schema,
        join_on,
        join_projection,
        join_type,
        left_stage,
        right_stage,
    })?;

    // Graft the MPP-shuffled `HashJoinExec` back into the standard plan tree
    // in place of the original `HashJoinExec`. Outer wrappers
    // (`VisibilityFilterExec`, `SegmentedTopKExec`, `TantivyLookupExec`,
    // `ProjectionExec`, ...) are preserved so ctid resolution, top-K, and
    // late column materialization still happen.
    let grafted =
        replace_first_hash_join(Arc::clone(&standard), mpp_hash_join)?.ok_or_else(|| {
            DataFusionError::Internal("mpp: HashJoinExec replacement failed to find target".into())
        })?;

    // `with_new_children` rebuilt any `VisibilityFilterExec` above the join
    // via `VisibilityFilterExec::new`, which resets `ctid_resolvers` to
    // empty. Re-run the resolver rule against the grafted tree so the new
    // exec is wired to the scans in its fresh subtree.
    use datafusion::common::config::ConfigOptions;
    use datafusion::physical_optimizer::PhysicalOptimizerRule;
    let config = ConfigOptions::default();
    crate::scan::visibility_ctid_resolver_rule::VisibilityCtidResolverRule
        .optimize(grafted, &config)
}

/// Walk `root` top-down, replacing the first `HashJoinExec` found with
/// `replacement`. Outer wrappers are rebuilt via `with_new_children` so their
/// identities (and per-node state) refresh, which is required for nodes like
/// `VisibilityFilterExec` whose resolver table must be re-wired against the
/// new subtree. Returns `Ok(None)` if no `HashJoinExec` is present.
fn replace_first_hash_join(
    root: Arc<dyn ExecutionPlan>,
    replacement: Arc<dyn ExecutionPlan>,
) -> Result<Option<Arc<dyn ExecutionPlan>>, DataFusionError> {
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

fn build_shuffle_wiring(
    key_columns: Vec<usize>,
    total_participants: u32,
    participant_index: u32,
    outbound_senders: Vec<Option<crate::postgres::customscan::mpp::transport::MppSender>>,
    cooperative_drain: Arc<DrainHandle>,
) -> ShuffleWiring {
    ShuffleWiring {
        partitioner: Arc::new(HashPartitioner::new(key_columns, total_participants)),
        outbound_senders,
        participant_index,
        cooperative_drain: Some(cooperative_drain),
    }
}

fn spawn_drain(
    inbound: Vec<Option<crate::postgres::customscan::mpp::transport::MppReceiver>>,
) -> Arc<DrainHandle> {
    // `inbound[participant_index]` is always `None`; flatten drops it. Every
    // other peer contributes exactly one receiver.
    let receivers: Vec<_> = inbound.into_iter().flatten().collect();
    let num_sources = receivers.len();
    // `DrainBuffer::new(0)` would flip to EOF immediately; give it at least 1.
    let buffer = DrainBuffer::new(num_sources.max(1) as u32);
    let _ = DEFAULT_MPP_QUEUE_BYTES; // only referenced for docs consistency

    // Cooperative (not thread-backed) drain: pgrx's `check_active_thread`
    // panics any pg FFI call from non-backend threads, so spawning a
    // `std::thread` to read from `shm_mq` would die on its first
    // `shm_mq_receive`. Instead the drain work runs inline from
    // `DrainGatherStream::poll_next` on the backend thread (see
    // `mpp::transport::DrainHandle::cooperative` +
    // `mpp::shuffle::DrainGatherStream::poll_next`).
    //
    // Returned as `Arc` so each same-mesh `MppSender` can hold a
    // cooperative-drain share (see `MppSender::with_cooperative_drain`),
    // letting the sender's would-block loop consume our inbound to
    // un-stall peer sends. The exec plan also keeps one strong reference
    // via `DrainGatherExec`.
    Arc::new(DrainHandle::cooperative(receivers, buffer))
}

/// Inject `drain` into each outbound sender so its `send_batch` can
/// cooperatively poll our inbound during would-block retries — breaks
/// the symmetric-send deadlock on a single-threaded runtime.
fn attach_cooperative_drain(
    senders: Vec<Option<crate::postgres::customscan::mpp::transport::MppSender>>,
    drain: &Arc<DrainHandle>,
) -> Vec<Option<crate::postgres::customscan::mpp::transport::MppSender>> {
    senders
        .into_iter()
        .map(|opt| opt.map(|s| s.with_cooperative_drain(Arc::clone(drain))))
        .collect()
}

/// Walk a standard physical plan to find the top-most `AggregateExec(Partial)`
/// whose transitive child (skipping `CoalescePartitionsExec` /
/// `CoalesceBatchesExec`) is a `HashJoinExec`. Returns references into the
/// original plan; the bridge then clones the pieces it needs.
fn find_partial_agg_and_join(
    plan: &dyn ExecutionPlan,
) -> Result<(&AggregateExec, &HashJoinExec), DataFusionError> {
    let partial = find_partial_agg(plan).ok_or_else(|| {
        DataFusionError::Plan(
            "mpp: could not locate AggregateExec(Partial) in standard physical plan".into(),
        )
    })?;
    let join = find_hash_join(partial.input().as_ref()).ok_or_else(|| {
        DataFusionError::Plan(
            "mpp: AggregateExec(Partial) child is not a HashJoinExec (through coalesce \
             layers) — plan shape unexpected for ScalarAggOnBinaryJoin"
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
/// rebuild from. The shape builder always emits `Partial` on top of the
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

/// Find a `HashJoinExec` underneath a Partial aggregate's input, tolerating
/// `CoalescePartitionsExec` / `CoalesceBatchesExec` layers the planner may
/// insert between them.
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
/// `FilterExec` above the post-shuffle output instead. See the matching
/// `FilterExec` wrapping in `shape::build_*_on_binary_join` for why
/// stripping is required for correctness (pre-shuffle filtering would drop
/// rows destined for peer participants).
fn strip_dynamic_filters_in_subtree(
    node: Arc<dyn ExecutionPlan>,
) -> Result<Arc<dyn ExecutionPlan>, DataFusionError> {
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
) -> Result<(Vec<usize>, Vec<usize>), DataFusionError> {
    let mut left = Vec::with_capacity(on.len());
    let mut right = Vec::with_capacity(on.len());
    for (li, ri) in on {
        left.push(col_index(li)?);
        right.push(col_index(ri)?);
    }
    Ok((left, right))
}

/// An `ExecutionPlan` that produces zero rows with the given schema. Kept
/// around as a utility even after scan sharding landed: there are future
/// edge cases (e.g. a participant whose entire shard is pruned at plan time)
/// where returning an empty source is the right answer.
#[allow(dead_code)]
fn empty_exec_with_schema(schema: SchemaRef) -> Arc<dyn ExecutionPlan> {
    use datafusion::physical_plan::empty::EmptyExec;
    Arc::new(EmptyExec::new(schema))
}

fn col_index(expr: &Arc<dyn PhysicalExpr>) -> Result<usize, DataFusionError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::physical_expr::expressions::{BinaryExpr, Literal};
    use datafusion::physical_expr::PhysicalExpr;
    use datafusion::scalar::ScalarValue;

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
        use datafusion::physical_plan::aggregates::PhysicalGroupBy;

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
}
