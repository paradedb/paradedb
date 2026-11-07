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

//! The counters behind `pg_stat_user_indexes` for a ParadeDB index. Postgres leaves them to the
//! access method, and its `pgstat_count_index_*` macros have no binding, so the same bumps happen
//! here for every path that reads the index.
//!
//! A parallel scan counts once, like a parallel btree scan on PostgreSQL 18. The leader sets it up
//! before any worker starts and may never read a row itself, so it counts the scan and the workers
//! that share it don't. A process that reads the whole index on its own, as the inner side of a
//! parallel hash join does, counts its own scan. Entries count where they are returned, so the
//! workers' counts add up.

use crate::postgres::rel::PgSearchRelation;
use pgrx::pg_sys;

/// Counts one scan of `indexrel`, as `pgstat_count_index_scan` does.
pub(crate) fn count_index_scan(indexrel: &PgSearchRelation) {
    bump(indexrel, |counts| {
        #[cfg(feature = "pg15")]
        {
            counts.t_numscans += 1;
        }
        #[cfg(not(feature = "pg15"))]
        {
            counts.numscans += 1;
        }
    });
}

/// Counts `n` entries returned by a scan of `indexrel`, as `pgstat_count_index_tuples` does.
pub(crate) fn count_index_tuples(indexrel: &PgSearchRelation, n: i64) {
    bump(indexrel, |counts| {
        #[cfg(feature = "pg15")]
        {
            counts.t_tuples_returned += n;
        }
        #[cfg(not(feature = "pg15"))]
        {
            counts.tuples_returned += n;
        }
    });
}

/// Counts one heap fetch through `indexrel` that found a visible row, as `index_fetch_heap` does
/// with `pgstat_count_heap_fetch`.
pub(crate) fn count_heap_fetch(indexrel: &PgSearchRelation) {
    bump(indexrel, |counts| {
        #[cfg(feature = "pg15")]
        {
            counts.t_tuples_fetched += 1;
        }
        #[cfg(not(feature = "pg15"))]
        {
            counts.tuples_fetched += 1;
        }
    });
}

/// Applies `bump` to the backend's pending counters for `indexrel`, unless it collects no
/// statistics. Mirrors `pgstat_should_count_relation`, which links the relation to its pending
/// entry on the first count.
fn bump(indexrel: &PgSearchRelation, bump: impl FnOnce(&mut pg_sys::PgStat_TableCounts)) {
    let indexrel = indexrel.as_ptr();
    // SAFETY: a `PgSearchRelation` holds the relation open. Its pending entry is backend-local
    // memory that only a flush between statements frees, and no other reference to it is live.
    unsafe {
        if (*indexrel).pgstat_info.is_null() {
            if !(*indexrel).pgstat_enabled {
                return;
            }
            pg_sys::pgstat_assoc_relation(indexrel);
        }
        if let Some(status) = (*indexrel).pgstat_info.as_mut() {
            #[cfg(feature = "pg15")]
            {
                bump(&mut status.t_counts);
            }
            #[cfg(not(feature = "pg15"))]
            {
                bump(&mut status.counts);
            }
        }
    }
}
