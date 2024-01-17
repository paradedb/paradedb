use once_cell::sync::Lazy;
use pgrx::{register_xact_callback, PgXactCallbackEvent};
use std::{
    collections::HashSet,
    ffi::CStr,
    panic::{RefUnwindSafe, UnwindSafe},
    path::PathBuf,
    sync::{Arc, Mutex, PoisonError},
};
use thiserror::Error;

/// We use this global variable to cache any values that can be re-used
/// after initialization.
static PARADE_ENV: Lazy<ParadeEnv> = Lazy::new(|| ParadeEnv {
    postgres_data_dir: Mutex::new(None),
});

struct ParadeEnv {
    postgres_data_dir: Mutex<Option<PathBuf>>,
}

pub fn postgres_data_dir_path() -> PathBuf {
    PARADE_ENV
        .postgres_data_dir
        .lock()
        .expect("Failed to lock mutex")
        .get_or_insert_with(|| unsafe {
            let data_dir = CStr::from_ptr(pgrx::pg_sys::DataDir)
                .to_string_lossy()
                .into_owned();
            PathBuf::from(data_dir)
        })
        .clone()
}

pub fn paradedb_data_dir_path() -> PathBuf {
    postgres_data_dir_path().join("paradedb")
}

pub fn paradedb_transfer_pipe_path() -> PathBuf {
    paradedb_data_dir_path().join("writer_transfer")
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
                    Err(err) => pgrx::log!(
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
                    Err(err) => pgrx::log!(
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
