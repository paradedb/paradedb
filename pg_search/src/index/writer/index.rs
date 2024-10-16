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

use crate::postgres::options::SearchIndexCreateOptions;
use crate::{
    index::SearchIndex,
    postgres::types::TantivyValueError,
    schema::{
        SearchDocument, SearchFieldConfig, SearchFieldName, SearchFieldType, SearchIndexSchema,
    },
};
use anyhow::Result;
use tantivy::directory::{Lock, META_LOCK};
use tantivy::{
    indexer::{AddOperation, SegmentWriter},
    IndexSettings,
};
use tantivy::{Directory, Index};
use thiserror::Error;

use crate::index::directory::blocking::{BlockingDirectory, META_FILEPATH};
use crate::index::WriterResources;

/// The entity that interfaces with Tantivy indexes.
pub struct SearchIndexWriter {
    pub underlying_writer: SegmentWriter,
    pub current_opstamp: tantivy::Opstamp,
    pub wants_merge: bool,
    pub commit_opstamp: tantivy::Opstamp,
    pub segment: tantivy::Segment,
}

impl SearchIndexWriter {
    pub fn new(
        index: &Index,
        resources: WriterResources,
        index_options: &SearchIndexCreateOptions,
    ) -> Result<Self> {
        let (_, memory_budget, _, _) = resources.resources(index_options);
        let segment = index.new_segment();
        let current_opstamp = index.load_metas()?.opstamp;
        let underlying_writer = SegmentWriter::for_segment(memory_budget, segment.clone())?;

        Ok(Self {
            underlying_writer,
            current_opstamp,
            commit_opstamp: current_opstamp,
            // TODO: Merge on insert
            wants_merge: false,
            segment,
        })
    }

    pub fn insert(&mut self, document: SearchDocument) -> Result<(), IndexError> {
        // Add the Tantivy document to the index.
        let tantivy_document: tantivy::TantivyDocument = document.into();
        self.current_opstamp += 1;
        self.underlying_writer.add_document(AddOperation {
            opstamp: self.current_opstamp,
            document: tantivy_document,
        })?;

        Ok(())
    }

    pub fn commit(mut self) -> Result<()> {
        self.current_opstamp += 1;
        let max_doc = self.underlying_writer.max_doc();
        self.underlying_writer.finalize()?;
        let segment = self.segment.with_max_doc(max_doc);
        let index = segment.index();

        let _lock = index.directory().acquire_lock(&Lock {
            filepath: META_LOCK.filepath.clone(),
            is_blocking: true,
        });

        let committed_meta = index.load_metas()?;
        let mut segments = committed_meta.segments.clone();
        segments.push(segment.meta().clone());

        let new_meta = tantivy::IndexMeta {
            segments,
            opstamp: self.current_opstamp,
            index_settings: committed_meta.index_settings,
            schema: committed_meta.schema,
            payload: committed_meta.payload,
        };

        index
            .directory()
            .atomic_write(*META_FILEPATH, &serde_json::to_vec(&new_meta)?)?;

        Ok(())
    }

    pub fn create_index(
        index_oid: pgrx::pg_sys::Oid,
        fields: Vec<(SearchFieldName, SearchFieldConfig, SearchFieldType)>,
        key_field_index: usize,
    ) -> Result<SearchIndex> {
        let schema = SearchIndexSchema::new(fields, key_field_index)?;
        let tantivy_dir = BlockingDirectory::new(index_oid);
        let settings = IndexSettings {
            docstore_compress_dedicated_thread: false,
            ..IndexSettings::default()
        };
        let mut underlying_index = Index::create(tantivy_dir, schema.schema.clone(), settings)?;

        SearchIndex::setup_tokenizers(&mut underlying_index, &schema);
        Ok(SearchIndex {
            index_oid,
            underlying_index,
            schema,
        })
    }
}

#[derive(Error, Debug)]
pub enum IndexError {
    #[error(transparent)]
    TantivyError(#[from] tantivy::TantivyError),

    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    #[error(transparent)]
    TantivyValueError(#[from] TantivyValueError),

    #[error("key_field column '{0}' cannot be NULL")]
    KeyIdNull(String),
}
