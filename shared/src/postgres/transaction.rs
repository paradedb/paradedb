use once_cell::sync::Lazy;
use pgrx::{register_xact_callback, PgXactCallbackEvent};
use std::{
    collections::HashSet,
    panic::{RefUnwindSafe, UnwindSafe},
    sync::{Arc, Mutex, PoisonError},
};
use thiserror::Error;
use tracing::error;

type TransactionCallbackCache = Lazy<Arc<Mutex<HashSet<String>>>>;

static TRANSACTION_CALL_ONCE_ON_PRECOMMIT_CACHE: TransactionCallbackCache =
    Lazy::new(|| Arc::new(Mutex::new(HashSet::new())));

static TRANSACTION_CALL_ONCE_ON_COMMIT_CACHE: TransactionCallbackCache =
    Lazy::new(|| Arc::new(Mutex::new(HashSet::new())));

static TRANSACTION_CALL_ONCE_ON_ABORT_CACHE: TransactionCallbackCache =
    Lazy::new(|| Arc::new(Mutex::new(HashSet::new())));

pub struct Transaction {}

impl Transaction {
    pub fn needs_commit(id: &str) -> Result<bool, TransactionError> {
        let cache = TRANSACTION_CALL_ONCE_ON_PRECOMMIT_CACHE.lock()?;
        Ok(cache.contains(id))
    }

    // The commit and abort events are mutually exclusive with each other.
    // Either one or the other will fire, which means that we must clear both
    // their caches at the same time.
    pub fn clear_commit_abort_caches(id: &str) -> Result<(), TransactionError> {
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

    pub fn call_once_on_precommit<F>(id: &'static str, callback: F) -> Result<(), TransactionError>
    where
        F: FnOnce() + Send + UnwindSafe + RefUnwindSafe + 'static,
    {
        // Clone the cache here for use inside the closure.
        let mut cache = TRANSACTION_CALL_ONCE_ON_PRECOMMIT_CACHE.lock()?;

        if !cache.contains(id) {
            // Now using `cache_clone` inside the closure.
            register_xact_callback(PgXactCallbackEvent::PreCommit, move || {
                // The precommit cache should be cleared on its own, as it is not
                // mutually exclusive with any other event.
                TRANSACTION_CALL_ONCE_ON_PRECOMMIT_CACHE
                    .clone()
                    .lock()
                    .expect("could not acquire lock in register transaction precommit callback")
                    .remove(id);

                // Actually call the callback.
                callback();
            });

            cache.insert(id.into());
        }

        Ok(())
    }

    pub fn call_once_on_commit<F>(id: &'static str, callback: F) -> Result<(), TransactionError>
    where
        F: FnOnce() + Send + UnwindSafe + RefUnwindSafe + 'static,
    {
        let mut cache = TRANSACTION_CALL_ONCE_ON_COMMIT_CACHE.lock()?;
        if !cache.contains(id) {
            // Now using `cache_clone` inside the closure.
            register_xact_callback(PgXactCallbackEvent::Commit, move || {
                // Clear the caches so callbacks can be registered on next transaction.
                Self::clear_commit_abort_caches(id)
                    .expect("could not acquire lock in register transaction commit callback");
                // Actually call the callback.
                callback();
            });

            cache.insert(id.into());
        }

        Ok(())
    }

    pub fn call_once_on_abort<F>(id: &'static str, callback: F) -> Result<(), TransactionError>
    where
        F: FnOnce() + Send + UnwindSafe + RefUnwindSafe + 'static,
    {
        let mut cache = TRANSACTION_CALL_ONCE_ON_ABORT_CACHE.lock()?;
        if !cache.contains(id) {
            // Now using `cache_clone` inside the closure.
            register_xact_callback(PgXactCallbackEvent::Abort, move || {
                // Clear the caches so callbacks can be registered on next transaction.
                Self::clear_commit_abort_caches(id)
                    .expect("could not acquire lock in register transaction abort callback");
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
