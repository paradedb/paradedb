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

//! Coordinator-dispatch blob: the leader serializes the distributed physical plan once and ships
//! per-stage subplans to the workers, so workers run their fragments without re-planning.
//!
//! The blob is one shared buffer in DSM. Every worker reads it and selects the `(stage, task)`
//! slots it owns under [`proc_for_task`]. The fork's `DistributedCodec` serializes the
//! `Network*Exec` boundaries; [`crate::scan::physical_codec`] serializes the `pg_search` execs.

use std::sync::Arc;

use datafusion::common::{DataFusionError, Result};
use datafusion::execution::TaskContext;
use datafusion::prelude::SessionContext;
use tantivy::index::SegmentId;

use crate::api::HashSet;
use crate::postgres::customscan::mpp::exec_worker::build_mpp_session_context;
use crate::postgres::customscan::mpp::runtime::{proc_for_task, MppMesh};
use crate::postgres::customscan::mpp::worker_fragments::{
    collect_dispatched_stages, FragmentAssignment, FragmentRouting,
};
use crate::postgres::ParallelScanState;
use crate::scan::codec::deserialize_logical_plan_with_runtime;
use crate::scan::physical_codec::{
    deserialize_physical_plan_with_runtime, serialize_physical_plan,
};

/// One stage of the dispatch blob: a serialized producer subplan plus the metadata a worker needs
/// to run and route it. Shared by all workers; each selects the `task_idx` slots it owns.
#[derive(serde::Serialize, serde::Deserialize)]
struct DispatchedStage {
    stage_num: u32,
    task_count: usize,
    routing: FragmentRouting,
    /// `PhysicalPlanNode`-encoded `stage.local_plan()` (via the combined codec).
    plan_proto: Vec<u8>,
}

/// First byte of the DSM plan region. Only the dispatch blob is shipped today (every MPP
/// customscan dispatches); the tag is kept so a future re-plan fallback can be distinguished
/// without a wire-format change.
pub const TAG_BLOB: u8 = 1;

/// bincode config for the dispatch blob. Pinned so leader and worker agree.
fn blob_config() -> impl bincode::config::Config {
    bincode::config::standard()
}

/// Physical-plan blobs grow with the logical plan but with a different constant: the physical
/// encoding repeats schemas per node and carries the dispatch descriptors. The factor is
/// deliberately generous because the region lives only for the query and an overflowing blob
/// falls back to serial, while an undersized region costs the MPP path.
const DISPATCH_BLOAT_FACTOR: usize = 64;
/// Floor for tiny logical plans, whose physical expansion is dominated by fixed overhead.
const DISPATCH_MIN_CAPACITY: usize = 1 << 20;

/// DSM plan-region capacity for the dispatch payload (`[tag][u64 len][blob][pad]`).
///
/// The region is sized at `estimate_dsm` time, before the physical plan (and so the real blob
/// size) can be known, because `ParallelScanState` doesn't exist yet. Size generously from the
/// logical-plan length; an overflowing blob falls back to serial. `estimate_dsm` and
/// `initialize_dsm` MUST call this with the same `logical_len` so the DSM layout matches.
pub fn dispatch_plan_capacity(logical_len: usize) -> usize {
    1 + 8
        + logical_len
            .saturating_mul(DISPATCH_BLOAT_FACTOR)
            .max(DISPATCH_MIN_CAPACITY)
}

/// Frame the blob into a fixed-capacity dispatch payload:
/// `[TAG_BLOB][u64 len LE][blob][zero pad to capacity]`. Errors if it doesn't fit `capacity` so
/// the caller can fall back to serial.
pub fn frame_dispatch_payload(blob: &[u8], capacity: usize) -> Result<Vec<u8>> {
    let needed = 1 + 8 + blob.len();
    if needed > capacity {
        return Err(DataFusionError::Internal(format!(
            "mpp dispatch: blob {needed} bytes exceeds DSM plan capacity {capacity}"
        )));
    }
    let mut payload = Vec::with_capacity(capacity);
    payload.push(TAG_BLOB);
    payload.extend_from_slice(&(blob.len() as u64).to_le_bytes());
    payload.extend_from_slice(blob);
    payload.resize(capacity, 0);
    Ok(payload)
}

/// Extract the blob from a framed dispatch payload body (the bytes after the mode tag).
fn unframe_dispatch_payload(body: &[u8]) -> Result<&[u8]> {
    let len_bytes = body.get(0..8).ok_or_else(|| {
        DataFusionError::Internal("mpp dispatch: payload too short for length prefix".into())
    })?;
    let len = u64::from_le_bytes(len_bytes.try_into().unwrap()) as usize;
    body.get(8..8 + len).ok_or_else(|| {
        DataFusionError::Internal(format!(
            "mpp dispatch: payload length prefix {len} exceeds payload {}",
            body.len()
        ))
    })
}

/// Build the dispatch blob on the leader at DSM-init: deserialize the leader's logical plan, build
/// the distributed physical plan once, and serialize each producer stage's subplan.
///
/// This is a structure-only build: no `parallel_state` is injected, so the leader never claims
/// segments while planning (a throttled sorted scan would otherwise checkout the workers' segments
/// against the shared state here; with no state it falls to an eager scan that the codec declines,
/// dropping the query to serial with the real state intact). Lazy scans ship source indices, and
/// each worker injects its own `ParallelScanState` + segments on decode. `mesh = None` keeps the
/// build off the transport; the stage numbering matches the consumer plan the leader builds at exec.
///
/// The logical bytes predate exec-time rewrites: joinscan injects a runtime `Limit` for
/// parameterized LIMIT/OFFSET only at exec, after this build. Workers re-planned from these same
/// bytes before dispatch existed, so the producer-stage shapes here match that precedent.
pub fn build_dispatch_blob(
    logical_bytes: &[u8],
    seed: SessionContext,
    n_workers: u32,
    runtime: &tokio::runtime::Runtime,
    non_partitioning_segments: &[HashSet<SegmentId>],
) -> Result<Vec<u8>> {
    let logical = deserialize_logical_plan_with_runtime(
        logical_bytes,
        &seed.task_ctx(),
        None,
        None,
        None,
        Vec::new(),
        Vec::new(),
    )?;
    let session = build_mpp_session_context(seed, None);
    let physical =
        runtime.block_on(async { session.state().create_physical_plan(&logical).await })?;

    let stages = collect_dispatched_stages(&physical, n_workers)?;
    let mut dispatched = Vec::with_capacity(stages.len());
    let decode_ctx = session.task_ctx();
    for stage in stages {
        let plan_proto = serialize_physical_plan(stage.plan)?;
        // Encode can succeed while decode fails (a codec gap). The first decode otherwise
        // happens in a worker, where failure is a hard query error instead of the serial
        // fallback this Result feeds. One extra decode per stage at init buys the fallback
        // (it re-opens the scans' readers, on top of the ones the physical planning above
        // already opened; both are released with the init context). With no ParallelScanState
        // here, a non-partitioning scan resolves its MVCC view from these canonical sets (by
        // its compacted `non_partitioning_index`); `index_segment_ids` stays empty, which
        // skips the UDF segment injection without failing it.
        deserialize_physical_plan_with_runtime(
            &plan_proto,
            &decode_ctx,
            None,
            non_partitioning_segments.to_vec(),
            Vec::new(),
        )?;
        dispatched.push(DispatchedStage {
            stage_num: stage.stage_num,
            task_count: stage.task_count,
            routing: stage.routing,
            plan_proto,
        });
    }
    bincode::serde::encode_to_vec(&dispatched, blob_config())
        .map_err(|e| DataFusionError::Internal(format!("mpp dispatch: blob encode: {e}")))
}

/// Decode the dispatch blob from the framed DSM payload a worker copied out of DSM.
fn read_dispatch_blob(payload: &[u8]) -> Result<Vec<DispatchedStage>> {
    let blob = unframe_dispatch_payload(payload)?;
    let (stages, _) = bincode::serde::decode_from_slice(blob, blob_config())
        .map_err(|e| DataFusionError::Internal(format!("mpp dispatch: blob decode: {e}")))?;
    Ok(stages)
}

/// Fan a stage out to the `task_idx` slots `this_proc` owns under `proc_for_task`.
fn push_owned_tasks(
    out: &mut Vec<FragmentAssignment>,
    stage_num: u32,
    task_count: usize,
    routing: &FragmentRouting,
    plan: &Arc<dyn datafusion::physical_plan::ExecutionPlan>,
    this_proc: u32,
    n_workers: u32,
) {
    for task_idx in 0..task_count {
        if proc_for_task(n_workers, task_idx as u32) == this_proc {
            out.push(FragmentAssignment {
                stage_id: stage_num,
                task_idx,
                task_count,
                plan: Arc::clone(plan),
                routing: routing.clone(),
            });
        }
    }
}

/// Builds and frames the dispatch payload for the DSM plan region: a single-thread runtime for
/// the planning pass, the per-stage blob, then the fixed-capacity frame. One `Err` covers the
/// whole pipeline so a caller warns once and falls back to serial. Capacity is derived from
/// `logical_bytes.len()`, the same input `estimate_dsm` sized the region with.
pub fn build_dispatch_payload(
    logical_bytes: &[u8],
    seed: SessionContext,
    n_workers: u32,
    non_partitioning_segments: &[HashSet<SegmentId>],
) -> Result<Vec<u8>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| DataFusionError::Internal(format!("mpp dispatch: runtime build: {e}")))?;
    let blob = build_dispatch_blob(
        logical_bytes,
        seed,
        n_workers,
        &runtime,
        non_partitioning_segments,
    )?;
    frame_dispatch_payload(&blob, dispatch_plan_capacity(logical_bytes.len()))
}

/// Expand the dispatch blob body into this worker's fragment assignments. Each stage's subplan is
/// decoded once (injecting the worker's runtime context) and fanned out to the `task_idx` slots
/// this proc owns.
#[allow(clippy::too_many_arguments)]
fn expand_to_assignments(
    body: &[u8],
    this_proc: u32,
    n_workers: u32,
    decode_ctx: &TaskContext,
    parallel_state: Option<*mut ParallelScanState>,
    non_partitioning_segments: &[HashSet<SegmentId>],
    index_segment_ids: &[HashSet<SegmentId>],
) -> Result<Vec<FragmentAssignment>> {
    let stages = read_dispatch_blob(body)?;
    let mut out = Vec::new();
    for stage in stages {
        // Decode opens index readers, so skip stages this proc owns no tasks of (broadcast
        // stages put all work on task 0, leaving the other procs with nothing).
        let owns_any = (0..stage.task_count)
            .any(|task_idx| proc_for_task(n_workers, task_idx as u32) == this_proc);
        if !owns_any {
            continue;
        }
        let plan = deserialize_physical_plan_with_runtime(
            &stage.plan_proto,
            decode_ctx,
            parallel_state,
            non_partitioning_segments.to_vec(),
            index_segment_ids.to_vec(),
        )?;
        push_owned_tasks(
            &mut out,
            stage.stage_num,
            stage.task_count,
            &stage.routing,
            &plan,
            this_proc,
            n_workers,
        );
    }
    Ok(out)
}

/// Build this worker's fragment assignments from the DSM dispatch payload: run the leader's
/// sliced per-stage subplans directly, without re-planning. Returns the distributed session
/// context too, since the caller needs it to run the fragments.
#[allow(clippy::too_many_arguments)]
pub fn fragments_for_worker(
    plan_bytes: &[u8],
    seed: SessionContext,
    mesh: Arc<MppMesh>,
    this_proc: u32,
    n_workers: u32,
    parallel_state: Option<*mut ParallelScanState>,
    non_partitioning_segments: &[HashSet<SegmentId>],
    index_segment_ids: &[HashSet<SegmentId>],
) -> Result<(Vec<FragmentAssignment>, SessionContext)> {
    let Some((&tag, body)) = plan_bytes.split_first() else {
        return Err(DataFusionError::Internal(
            "mpp dispatch: empty worker plan bytes".into(),
        ));
    };
    if tag != TAG_BLOB {
        return Err(DataFusionError::Internal(format!(
            "mpp dispatch: unexpected worker plan tag {tag}"
        )));
    }
    let session = build_mpp_session_context(seed, Some(mesh));
    // Decode against the full session task context: by-name functions (DataFusion built-ins)
    // resolve through its registry, while pg_search UDFs travel with their definition and
    // decode through the codec.
    let fragments = expand_to_assignments(
        body,
        this_proc,
        n_workers,
        &session.task_ctx(),
        parallel_state,
        non_partitioning_segments,
        index_segment_ids,
    )?;
    Ok((fragments, session))
}
