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
use crate::customscan::pdbscan::ExecMethodType;
use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::dsm::ParallelQueryCapable;
use crate::postgres::customscan::pdbscan::PdbScan;
use crate::postgres::ParallelScanState;

use pgrx::pg_sys::{self, shm_toc, ParallelContext, Size};
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
/// Compute the number of workers that should be used for the given ExecMethod, segment_count, and
/// presence of external vars (indicating a join), or return 0 if workers cannot or should not be
/// used.
///
pub fn compute_nworkers(
    exec_method: &ExecMethodType,
    limit: Option<Cardinality>,
    estimated_total_rows: Cardinality,
    segment_count: usize,
    contains_external_var: bool,
) -> usize {
    // We will try to parallelize based on the number of index segments. The leader is not included
    // in `nworkers`, so exclude it here. For example: if we expect to need to query 1 segment, then
    // we don't need any workers.
    let mut nworkers = segment_count.saturating_sub(1);

    // parallel workers available to a gather node are limited by max_parallel_workers_per_gather
    // and max_parallel_workers
    nworkers = unsafe {
        nworkers
            .min(pg_sys::max_parallel_workers_per_gather as usize)
            .min(pg_sys::max_parallel_workers as usize)
    };

    // if we are not sorting the data (which always requires fetching data from all segments), then
    // limit the number of workers to the number of segments we expect to have to query to reach
    // the limit.
    if let (false, Some(limit)) = (exec_method.is_sorted_topn(), limit) {
        let rows_per_segment = estimated_total_rows / segment_count.max(1) as f64;
        let segments_to_reach_limit = (limit / rows_per_segment).ceil() as usize;
        // See above re: the leader not being included in `nworkers`.
        let nworkers_for_limited_segments = segments_to_reach_limit.saturating_sub(1);
        nworkers = nworkers.min(nworkers_for_limited_segments);
    }

    if contains_external_var {
        // Don't attempt to parallelize during a join.
        // TODO: Re-evaluate.
        nworkers = 0;
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
    (*pscan_state).checkout_segment()
}

pub unsafe fn list_segment_ids(pscan_state: *mut ParallelScanState) -> HashSet<SegmentId> {
    (*pscan_state).segments().keys().cloned().collect()
}
