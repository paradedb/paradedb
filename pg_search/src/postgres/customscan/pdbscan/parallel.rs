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

        ParallelScanState::size_of_with_segments(
            state
                .custom_state()
                .search_reader
                .as_ref()
                .expect("search reader must be initialized to estimate DSM size")
                .segment_readers()
                .len(),
        )
    }

    fn initialize_dsm_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        pcxt: *mut ParallelContext,
        coordinate: *mut c_void,
    ) {
        let pscan_state = coordinate.cast::<ParallelScanState>();
        assert!(!pscan_state.is_null(), "coordinate is null");

        unsafe {
            (*pscan_state).mutex.init();
            (*pscan_state).assign_segment_ids(state.custom_state().search_reader.as_ref().unwrap());
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
    }
}

pub unsafe fn checkout_segment(pscan_state: *mut ParallelScanState) -> Option<SegmentId> {
    let mutex = (*pscan_state).mutex.acquire();
    if (*pscan_state).remaining_segments > 0 {
        (*pscan_state).remaining_segments -= 1;

        Some((*pscan_state).get_segment_id((*pscan_state).remaining_segments as usize))
    } else {
        None
    }
}
