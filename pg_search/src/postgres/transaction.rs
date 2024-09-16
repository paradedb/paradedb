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

use crate::globals::WriterGlobal;
use crate::index::SearchIndex;
use pgrx::{pg_guard, pg_sys};
use tracing::warn;

/// Initialize a transaction callback that pg_search uses to commit or abort pending tantivy
/// index changes.
///
/// This callback must be initialized **once per backend connection**, rather than once when
/// `pg_search.so` is loaded.  As such calling this function from `_PG_init()` does not work.
#[inline(always)]
pub fn register_callback() {
    static mut INITIALIZED: bool = false;
    unsafe {
        // SAFETY:  Postgres is single-threaded and we're the only ones that can see `INITIALIZED`.
        // Additionally, the call to RegisterXactCallback is unsafe simply b/c of FFI
        if !INITIALIZED {
            // register a XactCallback, once, for this backend connection where we'll decide to
            // commit or abort pending index changes
            pg_sys::RegisterXactCallback(Some(pg_search_xact_callback), std::ptr::null_mut());
            INITIALIZED = true;
        }
    }
}

#[pg_guard]
unsafe extern "C" fn pg_search_xact_callback(
    event: pg_sys::XactEvent::Type,
    _arg: *mut std::ffi::c_void,
) {
    match event {
        pg_sys::XactEvent::XACT_EVENT_PRE_COMMIT => {
            let writer = WriterGlobal::client();
            let cache = unsafe {
                // SAFETY:  Postgres being single-threaded means we're the only place
                // trying to access the cache
                SearchIndex::get_cache()
            };
            for search_index in cache.values_mut().filter(|index| index.is_dirty()) {
                // if this doesn't commit, the transaction will ABORT
                search_index
                    .commit(&writer)
                    .expect("SearchIndex should commit successfully");
            }
        }

        pg_sys::XactEvent::XACT_EVENT_ABORT => {
            let writer = WriterGlobal::client();
            let cache = unsafe {
                // SAFETY:  Postgres being single-threaded means we're the only place
                // trying to access the cache
                SearchIndex::get_cache()
            };
            for search_index in cache.values_mut().filter(|index| index.is_dirty()) {
                if let Err(e) = search_index.abort(&writer) {
                    // the abort didn't work, but we can't raise another panic here as that'll
                    // cause postgres to segfault
                    warn!("could not abort SearchIndex: {}", e)
                }
            }
        }

        _ => {
            // not an event we care about
        }
    }
}
