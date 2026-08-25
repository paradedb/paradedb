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

//! A thin wrapper over a Postgres `bytea` datum `tuplesort`.
//!
//! Records are ordered by `memcmp` of their bytes (the `bytea` `<` operator), so callers that
//! need a semantic order must encode it into the bytes (big-endian integers, or a serialization
//! that is consistent between the sort side and the probe side). The sort is bounded by the
//! caller's budget and spills to temporary files past it, like any other Postgres tuplesort.

use pgrx::pg_sys;
use std::os::raw::c_int;

/// Wraps a `Tuplesortstate` over `bytea` datums: `memcmp` order, caller-provided memory budget.
pub struct Sorter {
    state: *mut pg_sys::Tuplesortstate,
    /// PG15's `tuplesort_getdatum` has no `copy` flag and always hands back a copy palloc'd in
    /// the caller's context, which the caller owns. Held here so [`Self::next_sorted`] can free
    /// the previous record instead of leaking one allocation per record into a context that may
    /// live for the whole sort read (a CREATE INDEX, for the build path).
    #[cfg(feature = "pg15")]
    last_returned: Option<pg_sys::Datum>,
}

impl Sorter {
    /// `budget` is in bytes; the sort spills to temporary files beyond it.
    pub fn new(budget: usize) -> Self {
        unsafe {
            // `<` operator for bytea, so the sort orders by memcmp of the serialized keys.
            let tcache =
                pg_sys::lookup_type_cache(pg_sys::BYTEAOID, pg_sys::TYPECACHE_LT_OPR as c_int);
            let lt_op = (*tcache).lt_opr;
            let state = pg_sys::tuplesort_begin_datum(
                pg_sys::BYTEAOID,
                lt_op,
                pg_sys::InvalidOid, // bytea has no collation
                false,              // nullsFirstFlag (there are no nulls)
                (budget / 1024).max(64) as c_int,
                std::ptr::null_mut(), // not parallel
                0,                    // sortopt: no random access
            );
            Sorter {
                state,
                #[cfg(feature = "pg15")]
                last_returned: None,
            }
        }
    }

    pub fn put(&mut self, bytes: &[u8]) {
        unsafe {
            let datum = bytea_datum(bytes);
            pg_sys::tuplesort_putdatum(self.state, datum, false);
            // `tuplesort_putdatum` copies the value, so free our transient bytea.
            pg_sys::pfree(datum.cast_mut_ptr());
        }
    }

    /// Finish loading and sort. Call once, before the first [`Self::next_sorted`].
    pub fn performsort(&mut self) {
        unsafe { pg_sys::tuplesort_performsort(self.state) }
    }

    /// The next record in sorted order, or `None` once the sort is exhausted. The returned
    /// bytes are only valid until the next call.
    pub fn next_sorted(&mut self) -> Option<&[u8]> {
        unsafe {
            self.free_last_returned();
            let mut val: pg_sys::Datum = pg_sys::Datum::from(0);
            let mut is_null = false;
            let mut abbrev: pg_sys::Datum = pg_sys::Datum::from(0);
            if tuplesort_getdatum_forward(self.state, &mut val, &mut is_null, &mut abbrev) {
                #[cfg(feature = "pg15")]
                {
                    self.last_returned = Some(val);
                }
                Some(datum_bytea(val))
            } else {
                None
            }
        }
    }

    /// Release the sort's memory and temporary files.
    pub fn end(mut self) {
        unsafe {
            self.free_last_returned();
            pg_sys::tuplesort_end(self.state)
        }
    }

    /// Free the copy PG15's `tuplesort_getdatum` handed us for the previous record. A no-op on
    /// PG16+, where `copy: false` leaves ownership with the sort and the datum is simply valid
    /// until the next call.
    #[cfg_attr(not(feature = "pg15"), allow(clippy::unused_self))]
    unsafe fn free_last_returned(&mut self) {
        #[cfg(feature = "pg15")]
        if let Some(previous) = self.last_returned.take() {
            unsafe { pg_sys::pfree(previous.cast_mut_ptr()) };
        }
    }
}

/// Build a transient `bytea` Datum from `bytes` (palloc'd in the current context).
unsafe fn bytea_datum(bytes: &[u8]) -> pg_sys::Datum {
    use pgrx::IntoDatum;
    bytes
        .into_datum()
        .expect("byte slice should convert to a bytea datum")
}

/// View a `bytea` Datum's payload as bytes (valid until the datum is freed/overwritten).
unsafe fn datum_bytea<'a>(datum: pg_sys::Datum) -> &'a [u8] {
    let varlena = datum.cast_mut_ptr::<pg_sys::varlena>();
    pgrx::varlena_to_byte_slice(varlena)
}

/// `tuplesort_getdatum`, always reading forward. On PG16+, `copy: false` borrows the datum from
/// the sort, valid until the next call. PG15 predates the `copy` parameter and always returns a
/// palloc'd copy owned by the caller; [`Sorter`] tracks and frees it.
unsafe fn tuplesort_getdatum_forward(
    state: *mut pg_sys::Tuplesortstate,
    val: *mut pg_sys::Datum,
    is_null: *mut bool,
    abbrev: *mut pg_sys::Datum,
) -> bool {
    #[cfg(feature = "pg15")]
    {
        pg_sys::tuplesort_getdatum(state, true, val, is_null, abbrev)
    }
    #[cfg(not(feature = "pg15"))]
    {
        pg_sys::tuplesort_getdatum(state, true, false, val, is_null, abbrev)
    }
}
