use once_cell::sync::Lazy;
use pgrx::{register_xact_callback, PgXactCallbackEvent};
use std::{
    collections::HashSet,
    panic::{RefUnwindSafe, UnwindSafe},
    sync::{Arc, Mutex, PoisonError},
};
use thiserror::Error;
use tracing::error;

static TRANSACTION_CALL_ONCE_ON_COMMIT_CACHE: Lazy<Arc<Mutex<HashSet<String>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashSet::new())));

static TRANSACTION_CALL_ONCE_ON_ABORT_CACHE: Lazy<Arc<Mutex<HashSet<String>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashSet::new())));

pub struct Transaction {}

impl Transaction {
    pub fn needs_commit(id: &str) -> Result<bool, TransactionError> {
        let cache = TRANSACTION_CALL_ONCE_ON_COMMIT_CACHE.lock()?;
        Ok(cache.contains(id))
    }

    pub fn call_once_on_precommit<F>(id: &str, callback: F) -> Result<(), TransactionError>
    where
        F: FnOnce() + Send + UnwindSafe + RefUnwindSafe + 'static,
    {
        // Clone the cache here for use inside the closure.
        let cache_clone = TRANSACTION_CALL_ONCE_ON_COMMIT_CACHE.clone();

        let mut cache = TRANSACTION_CALL_ONCE_ON_COMMIT_CACHE.lock()?;
        if !cache.contains(id) {
            // Now using `cache_clone` inside the closure.
            register_xact_callback(PgXactCallbackEvent::PreCommit, move || {
                // Clear the cache so callbacks can be registered on next transaction.
                match cache_clone.lock() {
                    Ok(mut cache) => cache.clear(),
                    Err(err) => error!(
                        "could not acquire lock in register transaction commit callback: {err:?}"
                    ),
                }

                // Actually call the callback.
                callback();
            });

            cache.insert(id.into());
        }

        Ok(())
    }

    pub fn call_once_on_abort<F>(id: &str, callback: F) -> Result<(), TransactionError>
    where
        F: FnOnce() + Send + UnwindSafe + RefUnwindSafe + 'static,
    {
        // Clone the cache here for use inside the closure.
        let cache_clone = TRANSACTION_CALL_ONCE_ON_ABORT_CACHE.clone();

        let mut cache = TRANSACTION_CALL_ONCE_ON_ABORT_CACHE.lock()?;
        if !cache.contains(id) {
            // Now using `cache_clone` inside the closure.
            register_xact_callback(PgXactCallbackEvent::Abort, move || {
                // Clear the cache so callbacks can be registered on next transaction.
                match cache_clone.lock() {
                    Ok(mut cache) => cache.clear(),
                    Err(err) => error!(
                        "could not acquire lock in register transaction abort callback: {err:?}"
                    ),
                }

                // Actually call the callback.
                callback();
            });

            cache.insert(id.into());
        }

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
