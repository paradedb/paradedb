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

//! Predicate locking for reads that go straight to a bm25 index.
//!
//! The bm25 access method sets no `ampredlocks`, so a plain index scan over it gets a
//! relation-level SIREAD lock from `index_beginscan`, and a sequential scan gets one from
//! `heap_beginscan`. Our custom scans call neither, so they take the lock themselves.

use pgrx::pg_sys;

use crate::postgres::rel::PgSearchRelation;

/// Takes the SIREAD lock for a read of `heaprel` through its bm25 index.
///
/// SSI can only see a read/write dependency if the read leaves a lock behind. Without one, two
/// `SERIALIZABLE` transactions can each count the rows that match a search, then each insert a
/// row the other would have counted, and both commit.
///
/// The lock goes on the heap, not on the index. An index-level lock catches an `INSERT` and a
/// non-HOT `UPDATE`, because `index_insert` conflict-checks the index for an access method
/// without `ampredlocks`. It misses a `DELETE`, whose index entries `ambulkdelete` drops lazily
/// without any conflict check, and it misses a HOT `UPDATE`, which writes no index entry at all.
/// Every one of those writes conflict-checks the heap.
///
/// Relation granularity, which is coarser than what the scan would otherwise leave behind. A
/// search predicate covers no key range, and the columnar and aggregate paths answer from the
/// index without reading the rows they matched, so they have no tuple or page to lock. The
/// heap-visiting paths do leave per-tuple locks (`heap_hot_search_buffer` calls
/// `PredicateLockTID`), and this lock replaces them, so a write to a row the search never
/// matched now conflicts as well. Trading that for one lock per scan is the coarse-first step;
/// the finer shape is an index-relation lock plus a heap page lock wherever the all-visible
/// check skips the heap, the way `nodeIndexonlyscan.c` does it.
///
/// `snapshot` is the one the read itself runs under, and must stay valid for the call.
pub fn predicate_lock_read(heaprel: &PgSearchRelation, snapshot: pg_sys::Snapshot) {
    debug_assert!(!snapshot.is_null(), "a read needs a snapshot to lock under");
    if !serializable() || snapshot.is_null() {
        return;
    }
    unsafe { pg_sys::PredicateLockRelation(heaprel.as_ptr(), snapshot) }
}

/// [`predicate_lock_read`] for a caller that has the heap's oid but no open relation.
pub fn predicate_lock_read_oid(heaprelid: pg_sys::Oid, snapshot: pg_sys::Snapshot) {
    // Opening the relation costs a relcache round trip, which every non-serializable query
    // would otherwise pay for a lock that `PredicateLockRelation` goes on to skip.
    if !serializable() {
        return;
    }
    predicate_lock_read(&PgSearchRelation::open(heaprelid), snapshot)
}

fn serializable() -> bool {
    unsafe { pg_sys::XactIsoLevel as u32 == pg_sys::XACT_SERIALIZABLE }
}
