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

use crate::index::channel::{ChannelRequestHandler, ChannelRequestStats};
use crate::index::merge_policy::NPlusOneMergePolicy;
use crate::index::WriterResources;
use crate::postgres::options::SearchIndexCreateOptions;
use crate::{
    postgres::types::TantivyValueError,
    schema::{SearchDocument, SearchIndexSchema},
};
use anyhow::Result;
use std::num::NonZeroUsize;
use tantivy::merge_policy::{MergePolicy, NoMergePolicy};
use tantivy::{Index, Opstamp, TantivyError};
use tantivy::{IndexWriter, TantivyDocument};
use thiserror::Error;

// NB:  should this be a GUC?  Could be useful or could just complicate things for the user
/// How big should our insert queue get before we go ahead and add them to the tantivy index?
const MAX_INSERT_QUEUE_SIZE: usize = 1000;

/// The entity that interfaces with Tantivy indexes.
pub struct SearchIndexWriter {
    pub writer: IndexWriter,
    pub schema: SearchIndexSchema,
    pub handler: ChannelRequestHandler,
    wants_merge: bool,
    insert_queue: Vec<TantivyDocument>,
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

        let memory_budget = memory_budget / parallelism.get();
        let parallelism = NonZeroUsize::new(1).unwrap();

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

        let mut handler = handler.clone();
        let writer = handler
            .wait_for(|| index.writer_with_num_threads(parallelism.get(), memory_budget))
            .expect("scoped thread should not fail")?;
        writer.set_merge_policy(merge_policy);

        Ok(Self {
            writer,
            schema,
            handler,
            wants_merge,
            insert_queue: Vec::with_capacity(MAX_INSERT_QUEUE_SIZE),
        })
    }

    pub fn insert(&mut self, document: SearchDocument) -> Result<(), IndexError> {
        let tantivy_document: TantivyDocument = document.into();
        self.insert_queue.push(tantivy_document);

        if self.insert_queue.len() >= MAX_INSERT_QUEUE_SIZE {
            self.drain_insert_queue()?;
        }
        Ok(())
    }

    pub fn commit(mut self) -> Result<()> {
        self.drain_insert_queue()?;
        let _opstamp = self
            .handler
            .wait_for(|| {
                let opstamp = self.writer.commit()?;
                if self.wants_merge {
                    self.writer.wait_merging_threads()?;
                }
                tantivy::Result::Ok(opstamp)
            })
            .expect("spawned thread should not fail")?;
        Ok(())
    }

    fn drain_insert_queue(&mut self) -> Result<Option<Opstamp>, TantivyError> {
        // Add the Tantivy document to the index.
        self.handler
            .wait_for(|| {
                let mut opstamp = None;
                for tantivy_document in self.insert_queue.drain(..) {
                    opstamp = Some(self.writer.add_document(tantivy_document)?);
                }
                tantivy::Result::Ok(opstamp)
            })
            .expect("spawned thread should not fail")
    }

    pub fn vacuum(mut self) -> Result<ChannelRequestStats> {
        std::thread::scope(|scope| {
            let _opstamp = scope.spawn(|| {
                let opstamp = self.writer.commit()?;
                self.writer.wait_merging_threads()?;
                tantivy::Result::Ok(opstamp)
            });

            self.handler.receive_blocking()
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
