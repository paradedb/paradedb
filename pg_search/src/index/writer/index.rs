// Copyright (c) 2023-2025 Retake, Inc.
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
use pgrx::{pg_sys, PgRelation};
use std::collections::HashSet;
use std::sync::Arc;
use tantivy::index::SegmentId;
use tantivy::indexer::{NoMergePolicy, UserOperation};
use tantivy::schema::Field;
use tantivy::{DocId, Index, IndexSettings, IndexWriter, Opstamp, TantivyDocument, TantivyError};
use thiserror::Error;

use crate::index::channel::{ChannelDirectory, ChannelRequestHandler};
use crate::index::merge_policy::LayeredMergePolicy;
use crate::index::mvcc::MvccSatisfies;
use crate::index::{get_index_schema, setup_tokenizers, WriterResources};
use crate::{
    postgres::types::TantivyValueError,
    schema::{SearchDocument, SearchIndexSchema},
};

// NB:  should this be a GUC?  Could be useful or could just complicate things for the user
/// How big should our insert queue get before we go ahead and add them to the tantivy index?
const MAX_INSERT_QUEUE_SIZE: usize = 1000;

/// The entity that interfaces with Tantivy indexes.
pub struct SearchIndexWriter {
    pub indexrelid: pg_sys::Oid,
    pub schema: SearchIndexSchema,
    ctid_field: Field,

    // keep all these private -- leaking them to the public API would allow callers to
    // mis-use the IndexWriter in particular.
    writer: Arc<IndexWriter>,
    handler: ChannelRequestHandler,
    insert_queue: Vec<UserOperation>,

    cnt: usize,
}

impl SearchIndexWriter {
    pub fn open(
        index_relation: &PgRelation,
        directory_type: MvccSatisfies,
        resources: WriterResources,
    ) -> Result<Self> {
        let (parallelism, memory_budget) = resources.resources();

        let (req_sender, req_receiver) = crossbeam::channel::bounded(1);
        let channel_dir = ChannelDirectory::new(req_sender);
        let mut handler = directory_type.channel_request_handler(index_relation, req_receiver);

        let mut index = {
            handler
                .wait_for(move || {
                    let index = Index::open(channel_dir)?;
                    tantivy::Result::Ok(index)
                })
                .expect("scoped thread should not fail")?
        };
        setup_tokenizers(&mut index, index_relation);

        let index_clone = index.clone();
        let writer = handler
            .wait_for(move || {
                let writer =
                    index_clone.writer_with_num_threads(parallelism.get(), memory_budget)?;
                writer.set_merge_policy(Box::new(NoMergePolicy));
                tantivy::Result::Ok(writer)
            })
            .expect("scoped thread should not fail")?;

        let schema = SearchIndexSchema::open(index.schema(), index_relation);
        let ctid_field = schema.schema.get_field("ctid")?;

        Ok(Self {
            indexrelid: index_relation.oid(),
            writer: Arc::new(writer),
            schema,
            handler,
            ctid_field,
            insert_queue: Vec::with_capacity(MAX_INSERT_QUEUE_SIZE),
            cnt: 0,
        })
    }

    pub fn create_index(index_relation: &PgRelation) -> Result<Self> {
        let schema = get_index_schema(index_relation)?;
        let (parallelism, memory_budget) = WriterResources::CreateIndex.resources();

        let (req_sender, req_receiver) = crossbeam::channel::bounded(1);
        let channel_dir = ChannelDirectory::new(req_sender);
        let mut handler =
            MvccSatisfies::Snapshot.channel_request_handler(index_relation, req_receiver);

        let mut index = {
            let schema = schema.clone();
            let settings = IndexSettings {
                docstore_compress_dedicated_thread: false,
                ..IndexSettings::default()
            };

            handler
                .wait_for(move || {
                    let index = Index::create(channel_dir, schema.schema.clone(), settings)?;
                    tantivy::Result::Ok(index)
                })
                .expect("scoped thread should not fail")?
        };
        setup_tokenizers(&mut index, index_relation);

        let writer = handler
            .wait_for(move || {
                let writer = index.writer_with_num_threads(parallelism.get(), memory_budget)?;
                writer.set_merge_policy(Box::new(NoMergePolicy));
                tantivy::Result::Ok(writer)
            })
            .expect("scoped thread should not fail")?;

        let ctid_field = schema.schema.get_field("ctid")?;

        Ok(Self {
            indexrelid: index_relation.oid(),
            writer: Arc::new(writer),
            schema,
            ctid_field,
            handler,
            insert_queue: Vec::with_capacity(MAX_INSERT_QUEUE_SIZE),
            cnt: 0,
        })
    }

    /// Our default merge policy is tantivy's [`NoMergePolicy`], and if it's to be changed, we only
    /// support our own [`LayeredMergePolicy`]
    pub fn set_merge_policy(&mut self, merge_policy: LayeredMergePolicy) {
        self.writer.set_merge_policy(Box::new(merge_policy));
    }

    pub fn segment_ids(&mut self) -> HashSet<SegmentId> {
        let index = self.writer.index().clone();
        self.handler
            .wait_for(move || index.searchable_segment_ids().unwrap())
            .unwrap()
            .into_iter()
            .collect()
    }

    pub fn delete_document(&mut self, segment_id: SegmentId, doc_id: DocId) -> Result<()> {
        self.insert_queue
            .push(UserOperation::DeleteByAddress(segment_id, doc_id));
        if self.insert_queue.len() >= MAX_INSERT_QUEUE_SIZE {
            self.drain_insert_queue()?;
        }
        Ok(())
    }

    pub fn insert(&mut self, document: SearchDocument, ctid: u64) -> Result<()> {
        self.cnt += 1;
        let mut tantivy_document: TantivyDocument = document.into();

        tantivy_document.add_u64(self.ctid_field, ctid);

        self.insert_queue.push(UserOperation::Add(tantivy_document));

        if self.insert_queue.len() >= MAX_INSERT_QUEUE_SIZE {
            self.drain_insert_queue()?;
        }
        Ok(())
    }

    pub fn commit(mut self) -> Result<usize> {
        self.drain_insert_queue()?;
        let mut writer =
            Arc::into_inner(self.writer).expect("should not have an outstanding Arc<IndexWriter>");

        let writer = self
            .handler
            .wait_for(move || {
                writer.commit()?;
                tantivy::Result::Ok(writer)
            })
            .expect("spawned thread should not fail")?;

        self.handler
            .wait_for(move || writer.wait_merging_threads())??;

        Ok(self.cnt)
    }

    /// Causes the index to perform a merge.
    ///
    /// It is your responsibility to ensure any necessary locking is handled externally
    ///
    /// # Panics
    ///
    /// Will panic if the insert_queue is not empty.  Can also panic if our internal communications
    /// channels with tantivy fail for some reason.
    pub fn merge(self) -> Result<()> {
        assert!(self.insert_queue.is_empty());
        self.commit()?;
        Ok(())
    }

    fn drain_insert_queue(&mut self) -> Result<Opstamp, TantivyError> {
        let insert_queue = std::mem::take(&mut self.insert_queue);
        let writer = self.writer.clone();
        self.handler
            .wait_for(move || writer.run(insert_queue))
            .expect("spawned thread should not fail")
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
