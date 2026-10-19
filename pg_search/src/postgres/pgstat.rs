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

use pgrx::pg_sys;

/// Counts one scan of `indexrel`, as `pgstat_count_index_scan` does.
///
/// # Safety
///
/// `indexrel` must point at an open relation.
pub(crate) unsafe fn count_index_scan(indexrel: pg_sys::Relation) {
    unsafe {
        if let Some(counts) = pending_counts(indexrel) {
            #[cfg(feature = "pg15")]
            {
                counts.t_numscans += 1;
            }
            #[cfg(not(feature = "pg15"))]
            {
                counts.numscans += 1;
            }
        }
    }
}

/// Counts `n` entries returned by a scan of `indexrel`, as `pgstat_count_index_tuples` does.
///
/// # Safety
///
/// `indexrel` must point at an open relation.
pub(crate) unsafe fn count_index_tuples(indexrel: pg_sys::Relation, n: i64) {
    unsafe {
        if let Some(counts) = pending_counts(indexrel) {
            #[cfg(feature = "pg15")]
            {
                counts.t_tuples_returned += n;
            }
            #[cfg(not(feature = "pg15"))]
            {
                counts.tuples_returned += n;
            }
        }
    }
}

/// The backend's pending counters for `indexrel`, or `None` when it collects no statistics.
/// Mirrors `pgstat_should_count_relation`, which links the relation to its pending entry on the
/// first count.
unsafe fn pending_counts<'a>(
    indexrel: pg_sys::Relation,
) -> Option<&'a mut pg_sys::PgStat_TableCounts> {
    unsafe {
        if (*indexrel).pgstat_info.is_null() {
            if !(*indexrel).pgstat_enabled {
                return None;
            }
            pg_sys::pgstat_assoc_relation(indexrel);
        }
        let status = (*indexrel).pgstat_info.as_mut()?;
        #[cfg(feature = "pg15")]
        {
            Some(&mut status.t_counts)
        }
        #[cfg(not(feature = "pg15"))]
        {
            Some(&mut status.counts)
        }
    }
}
