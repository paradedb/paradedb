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
use crate::index::bulk_delete::BulkDeleteDirectory;
use crate::index::channel::{ChannelDirectory, ChannelRequestHandler};
use crate::index::mvcc::MVCCDirectory;
use crate::index::writer::index::SearchIndexWriter;
use crate::postgres::index::get_fields;
use crate::postgres::options::SearchIndexCreateOptions;
use crate::schema::{SearchFieldConfig, SearchIndexSchema, SearchIndexSchemaError};
use anyhow::Result;
use pgrx::PgRelation;
use std::num::NonZeroUsize;
use tantivy::{Index, IndexSettings, ReloadPolicy};
use thiserror::Error;
use tokenizers::{create_normalizer_manager, create_tokenizer_manager};
use tracing::trace;

pub enum WriterResources {
    CreateIndex,
    Statement,
    Vacuum,
}
pub type Parallelism = NonZeroUsize;
pub type MemoryBudget = usize;
pub type TargetSegmentCount = usize;
pub type DoMerging = bool;

enum BlockDirectoryType {
    Mvcc,
    BulkDelete,
}

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

/// How many messages should we buffer between the channels?
const CHANNEL_QUEUE_LEN: usize = 1000;

struct SearchIndex {
    schema: SearchIndexSchema,
    underlying_index: Index,
    handler: ChannelRequestHandler,
}

impl SearchIndex {
    fn create_index(
        index_relation: &PgRelation,
        resources: WriterResources,
    ) -> Result<SearchIndexWriter> {
        let schema = get_index_schema(index_relation)?;
        let create_options = index_relation.rd_options as *mut SearchIndexCreateOptions;

        let settings = IndexSettings {
            docstore_compress_dedicated_thread: false,
            ..IndexSettings::default()
        };
        let search_index = Self::prepare_index(
            index_relation,
            schema,
            BlockDirectoryType::Mvcc,
            |directory, schema| Index::create(directory, schema.schema.clone(), settings),
        )?;

        SearchIndexWriter::new(
            index_relation.oid(),
            search_index.underlying_index,
            search_index.schema,
            search_index.handler,
            resources,
            unsafe { &*create_options },
        )
    }

    fn open_writer(
        index_relation: &PgRelation,
        resources: WriterResources,
        directory_type: BlockDirectoryType,
    ) -> Result<SearchIndexWriter> {
        let schema = get_index_schema(index_relation)?;
        let create_options = index_relation.rd_options as *mut SearchIndexCreateOptions;
        let search_index =
            Self::prepare_index(index_relation, schema, directory_type, |directory, _| {
                Index::open(directory)
            })?;

        SearchIndexWriter::new(
            index_relation.oid(),
            search_index.underlying_index,
            search_index.schema,
            search_index.handler,
            resources,
            unsafe { &*create_options },
        )
    }

    fn prepare_index<
        F: FnOnce(ChannelDirectory, &SearchIndexSchema) -> tantivy::Result<Index>
            + Send
            + Sync
            + 'static,
    >(
        index_relation: &PgRelation,
        schema: SearchIndexSchema,
        directory_type: BlockDirectoryType,
        opener: F,
    ) -> Result<Self, SearchIndexError> {
        let index_oid = index_relation.oid();

        let (req_sender, req_receiver) = crossbeam::channel::bounded(CHANNEL_QUEUE_LEN);
        let channel_dir = ChannelDirectory::new(req_sender);
        let mut handler = match directory_type {
            BlockDirectoryType::Mvcc => {
                ChannelRequestHandler::open(&MVCCDirectory::new(index_oid), index_oid, req_receiver)
            }
            BlockDirectoryType::BulkDelete => ChannelRequestHandler::open(
                &BulkDeleteDirectory::new(index_oid),
                index_oid,
                req_receiver,
            ),
        };

        let underlying_index = {
            let schema = schema.clone();
            handler
                .wait_for(move || {
                    let mut index = opener(channel_dir, &schema)?;
                    SearchIndex::setup_tokenizers(&mut index, &schema);
                    tantivy::Result::Ok(index)
                })
                .expect("scoped thread should not fail")?
        };

        Ok(SearchIndex {
            schema,
            underlying_index,
            handler,
        })
    }

    fn open_reader(
        index_relation: &PgRelation,
        directory_type: BlockDirectoryType,
    ) -> Result<SearchIndexReader> {
        let mut index = match directory_type {
            BlockDirectoryType::Mvcc => Index::open(MVCCDirectory::new(index_relation.oid()))?,
            BlockDirectoryType::BulkDelete => {
                Index::open(BulkDeleteDirectory::new(index_relation.oid()))?
            }
        };
        let schema = get_index_schema(index_relation)?;
        SearchIndex::setup_tokenizers(&mut index, &schema);
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;
        let searcher = reader.searcher();
        Ok(SearchIndexReader::new(
            index_relation,
            index,
            searcher,
            reader,
            schema,
        ))
    }

    fn setup_tokenizers(underlying_index: &mut Index, schema: &SearchIndexSchema) {
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

pub fn get_index_schema(index_relation: &PgRelation) -> Result<SearchIndexSchema> {
    if index_relation.rd_options.is_null() {
        panic!("must specify key field")
    }
    let (fields, key_field_index) = unsafe { get_fields(index_relation) };
    let schema = SearchIndexSchema::new(fields, key_field_index)?;
    Ok(schema)
}

/// Open a (non-channel-based) [`SearchIndexReader`] for the specified Postgres index relation
pub fn open_mvcc_reader(index_relation: &PgRelation) -> Result<SearchIndexReader> {
    SearchIndex::open_reader(index_relation, BlockDirectoryType::Mvcc)
}

pub fn open_bulk_delete_reader(index_relation: &PgRelation) -> Result<SearchIndexReader> {
    SearchIndex::open_reader(index_relation, BlockDirectoryType::BulkDelete)
}

/// Open an existing index for writing
pub fn open_mvcc_writer(
    index_relation: &PgRelation,
    resources: WriterResources,
) -> Result<SearchIndexWriter> {
    SearchIndex::open_writer(index_relation, resources, BlockDirectoryType::Mvcc)
}

pub fn open_bulk_delete_writer(
    index_relation: &PgRelation,
    resources: WriterResources,
) -> Result<SearchIndexWriter> {
    SearchIndex::open_writer(index_relation, resources, BlockDirectoryType::BulkDelete)
}

/// Create a new, empty index for the specified Postgres index relation
pub fn create_new_index(
    index_relation: &PgRelation,
    resources: WriterResources,
) -> Result<SearchIndexWriter> {
    SearchIndex::create_index(index_relation, resources)
}
