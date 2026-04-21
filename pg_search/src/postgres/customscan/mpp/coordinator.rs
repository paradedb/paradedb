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

//! One-call façade that AggregateScan / JoinScan customscan hooks call into
//! for MPP DSM setup.
//!
//! Customscan code typically has three hook points where MPP needs to
//! participate:
//!
//! - `estimate_dsm_custom_scan` — leader, returns the DSM region size. Call
//!   [`estimate_mpp_dsm`] with the serialized-plan length and mesh config.
//! - `initialize_dsm_custom_scan` — leader, initializes DSM with leader seat
//!   wiring. Call [`init_mpp_dsm_leader`] and stash the returned
//!   [`LeaderMesh`](super::worker::LeaderMesh) in execution state.
//! - `initialize_worker_custom_scan` — worker, attaches to DSM with this
//!   worker's seat wiring. Call [`attach_mpp_dsm_worker`] and stash the
//!   returned [`WorkerMesh`](super::worker::WorkerMesh) + decoded plan.
//!
//! Each function is a thin wrapper over [`super::worker`] primitives plus a
//! plan-broadcast encode/decode. Keeping them here isolates the hook-facing
//! glue from the lower-level DSM math so future refactors on either side
//! stay localized.

#![allow(dead_code)] // First caller lands when AggregateScan MPP path is wired up.

use pgrx::pg_sys;

use crate::postgres::customscan::mpp::mesh::MeshLayout;
use crate::postgres::customscan::mpp::session::{MppPlanBroadcast, MppSessionProfile};
use crate::postgres::customscan::mpp::worker::{
    attach_dsm_as_worker, compute_dsm_layout, initialize_dsm_as_leader, DsmLayout, LeaderMesh,
};

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
    coordinate: *mut std::ffi::c_void,
    plan_broadcast_bytes: Vec<u8>,
    total_participants: u32,
    num_meshes: u32,
    session_profile: MppSessionProfile,
    queue_bytes: usize,
    seg: *mut pg_sys::dsm_segment,
) -> Result<LeaderMppContext, String> {
    // `plan_broadcast_bytes` is the already-serialized `MppPlanBroadcast`
    // payload — the caller is responsible for matching these exact bytes
    // against the `estimate_mpp_dsm` size the PG planner used to allocate
    // the DSM region. Re-wrapping here would produce a different length
    // than estimate and overrun the DSM allocation.
    let mesh = MeshLayout::new(total_participants, queue_bytes);
    let dsm = compute_dsm_layout(&mesh, num_meshes, plan_broadcast_bytes.len())
        .map_err(|e| format!("mpp: compute_dsm_layout failed: {e}"))?;

    let base = coordinate as *mut u8;
    let meshes = unsafe {
        initialize_dsm_as_leader(base, &dsm, &mesh, &plan_broadcast_bytes, seg)
            .map_err(|e| format!("mpp: initialize_dsm_as_leader failed: {e}"))?
    };

    Ok(LeaderMppContext {
        meshes,
        layout: dsm,
        participant_config: super::MppParticipantConfig {
            participant_index: 0,
            total_participants,
        },
        session_profile,
    })
}

/// Bundle returned to the leader after [`init_mpp_dsm_leader`]. The caller
/// owns both the mesh wiring (for the leader's own shuffle participation)
/// and the config that the leader feeds into its DataFusion session.
pub struct LeaderMppContext {
    /// One [`LeaderMesh`] per shuffle mesh, in the order the shape requested.
    pub meshes: Vec<LeaderMesh>,
    pub layout: DsmLayout,
    pub participant_config: super::MppParticipantConfig,
    pub session_profile: MppSessionProfile,
}

/// Worker's DSM-attach entry point. Reads the header, validates, attaches as
/// sender/receiver for this worker's seat, and decodes the plan broadcast.
///
/// # Safety
/// - `coordinate` must be the DSM region pointer the leader initialized.
/// - `region_total` must match the DSM's attached size.
/// - `worker_number` is the PG-assigned `ParallelWorkerNumber`; the worker's
///   seat is `worker_number + 1` (leader is seat 0).
/// - Caller is the worker backend and holds `pcxt` / `seg`.
pub unsafe fn attach_mpp_dsm_worker(
    coordinate: *mut std::ffi::c_void,
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

    Ok(WorkerMppContext {
        meshes: attach.meshes,
        plan: broadcast,
        participant_config,
    })
}

/// Bundle returned to a worker after [`attach_mpp_dsm_worker`]. The caller
/// rebuilds its DataFusion logical plan from `plan.logical_plan`, constructs
/// a session with `participant_config` + `plan.session_profile`, and wires
/// the returned meshes into `ShuffleExec` (one mesh per shuffle).
pub struct WorkerMppContext {
    pub meshes: Vec<crate::postgres::customscan::mpp::worker::WorkerMesh>,
    pub plan: MppPlanBroadcast,
    pub participant_config: super::MppParticipantConfig,
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // Regression: the caller must pass **pre-serialized** broadcast
        // bytes to `init_mpp_dsm_leader`, whose length matches the
        // `estimate_mpp_dsm(plan_broadcast_bytes.len(), …)` used to size
        // the DSM region. Earlier, `init_mpp_dsm_leader` re-wrapped the
        // raw logical plan in `MppPlanBroadcast` + bincode-serialized it
        // inside the body, producing bytes ~6 larger than estimate — the
        // tail of the last mesh queue then overran the DSM region and
        // corrupted adjacent `shm_toc` allocations (crash at
        // `dsa_release_in_place` teardown).
        //
        // This test pins the invariant *statically* by asserting, for a
        // representative raw plan, that `MppPlanBroadcast::serialize().len()
        // != raw_plan.len()`. If some future refactor ever undoes the
        // pre-serialize step, every test that actually exercises the DSM
        // path will crash; this one additionally documents the
        // why-it-matters so the regression is caught at code-review time.
        let raw_plan: Vec<u8> = (0..1024u32).map(|i| (i & 0xff) as u8).collect();
        let bc = MppPlanBroadcast::new(raw_plan.clone(), 2, MppSessionProfile::Aggregate);
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
