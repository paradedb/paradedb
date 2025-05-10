use crate::api::Cardinality;
use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::dsm::ParallelQueryCapable;
use crate::postgres::customscan::pdbscan::PdbScan;
use crate::postgres::ParallelScanState;

use pgrx::pg_sys::{self, shm_toc, ParallelContext, Size};
use tantivy::index::SegmentId;

use std::collections::HashSet;
use std::os::raw::c_void;
use std::time::{Duration, Instant};

impl ParallelQueryCapable for PdbScan {
    fn estimate_dsm_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        pcxt: *mut ParallelContext,
    ) -> Size {
        if state.custom_state().search_reader.is_none() {
            PdbScan::init_search_reader(state);
        }

        let serialized_query = serde_json::to_vec(&state.custom_state().search_query_input)
            .expect("should be able to serialize query");
        state.custom_state_mut().serialized_query = serialized_query;

        let segment_count = state
            .custom_state()
            .search_reader
            .as_ref()
            .expect("search reader must be initialized to estimate DSM size")
            .segment_readers()
            .len();

        ParallelScanState::size_of(segment_count, &state.custom_state_mut().serialized_query)
    }

    fn initialize_dsm_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        pcxt: *mut ParallelContext,
        coordinate: *mut c_void,
    ) {
        let pscan_state = coordinate.cast::<ParallelScanState>();
        assert!(!pscan_state.is_null(), "coordinate is null");

        unsafe {
            let segments = state
                .custom_state()
                .search_reader
                .as_ref()
                .expect("search_reader must be initialized to initialize DSM")
                .segment_readers();
            (*pscan_state).init(segments, &state.custom_state().serialized_query);

            state.custom_state_mut().parallel_state = Some(pscan_state);
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
            match (*pscan_state)
                .query()
                .expect("should be able to serialize the query from the ParallelScanState")
            {
                Some(query) => state.custom_state_mut().search_query_input = query,
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

pub unsafe fn checkout_segment(pscan_state: *mut ParallelScanState) -> Option<SegmentId> {
    #[cfg(not(any(feature = "pg14", feature = "pg15")))]
    let deadline = Instant::now() + Duration::from_millis(50);

    loop {
        let mutex = (*pscan_state).acquire_mutex();
        let remaining_segments = (*pscan_state).remaining_segments();
        if remaining_segments == 0 {
            break None;
        }

        // If debug_parallel_query is enabled and we're the leader, then do not take the first
        // segment (unless a deadline has passed, since in some cases we may not have any workers:
        // e.g. UNIONS under a Gather node, etc).
        //
        // This significantly improves the reproducibility of parallel worker issues with small
        // datasets, since it means that unlike in the non-parallel case, the leader will be
        // unlikely to emit all of the segments before the workers have had a chance to start up.
        #[cfg(not(any(feature = "pg14", feature = "pg15")))]
        if pg_sys::debug_parallel_query != 0
            && pg_sys::ParallelWorkerNumber == -1
            && remaining_segments == (*pscan_state).nsegments()
            && Instant::now() < deadline
        {
            continue;
        }

        let claimed_segment = (*pscan_state).decrement_remaining_segments();
        break Some((*pscan_state).segment_id(claimed_segment));
    }
}

pub unsafe fn list_segment_ids(pscan_state: *mut ParallelScanState) -> HashSet<SegmentId> {
    (*pscan_state).segments().keys().cloned().collect()
}
