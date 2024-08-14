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
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use tantivy::{query::QueryParser, Executor, Index, Searcher};
use tantivy::{schema::Value, IndexReader, IndexWriter, TantivyDocument, TantivyError};
use thiserror::Error;
use tokenizers::{create_normalizer_manager, create_tokenizer_manager};
use tracing::{error, info};

use super::state::SearchState;
use crate::schema::{
    SearchConfig, SearchDocument, SearchFieldConfig, SearchFieldName, SearchFieldType,
    SearchIndexSchema, SearchIndexSchemaError,
};
use crate::writer::{
    self, SearchDirectoryError, SearchFs, TantivyDirPath, WriterClient, WriterDirectory,
    WriterRequest, WriterTransferPipeFilePath,
};

// Must be at least 15,000,000 or Tantivy will panic.
pub const INDEX_TANTIVY_MEMORY_BUDGET_DEFAULT: usize = 500_000_000;
pub const INDEX_TANTIVY_MEMORY_BUDGET_MIN: usize = 15_000_000;
const CACHE_NUM_BLOCKS: usize = 10;

/// PostgreSQL operates in a process-per-client model, meaning every client connection
/// to PostgreSQL results in a new backend process being spawned on the PostgreSQL server.
///
/// `SEARCH_INDEX_MEMORY` is designed to act as a cache that persists for the lifetime of a
/// single backend process. When a client connects to PostgreSQL and triggers the extension's
/// functionality, this cache is initialized the first time it's accessed in that specific process.
///
/// In scenarios where connection pooling is used, such as by web servers maintaining
/// a pool of connections to PostgreSQL, the connections (and the associated backend processes)
/// are typically long-lived. While this cache initialization might happen once per connection,
/// it does not happen per query, leading to performance benefits for expensive operations.
///
/// It's also crucial to remember that this cache is NOT shared across different backend
/// processes. Each PostgreSQL backend process will have its own separate instance of
/// this cache, tied to its own lifecycle.
pub static mut SEARCH_INDEX_MEMORY: Lazy<HashMap<WriterDirectory, SearchIndex>> =
    Lazy::new(HashMap::new);

pub static mut SEARCH_EXECUTOR: Lazy<Executor> = Lazy::new(|| {
    let num_threads = num_cpus::get();
    Executor::multi_thread(num_threads, "prefix-here").expect("could not create search executor")
});

#[derive(Serialize)]
pub struct SearchIndex {
    pub schema: SearchIndexSchema,
    pub directory: WriterDirectory,
    #[serde(skip_serializing)]
    pub reader: IndexReader,
    #[serde(skip_serializing)]
    pub underlying_index: Index,
    pub uuid: String,
    pub memory_budget: usize,
}

impl SearchIndex {
    pub fn create_index<W: WriterClient<WriterRequest>>(
        writer: &Arc<Mutex<W>>,
        directory: WriterDirectory,
        fields: Vec<(SearchFieldName, SearchFieldConfig, SearchFieldType)>,
        uuid: String,
        key_field_index: usize,
        memory_budget: usize,
    ) -> Result<&'static mut Self, SearchIndexError> {
        writer.lock()?.request(WriterRequest::CreateIndex {
            directory: directory.clone(),
            fields,
            uuid: uuid.clone(),
            key_field_index,
            memory_budget,
        })?;

        // As the new index instance was created in a background process, we need
        // to load it from disk to use it.
        let new_self_ref = Self::from_disk(&directory)
            .unwrap_or_else(|err| panic!("error loading index from directory: {err}"));

        Ok(new_self_ref)
    }

    #[allow(static_mut_refs)]
    pub fn executor() -> &'static Executor {
        unsafe { &SEARCH_EXECUTOR }
    }

    pub fn setup_tokenizers(underlying_index: &mut Index, schema: &SearchIndexSchema) {
        let tokenizers = schema
            .fields
            .iter()
            .filter_map(|field| {
                let field_config = &field.config;
                let field_name: &str = field.name.as_ref();
                info!(field_name, "attempting to create tokenizer");
                match field_config {
                    SearchFieldConfig::Text { tokenizer, .. }
                    | SearchFieldConfig::Json { tokenizer, .. } => Some(tokenizer),
                    _ => None,
                }
            })
            .collect();

        underlying_index.set_tokenizers(create_tokenizer_manager(tokenizers));
        underlying_index.set_fast_field_tokenizers(create_normalizer_manager());
    }

    pub fn reader(index: &Index) -> Result<IndexReader, TantivyError> {
        index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::Manual)
            .try_into()
    }

    unsafe fn into_cache(self) {
        SEARCH_INDEX_MEMORY.insert(self.directory.clone(), self);
    }

    pub fn from_disk<'a>(directory: &WriterDirectory) -> Result<&'a mut Self, SearchIndexError> {
        let mut new_self: Self = directory.load_index()?;
        let uuid = new_self.uuid.clone();

        // In the case of a physical replication of the database, the absolute path that is stored
        // in the serialized WriterDirectory might refer to the source database's file system.
        // We should overwrite it with the dynamically generated one that's been passed as an
        // argument here.
        new_self.directory = directory.clone();

        // Since we've re-fetched the index, save it to the cache.
        unsafe {
            new_self.into_cache();
        }

        Self::from_cache(directory, &uuid)
    }

    pub fn from_cache<'a>(
        directory: &WriterDirectory,
        uuid: &str,
    ) -> Result<&'a mut Self, SearchIndexError> {
        unsafe {
            if let Some(new_self) = SEARCH_INDEX_MEMORY.get_mut(directory) {
                let cached_uuid = &new_self.uuid;
                if cached_uuid == uuid {
                    return Ok(new_self);
                }
            }
        }

        Self::from_disk(directory)
    }

    unsafe fn drop_from_cache(directory: &WriterDirectory) -> Result<()> {
        SEARCH_INDEX_MEMORY.remove(directory);
        Ok(())
    }

    pub fn query_parser(&self) -> QueryParser {
        QueryParser::for_index(
            &self.underlying_index,
            self.schema
                .fields
                .iter()
                .map(|search_field| search_field.id.0)
                .collect::<Vec<_>>(),
        )
    }

    pub fn search_state<W: WriterClient<WriterRequest>>(
        &self,
        writer: &Arc<Mutex<W>>,
        config: &SearchConfig,
        needs_commit: bool,
    ) -> Result<SearchState, SearchIndexError> {
        // Commit any inserts or deletes that have occurred during this transaction.
        if needs_commit {
            writer.lock()?.request(WriterRequest::Commit {
                directory: self.directory.clone(),
            })?
        }

        // Prepare to perform a search.
        // In case this is happening in the same transaction as an index build or an insert,
        // we want to commit first so that the most recent results appear.

        self.reader.reload()?;
        Ok(SearchState::new(self, config))
    }

    pub fn searcher(&self) -> Searcher {
        self.reader.searcher()
    }

    /// Retrieve an owned writer for a given index. This is a static method, as
    /// we expect to be called from the writer process. The return type needs to
    /// be entirely owned by the new process, with no references.
    pub fn writer(directory: &WriterDirectory) -> Result<IndexWriter, SearchIndexError> {
        let search_index: Self = directory.load_index()?;

        let memory_budget = if search_index.memory_budget < 15_000_000 {
            INDEX_TANTIVY_MEMORY_BUDGET_DEFAULT
        } else {
            search_index.memory_budget
        };

        let index_writer = search_index.underlying_index.writer(memory_budget)?;
        Ok(index_writer)
    }

    pub fn insert<W: WriterClient<WriterRequest> + Send + Sync + 'static>(
        &mut self,
        writer: &Arc<Mutex<W>>,
        document: SearchDocument,
    ) -> Result<(), SearchIndexError> {
        // Send the insert requests to the writer server.
        let request = WriterRequest::Insert {
            directory: self.directory.clone(),
            document: document.clone(),
        };

        let WriterTransferPipeFilePath(pipe_path) =
            self.directory.writer_transfer_pipe_path(true)?;

        writer.lock()?.transfer(pipe_path, request)?;

        Ok(())
    }

    pub fn delete<W: WriterClient<WriterRequest> + Send + Sync + 'static>(
        &mut self,
        writer: &Arc<Mutex<W>>,
        should_delete: impl Fn(u64) -> bool,
    ) -> Result<(u32, u32), SearchIndexError> {
        let mut deleted: u32 = 0;
        let mut not_deleted: u32 = 0;
        let mut ctids_to_delete: Vec<u64> = vec![];

        for segment_reader in self.searcher().segment_readers() {
            let store_reader = segment_reader
                .get_store_reader(CACHE_NUM_BLOCKS)
                .expect("Failed to get store reader");

            for (delete, ctid) in (0..segment_reader.num_docs())
                .filter_map(|id| store_reader.get(id).ok())
                .filter_map(|doc: TantivyDocument| {
                    doc.get_first(self.schema.ctid_field().id.0).cloned()
                })
                .filter_map(|value| (&value).as_u64())
                .map(|ctid_val| (should_delete(ctid_val), ctid_val))
            {
                if delete {
                    ctids_to_delete.push(ctid);
                    deleted += 1
                } else {
                    not_deleted += 1
                }
            }
        }

        let request = WriterRequest::Delete {
            field: self.schema.ctid_field().id.0,
            ctids: ctids_to_delete,
            directory: self.directory.clone(),
        };
        writer.lock()?.request(request)?;

        Ok((deleted, not_deleted))
    }

    pub fn drop_index<W: WriterClient<WriterRequest>>(
        writer: &Arc<Mutex<W>>,
        index_oid: u32,
    ) -> Result<(), SearchIndexError> {
        let directory = WriterDirectory::from_index_oid(index_oid);
        let request = WriterRequest::DropIndex {
            directory: directory.clone(),
        };

        // Request the background writer process to physically drop the index.
        writer.lock()?.request(request)?;

        // Drop the index from this connection's cache.
        unsafe { Self::drop_from_cache(&directory).map_err(SearchIndexError::from)? }

        Ok(())
    }

    pub fn vacuum<W: WriterClient<WriterRequest>>(
        &mut self,
        writer: &Arc<Mutex<W>>,
    ) -> Result<(), SearchIndexError> {
        let request = WriterRequest::Vacuum {
            directory: self.directory.clone(),
        };
        writer.lock()?.request(request)?;
        Ok(())
    }
}

impl<'de> Deserialize<'de> for SearchIndex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // A helper struct that lets us use the default serialization for most fields.
        #[derive(Deserialize)]
        struct SearchIndexHelper {
            schema: SearchIndexSchema,
            directory: WriterDirectory,
            // An index created in an older version of pg_search may not have serialized a uuid
            // to disk. Just use an empty string for backwards compatibility.
            #[serde(default)]
            uuid: String,
            #[serde(default = "default_memory_budget")]
            memory_budget: usize,
        }

        let SearchIndexHelper {
            schema,
            directory,
            uuid,
            memory_budget,
        } = SearchIndexHelper::deserialize(deserializer)?;

        let TantivyDirPath(tantivy_dir_path) = directory.tantivy_dir_path(true).unwrap();

        let mut underlying_index =
            Index::open_in_dir(tantivy_dir_path).expect("failed to open index");

        Self::setup_tokenizers(&mut underlying_index, &schema);

        let reader = Self::reader(&underlying_index)
            .unwrap_or_else(|_| panic!("failed to create index reader while retrieving index"));

        let memory_budget = if memory_budget < 15_000_000 {
            INDEX_TANTIVY_MEMORY_BUDGET_DEFAULT
        } else {
            memory_budget
        };

        Ok(SearchIndex {
            reader,
            underlying_index,
            directory,
            schema,
            uuid,
            memory_budget,
        })
    }
}

#[derive(Error, Debug)]
pub enum SearchIndexError {
    #[error(transparent)]
    SchemaError(#[from] SearchIndexSchemaError),

    #[error(transparent)]
    WriterClientError(#[from] writer::ClientError),

    #[error(transparent)]
    WriterIndexError(#[from] writer::IndexError),

    #[error(transparent)]
    TantivyError(#[from] tantivy::error::TantivyError),

    #[error(transparent)]
    TransactionError(#[from] shared::postgres::transaction::TransactionError),

    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),

    #[error(transparent)]
    WriterDirectoryError(#[from] SearchDirectoryError),

    #[error("mutex lock on writer client failed: {0}")]
    WriterClientRace(String),

    #[error(transparent)]
    AnyhowError(#[from] anyhow::Error),
}

impl<T> From<PoisonError<T>> for SearchIndexError {
    fn from(err: PoisonError<T>) -> Self {
        SearchIndexError::WriterClientRace(format!("{}", err))
    }
}

fn default_memory_budget() -> usize {
    INDEX_TANTIVY_MEMORY_BUDGET_DEFAULT
}

#[cfg(test)]
mod tests {
    use super::SearchIndex;
    use crate::{
        fixtures::{mock_dir, MockWriterDirectory},
        writer::SearchFs,
    };
    use rstest::*;

    /// Expected to panic because no index has been created in the directory.
    #[rstest]
    #[should_panic]
    fn test_index_from_disk_panics(mock_dir: MockWriterDirectory) {
        mock_dir.load_index::<SearchIndex>().unwrap();
    }
}
