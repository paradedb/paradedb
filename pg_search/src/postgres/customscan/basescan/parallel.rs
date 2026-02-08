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

use crate::postgres::customscan::basescan::BaseScan;
use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::dsm::ParallelQueryCapable;
use crate::postgres::ParallelScanState;

use pgrx::pg_sys::{shm_toc, ParallelContext, Size};

impl ParallelQueryCapable for BaseScan {
    fn estimate_dsm_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        _pcxt: *mut ParallelContext,
    ) -> Size {
        if state.custom_state().search_reader.is_none() {
            BaseScan::init_search_reader(state);
        }

        let args = state.custom_state().parallel_scan_args();
        ParallelScanState::size_of(
            args.segment_readers.len(),
            &args.solved_expressions,
            args.with_aggregates,
        )
    }

    fn initialize_dsm_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        _pcxt: *mut ParallelContext,
        coordinate: *mut c_void,
    ) {
        let args = state.custom_state().parallel_scan_args();

        unsafe {
            let pscan_state = coordinate.cast::<ParallelScanState>();
            assert!(!pscan_state.is_null(), "coordinate is null");
            (*pscan_state).create_and_populate(args);
            state.custom_state_mut().parallel_state = Some(pscan_state);
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
            (*pscan_state).reset();
        }
    }

    fn initialize_worker_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        _toc: *mut shm_toc,
        coordinate: *mut c_void,
    ) {
        let pscan_state = coordinate.cast::<ParallelScanState>();
        assert!(!pscan_state.is_null(), "coordinate is null");

        state.custom_state_mut().parallel_state = Some(pscan_state);
        unsafe {
            if let Some(solved) = (*pscan_state).solved_expressions().expect(
                "should be able to deserialize the solved expressions from the ParallelScanState",
            ) {
                let mut solved = std::collections::VecDeque::from(solved);
                state
                    .custom_state_mut()
                    .base_search_query_input_mut()
                    .apply_solved_expressions(&mut solved);
            }
        }
    }
}
