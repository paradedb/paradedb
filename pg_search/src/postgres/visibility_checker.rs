// Copyright (c) 2023-2024 Retake, Inc.
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

use crate::postgres::utils;
use pgrx::itemptr::item_pointer_get_block_number;
use pgrx::pg_sys;
use pgrx::pg_sys::Buffer;

//
// we redeclare these functions so we can use the directly without pgrx' "#[pg_guard]" overhead.
//
// Instead, when we call these, we make sure we've created our own ffi boundary guard and run all
// these functions within the same guard
//
#[allow(improper_ctypes)]
#[allow(non_snake_case)]
extern "C" {
    fn ReleaseAndReadBuffer(
        buffer: Buffer,
        relation: pg_sys::Relation,
        blockNum: pg_sys::BlockNumber,
    ) -> Buffer;

    fn LockBuffer(buffer: Buffer, mode: ::core::ffi::c_int);
    fn heap_hot_search_buffer(
        tid: pg_sys::ItemPointer,
        relation: pg_sys::Relation,
        buffer: Buffer,
        snapshot: pg_sys::Snapshot,
        heapTuple: pg_sys::HeapTuple,
        all_dead: *mut bool,
        first_call: bool,
    ) -> bool;
}

/// Helper to manage the information necessary to validate that a "ctid" is currently visible to
/// a snapshot
pub struct VisibilityChecker {
    relation: pg_sys::Relation,
    need_close: bool,
    snapshot: pg_sys::Snapshot,
    last_buffer: pg_sys::Buffer,
    ipd: pg_sys::ItemPointerData,
}

impl Drop for VisibilityChecker {
    fn drop(&mut self) {
        unsafe {
            if !pg_sys::IsTransactionState() {
                // we are not in a transaction, so we can't do things like release buffers and close relations
                return;
            }

            if self.last_buffer != pg_sys::InvalidBuffer as pg_sys::Buffer {
                pg_sys::ReleaseBuffer(self.last_buffer);
            }

            if self.need_close {
                // SAFETY:  `self.relation` is always a valid, open relation, created via `pg_sys::RelationGetRelation`
                pg_sys::RelationClose(self.relation);
            }
        }
    }
}

impl VisibilityChecker {
    /// Construct a new [`VisibilityChecker`] that can validate ctid visibility against the specified
    /// `relation` and `snapshot`
    pub fn with_rel_and_snap(relation: pg_sys::Relation, snapshot: pg_sys::Snapshot) -> Self {
        Self {
            relation,
            need_close: false,
            snapshot,
            last_buffer: pg_sys::InvalidBuffer as pg_sys::Buffer,
            ipd: pg_sys::ItemPointerData::default(),
        }
    }

    /// If the specified `ctid` is visible in the heap, run the provided closure and return its
    /// result as `Some(T)`.  If it's not visible, return `None` without running the provided closure
    pub fn exec_if_visible<T, F: FnMut(pg_sys::Oid, pg_sys::HeapTupleData, pg_sys::Buffer) -> T>(
        &mut self,
        ctid: u64,
        mut func: F,
    ) -> Option<T> {
        unsafe {
            // Using ctid, get itempointer => buffer => page => heaptuple
            utils::u64_to_item_pointer(ctid, &mut self.ipd);

            let blockno = item_pointer_get_block_number(&self.ipd);

            // SAFETY:  in order for us to properly handle possible ERRORs we need to create
            // our own ffi guard boundary.  The ReleaseAndReadBuffer, LockBuffer, and heap_hot_search_buffer (see below)
            // functions are internal to postgres and the ffi boundary needs to be guarded, but we
            // don't want to incur the overhead of guarding each one individually.
            //
            // This also create a requirement that we cannot raise a rust panic!() while in the
            // `pg_guard_ffi_boundary()` closure.
            pg_sys::ffi::pg_guard_ffi_boundary(|| {
                self.last_buffer = ReleaseAndReadBuffer(self.last_buffer, self.relation, blockno);

                LockBuffer(self.last_buffer, pg_sys::BUFFER_LOCK_SHARE as _);
                let (found, htup) = self.check_page_vis(self.last_buffer);
                let result = found.then(|| func((*self.relation).rd_id, htup, self.last_buffer));
                LockBuffer(self.last_buffer, pg_sys::BUFFER_LOCK_UNLOCK as _);
                result
            })
        }
    }

    unsafe fn check_page_vis(&mut self, buffer: pg_sys::Buffer) -> (bool, pg_sys::HeapTupleData) {
        unsafe {
            let mut heap_tuple = pg_sys::HeapTupleData::default();

            // Check if heaptuple is visible
            // In Postgres, the indexam `amgettuple` calls `heap_hot_search_buffer` for its visibility check
            let found = heap_hot_search_buffer(
                &mut self.ipd,
                self.relation,
                buffer,
                self.snapshot,
                &mut heap_tuple,
                std::ptr::null_mut(),
                true,
            );
            (found, heap_tuple)
        }
    }
}
