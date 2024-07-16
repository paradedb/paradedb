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
use shared::postgres::transaction::{Transaction, TransactionError};
use std::{
    ffi::CStr,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::writer::{SearchFs, WriterClient, WriterDirectory, WriterRequest};

const TRANSACTION_CALLBACK_CACHE_ID: &str = "parade_search_index";

/// We use this global variable to cache any values that can be re-used
/// after initialization.
static SEARCH_ENV: Lazy<SearchEnv> = Lazy::new(|| SearchEnv {
    postgres_data_dir: Mutex::new(None),
    postgres_database_oid: Mutex::new(None),
});

struct SearchEnv {
    postgres_data_dir: Mutex<Option<PathBuf>>,
    postgres_database_oid: Mutex<Option<u32>>,
}

pub fn postgres_data_dir_path() -> PathBuf {
    SEARCH_ENV
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

pub fn postgres_database_oid() -> u32 {
    *SEARCH_ENV
        .postgres_database_oid
        .lock()
        .expect("Failed to lock mutex")
        .get_or_insert_with(|| unsafe { pgrx::pg_sys::MyDatabaseId.as_u32() })
}

pub fn register_commit_callback<W: WriterClient<WriterRequest> + Send + Sync + 'static>(
    writer: &Arc<Mutex<W>>,
    directory: WriterDirectory,
) -> Result<(), TransactionError> {
    let writer_client = writer.clone();
    let commit_directory = directory.clone();
    Transaction::call_once_on_precommit(TRANSACTION_CALLBACK_CACHE_ID, move || {
        let mut error: Option<Box<dyn std::error::Error>> = None;
        {
            // This lock must happen in an enclosing block so it is dropped and
            // release before a possible panic.
            match writer_client.lock() {
                Err(err) => {
                    // This panic is fine, because the lock is broken anyways.
                    panic!("could not lock client in commit callback: {err}");
                }
                Ok(mut client) => {
                    if let Err(err) = client.request(WriterRequest::Commit {
                        directory: commit_directory.clone(),
                    }) {
                        error = Some(Box::new(err));
                    }
                }
            }
        }

        if let Some(err) = error {
            panic!("error sending commit request in callback: {err}")
        }
    })?;

    let writer_client = writer.clone();
    let abort_directory = directory.clone();
    Transaction::call_once_on_abort(TRANSACTION_CALLBACK_CACHE_ID, move || {
        let mut error: Option<Box<dyn std::error::Error>> = None;
        {
            // This lock must happen in an enclosing block so it is dropped and
            // release before a possible panic.
            match writer_client.lock() {
                Err(err) => {
                    // This panic is fine, because the lock is broken anyways.
                    panic!("could not lock client in abort callback: {err}");
                }
                Ok(mut client) => {
                    if let Err(err) = client.request(WriterRequest::Abort {
                        directory: abort_directory,
                    }) {
                        error = Some(Box::new(err));
                    }
                }
            }
        }

        if let Some(err) = error {
            panic!("error sending abort request in callback: {err}")
        }
    })?;

    Ok(())
}

pub fn needs_commit() -> bool {
    Transaction::needs_commit(TRANSACTION_CALLBACK_CACHE_ID)
        .expect("error performing commit check in transaction cache")
}

pub fn clear_commit_abort_caches() -> Result<(), TransactionError> {
    Transaction::clear_commit_abort_caches(TRANSACTION_CALLBACK_CACHE_ID)
}

pub fn drop_index_on_commit(directory: WriterDirectory) -> Result<(), TransactionError> {
    let directory = directory.clone();
    let index_name = directory.index_name.clone();

    Transaction::call_once_on_commit(&index_name, move || {
        directory
            .remove()
            .expect(&format!("failed to remove directory for {index_name}",))
    })?;

    Ok(())
}
