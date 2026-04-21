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

//! Glue between PostgreSQL customscan DSM hooks and the MPP coordinator.
//!
//! AggregateScan / JoinScan implement `ParallelQueryCapable` with trait
//! methods that receive raw pg_sys pointers. This module packages those raw
//! calls into typed helpers that each customscan can call with minimal
//! boilerplate:
//!
//! - [`MppExecutionState`] is the per-query MPP state the customscan's
//!   execution state stores in a `Option<MppExecutionState>` field. It owns
//!   the coordinator results (leader or worker mesh), the drain handle
//!   (created once the worker has wired the receivers), and the decoded
//!   plan broadcast.
//! - [`leader_estimate_dsm`] computes the DSM size for the customscan's
//!   `estimate_dsm_custom_scan` hook given the size of the serialized
//!   logical plan and the MPP GUC config.
//! - [`leader_init_dsm`] handles the full leader-side setup in one call.
//! - [`worker_init_dsm`] mirrors [`leader_init_dsm`] on the worker side:
//!   attach, decode plan, return everything the worker needs to rebuild its
//!   DataFusion session.
//!
//! Each helper is a thin wrapper over [`crate::postgres::customscan::mpp::coordinator`]
//! plus a couple of GUC reads — the intent is that AggregateScan's
//! `ParallelQueryCapable` impl is five lines of delegation and never touches
//! raw shm_mq FFI directly.

#![allow(dead_code)] // First caller is AggregateScan Phase 4b wiring.

use pgrx::pg_sys;

use crate::postgres::customscan::mpp::coordinator::{
    attach_mpp_dsm_worker, estimate_mpp_dsm, init_mpp_dsm_leader, LeaderMppContext,
    WorkerMppContext,
};
use crate::postgres::customscan::mpp::session::MppSessionProfile;

/// Per-query MPP state a customscan's execution state stores. Distinct
/// variants for leader vs worker because the two have different wiring
/// shapes — the leader pre-creates the mesh; the worker attaches to it —
/// and forcing a common bundle would either waste fields or require a
/// second level of Option.
pub enum MppExecutionState {
    Leader(LeaderMppContext),
    Worker(WorkerMppContext),
}

impl MppExecutionState {
    pub fn participant_config(&self) -> &crate::postgres::customscan::mpp::MppParticipantConfig {
        match self {
            MppExecutionState::Leader(l) => &l.participant_config,
            MppExecutionState::Worker(w) => &w.participant_config,
        }
    }

    pub fn session_profile(&self) -> MppSessionProfile {
        match self {
            MppExecutionState::Leader(l) => l.session_profile,
            MppExecutionState::Worker(w) => w.plan.session_profile,
        }
    }

    pub fn is_leader(&self) -> bool {
        matches!(self, MppExecutionState::Leader(_))
    }
}

/// True when the `paradedb.enable_mpp` GUC is on AND the worker count is at
/// least 2. The customscan checks this before announcing parallel-safe
/// paths; if false, the non-MPP serial path is used.
pub fn mpp_is_active() -> bool {
    crate::gucs::enable_mpp() && crate::gucs::mpp_worker_count() >= 2
}

/// Read the configured MPP worker count from GUC. Clamps below at 2 because
/// the MPP code paths assume at least 2 participants; callers that see
/// `< 2` here should fall back to the non-MPP path via [`mpp_is_active`].
pub fn mpp_worker_count() -> u32 {
    // `max(2)` rather than `clamp(2, ...)`: GUC is already `i32`-bounded,
    // and naive `clamp(2, u32::MAX as i32)` underflows the upper bound to -1.
    crate::gucs::mpp_worker_count().max(2) as u32
}

/// Per-edge shm_mq queue capacity in bytes. Fixed at 8 MiB — matches the
/// reference attempt's sizing and comfortably holds typical Arrow IPC
/// partial-aggregate batches. Eventually this should come from a GUC so
/// ops can tune without recompiling, but 8 MiB is sufficient for the
/// milestone-1 benchmark target.
// 64 MiB per directed edge, per mesh. The 25M-row benchmark's
// GROUP BY variant ships ~100 MiB of Partial rows through the postagg
// mesh per participant; at 8 MiB per edge the queue fills repeatedly
// and the cooperative-drain spin consumes most of the budget. 64 MiB
// lets each Partial burst fit in a single queue without backpressure,
// at the cost of a larger DSM allocation (N×(N-1)×num_meshes×64 MiB —
// 768 MiB at N=2 × 3 meshes, ~2.3 GiB at N=4 × 3 meshes).
pub const DEFAULT_MPP_QUEUE_BYTES: usize = 64 * 1024 * 1024;

/// Customscan `estimate_dsm_custom_scan` implementation for an MPP-enabled
/// plan. Given the serialized logical plan length, returns the total DSM
/// bytes the coordinator will need.
///
/// Call from the customscan's `ParallelQueryCapable::estimate_dsm_custom_scan`
/// when [`mpp_is_active`] is true.
pub fn leader_estimate_dsm(plan_bytes_len: usize, num_meshes: u32) -> Result<pg_sys::Size, String> {
    let n = mpp_worker_count();
    estimate_mpp_dsm(plan_bytes_len, n, num_meshes, DEFAULT_MPP_QUEUE_BYTES)
        .map(|sz| sz as pg_sys::Size)
        .map_err(|e| format!("mpp: estimate DSM failed: {e}"))
}

/// Customscan `initialize_dsm_custom_scan` implementation for an MPP leader.
/// Takes the DSM `coordinate` pointer the PG hook received, the pre-serialized
/// `MppPlanBroadcast` bytes (same bytes whose length was reported by
/// [`leader_estimate_dsm`]), the session profile, and the DSM segment pointer.
/// Returns the [`MppExecutionState::Leader`] the customscan should stash in
/// its execution state.
///
/// # Safety
/// - `coordinate` must be the DSM region pointer PG supplied to
///   `initialize_dsm_custom_scan`. It must remain valid for the query's
///   lifetime.
/// - `seg` must be valid. On the leader it comes from `pcxt->seg`.
/// - `plan_broadcast_bytes.len()` must equal the value passed to
///   [`leader_estimate_dsm`]; otherwise the DSM write overruns the region PG
///   allocated and corrupts adjacent `shm_toc` allocations (including the
///   DSA control regions used by parallel tuple queues).
pub unsafe fn leader_init_dsm(
    coordinate: *mut std::ffi::c_void,
    plan_broadcast_bytes: Vec<u8>,
    num_meshes: u32,
    session_profile: MppSessionProfile,
    seg: *mut pg_sys::dsm_segment,
) -> Result<MppExecutionState, String> {
    if coordinate.is_null() {
        return Err("mpp: leader_init_dsm given null coordinate".into());
    }

    let n = mpp_worker_count();
    let leader = unsafe {
        init_mpp_dsm_leader(
            coordinate,
            plan_broadcast_bytes,
            n,
            num_meshes,
            session_profile,
            DEFAULT_MPP_QUEUE_BYTES,
            seg,
        )?
    };
    Ok(MppExecutionState::Leader(leader))
}

/// Customscan `initialize_worker_custom_scan` implementation. Called on
/// worker backends with the `coordinate` pointer and the worker's
/// `ParallelWorkerNumber`.
///
/// # Safety
/// - `coordinate` must be the DSM region pointer PG supplied.
/// - `seg` may be NULL — `initialize_worker_custom_scan` does not surface
///   the DSM segment pointer, so callers typically pass NULL. `shm_mq_attach`
///   handles NULL seg by skipping its `on_dsm_detach` callback; cleanup
///   then relies on process exit. Safe for parallel-worker lifetimes where
///   the process dies with the query.
/// - `region_total` must be the size of the attached DSM region. Callers
///   without an independent source can read it from the header itself
///   (trading the independence of the validation check for the ability
///   to initialize without a seg pointer).
pub unsafe fn worker_init_dsm(
    coordinate: *mut std::ffi::c_void,
    region_total: u64,
    worker_number: i32,
    seg: *mut pg_sys::dsm_segment,
) -> Result<MppExecutionState, String> {
    if coordinate.is_null() {
        return Err("mpp: worker_init_dsm given null coordinate".into());
    }
    let worker = unsafe { attach_mpp_dsm_worker(coordinate, region_total, worker_number, seg)? };
    Ok(MppExecutionState::Worker(worker))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_queue_bytes_is_maxalign_safe() {
        // 8 MiB must be a multiple of MAXIMUM_ALIGNOF so
        // `aligned_queue_bytes(DEFAULT_MPP_QUEUE_BYTES) == DEFAULT_MPP_QUEUE_BYTES`.
        let aligned =
            crate::postgres::customscan::mpp::mesh::aligned_queue_bytes(DEFAULT_MPP_QUEUE_BYTES);
        assert_eq!(aligned, DEFAULT_MPP_QUEUE_BYTES);
    }

    #[test]
    fn mpp_worker_count_clamps_below_two() {
        // GUC default is 2; the function shouldn't drop below 2 even if
        // the GUC were set to 0 or 1 at runtime (which `mpp_is_active`
        // filters before this is called, but defense-in-depth).
        let n = mpp_worker_count();
        assert!(n >= 2, "mpp_worker_count must clamp below 2; got {n}");
    }
}
