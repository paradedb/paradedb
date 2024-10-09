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

use crate::index::SearchFs;
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
            // first, indexes in our cache that are pending a DROP need to be dropped
            for directory in SearchIndex::pending_drops() {
                directory.remove().unwrap_or_else(|err| {
                    warn!(
                        "unexpected error removing index directory during pre-commit: {:?}; {:?}",
                        directory, err
                    )
                });
            }

            // finally, any indexes that are marked as pending create are now created because the
            // transaction is committed
            SearchIndex::clear_pending_drops();
            SearchIndex::clear_pending_creates();
        }

        pg_sys::XactEvent::XACT_EVENT_ABORT => {
            // first, indexes in our cache that are pending a CREATE need to be dropped
            for directory in SearchIndex::pending_creates() {
                directory.remove().unwrap_or_else(|err| {
                    warn!(
                        "unexpected error removing index directory during abort: {:?}; {:?}",
                        directory, err
                    )
                });
            }

            SearchIndex::clear_pending_drops();
            SearchIndex::clear_pending_creates();
        }

        _ => {
            // not an event we care about
        }
    }
}
