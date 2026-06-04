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
use crate::postgres::customscan::mpp::runtime::proc_for_task;
use crate::postgres::customscan::mpp::worker_fragments::{
    collect_dispatched_stages, FragmentAssignment, FragmentRouting,
};
use crate::postgres::ParallelScanState;
use crate::scan::codec::deserialize_logical_plan_with_runtime;
use crate::scan::physical_codec::{deserialize_physical_plan_with_runtime, serialize_physical_plan};

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

/// bincode config for the dispatch blob. Pinned so leader and worker agree.
fn blob_config() -> impl bincode::config::Config {
    bincode::config::standard()
}

/// DSM plan-region capacity for the dispatch payload, including the 8-byte length prefix.
///
/// The region is sized at `estimate_dsm` time, before the physical plan (and so the real blob
/// size) can be known, because `ParallelScanState` doesn't exist yet. Size generously from the
/// logical-plan length; an overflowing blob falls back to serial. `estimate_dsm` and
/// `initialize_dsm` MUST call this with the same `logical_len` so the DSM layout matches.
pub fn dispatch_plan_capacity(logical_len: usize) -> usize {
    8 + logical_len.saturating_mul(64).max(1 << 20)
}

/// Frame the blob into a fixed-capacity DSM payload: `[u64 len LE][blob][zero pad to capacity]`.
/// Errors if the blob plus prefix exceeds `capacity` so the caller can fall back to serial.
pub fn frame_dispatch_payload(blob: &[u8], capacity: usize) -> Result<Vec<u8>> {
    let needed = 8 + blob.len();
    if needed > capacity {
        return Err(DataFusionError::Internal(format!(
            "mpp dispatch: blob {needed} bytes exceeds DSM plan capacity {capacity}"
        )));
    }
    let mut payload = Vec::with_capacity(capacity);
    payload.extend_from_slice(&(blob.len() as u64).to_le_bytes());
    payload.extend_from_slice(blob);
    payload.resize(capacity, 0);
    Ok(payload)
}

/// Extract the blob from a framed DSM payload (inverse of [`frame_dispatch_payload`]).
fn unframe_dispatch_payload(payload: &[u8]) -> Result<&[u8]> {
    let len_bytes = payload.get(0..8).ok_or_else(|| {
        DataFusionError::Internal("mpp dispatch: payload too short for length prefix".into())
    })?;
    let len = u64::from_le_bytes(len_bytes.try_into().unwrap()) as usize;
    payload.get(8..8 + len).ok_or_else(|| {
        DataFusionError::Internal(format!(
            "mpp dispatch: payload length prefix {len} exceeds payload {}",
            payload.len()
        ))
    })
}

/// Build the dispatch blob on the leader at DSM-init, after `ParallelScanState` is populated:
/// deserialize the leader's logical plan, build the distributed physical plan once, and serialize
/// each producer stage's subplan.
///
/// The runtime state (`parallel_state` + per-source segment sets) is only needed to deserialize
/// the logical plan; serialization ships source indices, and each worker injects its own segments
/// on decode. `mesh = None` keeps the build structure-only -- the leader never opens a
/// `WorkerConnection` here, and the stage numbering matches the consumer plan it builds at exec.
#[allow(clippy::too_many_arguments)]
pub fn build_dispatch_blob(
    logical_bytes: &[u8],
    seed: SessionContext,
    n_workers: u32,
    parallel_state: Option<*mut ParallelScanState>,
    non_partitioning_segments: Vec<HashSet<SegmentId>>,
    index_segment_ids: Vec<HashSet<SegmentId>>,
    runtime: &tokio::runtime::Runtime,
) -> Result<Vec<u8>> {
    let logical = deserialize_logical_plan_with_runtime(
        logical_bytes,
        &seed.task_ctx(),
        parallel_state,
        None,
        None,
        non_partitioning_segments,
        index_segment_ids,
    )?;
    let session = build_mpp_session_context(seed, None);
    let physical =
        runtime.block_on(async { session.state().create_physical_plan(&logical).await })?;

    let stages = collect_dispatched_stages(&physical, n_workers);
    let mut dispatched = Vec::with_capacity(stages.len());
    for stage in stages {
        let plan_proto = serialize_physical_plan(stage.plan)?;
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

/// Expand the dispatch blob into this worker's fragment assignments. Each stage's subplan is
/// decoded once (injecting the worker's runtime context) and fanned out to the `task_idx` slots
/// this proc owns. Mirrors what the worker re-plan + `find_worker_assignments` produced before.
#[allow(clippy::too_many_arguments)]
pub fn expand_to_assignments(
    payload: &[u8],
    this_proc: u32,
    n_workers: u32,
    decode_ctx: &TaskContext,
    parallel_state: Option<*mut ParallelScanState>,
    non_partitioning_segments: &[HashSet<SegmentId>],
    index_segment_ids: &[HashSet<SegmentId>],
) -> Result<Vec<FragmentAssignment>> {
    let stages = read_dispatch_blob(payload)?;
    let mut out = Vec::new();
    for stage in stages {
        let plan = deserialize_physical_plan_with_runtime(
            &stage.plan_proto,
            decode_ctx,
            parallel_state,
            non_partitioning_segments.to_vec(),
            index_segment_ids.to_vec(),
        )?;
        for task_idx in 0..stage.task_count {
            if proc_for_task(n_workers, task_idx as u32) == this_proc {
                out.push(FragmentAssignment {
                    stage_id: stage.stage_num,
                    task_idx,
                    task_count: stage.task_count,
                    plan: Arc::clone(&plan),
                    routing: stage.routing.clone(),
                });
            }
        }
    }
    Ok(out)
}
