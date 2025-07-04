// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use crate::api::{HashMap, HashSet};
use anyhow::Result;
use std::num::NonZeroUsize;
use tantivy::index::SegmentId;
use tantivy::indexer::{AddOperation, SegmentWriter};
use tantivy::schema::Field;
use tantivy::{
    Directory, Index, IndexMeta, IndexWriter, Opstamp, Segment, SegmentMeta, TantivyDocument,
};
use thiserror::Error;

use crate::index::mvcc::{MVCCDirectory, MvccSatisfies};
use crate::index::setup_tokenizers;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::SegmentMetaEntry;
use crate::{postgres::types::TantivyValueError, schema::SearchIndexSchema};

struct PendingSegment {
    segment: Segment,
    writer: SegmentWriter,
    opstamp: Opstamp,
}

impl PendingSegment {
    fn new(index: &Index, memory_budget: NonZeroUsize) -> Result<Self> {
        let segment = index.new_segment();
        let writer = SegmentWriter::for_segment(memory_budget.into(), segment.clone())?;
        Ok(Self {
            segment,
            writer,
            opstamp: Default::default(),
        })
    }

    fn add_document(&mut self, document: TantivyDocument) -> Result<()> {
        self.opstamp += 1;
        self.writer.add_document(AddOperation {
            opstamp: self.opstamp,
            document,
        })?;

        if self.opstamp % 100000 == 0 {
            pgrx::debug2!(
                "writer: added document {}, mem_usage: {}",
                self.opstamp,
                self.mem_usage()
            );
        }

        Ok(())
    }

    fn max_doc(&self) -> usize {
        self.writer.max_doc() as usize
    }

    fn mem_usage(&self) -> usize {
        self.writer.mem_usage()
    }

    fn finalize(self) -> Result<Segment> {
        let max_doc = self.writer.max_doc();
        self.writer.finalize()?;
        let segment = self.segment.with_max_doc(max_doc);
        Ok(segment)
    }
}

#[derive(Debug, Clone)]
pub struct IndexWriterConfig {
    pub memory_budget: NonZeroUsize,
    pub max_docs_per_segment: Option<u32>,
}

/// We want SerialIndexWriter to return a struct like SegmentMeta that implements Deserialize
#[derive(serde::Serialize, serde::Deserialize, Debug, Hash, Clone, PartialEq, Eq)]
pub struct CommittedSegment {
    pub segment_id: SegmentId,
    pub max_doc: u32,
}

/// Unlike Tantivy's IndexWriter, the SerialIndexWriter does not spin up any threads.
/// Everything happens in the foreground, making it ideal for Postgres.
pub struct SerialIndexWriter {
    // for logging purposes
    id: i32,
    indexrel: PgSearchRelation,
    ctid_field: Field,
    config: IndexWriterConfig,
    index: Index,
    pending_segment: Option<PendingSegment>,
    new_metas: Vec<SegmentMeta>,
    schema: SearchIndexSchema,
}

impl SerialIndexWriter {
    pub fn open(
        index_relation: &PgSearchRelation,
        config: IndexWriterConfig,
        worker_number: i32,
    ) -> Result<Self> {
        Self::with_mvcc(
            index_relation,
            MvccSatisfies::Snapshot,
            config,
            worker_number,
        )
    }

    pub fn with_mvcc(
        index_relation: &PgSearchRelation,
        mvcc_satisfies: MvccSatisfies,
        config: IndexWriterConfig,
        worker_number: i32,
    ) -> Result<Self> {
        pgrx::debug1!(
            "writer {}: opening index writer with config: {:?}, satisfies: {:?}",
            worker_number,
            config,
            mvcc_satisfies
        );

        let directory = mvcc_satisfies.directory(index_relation);
        let mut index = Index::open(directory)?;
        let schema = index_relation.schema()?;
        setup_tokenizers(index_relation, &mut index)?;
        let ctid_field = schema.ctid_field();

        Ok(Self {
            id: worker_number,
            indexrel: Clone::clone(index_relation),
            ctid_field,
            config,
            index,
            pending_segment: Default::default(),
            new_metas: Default::default(),
            schema,
        })
    }

    pub fn schema(&self) -> &SearchIndexSchema {
        &self.schema
    }

    pub fn insert(
        &mut self,
        mut document: TantivyDocument,
        ctid: u64,
    ) -> Result<Option<SegmentMeta>> {
        document.add_u64(self.ctid_field, ctid);

        if self.pending_segment.is_none() {
            self.pending_segment = Some(self.new_segment()?);
        }

        self.pending_segment
            .as_mut()
            .unwrap()
            .add_document(document)?;

        let pending_segment = self.pending_segment.as_ref().unwrap();
        let mem_usage = pending_segment.mem_usage();
        let max_doc = pending_segment.max_doc();

        if mem_usage >= self.config.memory_budget.into() {
            pgrx::debug1!(
                "writer {}: finalizing segment {} with {} docs, mem_usage: {} (out of {}), has created {} segments so far",
                self.id,
                pending_segment.segment.id(),
                max_doc,
                mem_usage,
                self.config.memory_budget.get(),
                self.new_metas.len()
            );
            return self.finalize_segment();
        }

        if let Some(max_docs_per_segment) = self.config.max_docs_per_segment {
            if max_doc >= max_docs_per_segment as usize {
                pgrx::debug1!(
                    "writer {}: finalizing segment {} with {} docs, has created {} segments so far",
                    self.id,
                    pending_segment.segment.id(),
                    max_doc,
                    self.new_metas.len()
                );
                return self.finalize_segment();
            }
        }

        Ok(None)
    }

    pub fn commit(mut self) -> Result<Option<(SegmentMeta, PgSearchRelation)>> {
        self.finalize_segment()
            .map(|segment_meta| segment_meta.map(|segment_meta| (segment_meta, self.indexrel)))
    }

    /// Intelligently create a new segment, backed by either a RamDirectory or a MVCCDirectory.
    ///
    /// If we know that the segment we're about to create will be merged with the last segment,
    /// we create a RAMDirectory-backed segment.
    ///
    /// Otherwise, we create a MVCCDirectory-backed segment.
    fn new_segment(&mut self) -> Result<PendingSegment> {
        PendingSegment::new(&self.index, self.config.memory_budget)
    }

    /// Once the memory budget is reached, we "finalize" the segment:
    ///
    /// 1. Serialize the segment to disk
    /// 2. Merge the segment with the previous segment if we're using a RAMDirectory
    /// 3. Save the new meta entry
    /// 4. Return any free space to the FSM
    fn finalize_segment(&mut self) -> Result<Option<SegmentMeta>> {
        pgrx::debug1!("writer {}: finalizing segment", self.id);
        let Some(pending_segment) = self.pending_segment.take() else {
            // no docs were ever added
            return Ok(None);
        };

        let finalized_segment = pending_segment.finalize()?;
        Ok(Some(self.commit_segment(finalized_segment)?))
    }

    fn commit_segment(&mut self, finalized_segment: Segment) -> Result<SegmentMeta> {
        pgrx::debug1!(
            "writer {}: committing segment {}",
            self.id,
            finalized_segment.id()
        );
        let previous_metas = self.new_metas.clone();
        let new_meta = finalized_segment.meta().clone();
        self.new_metas.push(new_meta.clone());
        self.save_metas(self.new_metas.clone(), previous_metas)?;
        Ok(new_meta)
    }

    fn save_metas(
        &mut self,
        new_metas: Vec<SegmentMeta>,
        previous_metas: Vec<SegmentMeta>,
    ) -> Result<()> {
        let current_metas = self.index.load_metas()?;
        let previous_index_meta = IndexMeta {
            segments: previous_metas,
            ..current_metas.clone()
        };
        let new_index_meta = IndexMeta {
            segments: new_metas,
            ..current_metas.clone()
        };
        self.index
            .directory()
            .save_metas(&new_index_meta, &previous_index_meta, &mut ())?;
        Ok(())
    }
}

pub struct SearchIndexMerger {
    merged_segment_ids: HashSet<SegmentId>,
    index: Index,
    directory: MVCCDirectory,
}

impl SearchIndexMerger {
    pub fn open(directory: MVCCDirectory) -> Result<SearchIndexMerger> {
        let index = Index::open(directory.clone())?;
        Ok(Self {
            index,
            merged_segment_ids: Default::default(),
            directory,
        })
    }

    pub fn all_entries(&self) -> HashMap<SegmentId, SegmentMetaEntry> {
        self.directory.all_entries()
    }

    pub fn searchable_segment_ids(&self) -> tantivy::Result<HashSet<SegmentId>> {
        Ok(self.index.searchable_segment_ids()?.into_iter().collect())
    }

    /// Only keep pins on the specified segments, releasing pins on all other segments.
    pub fn adjust_pins<'a>(
        mut self,
        segment_ids: impl Iterator<Item = &'a SegmentId>,
    ) -> tantivy::Result<impl Mergeable> {
        let keep = segment_ids.cloned().collect::<HashSet<_>>();
        let current = self.searchable_segment_ids()?;
        let remove = current.difference(&keep);

        for segment_id in remove {
            unsafe {
                // SAFETY:  we (SegmentIndexMerger) promise not to reference or otherwise
                // use the segments that we're no longer pinning
                self.directory.drop_pin(segment_id);
            }
        }
        Ok(self)
    }
}

pub trait Mergeable {
    /// Merge the specified [`SegmentId`]s together into a new segment.  This is a blocking,
    /// foreground operation.
    ///
    /// Once the segments are merged, we drop the pin held on each one which allows for subsequent
    /// merges to potentially use their previously-occupied space.
    ///
    /// It is your responsibility to ensure any necessary locking is handled externally
    ///
    /// # Panics
    ///
    /// Will panic if a segment_id has already been merged or if our internal tantivy communications
    /// channels fail for some reason.
    fn merge_segments(&mut self, segment_ids: &[SegmentId]) -> Result<Option<SegmentMeta>>;
}

impl Mergeable for SearchIndexMerger {
    fn merge_segments(&mut self, segment_ids: &[SegmentId]) -> Result<Option<SegmentMeta>> {
        assert!(
            segment_ids
                .iter()
                .all(|segment_id| !self.merged_segment_ids.contains(segment_id)),
            "segment was already merged by this merger instance"
        );

        let mut writer: IndexWriter = self.index.writer(15 * 1024 * 1024)?;
        let new_segment = writer.merge_foreground(segment_ids, true)?;
        unsafe {
            // SAFETY:  The important thing here is that these segments are not used in any way
            // after their pins are dropped, and [`SearchIndexMerger`] ensures that
            self.directory.drop_pins(segment_ids)?;
            self.merged_segment_ids.extend(segment_ids.iter().cloned());
        }

        Ok(new_segment)
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

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use crate::api::HashSet;
    use crate::postgres::rel::PgSearchRelation;
    use pgrx::prelude::*;
    use std::num::NonZeroUsize;

    fn get_relation_oid() -> pg_sys::Oid {
        Spi::run("SET client_min_messages = 'debug1';").unwrap();
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run("INSERT INTO t (data) VALUES ('test');").unwrap();
        Spi::run("CREATE INDEX t_idx ON t USING bm25(id, data) WITH (key_field = 'id')").unwrap();
        Spi::get_one::<pg_sys::Oid>(
            "SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';",
        )
        .expect("spi should succeed")
        .unwrap()
    }

    fn simulate_index_writer(
        config: IndexWriterConfig,
        relation_oid: pg_sys::Oid,
        num_docs: usize,
    ) -> HashSet<SegmentId> {
        let index_relation = PgSearchRelation::open(relation_oid);
        let mut writer =
            SerialIndexWriter::open(&index_relation, config, Default::default()).unwrap();
        let schema = writer.schema();
        let ctid_field = schema.ctid_field();
        let text_field = schema.search_field("data").unwrap().field();
        let mut segment_ids = HashSet::default();

        for i in 0..num_docs {
            let mut document = TantivyDocument::new();
            document.add_text(text_field, "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum. Curabitur pretium tincidunt lacus. Nulla gravida orci a odio. Nullam, turpis et commodo pharetra, est eros bibendum elit, nec luctus magna felis sollicitudin mauris. Integer in mauris eu nibh euismod gravida. Duis ac tellus et risus vulputate vehicula. Donec lobortis risus a elit. Etiam tempor.");
            document.add_u64(ctid_field, i as u64);
            if let Some(meta) = writer.insert(document, i as u64).unwrap() {
                segment_ids.insert(meta.id());
            }
        }

        segment_ids.extend(writer.commit().unwrap().iter().map(|(meta, _)| meta.id()));
        segment_ids
    }

    #[pg_test]
    fn test_index_writer_mem_budget() {
        let relation_oid = get_relation_oid();
        let config = IndexWriterConfig {
            memory_budget: NonZeroUsize::new(15 * 1024 * 1024).unwrap(),
            max_docs_per_segment: None,
        };
        let segment_ids = simulate_index_writer(config, relation_oid, 8);
        assert_eq!(segment_ids.len(), 1);

        let config = IndexWriterConfig {
            memory_budget: NonZeroUsize::new(15 * 1024 * 1024).unwrap(),
            max_docs_per_segment: None,
        };
        let segment_ids = simulate_index_writer(config, relation_oid, 25000);
        assert_eq!(segment_ids.len(), 5);
    }

    #[pg_test]
    fn test_index_writer_max_docs_per_segment() {
        let relation_oid = get_relation_oid();
        let config = IndexWriterConfig {
            memory_budget: NonZeroUsize::new(15 * 1024 * 1024).unwrap(),
            max_docs_per_segment: Some(1000),
        };
        let segment_ids = simulate_index_writer(config, relation_oid, 25000);
        assert_eq!(segment_ids.len(), 25);
    }
}
