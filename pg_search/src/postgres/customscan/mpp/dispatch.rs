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

//! Leader dispatch: the leader serializes the distributed physical plan once and ships each
//! task's subplan to the workers, so workers run their fragments without re-planning.
//!
//! Plans travel as `SetPlan` frames through the mesh rings, the same `SetPlanRequest` message
//! Flight ships over its coordinator stream. The DSM blob carries only the routing tables:
//! which destination proc each fragment's output partitions go to, which is push-side knowledge
//! the message has no field for. Every worker reads the blob and selects the `(stage, task)`
//! slots it owns under [`proc_for_task`]. The fork's `DistributedCodec` serializes the
//! `Network*Exec` boundaries; [`crate::scan::physical_codec`] serializes the `pg_search` execs.
//!
//! The blob rides DSM rather than the mesh rings: workers read it while setting up, before they
//! attach to the mesh, so startup needs no receive loop; one read-only copy serves every worker;
//! and the rings stay sized for row traffic (`paradedb.mpp_queue_size`), not plan payloads.

use std::sync::Arc;

use datafusion::common::{DataFusionError, Result};
use datafusion::physical_plan::ExecutionPlan;
use datafusion::prelude::SessionContext;

use datafusion_distributed::shm::{proc_for_task, MppMesh};

use crate::postgres::customscan::mpp::exec_worker::build_mpp_session_context;
use crate::postgres::customscan::mpp::worker_fragments::{
    collect_dispatched_stages, FragmentAssignment, FragmentRouting,
};

/// One stage of the dispatch blob: the metadata a worker needs to route a stage's output.
/// Shared by all workers; each selects the `task_idx` slots it owns. The stage's plan arrives
/// separately, as a per-task `SetPlan` frame.
#[derive(serde::Serialize, serde::Deserialize)]
struct DispatchedStage {
    stage_num: u32,
    task_count: usize,
    routing: FragmentRouting,
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

/// Build the dispatch payload (the routing blob, framed to `capacity`) from the leader's own
/// execution plan, and report how many producer stages it found. The stage subplans travel
/// separately: the coordinator serializes each through the leader's
/// [`crate::postgres::customscan::mpp::glue::StagePlanDispatchSource`] as it dispatches.
///
/// The leader executes the same plan object the routing derives from, so stage numbering and
/// routing cannot drift from what the coordinator dispatches.
pub fn dispatch_payload_from_plan(
    physical: &Arc<dyn ExecutionPlan>,
    n_workers: u32,
    capacity: usize,
) -> Result<(Vec<u8>, usize)> {
    let stages = collect_dispatched_stages(physical, n_workers)?;
    let stage_count = stages.len();
    let dispatched: Vec<DispatchedStage> = stages
        .into_iter()
        .map(|stage| DispatchedStage {
            stage_num: stage.stage_num,
            task_count: stage.task_count,
            routing: stage.routing,
        })
        .collect();
    let blob = bincode::serde::encode_to_vec(&dispatched, blob_config())
        .map_err(|e| DataFusionError::Internal(format!("mpp dispatch: blob encode: {e}")))?;
    let payload = frame_dispatch_payload(&blob, capacity)?;
    Ok((payload, stage_count))
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
    this_proc: u32,
    n_workers: u32,
) {
    for task_idx in 0..task_count {
        if proc_for_task(n_workers, task_idx as u32) == this_proc {
            out.push(FragmentAssignment {
                stage_id: stage_num,
                task_idx,
                task_count,
                routing: routing.clone(),
            });
        }
    }
}

/// Expand the dispatch blob body into this worker's fragment assignments: the `(stage, task)`
/// slots this proc owns plus their routing. The plans arrive separately, one `SetPlan` frame per
/// fragment.
fn expand_to_assignments(
    body: &[u8],
    this_proc: u32,
    n_workers: u32,
) -> Result<Vec<FragmentAssignment>> {
    let stages = read_dispatch_blob(body)?;
    let mut out = Vec::new();
    for stage in stages {
        push_owned_tasks(
            &mut out,
            stage.stage_num,
            stage.task_count,
            &stage.routing,
            this_proc,
            n_workers,
        );
    }
    Ok(out)
}

/// Build this worker's fragment assignments from the DSM dispatch payload. Returns the
/// distributed session context too, since the caller needs it to decode and run the fragments.
pub fn fragments_for_worker(
    plan_bytes: &[u8],
    seed: SessionContext,
    mesh: Arc<MppMesh>,
    this_proc: u32,
    n_workers: u32,
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
    let fragments = expand_to_assignments(body, this_proc, n_workers)?;
    Ok((fragments, session))
}
