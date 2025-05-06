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

use std::sync::Arc;

use crate::postgres::utils;
use pgrx::pg_sys;

use parking_lot::Mutex;

/// Helper to manage the information necessary to validate that a "ctid" is currently visible to
/// a snapshot
#[derive(Clone)]
pub struct VisibilityChecker(Arc<Mutex<VisibilityCheckerInner>>);

struct VisibilityCheckerInner {
    scan: *mut pg_sys::IndexFetchTableData,
    snapshot: pg_sys::Snapshot,
    tid: pg_sys::ItemPointerData,
    heap_tuple_fetch_count: usize,
    heap_tuple_check_count: usize,
    invisible_tuple_count: usize,
}

// SAFETY: `VisibilityChecker` is not actually `Send`... because ~nothing in Postgres' API is
// Send. But this bound is required due to Tantivy's API, which wants to be able to send
// `(Segment)Collector`s to background threads... which we cannot use.
unsafe impl Send for VisibilityCheckerInner {}

impl Drop for VisibilityCheckerInner {
    fn drop(&mut self) {
        unsafe {
            if !crate::postgres::utils::IsTransactionState() {
                // we are not in a transaction, so we can't do things like release buffers and close relations
                return;
            }

            pg_sys::table_index_fetch_end(self.scan);
        }
    }
}

impl VisibilityChecker {
    /// Construct a new [`VisibilityChecker`] that can validate ctid visibility against the specified
    /// `relation` and `snapshot`
    pub fn with_rel_and_snap(heaprel: pg_sys::Relation, snapshot: pg_sys::Snapshot) -> Self {
        Self(Arc::new(Mutex::new(unsafe {
            VisibilityCheckerInner {
                scan: pg_sys::table_index_fetch_begin(heaprel),
                snapshot,
                tid: pg_sys::ItemPointerData::default(),
                heap_tuple_fetch_count: 0,
                heap_tuple_check_count: 0,
                invisible_tuple_count: 0,
            }
        })))
    }

    /// If the specified `ctid` is visible in the heap, run the provided closure and return its
    /// result as `Some(T)`.  If it's not visible, return `None` without running the provided closure
    pub fn exec_if_visible<T, F: FnMut(pg_sys::Relation) -> T>(
        &self,
        ctid: u64,
        slot: *mut pg_sys::TupleTableSlot,
        mut func: F,
    ) -> Option<T> {
        let mut inner = self.0.lock();
        unsafe {
            utils::u64_to_item_pointer(ctid, &mut inner.tid);

            let mut call_again = false;
            let mut all_dead = false;
            let found = pg_sys::table_index_fetch_tuple(
                inner.scan,
                &mut inner.tid,
                inner.snapshot,
                slot,
                &mut call_again,
                &mut all_dead,
            );

            if found {
                inner.heap_tuple_fetch_count += 1;
                Some(func((*inner.scan).rel))
            } else {
                inner.invisible_tuple_count += 1;
                None
            }
        }
    }

    pub fn is_visible(&self, ctid: u64) -> bool {
        let mut inner = self.0.lock();
        unsafe {
            utils::u64_to_item_pointer(ctid, &mut inner.tid);

            let mut all_dead = false;
            let is_visible = pg_sys::table_index_fetch_tuple_check(
                (*inner.scan).rel,
                &mut inner.tid,
                inner.snapshot,
                &mut all_dead,
            );

            if is_visible {
                inner.heap_tuple_check_count += 1;
            } else {
                inner.invisible_tuple_count += 1;
            }

            is_visible
        }
    }

    pub fn heap_tuple_fetch_count(&self) -> usize {
        self.0.lock().heap_tuple_fetch_count
    }

    pub fn heap_tuple_check_count(&self) -> usize {
        self.0.lock().heap_tuple_check_count
    }

    pub fn invisible_tuple_count(&self) -> usize {
        self.0.lock().invisible_tuple_count
    }

    pub fn reset(&self) {
        let mut inner = self.0.lock();
        inner.heap_tuple_fetch_count = 0;
        inner.heap_tuple_check_count = 0;
        inner.invisible_tuple_count = 0;
    }
}
