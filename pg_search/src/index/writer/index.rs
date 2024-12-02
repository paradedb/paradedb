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
use std::sync::Arc;
use tantivy::indexer::{MergePolicy, NoMergePolicy, UserOperation};
use tantivy::schema::Field;
use tantivy::{Index, IndexWriter, Opstamp, Searcher, TantivyDocument, TantivyError, Term};
use thiserror::Error;

use crate::index::channel::ChannelRequestHandler;
use crate::index::merge_policy::NPlusOneMergePolicy;
use crate::index::WriterResources;
use crate::postgres::options::SearchIndexCreateOptions;
use crate::{
    postgres::types::TantivyValueError,
    schema::{SearchDocument, SearchIndexSchema},
};

// NB:  should this be a GUC?  Could be useful or could just complicate things for the user
/// How big should our insert queue get before we go ahead and add them to the tantivy index?
const MAX_INSERT_QUEUE_SIZE: usize = 1000;

/// The entity that interfaces with Tantivy indexes.
pub struct SearchIndexWriter {
    pub schema: SearchIndexSchema,

    // keep all these private -- leaking them to the public API would allow callers to
    // mis-use the IndexWriter in particular.
    writer: Arc<IndexWriter>,
    handler: ChannelRequestHandler,
    wants_merge: bool,
    insert_queue: Vec<UserOperation>,
}

impl SearchIndexWriter {
    pub fn new(
        index: Index,
        schema: SearchIndexSchema,
        handler: ChannelRequestHandler,
        resources: WriterResources,
        index_options: &SearchIndexCreateOptions,
    ) -> Result<Self> {
        let (parallelism, memory_budget, target_segment_count, merge_on_insert) =
            resources.resources(index_options);

        // let memory_budget = memory_budget / parallelism.get();
        // let parallelism = std::num::NonZeroUsize::new(12).unwrap();

        let (wants_merge, merge_policy) = match resources {
            // During a CREATE INDEX we use `target_segment_count` but require twice
            // as many segments before we'll do a merge.
            WriterResources::CreateIndex => {
                let policy: Box<dyn MergePolicy> = Box::new(NPlusOneMergePolicy {
                    n: target_segment_count,
                    min_num_segments: target_segment_count * 2,
                });
                (true, policy)
            }

            // During a VACUUM we want to merge down to our `target_segment_count`
            WriterResources::Vacuum => {
                let policy: Box<dyn MergePolicy> = Box::new(NPlusOneMergePolicy {
                    n: target_segment_count,
                    min_num_segments: 0,
                });
                (true, policy)
            }

            // During regular INSERT/UPDATE/COPY statements, if we were asked to "merge_on_insert"
            // then we use our `NPlusOneMergePolicy` which will ensure we don't more than
            // `target_segment_count` segments, requiring at least 2 to merge together.
            // The idea being that only the very smallest segments will be merged together, reducing write amplification
            WriterResources::Statement if merge_on_insert => {
                let policy: Box<dyn MergePolicy> = Box::new(NPlusOneMergePolicy {
                    n: target_segment_count,
                    min_num_segments: 2,
                });
                (true, policy)
            }

            // During regular INSERT/UPDATE/COPY statements, if we were told not to "merge_on_insert"
            // then we don't do any merging at all.
            WriterResources::Statement => {
                let policy: Box<dyn MergePolicy> = Box::new(NoMergePolicy);
                (false, policy)
            }
        };

        let writer = handler
            .wait_for(move || {
                let writer = index.writer_with_num_threads(parallelism.get(), memory_budget)?;
                writer.set_merge_policy(merge_policy);
                tantivy::Result::Ok(writer)
            })
            .expect("scoped thread should not fail")?;

        Ok(Self {
            writer: Arc::new(writer),
            schema,
            handler,
            wants_merge,
            insert_queue: Vec::with_capacity(MAX_INSERT_QUEUE_SIZE),
        })
    }

    pub fn searcher(&self) -> Result<Searcher> {
        Ok(self.writer.index().reader()?.searcher())
    }

    pub fn get_ctid_field(&self) -> Result<Field> {
        Ok(self.schema.schema.get_field("ctid")?)
    }

    pub fn delete_term(&mut self, term: Term) -> Result<()> {
        self.insert_queue.push(UserOperation::Delete(term));
        if self.insert_queue.len() >= MAX_INSERT_QUEUE_SIZE {
            self.drain_insert_queue()?;
        }
        Ok(())
    }

    pub fn insert(&mut self, document: SearchDocument) -> Result<()> {
        let tantivy_document: TantivyDocument = document.into();
        self.insert_queue.push(UserOperation::Add(tantivy_document));

        if self.insert_queue.len() >= MAX_INSERT_QUEUE_SIZE {
            self.drain_insert_queue()?;
        }
        Ok(())
    }

    pub fn commit(mut self) -> Result<()> {
        self.drain_insert_queue()?;
        let mut writer =
            Arc::into_inner(self.writer).expect("should not have an outstanding Arc<IndexWriter>");
        let _opstamp = self
            .handler
            .wait_for(move || {
                let opstamp = writer.commit()?;
                if self.wants_merge {
                    writer.wait_merging_threads()?;
                }
                tantivy::Result::Ok(opstamp)
            })
            .expect("spawned thread should not fail")?;
        Ok(())
    }

    pub fn vacuum(self) -> Result<()> {
        assert!(self.insert_queue.is_empty());
        self.commit()
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
