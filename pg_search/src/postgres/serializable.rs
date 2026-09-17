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
//! A plain index scan gets its SIREAD lock from `index_beginscan`, and a sequential scan from
//! `heap_beginscan`. Our custom scans call neither, so they have to take the lock themselves.

use pgrx::pg_sys;

use crate::postgres::rel::PgSearchRelation;

/// Takes the SIREAD lock for a read of `heaprel` through its bm25 index.
///
/// SSI can only see a read/write dependency if the read leaves a lock behind. Without one, two
/// `SERIALIZABLE` transactions can each count the rows that match a search, then each insert a
/// row the other would have counted, and both commit.
///
/// The lock goes on the heap rather than on the index because every write to the table checks
/// the heap's target, while an index-level lock would miss `DELETE`: index entries are dropped
/// lazily by `ambulkdelete`, which never conflict-checks.
///
/// Relation granularity, for the reason `heap_beginscan` gives for a sequential scan: there is
/// nothing finer to lock. A search predicate covers no key range, and a scan answered from the
/// index alone (a `count(*)` reads no heap page at all) has no tuple or page to lock either.
pub fn predicate_lock_read(heaprel: &PgSearchRelation, snapshot: pg_sys::Snapshot) {
    if snapshot.is_null() {
        return;
    }
    unsafe { pg_sys::PredicateLockRelation(heaprel.as_ptr(), snapshot) }
}

/// [`predicate_lock_read`] for a caller that has the heap's oid but no open relation.
pub fn predicate_lock_read_oid(heaprelid: pg_sys::Oid, snapshot: pg_sys::Snapshot) {
    if snapshot.is_null() || heaprelid == pg_sys::Oid::INVALID {
        return;
    }
    predicate_lock_read(&PgSearchRelation::open(heaprelid), snapshot)
}
