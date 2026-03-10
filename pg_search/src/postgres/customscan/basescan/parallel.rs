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

use std::os::raw::c_void;

use crate::parallel_worker::{estimate_chunk, estimate_keys};
use crate::postgres::customscan::basescan::BaseScan;
use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::dsm::ParallelQueryCapable;
use crate::postgres::{ParallelScanState, PartitionEarlyTermState};

use pgrx::pg_sys::{self, shm_toc, ParallelContext, Size};

/// shm_toc key for the shared PartitionEarlyTermState.
/// Chosen to avoid collisions with PostgreSQL internal keys (0xE0...) and
/// our parallel_worker keys (1..3).
const EARLY_TERM_TOC_KEY: u64 = 0xB250_0000_0000_0001;

impl ParallelQueryCapable for BaseScan {
    fn estimate_dsm_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        pcxt: *mut ParallelContext,
    ) -> Size {
        if state.custom_state().search_reader.is_none() {
            BaseScan::init_search_reader(state);
        }

        let args = state.custom_state().parallel_scan_args();
        let size = ParallelScanState::size_of(
            args.segment_readers.len(),
            &args.query,
            args.with_aggregates,
        );

        // Each eligible partition child adds a TOC estimate for the shared
        // PartitionEarlyTermState. Over-estimation is acceptable per the PG API;
        // only one child will actually allocate during initialize_dsm.
        if state.custom_state().partition_early_term.is_some() {
            unsafe {
                estimate_keys(pcxt, 1);
                estimate_chunk(pcxt, PartitionEarlyTermState::size_of());
            }
        }

        size
    }

    fn initialize_dsm_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        pcxt: *mut ParallelContext,
        coordinate: *mut c_void,
    ) {
        let args = state.custom_state().parallel_scan_args();
        let nsegments = args.segment_readers.len();

        unsafe {
            let pscan_state = coordinate.cast::<ParallelScanState>();
            assert!(!pscan_state.is_null(), "coordinate is null");
            (*pscan_state).create_and_populate(args);
            state.custom_state_mut().parallel_state = Some(pscan_state);

            if let Some(et) = state.custom_state().partition_early_term.as_ref() {
                let toc = (*pcxt).toc;
                let sort_direction = et.sort_direction;

                // Check if another partition child already allocated the shared state.
                let existing = pg_sys::shm_toc_lookup(toc, EARLY_TERM_TOC_KEY, true);

                let shared_state = if existing.is_null() {
                    // First partition child: allocate and initialize.
                    let et_ptr = pg_sys::shm_toc_allocate(toc, PartitionEarlyTermState::size_of())
                        as *mut PartitionEarlyTermState;

                    let n_partitions = state.custom_state().heaprel().get_n_partitions();
                    let limit = state.custom_state().limit().unwrap_or(0) as u32;
                    (*et_ptr).init(limit, n_partitions);

                    pg_sys::shm_toc_insert(toc, EARLY_TERM_TOC_KEY, et_ptr as *mut c_void);
                    et_ptr
                } else {
                    // Subsequent partition child: reuse existing.
                    existing as *mut PartitionEarlyTermState
                };

                let sort_rank = state
                    .custom_state()
                    .heaprel()
                    .compute_partition_rank(sort_direction);

                let et = state
                    .custom_state_mut()
                    .partition_early_term
                    .as_mut()
                    .unwrap();
                et.shared_state = Some(shared_state);
                et.sort_rank = sort_rank;

                // Register this partition's segment count for gating.
                if let Some(rank) = sort_rank {
                    (*shared_state).register_segments(rank, nsegments as u32);
                }
            }
        }
    }

    fn reinitialize_dsm_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        _pcxt: *mut ParallelContext,
        coordinate: *mut c_void,
    ) {
        let pscan_state = coordinate.cast::<ParallelScanState>();
        assert!(!pscan_state.is_null(), "coordinate is null");
        unsafe {
            let nsegments = (*pscan_state).segment_count();
            (*pscan_state).reset();

            if let Some(et) = state.custom_state().partition_early_term.as_ref() {
                if let (Some(et_state), Some(rank)) = (et.shared_state, et.sort_rank) {
                    // Reset only this rank's counters (not the full state) to avoid
                    // the race where one child's reset clears another child's
                    // already-registered segment count.
                    (*et_state).reset_rank(rank);
                    (*et_state).register_segments(rank, nsegments as u32);
                }
            }
        }
    }

    fn initialize_worker_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        toc: *mut shm_toc,
        coordinate: *mut c_void,
    ) {
        let pscan_state = coordinate.cast::<ParallelScanState>();
        assert!(!pscan_state.is_null(), "coordinate is null");

        state.custom_state_mut().parallel_state = Some(pscan_state);
        unsafe {
            match (*pscan_state)
                .query()
                .expect("should be able to deserialize the query from the ParallelScanState")
            {
                Some(query) => state.custom_state_mut().set_base_search_query_input(query),
                None => panic!("no query in ParallelScanState"),
            }

            // Look up the shared early termination state via TOC key.
            let et_ptr = pg_sys::shm_toc_lookup(toc, EARLY_TERM_TOC_KEY, true);
            if !et_ptr.is_null() {
                if let Some(sort_direction) = state
                    .custom_state()
                    .partition_early_term
                    .as_ref()
                    .map(|et| et.sort_direction)
                {
                    let sort_rank = state
                        .custom_state()
                        .heaprel()
                        .compute_partition_rank(sort_direction);

                    let et = state
                        .custom_state_mut()
                        .partition_early_term
                        .as_mut()
                        .unwrap();
                    et.shared_state = Some(et_ptr as *mut PartitionEarlyTermState);
                    et.sort_rank = sort_rank;
                }
            }
        }
    }
}
