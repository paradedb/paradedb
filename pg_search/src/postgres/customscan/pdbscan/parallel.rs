use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::dsm::ParallelQueryCapable;
use crate::postgres::customscan::pdbscan::PdbScan;
use crate::postgres::customscan::CustomScan;
use crate::postgres::ParallelScanState;
use pgrx::pg_sys::{shm_toc, ParallelContext, Size};
use std::os::raw::c_void;
use tantivy::index::SegmentId;

impl ParallelQueryCapable for PdbScan {
    fn estimate_dsm_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        pcxt: *mut ParallelContext,
    ) -> Size {
        if state.custom_state().search_reader.is_none() {
            PdbScan::rescan_custom_scan(state);
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

pub unsafe fn checkout_segment(pscan_state: *mut ParallelScanState) -> Option<SegmentId> {
    let mutex = (*pscan_state).acquire_mutex();
    if (*pscan_state).remaining_segments() > 0 {
        let remaining_segments = (*pscan_state).decrement_remaining_segments();
        Some((*pscan_state).segment_id(remaining_segments))
    } else {
        None
    }
}
