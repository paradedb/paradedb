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

use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::buffer::BufferManager;
use crate::postgres::utils;
use pgrx::itemptr::item_pointer_get_block_number;
use pgrx::pg_sys;
use pgrx::PgList;

/// Helper to manage the information necessary to validate that a "ctid" is currently visible to
/// a snapshot
pub struct VisibilityChecker {
    scan: *mut pg_sys::IndexFetchTableData,
    snapshot: pg_sys::Snapshot,
    tid: pg_sys::ItemPointerData,

    // we hold onto this b/c `scan` points to the relation this does
    heaprel: PgSearchRelation,
    bman: BufferManager,

    vmbuff: pg_sys::Buffer,
    // tracks our previous block visibility so we can elide checking again
    blockvis: (pg_sys::BlockNumber, bool),

    pub heap_tuple_check_count: usize,
    pub invisible_tuple_count: usize,
}

impl Clone for VisibilityChecker {
    fn clone(&self) -> Self {
        Self::with_rel_and_snap(&self.heaprel, self.snapshot)
    }
}

crate::impl_safe_drop!(VisibilityChecker, |self| {
    unsafe {
        if crate::postgres::utils::IsTransactionState() {
            if self.vmbuff != pg_sys::InvalidBuffer as pg_sys::Buffer {
                pg_sys::ReleaseBuffer(self.vmbuff);
            }
            pg_sys::table_index_fetch_end(self.scan);
        }
    }
});

impl VisibilityChecker {
    /// Construct a new [`VisibilityChecker`] that can validate ctid visibility against the specified
    /// `relation` and `snapshot`
    pub fn with_rel_and_snap(heaprel: &PgSearchRelation, snapshot: pg_sys::Snapshot) -> Self {
        unsafe {
            Self {
                scan: pg_sys::table_index_fetch_begin(heaprel.as_ptr()),
                snapshot,
                tid: pg_sys::ItemPointerData::default(),
                heaprel: Clone::clone(heaprel),
                bman: BufferManager::new(heaprel),
                vmbuff: pg_sys::InvalidBuffer as pg_sys::Buffer,
                blockvis: (pg_sys::InvalidBlockNumber, false),
                heap_tuple_check_count: 0,
                invisible_tuple_count: 0,
            }
        }
    }

    /// If the specified `ctid` is visible in the heap, run the provided closure and return its
    /// result as `Some(T)`. If it's not visible, return `None` without running the provided closure.
    ///
    /// NOTE: Does _not_ check the visibility map first: is for use in contexts which have already
    /// applied visibility checking if needed.
    pub fn exec_if_visible<T, F: FnMut(pg_sys::Relation) -> T>(
        &mut self,
        ctid: u64,
        slot: *mut pg_sys::TupleTableSlot,
        mut func: F,
    ) -> Option<T> {
        self.heap_tuple_check_count += 1;
        unsafe {
            utils::u64_to_item_pointer(ctid, &mut self.tid);

            let mut call_again = false;
            let mut all_dead = false;
            let found = pg_sys::table_index_fetch_tuple(
                self.scan,
                &mut self.tid,
                self.snapshot,
                slot,
                &mut call_again,
                &mut all_dead,
            );

            if found {
                Some(func((*self.scan).rel))
            } else {
                self.invisible_tuple_count += 1;
                None
            }
        }
    }

    /// Returns true if the block is all visible.
    pub fn is_block_all_visible(&mut self, blockno: pg_sys::BlockNumber) -> bool {
        if blockno == self.blockvis.0 {
            return self.blockvis.1;
        }
        self.blockvis.0 = blockno;
        unsafe {
            let status =
                pg_sys::visibilitymap_get_status(self.heaprel.as_ptr(), blockno, &mut self.vmbuff);
            self.blockvis.1 = status != 0;
        }
        self.blockvis.1
    }

    /// If the specified `ctid` is visible in the heap, return the visible `ctid`.
    /// The returned `ctid` might differ from the input `ctid` if a HOT chain was followed.
    fn check_visibility(&mut self, ctid: u64) -> Option<u64> {
        unsafe {
            utils::u64_to_item_pointer(ctid, &mut self.tid);

            let block_num = item_pointer_get_block_number(&self.tid);

            let mut heap_tuple_data: pg_sys::HeapTupleData = std::mem::zeroed();
            let mut all_dead = false;

            // get_buffer acquires a pin and a share lock, and will release them on Drop
            // TODO: Consider exposing a method for bulk filtering of ctids which holds buffers
            // across multiple ctids.
            let buffer = self.bman.get_buffer(block_num);
            let found = pg_sys::heap_hot_search_buffer(
                &mut self.tid,
                self.heaprel.as_ptr(),
                *buffer,
                self.snapshot,
                &mut heap_tuple_data,
                &mut all_dead,
                true, // first_call
            );
            std::mem::drop(buffer);

            if found {
                Some(utils::item_pointer_to_u64(self.tid))
            } else {
                None
            }
        }
    }

    /// Checks if a row is visible (and returns its new ctid if so), without actually fetching
    /// it into a slot. Uses the visibility map and consults the heap if necessary.
    ///
    /// Returns `Some(ctid)` if the row is visible, potentially updating the ctid
    /// (e.g. if following a HOT chain). Returns `None` if the row is not visible.
    pub fn check(&mut self, mut ctid: u64) -> Option<u64> {
        unsafe {
            utils::u64_to_item_pointer(ctid, &mut self.tid);
            let blockno = item_pointer_get_block_number(&self.tid);
            let is_visible = self.is_block_all_visible(blockno);

            if is_visible {
                Some(ctid)
            } else {
                self.heap_tuple_check_count += 1;
                if let Some(visible_ctid) = self.check_visibility(ctid) {
                    if visible_ctid != ctid {
                        ctid = visible_ctid;
                    }
                    Some(ctid)
                } else {
                    self.invisible_tuple_count += 1;
                    None
                }
            }
        }
    }
}

/// A wrapper for an owned scan and slot for repeated use with table_index_fetch_tuple.
///
/// TODO: Similar to `VisibilityChecker`, but uses an owned slot in the shape of the table, rather
/// than borrowing a slot in the shape of the custom scan.
#[derive(Debug)]
pub struct HeapFetchState {
    pub scan: *mut pg_sys::IndexFetchTableData,
    slot: *mut pg_sys::BufferHeapTupleTableSlot,
}

impl HeapFetchState {
    /// Create a HeapFetchState which will fetch the entire content of the given relation.
    pub fn new(heaprel: &PgSearchRelation) -> Self {
        unsafe {
            let scan = pg_sys::table_index_fetch_begin(heaprel.as_ptr());
            let slot = pg_sys::MakeTupleTableSlot(heaprel.rd_att, &pg_sys::TTSOpsBufferHeapTuple);
            Self {
                scan,
                slot: slot.cast(),
            }
        }
    }

    pub fn slot(&self) -> *mut pg_sys::TupleTableSlot {
        self.slot.cast()
    }

    pub fn buffer_slot(&self) -> *mut pg_sys::BufferHeapTupleTableSlot {
        self.slot
    }
}

crate::impl_safe_drop!(HeapFetchState, |self| {
    unsafe {
        if crate::postgres::utils::IsTransactionState() {
            pg_sys::ExecDropSingleTupleTableSlot(self.slot.cast());
            pg_sys::table_index_fetch_end(self.scan);
        }
    }
});

/// A wrapper for expression evaluation state.
#[derive(Debug)]
pub struct ExpressionState {
    econtext: *mut pg_sys::ExprContext,
    expr_states: Vec<*mut pg_sys::ExprState>,
}

impl ExpressionState {
    /// Create an ExpressionState for the given index relation.
    pub fn new(indexrel: &PgSearchRelation) -> Self {
        let index_exprs = unsafe { pg_sys::RelationGetIndexExpressions(indexrel.as_ptr()) };
        let mut econtext: *mut pg_sys::ExprContext = std::ptr::null_mut();
        let expr_states = if !index_exprs.is_null() {
            econtext = unsafe {
                pgrx::PgMemoryContexts::TopTransactionContext
                    .switch_to(|_| pg_sys::CreateStandaloneExprContext())
            };
            let expr_list: PgList<pg_sys::Node> = unsafe { PgList::from_pg(index_exprs) };

            let old_context =
                unsafe { pg_sys::MemoryContextSwitchTo((*econtext).ecxt_per_query_memory) };

            let states = expr_list
                .iter_ptr()
                .map(|expr_node| unsafe {
                    pg_sys::ExecInitExpr(expr_node.cast(), std::ptr::null_mut())
                })
                .collect::<Vec<_>>();

            unsafe { pg_sys::MemoryContextSwitchTo(old_context) };
            states
        } else {
            vec![]
        };

        Self {
            econtext,
            expr_states,
        }
    }

    /// Evaluate expressions for the tuple in the given slot.
    pub fn evaluate(&self, slot: *mut pg_sys::TupleTableSlot) -> Vec<(pg_sys::Datum, bool)> {
        let mut expr_results = Vec::new();
        if !self.econtext.is_null() {
            unsafe {
                (*self.econtext).ecxt_scantuple = slot;
            }
            for expr_state in &self.expr_states {
                let mut is_null = false;
                let datum =
                    unsafe { pg_sys::ExecEvalExpr(*expr_state, self.econtext, &mut is_null) };
                expr_results.push((datum, is_null));
            }
        }
        expr_results
    }
}
