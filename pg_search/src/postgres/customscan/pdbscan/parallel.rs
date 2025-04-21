use crate::api::Cardinality;
use crate::index::mvcc::MVCCDirectory;
use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::dsm::ParallelQueryCapable;
use crate::postgres::customscan::pdbscan::PdbScan;
use crate::postgres::customscan::CustomScan;
use crate::postgres::ParallelScanState;
use pgrx::pg_sys::{self, panic::ErrorReport, shm_toc, ParallelContext, Size};
use pgrx::{function_name, PgLogLevel, PgRelation, PgSqlErrorCode};
use std::collections::HashSet;
use std::os::raw::c_void;
use tantivy::{index::SegmentId, Index};

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

///
/// Compute the number of workers that should be used for the given limit, segment_count, and sort
/// condition, or return 0 if workers cannot or should not be used.
///
pub fn compute_nworkers(limit: Option<Cardinality>, segment_count: usize, sorted: bool) -> usize {
    // we will try to parallelize based on the number of index segments
    let mut nworkers = unsafe { segment_count.min(pg_sys::max_parallel_workers as usize) };

    if let Some(limit) = limit {
        if !sorted && limit <= (segment_count * segment_count * segment_count) as Cardinality {
            // not worth it to do a parallel scan
            return 0;
        }

        // if the limit is less than some arbitrarily large value
        // use at most half the number of parallel workers as there are segments
        // this generally seems to perform better than directly using `max_parallel_workers`
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

#[allow(dead_code)]
pub unsafe fn check_for_concurrent_vacuum(pscan_state: &mut CustomScanStateWrapper<PdbScan>) {
    let indexrel = pscan_state
        .custom_state()
        .indexrel
        .as_ref()
        .map(|indexrel| unsafe { PgRelation::from_pg(*indexrel) })
        .expect("end_custom_scan: custom_state.indexrel should already be open");
    let old_segments = unsafe { (*pscan_state.custom_state().parallel_state.unwrap()).segments() };
    let directory =
        MVCCDirectory::parallel_worker(indexrel.oid(), old_segments.keys().cloned().collect());
    let index = Index::open(directory).expect("end_custom_scan: should be able to open index");
    let new_metas = index
        .searchable_segment_metas()
        .expect("end_custom_scan: should be able to get segment metas");

    let new_segments: std::collections::HashMap<_, _> = new_metas
        .iter()
        .map(|meta| (meta.id(), meta.num_deleted_docs()))
        .collect();

    for (segment_id, num_deleted_docs) in old_segments {
        if new_segments.get(&segment_id).unwrap_or(&0) != &num_deleted_docs {
            ErrorReport::new(
                PgSqlErrorCode::ERRCODE_QUERY_CANCELED,
                "cancelling query due to conflict with vacuum",
                function_name!(),
            )
            .set_detail("a concurrent vacuum operation on the WAL sender is running")
            .set_hint("retry the query when the vacuum operation has completed")
            .report(PgLogLevel::ERROR);
        }
    }
}
