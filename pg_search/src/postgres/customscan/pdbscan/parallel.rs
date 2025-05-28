use crate::api::Cardinality;
use crate::api::HashSet;
use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::dsm::ParallelQueryCapable;
use crate::postgres::customscan::pdbscan::PdbScan;
use crate::postgres::ParallelScanState;
use pgrx::pg_sys::{self, shm_toc, ParallelContext, Size};
use std::os::raw::c_void;
use tantivy::index::SegmentId;

impl ParallelQueryCapable for PdbScan {
    fn estimate_dsm_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        pcxt: *mut ParallelContext,
    ) -> Size {
        if state.custom_state().search_reader.is_none() {
            PdbScan::init_search_reader(state);
        }

        let (segments, serialized_query) = state.custom_state().parallel_serialization_data();
        ParallelScanState::size_of(segments.len(), &serialized_query)
    }

    fn initialize_dsm_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        pcxt: *mut ParallelContext,
        coordinate: *mut c_void,
    ) {
        let (segments, serialized_query) = state.custom_state().parallel_serialization_data();

        unsafe {
            let pscan_state = coordinate.cast::<ParallelScanState>();
            assert!(!pscan_state.is_null(), "coordinate is null");
            (*pscan_state).init(segments, &serialized_query);
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

pub unsafe fn checkout_segment(pscan_state: *mut ParallelScanState) -> Option<SegmentId> {
    let mutex = (*pscan_state).acquire_mutex();
    if (*pscan_state).remaining_segments() > 0 {
        let remaining_segments = (*pscan_state).decrement_remaining_segments();
        Some((*pscan_state).segment_id(remaining_segments))
    } else {
        None
    }
}

pub unsafe fn list_segment_ids(pscan_state: *mut ParallelScanState) -> HashSet<SegmentId> {
    (*pscan_state).segments().keys().cloned().collect()
}
