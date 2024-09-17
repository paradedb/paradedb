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

use crate::writer::{WriterClient, WriterDirectory, WriterRequest};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use shared::postgres::transaction::Transaction;
use std::panic::AssertUnwindSafe;
use std::{ffi::CStr, path::PathBuf, sync::Arc};
use tracing::warn;

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
) {
    let writer_client = Clone::clone(writer);
    let commit_directory = directory.clone();
    Transaction::call_once_on_precommit(
        directory.clone().index_oid,
        AssertUnwindSafe(move || {
            // This lock must happen in an enclosing block so it is dropped and
            // release before a possible panic.
            if let Err(err) = writer_client.lock().request(WriterRequest::Commit {
                directory: commit_directory,
            }) {
                panic!("error with request to writer in commit callback: {err}");
            }
        }),
    );

    let writer_client = Clone::clone(writer);
    let abort_directory = directory.clone();
    Transaction::call_once_on_abort(
        directory.clone().index_oid,
        AssertUnwindSafe(move || {
            {
                // This lock must happen in an enclosing block so it is dropped and
                // release before a possible panic.
                if let Err(err) = writer_client.lock().request(WriterRequest::Abort {
                    directory: abort_directory,
                }) {
                    // we're already in a transaction ABORT state and cannot panic again otherwise
                    // Postgres will PANIC, which crashes the whole cluster

                    warn!("error with request to writer in abort callback: {err}");
                }
            }
        }),
    );
}

pub fn needs_commit(index_oid: u32) -> bool {
    Transaction::needs_commit(index_oid)
}
