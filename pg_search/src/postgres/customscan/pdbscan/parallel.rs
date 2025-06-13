// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use crate::api::Cardinality;
use crate::api::HashSet;
use crate::chunk_range;
use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::dsm::ParallelQueryCapable;
use crate::postgres::customscan::pdbscan::PdbScan;
use crate::postgres::ParallelScanState;
use pgrx::check_for_interrupts;
use pgrx::pg_sys::{self, shm_toc, ParallelContext, Size};
use std::ptr::NonNull;
use tantivy::index::SegmentId;

impl ParallelQueryCapable for PdbScan {
    fn estimate_dsm_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        pcxt: *mut ParallelContext,
    ) -> Size {
        if state.custom_state().search_reader.is_none() {
            PdbScan::init_search_reader(state);
        }

        let (segments, serialized_query, _) = state.custom_state().parallel_serialization_data();
        ParallelScanState::size_of(segments.len(), &serialized_query)
    }

    fn initialize_dsm_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        pcxt: *mut ParallelContext,
        coordinate: *mut c_void,
    ) {
        let (segments, serialized_query, requested_workers) =
            state.custom_state().parallel_serialization_data();

        unsafe {
            let pscan_state = coordinate.cast::<ParallelScanState>();
            assert!(!pscan_state.is_null(), "coordinate is null");
            (*pscan_state).init(requested_workers, segments, &serialized_query);
            state.custom_state_mut().parallel_state = Some(pscan_state);
            state.custom_state_mut().pcxt = NonNull::new(pcxt);
        }
    }

    fn reinitialize_dsm_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        pcxt: *mut ParallelContext,
        coordinate: *mut c_void,
    ) {
        let pscan_state = coordinate.cast::<ParallelScanState>();
        assert!(!pscan_state.is_null(), "coordinate is null");
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
            if pg_sys::parallel_leader_participation {
                // if there's leader participation, we'll spin and wait for the leader to tell us
                // exactly how many workers were launched
                loop {
                    {
                        let nlaunched = {
                            let _mutex = (*pscan_state).acquire_mutex();
                            (*pscan_state).nlaunched()
                        };

                        if nlaunched > 0 {
                            break;
                        }
                    }

                    // do this outside of holding the _mutex
                    check_for_interrupts!();
                    std::thread::yield_now();
                }
            } else {
                // there is no leader participation, so we have no idea how many workers were
                // actually launched.
                //
                // the best we can do is loop waiting for as many workers to start as we originally
                // requested.
                //
                // since it's possible for Postgres to launch fewer than we requested, we have to
                // give up after a (very short) period of time and just assume what we've counted as
                // started is all we're going to get
                //
                // NB: on my (@eebbrr) computer, 10ms is not long enough to pick up all the workers
                // but 15ms is.  We could make this a GUC but I think the better answer is to tell
                // users to not turn off `parallel_leader_participation`

                // first off, tell our shared parallel state that we've started
                // this is technically only necessary when leader participation is off
                {
                    let _mutex = (*pscan_state).acquire_mutex();
                    (*pscan_state).inc_nstarted();
                }

                // now we loop, waiting out or deadline to count the number of started workers
                // to use as the number that were launched
                let deadline = std::time::Instant::now() + std::time::Duration::from_millis(15);
                let nrequested = (*pscan_state).requested_workers();
                loop {
                    {
                        let _mutex = (*pscan_state).acquire_mutex();
                        if (*pscan_state).nlaunched() > 0 {
                            // it's already been set by another worker
                            break;
                        }

                        let nstarted = (*pscan_state).nstarted();
                        if nstarted == nrequested || std::time::Instant::now() >= deadline {
                            // all the workers we requested have started or we've run out of time waiting
                            (*pscan_state).set_nlaunched(nstarted);
                            break;
                        }
                    }

                    // do this outside of holding the _mutex
                    check_for_interrupts!();
                    std::thread::yield_now();
                }
            }

            match (*pscan_state)
                .query()
                .expect("should be able to serialize the query from the ParallelScanState")
            {
                Some(query) => state.custom_state_mut().set_base_search_query_input(query),
                None => panic!("no query in ParallelScanState"),
            }
        }
    }
}

///
/// Compute the number of workers that should be used for the given limit, segment_count, and sort
/// condition, or return 0 if workers cannot or should not be used.
///
pub fn compute_nworkers(limit: Option<Cardinality>, segment_count: usize, sorted: bool) -> usize {
    // we will try to parallelize based on the number of index segments
    // parallel workers available to a gather node are limited by max_parallel_workers_per_gather and max_parallel_workers
    let mut nworkers = unsafe {
        segment_count
            .min(pg_sys::max_parallel_workers_per_gather as usize)
            .min(pg_sys::max_parallel_workers as usize)
    };

    if let Some(limit) = limit {
        if !sorted && limit <= (segment_count * segment_count * segment_count) as Cardinality {
            // not worth it to do a parallel scan
            return 0;
        }

        // if the limit is less than some arbitrarily large value
        // use at most half the number of parallel workers as there are segments
        // this generally seems to perform better than directly using `max_parallel_workers_per_gather`
        if limit < 1_000_000.0 {
            nworkers = (segment_count / 2).min(nworkers);
        }
    }

    #[cfg(not(any(feature = "pg14", feature = "pg15")))]
    unsafe {
        if nworkers == 0 && pg_sys::debug_parallel_query != 0 {
            // force a parallel worker if the `debug_parallel_query` GUC is on
            nworkers = 1;
        }
    }

    nworkers
}

/// Checkout the next available segment, if there is one.
///
/// This is used by our non-"Top N" queries (but probably everyone should use
/// [`checkout_my_segment_block`] so that workers have a consistent view of their
/// segments set up front).
pub unsafe fn checkout_segment(pscan_state: *mut ParallelScanState) -> Option<SegmentId> {
    let _mutex = (*pscan_state).acquire_mutex();
    if (*pscan_state).remaining_segments() == 0 {
        return None;
    }
    let claimed_segment = (*pscan_state).decrement_remaining_segments();
    Some((*pscan_state).segment_id(claimed_segment))
}

/// Given the current [`pg_sys::ParallelWorkerNumber`] and the state in the `pscan_state` argument,
/// checkout the correct number of segments for the current worker.
///
/// Currently, this is used for our "Top N" queries
pub unsafe fn checkout_my_segment_block(
    nworkers: usize,
    pscan_state: *mut ParallelScanState,
) -> Vec<SegmentId> {
    let worker_number = unsafe {
        pg_sys::ParallelWorkerNumber
            + if pg_sys::parallel_leader_participation {
                1
            } else {
                0
            }
    } as usize;

    if worker_number >= nworkers {
        return vec![];
    }

    let (_, len) = chunk_range(unsafe { (*pscan_state).nsegments }, nworkers, worker_number);

    let mut segment_ids = Vec::with_capacity(len);
    let mutex = (*pscan_state).acquire_mutex();
    while (*pscan_state).remaining_segments() > 0 && segment_ids.len() < len {
        let idx = (*pscan_state).decrement_remaining_segments();
        let segment_id = (*pscan_state).segment_id(idx);
        segment_ids.push(segment_id);
    }

    segment_ids
}

pub unsafe fn list_segment_ids(pscan_state: *mut ParallelScanState) -> HashSet<SegmentId> {
    (*pscan_state).segments().keys().cloned().collect()
}
