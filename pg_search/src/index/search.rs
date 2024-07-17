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
use pgrx::{
    pg_sys::{Datum, ItemPointerData},
    PgTupleDesc,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use tantivy::{query::QueryParser, Executor, Index, Searcher};
use tantivy::{schema::Value, IndexReader, IndexWriter, TantivyDocument, TantivyError};
use thiserror::Error;
use tokenizers::{create_normalizer_manager, create_tokenizer_manager};
use tracing::{error, info};

use super::state::SearchState;
use crate::postgres::utils::row_to_search_document;
use crate::schema::{
    SearchConfig, SearchDocument, SearchFieldConfig, SearchFieldName, SearchFieldType,
    SearchIndexSchema, SearchIndexSchemaError,
};
use crate::writer::{
    self, SearchDirectoryError, SearchFs, TantivyDirPath, WriterClient, WriterDirectory,
    WriterRequest, WriterTransferPipeFilePath,
};

// Must be at least 15,000,000 or Tantivy will panic.
const INDEX_TANTIVY_MEMORY_BUDGET: usize = 500_000_000;
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
}

impl SearchIndex {
    pub fn create_index<W: WriterClient<WriterRequest>>(
        writer: &Arc<Mutex<W>>,
        directory: WriterDirectory,
        fields: Vec<(SearchFieldName, SearchFieldConfig, SearchFieldType)>,
    ) -> Result<&'static mut Self, SearchIndexError> {
        writer.lock()?.request(WriterRequest::CreateIndex {
            directory: directory.clone(),
            fields,
        })?;

        // As the new index instance was created in a background process, we need
        // to load it from disk to use it.
        let new_self_ref = Self::from_cache(&directory)
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

    pub fn from_cache<'a>(directory: &WriterDirectory) -> Result<&'a mut Self, SearchIndexError> {
        unsafe {
            if let Some(new_self) = SEARCH_INDEX_MEMORY.get_mut(directory) {
                return Ok(new_self);
            }
        }

        let new_self: Self = directory.load_index()?;

        // Since we've re-fetched the index, save it to the cache.
        unsafe {
            new_self.into_cache();
        }

        Self::from_cache(directory)
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
        let index_writer = search_index
            .underlying_index
            .writer(INDEX_TANTIVY_MEMORY_BUDGET)?;
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
        should_delete: impl Fn(*mut ItemPointerData) -> bool,
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
                .map(|ctid_val| {
                    let mut ctid = ItemPointerData::default();
                    pgrx::u64_to_item_pointer(ctid_val, &mut ctid);
                    (should_delete(&mut ctid), ctid_val)
                })
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
        index_name: &str,
    ) -> Result<(), SearchIndexError> {
        let directory = WriterDirectory::from_index_name(index_name);
        let request = WriterRequest::DropIndex { directory };

        writer.lock()?.request(request)?;

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

    pub fn row_to_search_document(
        &self,
        ctid: ItemPointerData,
        tupdesc: &PgTupleDesc,
        values: *mut Datum,
        isnull: *mut bool,
    ) -> Result<SearchDocument, SearchIndexError> {
        // Create a vector of index entries from the postgres row.
        let mut search_document =
            unsafe { row_to_search_document(tupdesc, values, isnull, &self.schema) }?;

        // Insert the ctid value into the entries.
        let ctid_index_value = pgrx::item_pointer_to_u64(ctid);
        search_document.insert(self.schema.ctid_field().id, ctid_index_value.into());

        Ok(search_document)
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
        }

        // Deserialize into the struct with automatic handling for most fields
        let SearchIndexHelper { schema, directory } = SearchIndexHelper::deserialize(deserializer)?;

        let TantivyDirPath(tantivy_dir_path) = directory.tantivy_dir_path(true).unwrap();

        let mut underlying_index =
            Index::open_in_dir(tantivy_dir_path).expect("failed to open index");

        // We need to setup tokenizers again after retrieving an index from disk.
        Self::setup_tokenizers(&mut underlying_index, &schema);

        let reader = Self::reader(&underlying_index)
            .unwrap_or_else(|_| panic!("failed to create index reader while retrieving index"));

        // Construct the SearchIndex.
        Ok(SearchIndex {
            reader,
            underlying_index,
            directory,
            schema,
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
}

impl<T> From<PoisonError<T>> for SearchIndexError {
    fn from(err: PoisonError<T>) -> Self {
        SearchIndexError::WriterClientRace(format!("{}", err))
    }
}

#[cfg(test)]
mod tests {
    use crate::{fixtures::*, schema::SearchConfig};
    use rstest::*;
    use tantivy::schema::OwnedValue;

    use super::SearchIndex;

    /// Expected to panic because no index has been created in the directory.
    #[rstest]
    #[should_panic]
    fn test_index_from_disk_panics(mock_dir: MockWriterDirectory) {
        mock_dir.load_index::<SearchIndex>().unwrap();
    }

    #[rstest]
    fn test_chinese_compatible_tokenizer(mut chinese_index: MockSearchIndex) {
        let client = TestClient::new_arc();

        let index = &mut chinese_index.index;
        let schema = &index.schema;

        // Insert fields into document.
        let mut doc = schema.new_document();
        let id_field = schema.key_field();
        let ctid_field = schema.ctid_field();
        let author_field = schema.get_search_field(&"author".into()).unwrap();
        doc.insert(id_field.id, OwnedValue::I64(0));
        doc.insert(ctid_field.id, OwnedValue::U64(0));
        doc.insert(author_field.id, OwnedValue::Str("张伟".into()));

        // Insert document into index.
        index.insert(&client, doc.clone()).unwrap();

        // Search in index
        let search_config = SearchConfig {
            query: crate::query::SearchQueryInput::Parse {
                query_string: "author:张".into(),
            },
            key_field: "id".into(),
            ..Default::default()
        };
        let state = index.search_state(&client, &search_config, true).unwrap();

        let (_, doc_address, _, _) = *state
            .search(SearchIndex::executor())
            .first()
            .expect("query returned no results");
        let found: tantivy::TantivyDocument = state
            .searcher
            .doc(doc_address)
            .expect("no document at address");

        assert_eq!(&found, &doc.doc);
    }
}
