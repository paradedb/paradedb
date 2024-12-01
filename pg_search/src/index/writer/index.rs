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
use pgrx::pg_sys;
use tantivy::Index;
use tantivy::{
    indexer::{AddOperation, SegmentWriter},
    IndexSettings,
};
use thiserror::Error;

use crate::index::directory::blocking::BlockingDirectory;
use crate::index::WriterResources;
use crate::postgres::options::SearchIndexCreateOptions;
use crate::postgres::storage::block::SEGMENT_METAS_START;
use crate::postgres::storage::LinkedItemList;
use crate::{
    index::SearchIndex,
    postgres::storage::block::SegmentMetaEntry,
    postgres::types::TantivyValueError,
    schema::{
        SearchDocument, SearchFieldConfig, SearchFieldName, SearchFieldType, SearchIndexSchema,
    },
};

/// The entity that interfaces with Tantivy indexes.
pub struct SearchIndexWriter {
    pub underlying_writer: SegmentWriter,
    pub current_opstamp: tantivy::Opstamp,
    pub wants_merge: bool,
    pub commit_opstamp: tantivy::Opstamp,
    pub segment: tantivy::Segment,
    relation_oid: pg_sys::Oid,
}

impl SearchIndexWriter {
    pub fn new(
        index: &Index,
        resources: WriterResources,
        index_options: &SearchIndexCreateOptions,
        relation_oid: pg_sys::Oid,
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
            relation_oid,
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

        let entry = SegmentMetaEntry {
            meta: segment.meta().tracked.as_ref().clone(),
            opstamp: self.current_opstamp,
            xmin: unsafe { pgrx::pg_sys::GetCurrentTransactionId() },
            xmax: pgrx::pg_sys::InvalidTransactionId,
        };

        crate::log_message(&format!("-- COMMITTED {:?}", entry.clone()));

        unsafe {
            let mut segment_metas =
                LinkedItemList::<SegmentMetaEntry>::open(self.relation_oid, SEGMENT_METAS_START);
            segment_metas.add_items(vec![entry]).unwrap();
        }

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
