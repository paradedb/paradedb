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

<<<<<<< Updated upstream
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::{ffi::CStr, path::PathBuf};
=======
use crate::index::SearchIndex;
use once_cell::sync::Lazy;
use pgrx::{register_xact_callback, PgXactCallbackEvent};
use shared::postgres::transaction::Transaction;
use std::{ffi::CStr, path::PathBuf, sync::Mutex};
use tracing::warn;
>>>>>>> Stashed changes

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
<<<<<<< Updated upstream
=======

pub fn register_commit_callback(search_index: &SearchIndex) {
    let commit_directory = search_index.directory.clone();
    let commit_uuid = search_index.uuid.clone();
    register_xact_callback(PgXactCallbackEvent::PreCommit, move || {
        let search_index = SearchIndex::from_cache(&commit_directory, &commit_uuid)
            .expect("SearchIndex should be in cache");
        search_index
            .commit()
            .expect("SearchIndex commit should succeed")
    });

    let abort_directory = search_index.directory.clone();
    let abort_uuid = search_index.uuid.clone();
    register_xact_callback(
        PgXactCallbackEvent::Abort,
        move || match SearchIndex::from_cache(&abort_directory, &abort_uuid) {
            Err(e) => warn!(
                "SearchIndex with uuid {abort_uuid} and oid {} not found in cache: {e}",
                abort_directory.index_oid
            ),
            Ok(search_index) => match search_index.abort() {
                Err(e) => warn!("unable to abort SearchIndex transaction: {e}"),
                Ok(_) => {}
            },
        },
    );
}

pub fn needs_commit(index_oid: u32) -> bool {
    Transaction::needs_commit(index_oid)
        .expect("error performing commit check in transaction cache")
}
>>>>>>> Stashed changes
