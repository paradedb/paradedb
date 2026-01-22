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

pub(crate) mod fast_fields;
pub(crate) mod normal;
pub(crate) mod top_n;

use crate::postgres::customscan::basescan::scan_state::BaseScanState;
use pgrx::pg_sys;
use tantivy::{DocAddress, Score};

pub enum ExecState {
    /// Causes a tuple to be fetched from the heap, which will implicitly cause it to be visibility
    /// checked.
    FromHeap {
        ctid: u64,
        score: Score,
        doc_address: DocAddress,
    },
    /// Produces the given tuple directly.
    ///
    /// NOTE: A virtual tuple must already be MVCC-correct, as the consumer of ExecState does not
    /// do any further checking. If it corresponds to a heap-tuple, and has been produced via a
    /// covering index scan like the MixedFastField scan, then it should already have been proven
    /// visible via the visibility map or our VisibilityChecker.
    Virtual { slot: *mut pg_sys::TupleTableSlot },
    /// Indicates that there are no more tuples available.
    Eof,
}

impl Default for Box<dyn ExecMethod> {
    fn default() -> Self {
        Box::new(UnknownScanStyle)
    }
}

pub trait ExecMethod {
    /// Called after an ExecMethod is created, but before the first calls to `query`/`next`.
    ///
    /// By default this method calls `reset`, which is the behavior you will want if you do not
    /// want to differentiate "being created from scratch" from "preserving some state but
    /// starting over from the beginning of the scan".
    fn init(&mut self, state: &mut BaseScanState, _cstate: *mut pg_sys::CustomScanState) {
        self.reset(state)
    }

    fn uses_visibility_map(&self, state: &BaseScanState) -> bool {
        true
    }

    fn query(&mut self, state: &mut BaseScanState) -> bool;

    fn next(&mut self, state: &mut BaseScanState) -> ExecState {
        loop {
            match self.internal_next(state) {
                ExecState::Eof => {
                    if !self.query(state) {
                        return ExecState::Eof;
                    }
                }
                other => return other,
            }
        }
    }

    fn internal_next(&mut self, state: &mut BaseScanState) -> ExecState;

    fn increment_visible(&mut self) {
        // default of noop
    }

    /// This is called:
    /// * by the `init` method (by default)
    /// * in a parallel-workers leader (_not_ in any parallel workers) during re-scans
    ///     * instead, parallel workers are re-created during re-scans, and so will have `init`
    ///       called.
    ///
    /// [`BaseScanState::reset()`] will already have been called for you.
    fn reset(&mut self, state: &mut BaseScanState);
}

struct UnknownScanStyle;

impl ExecMethod for UnknownScanStyle {
    fn init(&mut self, _state: &mut BaseScanState, _cstate: *mut pg_sys::CustomScanState) {
        unimplemented!(
            "logic error in pg_search:  `UnknownScanStyle::init()` should never be called"
        )
    }

    fn query(&mut self, _state: &mut BaseScanState) -> bool {
        unimplemented!(
            "logic error in pg_search: `UnknownScanStyle::query()` should never be called"
        )
    }

    fn internal_next(&mut self, _state: &mut BaseScanState) -> ExecState {
        unimplemented!(
            "logic error in pg_search:  `UnknownScanStyle::internal_next()` should never be called"
        )
    }

    fn reset(&mut self, _state: &mut BaseScanState) {
        unimplemented!(
            "logic error in pg_search:  `UnknownScanStyle::reset()` should never be called"
        )
    }
}
