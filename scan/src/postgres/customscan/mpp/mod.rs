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

pub use index_read::postgres::customscan::mpp::*;

pub mod dsm_stream;
pub mod interrupt;

pub use interrupt::{
    block_on_next, catch_worker_panic, check_for_cancel, interrupted, is_parallel_worker,
    try_run_on_main_thread,
};

pub mod glue {
    use pgrx::pg_sys;

    use crate::postgres::customscan::mpp::dsm_stream::{
        MIN_SPSC_RING_CAPACITY, dsm_region_bytes, total_spsc_rings,
    };

    /// Whether the leader backend participates as an in-process DataFusion `Worker` (`worker-0`)
    /// alongside background parallel workers (`worker-1..=W`), controlled by PostgreSQL's
    /// `parallel_leader_participation` GUC.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum LeaderParticipation {
        Participating,
        CoordinatorOnly,
    }

    impl LeaderParticipation {
        pub fn current() -> Self {
            if unsafe { pg_sys::parallel_leader_participation } {
                Self::Participating
            } else {
                Self::CoordinatorOnly
            }
        }

        pub const fn is_participating(self) -> bool {
            matches!(self, Self::Participating)
        }

        pub const fn leader_share(self) -> u32 {
            match self {
                Self::Participating => 1,
                Self::CoordinatorOnly => 0,
            }
        }

        /// Minimum background workers required so that `bg_workers.participants(self)` reaches
        /// [`ParticipantCount::MIN_MPP_PARTICIPANTS`] (`2`).
        pub const fn min_mpp_bg_workers(self) -> BgWorkerCount {
            match self {
                Self::Participating => BgWorkerCount(1),
                Self::CoordinatorOnly => BgWorkerCount(2),
            }
        }
    }

    /// Number of background parallel workers (excluding the leader backend).
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
    pub struct BgWorkerCount(pub u32);

    impl BgWorkerCount {
        pub const ZERO: Self = Self(0);

        pub const fn new(count: u32) -> Self {
            Self(count)
        }

        pub const fn get(self) -> u32 {
            self.0
        }

        /// Total processes in the DSM SPSC mesh: leader (`proc 0`) plus `self.0` background
        /// workers (`1..=self.0`).
        pub const fn mesh_procs(self) -> u32 {
            self.0.saturating_add(1)
        }

        /// Number of task-executing DataFusion `Worker` participants when combined with `leader`.
        /// Zero background workers always yields zero MPP participants.
        pub const fn participants(self, leader: LeaderParticipation) -> ParticipantCount {
            if self.0 == 0 {
                ParticipantCount(0)
            } else {
                ParticipantCount(self.0.saturating_add(leader.leader_share()))
            }
        }
    }

    /// Background-worker ceiling paired with the leader's participation mode for an MPP scan.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct MppBudget {
        pub bg_workers: BgWorkerCount,
        pub leader: LeaderParticipation,
    }

    impl MppBudget {
        pub const fn new(bg_workers: BgWorkerCount, leader: LeaderParticipation) -> Self {
            Self { bg_workers, leader }
        }

        pub const fn participants(self) -> ParticipantCount {
            self.bg_workers.participants(self.leader)
        }

        /// Background workers to request for a distributed plan whose widest producer stage has
        /// `max_tasks` tasks, bounded by `self.bg_workers` and `self.leader`. Returns `None` when
        /// either the stage task count or `self.bg_workers` cannot form an active
        /// `>= 2`-participant MPP mesh.
        pub fn for_stage_tasks(self, max_tasks: usize) -> Option<BgWorkerCount> {
            let min_bg = self.leader.min_mpp_bg_workers();
            if max_tasks < ParticipantCount::MIN_MPP_PARTICIPANTS as usize
                || self.bg_workers < min_bg
            {
                return None;
            }
            let tasks_u32 = u32::try_from(max_tasks).unwrap_or(u32::MAX);
            Some(BgWorkerCount(
                tasks_u32
                    .saturating_sub(self.leader.leader_share())
                    .max(min_bg.get())
                    .min(self.bg_workers.get()),
            ))
        }
    }

    /// Number of task-executing DataFusion `Worker` participants in an MPP query.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
    pub struct ParticipantCount(pub u32);

    impl ParticipantCount {
        pub const MIN_MPP_PARTICIPANTS: u32 = 2;

        pub const fn new(count: u32) -> Self {
            Self(count)
        }

        pub const fn get(self) -> u32 {
            self.0
        }

        pub const fn as_usize(self) -> usize {
            self.0 as usize
        }

        pub const fn is_mpp_active(self) -> bool {
            self.0 >= Self::MIN_MPP_PARTICIPANTS
        }
    }

    /// Global ceiling on background parallel workers per Gather/MPP node from PostgreSQL GUCs
    /// (`max_parallel_workers_per_gather`, `max_parallel_workers`, `max_worker_processes`).
    pub fn bg_worker_cap() -> BgWorkerCount {
        unsafe {
            let cap = pg_sys::max_parallel_workers_per_gather
                .min(pg_sys::max_parallel_workers)
                .min(pg_sys::max_worker_processes)
                .max(0) as u32;
            BgWorkerCount(cap)
        }
    }

    pub fn debug_parallel_query_is_active() -> bool {
        #[cfg(not(feature = "pg15"))]
        {
            unsafe {
                pg_sys::debug_parallel_query != pg_sys::DebugParallelMode::DEBUG_PARALLEL_OFF as i32
            }
        }
        #[cfg(feature = "pg15")]
        {
            false
        }
    }

    pub fn debug_parallel_query_is_regress() -> bool {
        #[cfg(not(feature = "pg15"))]
        {
            unsafe {
                pg_sys::debug_parallel_query
                    == pg_sys::DebugParallelMode::DEBUG_PARALLEL_REGRESS as i32
            }
        }
        #[cfg(feature = "pg15")]
        {
            false
        }
    }

    pub fn query_allows_parallel_mode(root: &pg_sys::PlannerInfo) -> bool {
        !root.glob.is_null() && unsafe { (*root.glob).parallelModeOK }
    }

    #[pgrx::pg_guard]
    unsafe extern "C-unwind" fn non_parallel_safe_func_checker(
        func_id: pg_sys::Oid,
        _context: *mut std::ffi::c_void,
    ) -> bool {
        unsafe { pg_sys::func_parallel(func_id) != pg_sys::PROPARALLEL_SAFE as std::ffi::c_char }
    }

    #[pgrx::pg_guard]
    unsafe extern "C-unwind" fn expr_has_mpp_parallel_hazard_walker(
        node: *mut pg_sys::Node,
        context: *mut std::ffi::c_void,
    ) -> bool {
        if node.is_null() {
            return false;
        }
        unsafe {
            if pg_sys::check_functions_in_node(
                node,
                Some(non_parallel_safe_func_checker),
                std::ptr::null_mut(),
            ) {
                return true;
            }
            match (*node).type_ {
                pg_sys::NodeTag::T_CoerceToDomain
                | pg_sys::NodeTag::T_NextValueExpr
                | pg_sys::NodeTag::T_WindowFunc
                | pg_sys::NodeTag::T_SubLink => true,
                pg_sys::NodeTag::T_RestrictInfo => {
                    let rinfo = node.cast::<pg_sys::RestrictInfo>();
                    expr_has_mpp_parallel_hazard_walker((*rinfo).clause.cast(), context)
                }
                // `T_Param` (both `PARAM_EXTERN` and correlated `PARAM_EXEC`) is solved on the
                // leader before worker dispatch on first pull and on every `rescan_custom_scan`,
                // and `T_SubPlan` only reaches `PostgresExpression` (also solved on the leader;
                // `expression_tree_walker` still inspects its `testexpr` and `args`).
                _ => pg_sys::expression_tree_walker(
                    node,
                    Some(expr_has_mpp_parallel_hazard_walker),
                    context,
                ),
            }
        }
    }

    /// Verify that `root` permits parallel mode, every absorbed source relation `heaprelid` is
    /// non-temporary (`RELPERSISTENCE_TEMP` tables cannot be accessed by parallel workers), and
    /// every custom-scan expression `expr` is free of MPP parallel hazards (non-`PROPARALLEL_SAFE`
    /// functions, `CoerceToDomain`, `NextValueExpr`, `WindowFunc`, or unlowered `SubLink`s).
    ///
    /// Unlike PostgreSQL's `is_parallel_safe`, correlated `PARAM_EXEC` parameters (such as a
    /// `LATERAL` outer reference) remain MPP-eligible because the leader resolves all `T_Param`s
    /// into constants / concrete `SearchQueryInput`s on every scan and `rescan_custom_scan` before
    /// encoding `PgSearchScanPlan` for workers.
    ///
    /// # Safety
    /// `root` and every non-null `expr` must be valid PostgreSQL planner nodes for the current
    /// query, and every `heaprelid` must be a valid relation OID in the current transaction.
    pub unsafe fn sources_and_exprs_allow_parallel(
        root: *mut pg_sys::PlannerInfo,
        source_heaprelids: impl IntoIterator<Item = pg_sys::Oid>,
        exprs: impl IntoIterator<Item = *mut pg_sys::Node>,
    ) -> bool {
        if root.is_null() || !query_allows_parallel_mode(unsafe { &*root }) {
            return false;
        }
        for heaprelid in source_heaprelids {
            if heaprelid == pg_sys::InvalidOid
                || unsafe {
                    pg_sys::get_rel_persistence(heaprelid)
                        == pg_sys::RELPERSISTENCE_TEMP as std::ffi::c_char
                }
            {
                return false;
            }
        }
        for expr in exprs {
            if !expr.is_null()
                && unsafe { expr_has_mpp_parallel_hazard_walker(expr, std::ptr::null_mut()) }
            {
                return false;
            }
        }
        true
    }

    pub fn spsc_ring_capacity(bg_workers: BgWorkerCount) -> Result<usize, String> {
        let rings = total_spsc_rings(bg_workers.mesh_procs())?;
        let raw = (crate::gucs::mpp_queue_size() / rings).max(MIN_SPSC_RING_CAPACITY);
        Ok(1usize << raw.ilog2())
    }

    pub fn estimate_dsm_size(max_bg_workers: BgWorkerCount) -> Result<usize, String> {
        if max_bg_workers.get() == 0 {
            return Err("mpp: bg_workers must be >= 1, got 0".to_string());
        }
        let mut max_bytes = 0;
        for k in 1..=max_bg_workers.get() {
            let bg = BgWorkerCount(k);
            max_bytes = max_bytes.max(dsm_region_bytes(bg.mesh_procs(), spsc_ring_capacity(bg)?)?);
        }
        Ok(max_bytes)
    }

    #[derive(Default, Clone, Copy, Debug)]
    pub struct MppLaunchTiming {
        pub prepare_us: u64,
        pub plan_us: u64,
        pub attach_us: u64,
        pub leader_setup_us: u64,
        pub exec_us: u64,
        pub first_frame_us: u64,
        pub workers: u32,
    }

    impl MppLaunchTiming {
        pub fn explain_text(&self) -> String {
            format!(
                "workers={} prepare={}us plan={}us attach={}us \
                 leader_setup={}us exec={}us first_frame={}us",
                self.workers,
                self.prepare_us,
                self.plan_us,
                self.attach_us,
                self.leader_setup_us,
                self.exec_us,
                self.first_frame_us,
            )
        }
    }
}
