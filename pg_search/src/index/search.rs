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

use super::state::SearchState;
use crate::schema::{
    SearchConfig, SearchDocument, SearchFieldConfig, SearchFieldName, SearchFieldType,
    SearchIndexSchema, SearchIndexSchemaError,
};
use crate::writer::{
    self, SearchDirectoryError, SearchFs, TantivyDirPath, UnlockedDirectory, Writer, WriterClient,
    WriterDirectory, WriterRequest,
};
use anyhow::Result;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::ptr::addr_of_mut;
use std::sync::Arc;
use tantivy::{query::QueryParser, Executor, Index, Searcher};
use tantivy::{schema::Value, IndexReader, IndexWriter, TantivyDocument, TantivyError};
use thiserror::Error;
use tokenizers::{create_normalizer_manager, create_tokenizer_manager};
use tracing::{debug, trace};

// Must be at least 15,000,000 or Tantivy will panic.
const INDEX_TANTIVY_MEMORY_BUDGET: usize = 500_000_000;
const CACHE_NUM_BLOCKS: usize = 10;

pub type SearchIndexCacheType = Lazy<HashMap<WriterDirectory, SearchIndex>>;

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
static mut SEARCH_INDEX_MEMORY: SearchIndexCacheType = Lazy::new(HashMap::new);

pub static mut SEARCH_EXECUTOR: Lazy<Executor> = Lazy::new(|| {
    let num_threads = num_cpus::get();
    Executor::multi_thread(num_threads, "prefix-here").expect("could not create search executor")
});

static mut SEARCH_WRITER: Lazy<Writer> = Lazy::new(Writer::new);

#[derive(Serialize)]
pub struct SearchIndex {
    pub schema: SearchIndexSchema,
    pub directory: WriterDirectory,
    #[serde(skip_serializing)]
    pub reader: IndexReader,
    #[serde(skip_serializing)]
    pub underlying_index: Index,
    pub uuid: String,
    pub is_dirty: bool,
    pub is_pending_create: bool,
    pub is_pending_drop: bool,
}

impl SearchIndex {
    pub fn create_index<W: WriterClient<WriterRequest>>(
        _writer: &Arc<Mutex<W>>,
        directory: WriterDirectory,
        fields: Vec<(SearchFieldName, SearchFieldConfig, SearchFieldType)>,
        uuid: String,
        key_field_index: usize,
    ) -> Result<&'static mut Self, SearchIndexError> {
        let writer = unsafe { Self::get_writer() };
        writer.create_index(directory.clone(), fields, uuid, key_field_index)?;

        // As the new index instance was created in a background process, we need
        // to load it from disk to use it.
        let new_self_ref = Self::from_disk(&directory)
            .unwrap_or_else(|err| panic!("error loading index from directory: {err}"));

        // flag for later if the creating transaction is aborted
        new_self_ref.is_pending_create = true;

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
                trace!(field_name, "attempting to create tokenizer");
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

    pub unsafe fn get_writer() -> &'static mut Writer {
        addr_of_mut!(SEARCH_WRITER)
            .as_mut()
            .expect("global SEARCH_INDEX_MEMORY must not be null")
    }

    /// # Safety
    ///
    /// This function is unsafe as it returns a mutable reference to a mutable static global.  It is your
    /// responsibility to ensure, at the time of calling this function, there are no other outstanding
    /// references to the returned static global.
    pub unsafe fn get_cache() -> &'static mut SearchIndexCacheType {
        addr_of_mut!(SEARCH_INDEX_MEMORY)
            .as_mut()
            .expect("global SEARCH_INDEX_MEMORY must not be null")
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

    /// Remove the specified `directory` from the internal cache
    ///
    /// # Safety
    ///
    /// It is the caller's responsibility to ensure they don't have an outstanding mutable reference
    /// to the internal cache object
    pub unsafe fn drop_from_cache(directory: &WriterDirectory) {
        SEARCH_INDEX_MEMORY.remove(directory);
    }

    /// If this [`SearchIndex]` instance has changes in need of commit *or* abort, return true
    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    /// If this [`SearchIndex`] is newly created, return true
    pub fn is_pending_create(&self) -> bool {
        self.is_pending_create
    }

    /// If this [`SearchIndex`] has been dropped, return true
    pub fn is_pending_drop(&self) -> bool {
        self.is_pending_drop
    }

    /// Returns the index size, in bytes, according to tantivy
    pub fn byte_size(&self) -> Result<u64> {
        Ok(self
            .reader
            .searcher()
            .space_usage()
            .map(|space| space.total().get_bytes())?)
    }

    pub fn query_parser(&self, config: &SearchConfig) -> QueryParser {
        let mut query_parser = QueryParser::for_index(
            &self.underlying_index,
            self.schema
                .fields
                .iter()
                .map(|search_field| search_field.id.0)
                .collect::<Vec<_>>(),
        );

        if let Some(true) = config.conjunction_mode {
            query_parser.set_conjunction_by_default();
        }

        query_parser
    }

    pub fn search_state<W: WriterClient<WriterRequest> + Send + Sync + 'static>(
        &mut self,
        writer: &Arc<Mutex<W>>,
        config: &SearchConfig,
    ) -> Result<SearchState, SearchIndexError> {
        // Commit any inserts or deletes that have occurred during this transaction.
        if self.is_dirty {
            self.commit(writer)?;
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

    /// Commit pending index changes to the underlying tantivy index, changing the internal state
    /// from "dirty" to clean.
    ///
    /// # Errors
    ///
    /// If the index is not dirty a [`SearchIndexError::IndexNotDirty`] error is returned.
    ///
    /// If problems are encountered while performing the commit, those errors are returned to the
    /// caller.
    pub fn commit<W: WriterClient<WriterRequest> + Send + Sync + 'static>(
        &mut self,
        _writer: &Arc<Mutex<W>>,
    ) -> Result<(), SearchIndexError> {
        let writer = unsafe { Self::get_writer() };
        if !self.is_dirty() {
            return Err(SearchIndexError::IndexNotDirty);
        }

        self.is_dirty = false;
        writer.commit(self.directory.clone())?;
        Ok(())
    }

    /// Abort pending index changes to the underlying tantivy index, changing the internal state
    /// from "dirty" to clean.
    ///
    /// # Errors
    ///
    /// If the index is not dirty a [`SearchIndexError::IndexNotDirty`] error is returned.
    ///
    /// If problems are encountered while performing the abort, those errors are returned to the
    /// caller.
    pub fn abort<W: WriterClient<WriterRequest> + Send + Sync + 'static>(
        &mut self,
        _writer: &Arc<Mutex<W>>,
    ) -> Result<(), SearchIndexError> {
        let writer = unsafe { Self::get_writer() };
        if !self.is_dirty() {
            return Err(SearchIndexError::IndexNotDirty);
        }

        self.is_dirty = false;
        writer.abort(self.directory.clone())?;
        Ok(())
    }

    pub fn insert<W: WriterClient<WriterRequest> + Send + Sync + 'static>(
        &mut self,
        _writer: &Arc<Mutex<W>>,
        document: SearchDocument,
    ) -> Result<(), SearchIndexError> {
        // the index is about to change, and that requires our transaction callbacks be registered
        crate::postgres::transaction::register_callback();

        let writer = unsafe { Self::get_writer() };
        writer.insert(self.directory.clone(), document)?;
        self.is_dirty = true;

        Ok(())
    }

    /// Using the `should_delete` argument, determine, one-by-one, if a document in this index
    /// needs to be deleted.
    ///
    /// This function is atomic in that it ensures the underlying changes to the tantivy index
    /// are committed before returning an [`Ok`] response.
    pub fn delete<W: WriterClient<WriterRequest> + Send + Sync + 'static>(
        &mut self,
        _writer: &Arc<Mutex<W>>,
        should_delete: impl Fn(u64) -> bool,
    ) -> Result<(u32, u32), SearchIndexError> {
        let mut deleted: u32 = 0;
        let mut not_deleted: u32 = 0;
        let mut ctids_to_delete: Vec<u64> = vec![];

        let ctid_field = self.schema.ctid_field().id.0;
        for segment_reader in self.searcher().segment_readers() {
            let store_reader = segment_reader
                .get_store_reader(CACHE_NUM_BLOCKS)
                .expect("Failed to get store reader");

            for doc in store_reader.iter::<TantivyDocument>(segment_reader.alive_bitset()) {
                // if a document failed to deserialize, that's probably a hard error indicating the
                // index is corrupt.  So return that back to the caller immediately
                let doc = doc?;

                if let Some(ctid) = doc.get_first(ctid_field).and_then(|ctid| ctid.as_u64()) {
                    if should_delete(ctid) {
                        ctids_to_delete.push(ctid);
                        deleted += 1;
                    } else {
                        not_deleted += 1;
                    }
                } else {
                    // NB:  in a perfect world, this shouldn't happen.  But we did have a bug where
                    // the "ctid" field was not being `STORED`, which caused this
                    debug!(
                        "document `{doc:?}` in segment `{}` has no ctid",
                        segment_reader.segment_id()
                    );
                }
            }
        }

        if !ctids_to_delete.is_empty() {
            let writer = unsafe { Self::get_writer() };

            writer.delete(self.directory.clone(), &ctid_field, &ctids_to_delete)?;

            writer.commit(self.directory.clone())?;
        }

        Ok((deleted, not_deleted))
    }

    pub fn drop_index<W: WriterClient<WriterRequest>>(
        &mut self,
        _writer: &Arc<Mutex<W>>,
        directory: &WriterDirectory,
    ) -> Result<(), SearchIndexError> {
        // the index is about to be queued to drop and that requires our transaction callbacks be registered
        crate::postgres::transaction::register_callback();

        let writer = unsafe { Self::get_writer() };

        writer.drop_index(directory.clone())?;

        self.is_dirty = true;
        self.is_pending_drop = true;

        Ok(())
    }

    pub fn vacuum<W: WriterClient<WriterRequest>>(
        &mut self,
        _writer: &Arc<Mutex<W>>,
    ) -> Result<(), SearchIndexError> {
        let writer = unsafe { Self::get_writer() };
        writer.vacuum(self.directory.clone())?;
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
        }

        // Deserialize into the struct with automatic handling for most fields
        let SearchIndexHelper {
            schema,
            directory,
            uuid,
        } = SearchIndexHelper::deserialize(deserializer)?;

        let TantivyDirPath(tantivy_dir_path) = directory
            .tantivy_dir_path(true)
            .expect("tantivy directory path should be valid");

        let tantivy_dir = UnlockedDirectory::open(tantivy_dir_path)
            .expect("need a valid path to open a tantivy index");
        let mut underlying_index = Index::open(tantivy_dir).expect("index should be openable");

        // We need to setup tokenizers again after retrieving an index from disk.
        Self::setup_tokenizers(&mut underlying_index, &schema);

        let reader = Self::reader(&underlying_index).unwrap_or_else(|err| {
            panic!("failed to create index reader while retrieving index: {err}")
        });

        // Construct the SearchIndex.
        Ok(SearchIndex {
            reader,
            underlying_index,
            directory,
            schema,
            uuid,
            is_dirty: false,
            is_pending_drop: false,
            is_pending_create: false,
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
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),

    #[error(transparent)]
    WriterDirectoryError(#[from] SearchDirectoryError),

    #[error(transparent)]
    AnyhowError(#[from] anyhow::Error),

    #[error("Index has no pending changes")]
    IndexNotDirty,
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
