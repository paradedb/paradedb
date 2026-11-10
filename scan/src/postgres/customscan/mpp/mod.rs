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
    block_on_next, catch_worker_panic, check_for_cancel, interrupted, try_run_on_main_thread,
};

pub mod glue {
    use pgrx::pg_sys;

    use crate::postgres::customscan::mpp::dsm_stream::{
        MIN_SPSC_RING_CAPACITY, dsm_region_bytes, total_spsc_rings,
    };

    pub const MIN_TOTAL_WORKER_COUNT: u32 = 3;

    pub fn mpp_is_active() -> bool {
        producer_worker_cap() >= MIN_TOTAL_WORKER_COUNT - 1
    }

    pub fn query_allows_parallel_mode(root: &pg_sys::PlannerInfo) -> bool {
        !root.glob.is_null() && unsafe { (*root.glob).parallelModeOK }
    }

    pub fn spsc_ring_capacity(n_procs: u32) -> Result<usize, String> {
        let rings = total_spsc_rings(n_procs)?;
        let raw = (crate::gucs::mpp_queue_size() / rings).max(MIN_SPSC_RING_CAPACITY);
        Ok(1usize << raw.ilog2())
    }

    pub fn estimate_dsm_size(n_procs: u32) -> Result<usize, String> {
        let min_procs = MIN_TOTAL_WORKER_COUNT.min(n_procs);
        let mut max_bytes = 0;
        for k in min_procs..=n_procs {
            max_bytes = max_bytes.max(dsm_region_bytes(k, spsc_ring_capacity(k)?)?);
        }
        Ok(max_bytes)
    }

    pub fn producer_worker_cap() -> u32 {
        let max_per_gather = unsafe { pg_sys::max_parallel_workers_per_gather };
        let max_workers = unsafe { pg_sys::max_parallel_workers };
        max_per_gather.min(max_workers).max(0) as u32
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
