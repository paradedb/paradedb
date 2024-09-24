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
use crate::writer::{ClientError, WriterClient, WriterDirectory, WriterRequest};
use parking_lot::Mutex;
use pgrx::{direct_function_call, pg_guard, pg_sys, IntoDatum};
use std::sync::Arc;
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
            // first, indexes in our cache that are pending a DROP need to be dropped
            let pending_drops = SearchIndex::get_cache()
                .values_mut()
                .filter(|index| index.is_pending_drop())
                .map(|index| index.directory.clone())
                .collect::<Vec<_>>();

            for directory in pending_drops {
                finalize_drop(&writer, &directory)
                    .expect("finalizing dropping of pending DROP index should succeed");

                // SAFETY:  We don't have an outstanding reference to the SearchIndex cache here
                // because we collected the pending drop directories into an owned Vec
                SearchIndex::drop_from_cache(&directory)
            }

            // next, we can commit any of the other dirty indexes
            for search_index in SearchIndex::get_cache()
                .values_mut()
                .filter(|index| index.is_dirty())
            {
                // if this doesn't commit, the transaction will ABORT
                search_index
                    .commit(&writer)
                    .expect("SearchIndex should commit successfully");
            }

            // finally, any indexes that are marked as pending create are now created because the
            // transaction is committed
            for search_index in SearchIndex::get_cache()
                .values_mut()
                .filter(|index| index.is_pending_create())
            {
                search_index.is_pending_create = false;
            }
        }

        pg_sys::XactEvent::XACT_EVENT_ABORT => {
            let writer = WriterGlobal::client();

            // first, indexes in our cache that are pending a CREATE need to be dropped
            let pending_creates = SearchIndex::get_cache()
                .values_mut()
                .filter(|index| index.is_pending_create())
                .map(|index| index.directory.clone())
                .collect::<Vec<_>>();

            for directory in pending_creates {
                if let Err(e) = finalize_drop(&writer, &directory) {
                    warn!("could not finalize dropping a pending CREATE index: {e}");
                }

                // SAFETY:  We don't have an outstanding reference to the SearchIndex cache here
                // because we collected the pending create directories into an owned Vec
                SearchIndex::drop_from_cache(&directory)
            }

            // next, we can abort any of the other dirty indexes
            for search_index in SearchIndex::get_cache()
                .values_mut()
                .filter(|index| index.is_dirty())
            {
                if let Err(e) = search_index.abort(&writer) {
                    // the abort didn't work, but we can't raise another panic here as that'll
                    // cause postgres to segfault
                    warn!("could not abort SearchIndex: {e}")
                }
            }

            // finally, any index that was pending drop is no longer to be dropped because the
            // transaction has aborted
            for search_index in SearchIndex::get_cache()
                .values_mut()
                .filter(|index| index.is_pending_drop())
            {
                search_index.is_pending_drop = false;
            }
        }

        _ => {
            // not an event we care about
        }
    }
}

fn finalize_drop<W: WriterClient<WriterRequest> + Send + Sync + 'static>(
    _writer: &Arc<Mutex<W>>,
    directory: &WriterDirectory,
) -> Result<(), ClientError> {
    let writer = unsafe { SearchIndex::get_writer() };

    writer
        .drop_index(directory.clone())
        .expect("must be able to finalize drop");
    writer
        .commit(directory.clone())
        .expect("commit must work in finalize drop");

    Ok(())
}

/// Create a new index writer advisory lock.
/// Advisory locks are global across the entire Postgres cluster,
/// so its important that the key is qualified with the database oid.
pub struct IndexWriterLock {
    database_oid: u32,
    index_oid: u32,
}

impl IndexWriterLock {
    pub fn new(database_oid: u32, index_oid: u32) -> Self {
        Self {
            database_oid,
            index_oid,
        }
    }

    // This method combines the two OIDs into a single 64-bit lock key
    fn key(&self) -> i64 {
        // Shift the database_oid into the high 32 bits, then combine with index_oid
        ((self.database_oid as i64) << 32) | (self.index_oid as i64)
    }

    pub fn acquire<T>(&self, func: impl FnOnce() -> T) -> T {
        let lock_key = self.key(); // Get the lock key

        // Attempt to acquire an exclusive lock on the generated key
        // The lock will automatically be released at the end of the
        // transaction.
        unsafe {
            direct_function_call::<()>(
                pg_sys::pg_advisory_xact_lock_int8,
                &[lock_key.into_datum()],
            );
        }

        // Once the lock is acquired, execute the critical section
        func()
    }
}
