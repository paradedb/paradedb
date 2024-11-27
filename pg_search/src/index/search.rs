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

use super::reader::index::SearchIndexReader;
use super::writer::index::IndexError;
use crate::gucs;
use crate::index::writer::index::SearchIndexWriter;
use crate::postgres::options::SearchIndexCreateOptions;
use crate::query::SearchQueryInput;
use crate::schema::{
    SearchDocument, SearchField, SearchFieldConfig, SearchFieldName, SearchFieldType,
    SearchIndexSchema, SearchIndexSchemaError,
};
use anyhow::Result;
use once_cell::sync::Lazy;
use pgrx::PgRelation;
use serde::Serialize;
use std::num::NonZeroUsize;
use tantivy::query::Query;
use tantivy::{query::QueryParser, Executor, Index};
use thiserror::Error;
use tokenizers::{create_normalizer_manager, create_tokenizer_manager};
use tracing::trace;

/// PostgreSQL operates in a process-per-client model, meaning every client connection
/// to PostgreSQL results in a new backend process being spawned on the PostgreSQL server.
pub static mut SEARCH_EXECUTOR: Lazy<Executor> = Lazy::new(Executor::single_thread);

pub enum WriterResources {
    CreateIndex,
    Statement,
    Vacuum,
}
pub type Parallelism = NonZeroUsize;
pub type MemoryBudget = usize;
pub type TargetSegmentCount = usize;
pub type DoMerging = bool;

impl WriterResources {
    pub fn resources(
        &self,
        index_options: &SearchIndexCreateOptions,
    ) -> (Parallelism, MemoryBudget, TargetSegmentCount, DoMerging) {
        match self {
            WriterResources::CreateIndex => (
                gucs::create_index_parallelism(),
                gucs::create_index_memory_budget(),
                index_options.target_segment_count(),
                true, // we always want a merge on CREATE INDEX
            ),
            WriterResources::Statement => (
                gucs::statement_parallelism(),
                gucs::statement_memory_budget(),
                index_options.target_segment_count(),
                index_options.merge_on_insert(), // user/index decides if we merge for INSERT/UPDATE statements
            ),
            WriterResources::Vacuum => (
                gucs::statement_parallelism(),
                gucs::statement_memory_budget(),
                index_options.target_segment_count(),
                true, // we always want a merge on (auto)VACUUM
            ),
        }
    }
}

#[derive(Serialize)]
pub struct SearchIndex {
    pub schema: SearchIndexSchema,
    pub index_oid: pgrx::pg_sys::Oid,
    #[serde(skip_serializing)]
    pub underlying_index: Index,
}

impl SearchIndex {
    pub fn create_index(
        index_oid: pgrx::pg_sys::Oid,
        fields: Vec<(SearchFieldName, SearchFieldConfig, SearchFieldType)>,
        key_field_index: usize,
    ) -> Result<Self> {
        SearchIndexWriter::create_index(index_oid, fields, key_field_index)
    }

    pub fn get_reader(&self) -> Result<SearchIndexReader> {
        SearchIndexReader::new(self)
    }

    /// Retrieve an owned writer for a given index. This will block until this process
    /// can get an exclusive lock on the Tantivy writer. The return type needs to
    /// be entirely owned by the new process, with no references.
    pub fn get_writer(
        &self,
        resources: WriterResources,
        index_options: &SearchIndexCreateOptions,
    ) -> Result<SearchIndexWriter> {
        SearchIndexWriter::new(&self.underlying_index, resources, index_options, self.index_oid)
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

    pub fn key_field(&self) -> SearchField {
        self.schema.key_field()
    }

    pub fn key_field_name(&self) -> String {
        self.key_field().name.to_string()
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

    pub fn query(
        &self,
        indexrel: &PgRelation,
        search_query_input: &SearchQueryInput,
        reader: &SearchIndexReader,
    ) -> Box<dyn Query> {
        let mut parser = self.query_parser();
        let searcher = reader.underlying_reader.searcher();
        search_query_input
            .clone()
            .into_tantivy_query(&(indexrel, &self.schema), &mut parser, &searcher)
            .expect("must be able to parse query")
    }

    pub fn insert(
        &self,
        writer: &mut SearchIndexWriter,
        document: SearchDocument,
    ) -> Result<(), SearchIndexError> {
        writer.insert(document)?;
        Ok(())
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
    AnyhowError(#[from] anyhow::Error),
}
