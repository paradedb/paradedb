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

use super::reader::SearchIndexReader;
use super::IndexError;
use crate::gucs;
use crate::index::SearchIndexWriter;
use crate::index::{
    BlockingDirectory, SearchDirectoryError, SearchFs, TantivyDirPath, WriterDirectory,
};
use crate::schema::{
    SearchConfig, SearchDocument, SearchFieldConfig, SearchFieldName, SearchFieldType,
    SearchIndexSchema, SearchIndexSchemaError,
};
use anyhow::Result;
use once_cell::sync::Lazy;
use serde::{Deserialize, Deserializer, Serialize};
use std::num::NonZeroUsize;
use tantivy::query::Query;
use tantivy::{query::QueryParser, Executor, Index};
use thiserror::Error;
use tokenizers::{create_normalizer_manager, create_tokenizer_manager};
use tracing::trace;

/// PostgreSQL operates in a process-per-client model, meaning every client connection
/// to PostgreSQL results in a new backend process being spawned on the PostgreSQL server.
pub static mut SEARCH_EXECUTOR: Lazy<Executor> = Lazy::new(|| {
    let num_threads = std::thread::available_parallelism()
        .expect("this computer should have at least one CPU")
        .get();
    Executor::multi_thread(num_threads, "prefix-here").expect("could not create search executor")
});

pub enum WriterResources {
    CreateIndex,
    Statement,
    Vacuum,
}
pub type Parallelism = NonZeroUsize;
pub type MemoryBudget = usize;

impl WriterResources {
    pub fn resources(&self) -> (Parallelism, MemoryBudget) {
        match self {
            WriterResources::CreateIndex => (
                gucs::create_index_parallelism(),
                gucs::create_index_memory_budget(),
            ),
            WriterResources::Statement => (
                gucs::statement_parallelism(),
                gucs::statement_memory_budget(),
            ),
            WriterResources::Vacuum => (
                gucs::statement_parallelism(),
                gucs::statement_memory_budget(),
            ),
        }
    }
}

#[derive(Serialize)]
pub struct SearchIndex {
    pub schema: SearchIndexSchema,
    pub directory: WriterDirectory,
    #[serde(skip_serializing)]
    pub underlying_index: Index,
}

impl SearchIndex {
    pub fn create_index(
        directory: WriterDirectory,
        fields: Vec<(SearchFieldName, SearchFieldConfig, SearchFieldType)>,
        key_field_index: usize,
    ) -> Result<Self, SearchIndexError> {
        SearchIndexWriter::create_index(directory.clone(), fields, key_field_index)?;

        // As the new index instance was created in a background process, we need
        // to load it from disk to use it.
        let new_self_ref = Self::from_disk(&directory)
            .unwrap_or_else(|err| panic!("error loading index from directory: {err}"));

        Ok(new_self_ref)
    }

    pub fn get_reader(&self) -> Result<SearchIndexReader> {
        SearchIndexReader::new(self)
    }

    /// Retrieve an owned writer for a given index. This will block until this process
    /// can get an exclusive lock on the Tantivy writer. The return type needs to
    /// be entirely owned by the new process, with no references.
    pub fn get_writer(&self, resources: WriterResources) -> Result<SearchIndexWriter> {
        let (parallelism, memory_budget) = resources.resources();
        let underlying_writer = self
            .underlying_index
            .writer_with_num_threads(parallelism.get(), memory_budget)?;
        Ok(SearchIndexWriter {
            underlying_writer: Some(underlying_writer),
        })
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

    pub fn from_disk(directory: &WriterDirectory) -> Result<Self, SearchIndexError> {
        let mut new_self: Self = directory.load_index()?;

        // In the case of a physical replication of the database, the absolute path that is stored
        // in the serialized WriterDirectory might refer to the source database's file system.
        // We should overwrite it with the dynamically generated one that's been passed as an
        // argument here.
        new_self.directory = directory.clone();

        Ok(new_self)
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

    pub fn query(&self, config: &SearchConfig, reader: &SearchIndexReader) -> Box<dyn Query> {
        let mut parser = self.query_parser();
        let searcher = reader.underlying_reader.searcher();
        config
            .query
            .clone()
            .into_tantivy_query(&self.schema, &mut parser, &searcher, config)
            .expect("must be able to parse query")
    }

    pub fn insert(
        &self,
        writer: &SearchIndexWriter,
        document: SearchDocument,
    ) -> Result<(), SearchIndexError> {
        // the index is about to change, and that requires our transaction callbacks be registered
        crate::postgres::transaction::register_callback();

        writer.insert(document)?;

        Ok(())
    }

    /// Using the `should_delete` argument, determine, one-by-one, if a document in this index
    /// needs to be deleted.
    ///
    /// This function is atomic in that it ensures the underlying changes to the tantivy index
    /// are committed before returning an [`Ok`] response.
    pub fn delete(
        &self,
        reader: &SearchIndexReader,
        writer: &SearchIndexWriter,
        should_delete: impl Fn(u64) -> bool,
    ) -> Result<(u32, u32), SearchIndexError> {
        let ctid_field = self.schema.ctid_field().id.0;
        let (ctids_to_delete, not_deleted) = reader.get_ctids_to_delete(should_delete)?;
        if !ctids_to_delete.is_empty() {
            writer.delete(&ctid_field, &ctids_to_delete)?;
        }

        Ok((ctids_to_delete.len() as u32, not_deleted))
    }

    pub fn drop_index(&mut self) -> Result<(), SearchIndexError> {
        // the index is about to be queued to drop and that requires our transaction callbacks be registered
        crate::postgres::transaction::register_callback();

        // Mark in our global store that this index is pending drop so it can be physically
        // deleted on commit, or in case it needs to be rolled back on abort.
        SearchIndexWriter::mark_pending_drop(&self.directory);

        Ok(())
    }

    pub fn vacuum(&self, writer: &SearchIndexWriter) -> Result<(), SearchIndexError> {
        writer.vacuum()?;
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
        }

        // Deserialize into the struct with automatic handling for most fields
        let SearchIndexHelper { schema, directory } = SearchIndexHelper::deserialize(deserializer)?;

        let TantivyDirPath(tantivy_dir_path) = directory
            .tantivy_dir_path(true)
            .expect("tantivy directory path should be valid");

        let tantivy_dir = BlockingDirectory::open(tantivy_dir_path)
            .expect("need a valid path to open a tantivy index");
        let mut underlying_index = Index::open(tantivy_dir).expect("index should be openable");

        // We need to setup tokenizers again after retrieving an index from disk.
        Self::setup_tokenizers(&mut underlying_index, &schema);

        // Construct the SearchIndex.
        Ok(SearchIndex {
            underlying_index,
            directory,
            schema,
        })
    }
}

#[derive(Error, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum SearchIndexError {
    #[error(transparent)]
    SchemaError(#[from] SearchIndexSchemaError),

    #[error(transparent)]
    WriterIndexError(#[from] IndexError),

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
}
