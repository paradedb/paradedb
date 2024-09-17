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

use once_cell::sync::Lazy;
use parking_lot::Mutex;
use pgrx::{register_xact_callback, PgXactCallbackEvent};
use std::sync::Arc;
use std::{
    collections::HashSet,
    panic::{RefUnwindSafe, UnwindSafe},
};

type TransactionCallbackCache = Lazy<Arc<Mutex<HashSet<u32>>>>;

static TRANSACTION_CALL_ONCE_ON_PRECOMMIT_CACHE: TransactionCallbackCache =
    Lazy::new(|| Arc::new(Mutex::new(HashSet::new())));

static TRANSACTION_CALL_ONCE_ON_COMMIT_CACHE: TransactionCallbackCache =
    Lazy::new(|| Arc::new(Mutex::new(HashSet::new())));

static TRANSACTION_CALL_ONCE_ON_ABORT_CACHE: TransactionCallbackCache =
    Lazy::new(|| Arc::new(Mutex::new(HashSet::new())));

pub struct Transaction {}

impl Transaction {
    pub fn needs_commit(id: u32) -> bool {
        TRANSACTION_CALL_ONCE_ON_PRECOMMIT_CACHE
            .lock()
            .contains(&id)
    }

    pub fn clear_commit_abort_caches(id: u32) {
        TRANSACTION_CALL_ONCE_ON_PRECOMMIT_CACHE
            .clone()
            .lock()
            .remove(&id);
        TRANSACTION_CALL_ONCE_ON_ABORT_CACHE
            .clone()
            .lock()
            .remove(&id);
        TRANSACTION_CALL_ONCE_ON_COMMIT_CACHE
            .clone()
            .lock()
            .remove(&id);
    }

    pub fn call_once_on_precommit<F>(id: u32, callback: F)
    where
        F: FnOnce() + Send + UnwindSafe + RefUnwindSafe + 'static,
    {
        // Clone the cache here for use inside the closure.
        let mut cache = TRANSACTION_CALL_ONCE_ON_PRECOMMIT_CACHE.lock();

        if !cache.contains(&id) {
            // Now using `cache_clone` inside the closure.
            let cloned_id = id;
            register_xact_callback(PgXactCallbackEvent::PreCommit, move || {
                // The precommit cache should be cleared on its own, as it is not
                // mutually exclusive with any other event.
                TRANSACTION_CALL_ONCE_ON_PRECOMMIT_CACHE
                    .clone()
                    .lock()
                    .remove(&cloned_id);

                // Actually call the callback.
                callback();
            });

            cache.insert(id);
        }
    }

    pub fn call_once_on_commit<F>(id: u32, callback: F)
    where
        F: FnOnce() + Send + UnwindSafe + RefUnwindSafe + 'static,
    {
        let mut cache = TRANSACTION_CALL_ONCE_ON_COMMIT_CACHE.lock();
        if !cache.contains(&id) {
            // Now using `cache_clone` inside the closure.
            let cloned_id = id;
            register_xact_callback(PgXactCallbackEvent::Commit, move || {
                // Clear the caches so callbacks can be registered on next transaction.
                Self::clear_commit_abort_caches(cloned_id);

                // Actually call the callback.
                callback();
            });

            cache.insert(id);
        }
    }

    pub fn call_once_on_abort<F>(id: u32, callback: F)
    where
        F: FnOnce() + Send + UnwindSafe + RefUnwindSafe + 'static,
    {
        let mut cache = TRANSACTION_CALL_ONCE_ON_ABORT_CACHE.lock();
        if !cache.contains(&id) {
            // Now using `cache_clone` inside the closure.
            let cloned_id = id;
            register_xact_callback(PgXactCallbackEvent::Abort, move || {
                // Clear the caches so callbacks can be registered on next transaction.
                Self::clear_commit_abort_caches(cloned_id);

                // Actually call the callback.
                callback();
            });

            cache.insert(id);
        }
    }
}
