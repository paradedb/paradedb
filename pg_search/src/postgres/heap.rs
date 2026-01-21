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
use crate::postgres::utils;
use pgrx::pg_sys;
use pgrx::PgList;

/// Helper to validate that a "ctid" is currently visible to a snapshot.
///
/// ## Two-Layer Visibility in pg_search
///
/// When querying BM25 indexes, visibility must be checked at two levels:
///
/// 1. **Segment-level** (`MvccSatisfies::Snapshot`): Determines which Tantivy segments
///    are visible based on transaction visibility. A segment is visible if it was
///    committed before our snapshot.
///
/// 2. **Tuple-level** (this struct): Within a visible segment, individual ctid entries
///    may be stale. After an UPDATE, the old tuple is marked dead and a new tuple is
///    created at a new ctid, but the index still has the old ctid until VACUUM runs.
///
/// The visibility checker uses `table_index_fetch_tuple` which:
/// - Verifies the tuple exists and is visible (following HOT chains if needed)
/// - Returns the tuple at its current heap location
///
/// Use [`get_current_ctid`](Self::get_current_ctid) to resolve an index ctid to the
/// tuple's current heap location.
pub struct VisibilityChecker {
    scan: *mut pg_sys::IndexFetchTableData,
    snapshot: pg_sys::Snapshot,
    tid: pg_sys::ItemPointerData,

    // we hold onto this b/c `scan` points to the relation this does
    _heaprel: PgSearchRelation,
}

crate::impl_safe_drop!(VisibilityChecker, |self| {
    unsafe {
        if crate::postgres::utils::IsTransactionState() {
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
                _heaprel: Clone::clone(heaprel),
            }
        }
    }

    /// If the specified `ctid` is visible in the heap, run the provided closure and return its
    /// result as `Some(T)`.  If it's not visible, return `None` without running the provided closure.
    ///
    /// This uses table_index_fetch_tuple which is designed for ctids from an INDEX scan.
    /// For ctids from a sequential scan, use `fetch_tuple_direct` instead.
    pub fn exec_if_visible<T, F: FnMut(pg_sys::Relation) -> T>(
        &mut self,
        ctid: u64,
        slot: *mut pg_sys::TupleTableSlot,
        mut func: F,
    ) -> Option<T> {
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
                None
            }
        }
    }

    /// Fetch a tuple directly by ctid, without going through the index fetch machinery.
    ///
    /// This is the correct method to use when the ctid was obtained from a sequential scan
    /// (e.g., from building a hash table). Unlike exec_if_visible which uses table_index_fetch_tuple
    /// and handles HOT chains from index ctids, this uses table_tuple_fetch_row_version which
    /// directly fetches the tuple at the given ctid.
    ///
    /// Returns true if the tuple was found and visible, false otherwise.
    pub fn fetch_tuple_direct(&self, ctid: u64, slot: *mut pg_sys::TupleTableSlot) -> bool {
        unsafe {
            let mut tid = pg_sys::ItemPointerData::default();
            utils::u64_to_item_pointer(ctid, &mut tid);

            pg_sys::table_tuple_fetch_row_version(
                self._heaprel.as_ptr(),
                &mut tid,
                self.snapshot,
                slot,
            )
        }
    }

    /// Check if a ctid from an index is visible, and return its current heap location.
    ///
    /// This handles the case where a tuple has been updated (moving to a new ctid) but
    /// the index hasn't been vacuumed yet. The index ctid may be stale, but
    /// `table_index_fetch_tuple` follows HOT chains to find the current tuple location.
    ///
    /// Returns `Some(current_ctid)` if the tuple is visible, `None` if deleted/not visible.
    /// The returned ctid may differ from the input ctid if the tuple moved.
    pub fn get_current_ctid(
        &mut self,
        index_ctid: u64,
        slot: *mut pg_sys::TupleTableSlot,
    ) -> Option<u64> {
        self.exec_if_visible(index_ctid, slot, |_| {
            // The slot's tts_tid contains the current heap location of the tuple
            unsafe { utils::item_pointer_to_u64((*slot).tts_tid) }
        })
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

/// A [`VisibilityChecker`] that owns its slot.
///
/// This is useful when you need to check visibility of ctids and extract current heap locations,
/// but don't have an existing slot to use. The slot is automatically cleaned up on drop.
pub struct OwnedVisibilityChecker {
    checker: VisibilityChecker,
    slot: *mut pg_sys::TupleTableSlot,
}

impl OwnedVisibilityChecker {
    /// Create a new `OwnedVisibilityChecker` for the given heap relation and snapshot.
    pub fn new(heaprel: &PgSearchRelation, snapshot: pg_sys::Snapshot) -> Self {
        unsafe {
            let checker = VisibilityChecker::with_rel_and_snap(heaprel, snapshot);
            let slot = pg_sys::MakeTupleTableSlot(heaprel.rd_att, &pg_sys::TTSOpsBufferHeapTuple);
            Self { checker, slot }
        }
    }

    /// Check if a ctid from an index is visible, and return its current heap location.
    ///
    /// Returns `Some(current_ctid)` if the tuple is visible, `None` if deleted/not visible.
    /// The returned ctid may differ from the input ctid if the tuple moved after UPDATE.
    pub fn get_current_ctid(&mut self, index_ctid: u64) -> Option<u64> {
        self.checker.get_current_ctid(index_ctid, self.slot)
    }
}

crate::impl_safe_drop!(OwnedVisibilityChecker, |self| {
    unsafe {
        if !crate::postgres::utils::IsTransactionState() {
            return;
        }
        pg_sys::ExecDropSingleTupleTableSlot(self.slot);
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
