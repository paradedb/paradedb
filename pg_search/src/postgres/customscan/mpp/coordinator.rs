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

//! Façade for AggregateScan / JoinScan customscan hooks: one helper per DSM
//! hook point (estimate, leader-init, worker-attach), each a thin wrapper
//! over [`super::worker`] primitives plus plan-broadcast serialization.

use std::ffi::c_void;

use pgrx::pg_sys;

use crate::postgres::customscan::mpp::mesh::MeshLayout;
use crate::postgres::customscan::mpp::session::MppPlanBroadcast;
use crate::postgres::customscan::mpp::worker::{
    attach_dsm_as_worker, compute_dsm_layout, initialize_dsm_as_leader, LeaderMesh, WorkerMesh,
};
use crate::postgres::customscan::mpp::MppParticipantConfig;

/// Compute the exact DSM size the MPP custom scan needs for a plan of
/// `plan_bytes_len` bytes, N participants, `num_meshes` independent shuffle
/// meshes, and the configured per-edge queue size.
///
/// Returns `Err` on any arithmetic overflow or if the resulting region would
/// exceed [`super::worker::MPP_DSM_MAX_BYTES`].
pub fn estimate_mpp_dsm(
    plan_bytes_len: usize,
    total_participants: u32,
    num_meshes: u32,
    queue_bytes: usize,
) -> Result<usize, &'static str> {
    let mesh = MeshLayout::new(total_participants, queue_bytes);
    let dsm = compute_dsm_layout(&mesh, num_meshes, plan_bytes_len)?;
    Ok(dsm.total)
}

/// Leader's DSM-init entry point. Serializes `plan` into a
/// [`MppPlanBroadcast`], writes the bytes + header + shm_mq regions into the
/// DSM region at `coordinate`, and returns the leader's mesh wiring.
///
/// # Safety
/// - `coordinate` must point to the start of a DSM region of size >= the
///   estimate returned by [`estimate_mpp_dsm`] with the same parameters.
/// - Caller is the leader backend and holds `pcxt` / `seg`.
/// - Caller must not have written to the region already.
///
/// On `Err`, the DSM region may be partially initialized; the caller should
/// treat this as an abort condition and tear down the parallel context
/// rather than retrying.
pub unsafe fn init_mpp_dsm_leader(
    coordinate: *mut c_void,
    plan_broadcast_bytes: Vec<u8>,
    total_participants: u32,
    num_meshes: u32,
    queue_bytes: usize,
    seg: *mut pg_sys::dsm_segment,
) -> Result<LeaderMppContext, String> {
    // Caller must hand us the same bytes whose length they passed to
    // `estimate_mpp_dsm`; re-wrapping here would risk overrun.
    let mesh = MeshLayout::new(total_participants, queue_bytes);
    let dsm = compute_dsm_layout(&mesh, num_meshes, plan_broadcast_bytes.len())
        .map_err(|e| format!("mpp: compute_dsm_layout failed: {e}"))?;

    // Peek query_id from the broadcast — threading it separately would
    // duplicate the same truth and risk drift.
    let query_id = MppPlanBroadcast::deserialize(&plan_broadcast_bytes)
        .map_err(|e| format!("mpp: leader broadcast peek failed: {e}"))?
        .query_id;

    let base = coordinate as *mut u8;
    let meshes = unsafe {
        initialize_dsm_as_leader(base, &dsm, &mesh, &plan_broadcast_bytes, seg)
            .map_err(|e| format!("mpp: initialize_dsm_as_leader failed: {e}"))?
    };

    Ok(LeaderMppContext {
        meshes,
        participant_config: MppParticipantConfig {
            participant_index: 0,
            total_participants,
        },
        query_id,
    })
}

/// Bundle returned to the leader after [`init_mpp_dsm_leader`]. The caller
/// owns both the mesh wiring (for the leader's own shuffle participation)
/// and the config that the leader feeds into its DataFusion session.
pub struct LeaderMppContext {
    /// One [`LeaderMesh`] per shuffle mesh, in the order the shape requested.
    pub meshes: Vec<LeaderMesh>,
    pub participant_config: MppParticipantConfig,
    /// Per-query identifier the leader derived at plan time. Stamped (as
    /// the low 64 bits) on every boundary
    /// [`Stage`](datafusion_distributed::Stage) so workers and leader
    /// agree on the key that keys cross-participant mesh traffic.
    pub query_id: u64,
}

/// Worker's DSM-attach entry point. Reads the header, validates, attaches as
/// sender/receiver for this worker's participant index, and decodes the plan broadcast.
///
/// # Safety
/// - `coordinate` must be the DSM region pointer the leader initialized.
/// - `region_total` must match the DSM's attached size.
/// - `worker_number` is the PG-assigned `ParallelWorkerNumber`; the worker's
///   participant index is `worker_number + 1` (leader is participant 0).
/// - Caller is the worker backend and holds `pcxt` / `seg`.
pub unsafe fn attach_mpp_dsm_worker(
    coordinate: *mut c_void,
    region_total: u64,
    worker_number: i32,
    seg: *mut pg_sys::dsm_segment,
) -> Result<WorkerMppContext, String> {
    if worker_number < 0 {
        return Err("mpp: worker_number < 0".into());
    }
    let participant_index = (worker_number as u32)
        .checked_add(1)
        .ok_or_else(|| "mpp: worker_number + 1 overflowed u32".to_string())?;

    let base = coordinate as *mut u8;
    let attach = unsafe { attach_dsm_as_worker(base, region_total, participant_index, seg) }
        .map_err(|e| format!("mpp: attach_dsm_as_worker failed: {e}"))?;

    // The plan bytes live inside DSM. Copy them out so we don't hold a borrow
    // across subsequent FFI. `bincode::decode_from_slice` wants `&[u8]`.
    let plan_bytes = unsafe { attach.copy_plan_bytes() };
    let broadcast = MppPlanBroadcast::deserialize(&plan_bytes)
        .map_err(|e| format!("mpp: plan deserialize failed: {e}"))?;
    let participant_config = broadcast.participant_config(participant_index);
    let query_id = broadcast.query_id;

    Ok(WorkerMppContext {
        meshes: attach.meshes,
        participant_config,
        query_id,
    })
}

/// Bundle returned to a worker after [`attach_mpp_dsm_worker`]. The caller
/// constructs a session with `participant_config` and wires the returned
/// meshes into `ShuffleExec` (one mesh per shuffle).
pub struct WorkerMppContext {
    pub meshes: Vec<WorkerMesh>,
    pub participant_config: MppParticipantConfig,
    /// Per-query identifier read from the leader's broadcast.
    pub query_id: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres::customscan::mpp::session::MppSessionProfile;

    #[test]
    fn estimate_matches_compute_dsm_layout() {
        // Cross-check: estimate_mpp_dsm returns the same `total` as a direct
        // compute_dsm_layout call for any given config.
        for &(plan_len, n, num_meshes, q) in &[
            (0usize, 2u32, 1u32, 8 * 1024usize),
            (1024, 2, 1, 8 * 1024),
            (500_000, 4, 3, 64 * 1024),
        ] {
            let mesh = MeshLayout::new(n, q);
            let direct = compute_dsm_layout(&mesh, num_meshes, plan_len).unwrap();
            let via_estimate = estimate_mpp_dsm(plan_len, n, num_meshes, q).unwrap();
            assert_eq!(direct.total, via_estimate);
        }
    }

    #[test]
    fn estimate_rejects_overflow() {
        assert!(estimate_mpp_dsm(usize::MAX, 2, 1, 8 * 1024).is_err());
    }

    #[test]
    fn estimate_rejects_zero_participants() {
        assert!(estimate_mpp_dsm(100, 0, 1, 8 * 1024).is_err());
    }

    #[test]
    fn estimate_rejects_zero_meshes() {
        assert!(estimate_mpp_dsm(100, 2, 0, 8 * 1024).is_err());
    }

    #[test]
    fn estimate_scales_with_num_meshes() {
        let one = estimate_mpp_dsm(0, 4, 1, 64 * 1024).unwrap();
        let three = estimate_mpp_dsm(0, 4, 3, 64 * 1024).unwrap();
        // Header + padding is constant; per-mesh bytes is N*(N-1)*q.
        // Difference between 3 and 1 meshes is exactly 2 * per-mesh bytes.
        let per_mesh = 4 * 3 * 64 * 1024;
        assert_eq!(three - one, 2 * per_mesh);
    }

    #[test]
    fn init_does_not_rewrap_plan_bytes() {
        // Pins the contract: callers must pass already-serialized broadcast
        // bytes to `init_mpp_dsm_leader`, sized to match
        // `estimate_mpp_dsm(plan_broadcast_bytes.len(), …)`. Re-wrapping
        // inside the function would inflate the bytes past the estimate
        // and overrun the DSM region.
        let raw_plan: Vec<u8> = (0..1024u32).map(|i| (i & 0xff) as u8).collect();
        let bc = MppPlanBroadcast::new(raw_plan.clone(), 2, MppSessionProfile::Aggregate, 0);
        let broadcast_bytes = bc.serialize().unwrap();
        assert_ne!(
            broadcast_bytes.len(),
            raw_plan.len(),
            "MppPlanBroadcast wrapping adds framing; callers must size DSM against \
             the wrapped bytes, not raw plan bytes"
        );
        assert!(
            broadcast_bytes.len() > raw_plan.len(),
            "wrapped bytes must be strictly larger than raw (sanity)"
        );
    }
}
