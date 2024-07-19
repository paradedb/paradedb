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

use anyhow::Result;
use once_cell::sync::Lazy;
use pgrx::{register_xact_callback, PgXactCallbackEvent};
use std::{
    collections::HashMap,
    panic::{RefUnwindSafe, UnwindSafe},
    sync::{Arc, Mutex, PoisonError},
};
use thiserror::Error;
use tracing::error;

#[derive(PartialEq, Eq, Hash)]
enum CallbackStatus {
    Scheduled,
    Cancelled,
}

// Storing a tuple of (index_id, CallbackStatus).
// Really, we should be storing the XactCallbackReceipt we get when we register
// the callback, as that would let us properly cancel it if necessary.
//
// However, XactCallbackReceipt is not thread-safe.
//
// As a workaround, we'll mutate this boolean and check it inside the callback,
// only performing the requested action if the callback has not been cancelled.
type TransactionCallbackCache = Lazy<Arc<Mutex<HashMap<String, CallbackStatus>>>>;

static TRANSACTION_CALL_ONCE_ON_PRECOMMIT_CACHE: TransactionCallbackCache =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

static TRANSACTION_CALL_ONCE_ON_COMMIT_CACHE: TransactionCallbackCache =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

static TRANSACTION_CALL_ONCE_ON_ABORT_CACHE: TransactionCallbackCache =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

pub struct Transaction {}

impl Transaction {
    pub fn needs_commit(id: &str) -> Result<bool, TransactionError> {
        let cache = TRANSACTION_CALL_ONCE_ON_PRECOMMIT_CACHE.lock()?;
        match cache.get(id) {
            Some(CallbackStatus::Scheduled) => Ok(true),
            _ => Ok(false),
        }
    }

    pub fn clear_commit_abort_caches(id: &str) -> Result<(), TransactionError> {
        TRANSACTION_CALL_ONCE_ON_PRECOMMIT_CACHE
            .clone()
            .lock()
            .expect("could not acquire lock in register transaction precommit callback")
            .remove(id);
        TRANSACTION_CALL_ONCE_ON_ABORT_CACHE
            .clone()
            .lock()?
            .remove(id);
        TRANSACTION_CALL_ONCE_ON_COMMIT_CACHE
            .clone()
            .lock()?
            .remove(id);
        Ok(())
    }

    pub fn call_once_on_precommit<F>(id: String, callback: F) -> Result<(), TransactionError>
    where
        F: FnOnce() + Send + UnwindSafe + RefUnwindSafe + 'static,
    {
        // Clone the cache here for use inside the closure.
        let mut cache = TRANSACTION_CALL_ONCE_ON_PRECOMMIT_CACHE.lock()?;

        if !cache.contains_key(&id) {
            // Now using `cache_clone` inside the closure.
            let cloned_id = id.clone();
            register_xact_callback(PgXactCallbackEvent::PreCommit, move || {
                // The precommit cache should be cleared on its own, as it is not
                // mutually exclusive with any other event.
                let callback_status = TRANSACTION_CALL_ONCE_ON_PRECOMMIT_CACHE
                    .clone()
                    .lock()
                    .expect("could not acquire lock in register transaction precommit callback")
                    .remove(&cloned_id);

                // If the callback was explicitly cancelled, we will skip calling it.
                // Otherwise, call it.
                match callback_status {
                    Some(CallbackStatus::Cancelled) => {}
                    _ => {
                        callback();
                    }
                }
            });

            cache.insert(id, CallbackStatus::Scheduled);
        }

        Ok(())
    }

    pub fn call_once_on_commit<F>(id: String, callback: F) -> Result<(), TransactionError>
    where
        F: FnOnce() + Send + UnwindSafe + RefUnwindSafe + 'static,
    {
        let mut cache = TRANSACTION_CALL_ONCE_ON_COMMIT_CACHE.lock()?;
        if !cache.contains_key(&id) {
            // Now using `cache_clone` inside the closure.
            let cloned_id = id.clone();
            register_xact_callback(PgXactCallbackEvent::Commit, move || {
                // Clear the caches so callbacks can be registered on next transaction.
                Self::clear_commit_abort_caches(&cloned_id)
                    .expect("could not acquire lock in register transaction commit callback");

                // Actually call the callback.
                callback();
            });

            cache.insert(id, CallbackStatus::Scheduled);
        }

        Ok(())
    }

    pub fn call_once_on_abort<F>(id: String, callback: F) -> Result<(), TransactionError>
    where
        F: FnOnce() + Send + UnwindSafe + RefUnwindSafe + 'static,
    {
        let mut cache = TRANSACTION_CALL_ONCE_ON_ABORT_CACHE.lock()?;
        if !cache.contains_key(&id) {
            // Now using `cache_clone` inside the closure.
            let cloned_id = id.clone();
            register_xact_callback(PgXactCallbackEvent::Abort, move || {
                // Clear the caches so callbacks can be registered on next transaction.
                Self::clear_commit_abort_caches(&cloned_id)
                    .expect("could not acquire lock in register transaction abort callback");

                // Actually call the callback.
                callback();
            });

            cache.insert(id, CallbackStatus::Scheduled);
        }

        Ok(())
    }

    pub fn cancel_precommit_callback(id: String) -> Result<()> {
        TRANSACTION_CALL_ONCE_ON_PRECOMMIT_CACHE
            .clone()
            .lock()
            .expect("could not acquire lock in register transaction precommit callback")
            .insert(id, CallbackStatus::Cancelled);
        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum TransactionError {
    #[error("could not acquire lock in transaction callback")]
    AcquireLock,
}

impl<T> From<PoisonError<T>> for TransactionError {
    fn from(_: PoisonError<T>) -> Self {
        TransactionError::AcquireLock
    }
}
