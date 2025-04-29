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

use crate::postgres::utils;
use pgrx::pg_sys;

/// Helper to manage the information necessary to validate that a "ctid" is currently visible to
/// a snapshot
pub struct VisibilityChecker {
    scan: *mut pg_sys::IndexFetchTableData,
    snapshot: pg_sys::Snapshot,
    tid: pg_sys::ItemPointerData,
}

impl Drop for VisibilityChecker {
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
        unsafe {
            Self {
                scan: pg_sys::table_index_fetch_begin(heaprel),
                snapshot,
                tid: pg_sys::ItemPointerData::default(),
            }
        }
    }

    /// If the specified `ctid` is visible in the heap, run the provided closure and return its
    /// result as `Some(T)`.  If it's not visible, return `None` without running the provided closure
    pub fn exec_if_visible<F, T>(&mut self, ctid: u64, slot: *mut pg_sys::TupleTableSlot, func: F) -> Option<T>
    where
        F: FnOnce(pg_sys::Relation) -> T,
    {
        unsafe {
            pgrx::warning!("VISIBILITY_CHECKER: Checking visibility for ctid={}", ctid);
            
            let mut tid = pg_sys::ItemPointerData::default();
            crate::postgres::utils::u64_to_item_pointer(ctid, &mut tid);

            // Try to read the tuple into the provided slot to avoid allocating a new one
            let mut call_again = false;
            let mut all_dead = false;

            // Create a memory context for this operation
            let context_name = format!("NestedLoopVisibilityContext_{}", ctid);
            let mut memory_context = pgrx::PgMemoryContexts::new(&context_name);
            pgrx::warning!("VISIBILITY_CHECKER: Created memory context '{}'", context_name);
            
            // Use the memory context for the operation
            let result = memory_context.switch_to(|_| {
                pgrx::warning!("VISIBILITY_CHECKER: Switched to visibility memory context for ctid={}", ctid);
                
                let found = pg_sys::table_index_fetch_tuple(
                    self.scan,
                    &mut tid,
                    self.snapshot,
                    slot,
                    &mut call_again,
                    &mut all_dead,
                );
                
                pgrx::warning!("VISIBILITY_CHECKER: Fetch tuple result={}, call_again={}, all_dead={} for ctid={}", 
                             found, call_again, all_dead, ctid);
                
                if found {
                    pgrx::warning!("VISIBILITY_CHECKER: Document ctid={} is visible", ctid);
                    Some(func((*self.scan).rel))
                } else {
                    pgrx::warning!("VISIBILITY_CHECKER: Document ctid={} is not visible", ctid);
                    None
                }
            });
            
            result
        }
    }
}
