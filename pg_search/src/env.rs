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

use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use shared::postgres::transaction::{Transaction, TransactionError};
use std::{
    ffi::CStr,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::writer::{WriterClient, WriterDirectory, WriterRequest};

/// We use this global variable to cache any values that can be re-used
/// after initialization.
static SEARCH_ENV: Lazy<SearchEnv> = Lazy::new(|| SearchEnv {
    postgres_data_dir: Mutex::new(None),
});

struct SearchEnv {
    postgres_data_dir: Mutex<Option<PathBuf>>,
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

pub fn register_commit_callback<W: WriterClient<WriterRequest> + Send + Sync + 'static>(
    writer: &Arc<Mutex<W>>,
    directory: WriterDirectory,
) -> Result<(), TransactionError> {
    let writer_client = writer.clone();
    let commit_directory = directory.clone();
    Transaction::call_once_on_precommit(directory.clone().index_oid, move || {
        let mut error: Option<anyhow::Error> = None;
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
                        error = Some(anyhow!(
                            "error with request to writer in commit callback: {err}"
                        ));
                    }
                }
            }
        }

        if let Some(err) = error {
            panic!("{err}")
        }
    })?;

    let writer_client = writer.clone();
    let abort_directory = directory.clone();
    Transaction::call_once_on_abort(directory.clone().index_oid, move || {
        let mut error: Option<anyhow::Error> = None;
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
                        error = Some(anyhow!(
                            "error with request to writer in abort callback: {err}"
                        ));
                    }
                }
            }
        }

        if let Some(err) = error {
            panic!("{err}")
        }
    })?;

    Ok(())
}

pub fn needs_commit(index_oid: u32) -> bool {
    Transaction::needs_commit(index_oid)
        .expect("error performing commit check in transaction cache")
}
