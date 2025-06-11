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
use pgrx::{pg_sys, PgRelation};
use tantivy::directory::RamDirectory;
use tantivy::index::SegmentId;
use tantivy::indexer::merger::IndexMerger;
use tantivy::indexer::segment_serializer::SegmentSerializer;
use tantivy::indexer::{AddOperation, SegmentWriter};
use tantivy::schema::{Field, Schema};
use tantivy::{Directory, Index, IndexMeta, IndexWriter, Segment, SegmentMeta, TantivyDocument};
use thiserror::Error;

use crate::index::mvcc::{MVCCDirectory, MvccSatisfies};
use crate::index::setup_tokenizers;
use crate::postgres::storage::block::SegmentMetaEntry;
use crate::{postgres::types::TantivyValueError, schema::SearchIndexSchema};

#[derive(Clone, Debug)]
enum DirectoryType {
    Mvcc,
    Ram,
}

struct PendingSegment {
    segment: Segment,
    writer: SegmentWriter,
    directory_type: DirectoryType,
}

impl PendingSegment {
    fn new_ram(index: &Index, memory_budget: usize) -> Result<Self> {
        let segment = index.new_segment();
        let writer = SegmentWriter::for_segment(memory_budget, segment.clone())?;
        Ok(Self {
            segment,
            writer,
            directory_type: DirectoryType::Ram,
        })
    }

    fn new_mvcc(index: &Index, memory_budget: usize) -> Result<Self> {
        let segment = index.new_segment();
        let writer = SegmentWriter::for_segment(memory_budget, segment.clone())?;
        Ok(Self {
            segment,
            writer,
            directory_type: DirectoryType::Mvcc,
        })
    }

    fn add_document(&mut self, document: TantivyDocument) -> Result<()> {
        self.writer.add_document(AddOperation {
            opstamp: 0,
            document,
        })?;
        Ok(())
    }

    fn directory_type(&self) -> DirectoryType {
        self.directory_type.clone()
    }

    fn mem_usage(&self) -> usize {
        match self.directory_type {
            DirectoryType::Ram => {
                let writer_mem_usage = self.writer.mem_usage();
                let directory = self.segment.index().directory();
                let directory_mem_usage = directory
                    .inner()
                    .as_any()
                    .downcast_ref::<RamDirectory>()
                    .expect("directory should be a RamDirectory")
                    .total_mem_usage();
                writer_mem_usage + directory_mem_usage
            }
            DirectoryType::Mvcc => self.writer.mem_usage(),
        }
    }

    fn finalize(self) -> Result<Segment> {
        let max_doc = self.writer.max_doc();
        self.writer.finalize()?;
        let segment = self.segment.with_max_doc(max_doc);
        Ok(segment)
    }
}

/// Unlike Tantivy's IndexWriter, the SerialIndexWriter does not spin up any threads.
/// Everything happens in the foreground, making it ideal for Postgres.
pub struct SerialIndexWriter {
    indexrelid: pg_sys::Oid,
    ctid_field: Field,
    pending_segment: Option<PendingSegment>,
    last_flushed_segment_meta: Option<SegmentMeta>,
    memory_budget: usize,
    index: Index,
    target_docs_per_segment: Option<usize>,
    max_doc: Option<usize>,
    mvcc_satisfies: MvccSatisfies,
}

impl SerialIndexWriter {
    pub fn open(
        index_relation: &PgRelation,
        memory_budget: usize,
        target_docs_per_segment: Option<usize>,
    ) -> Result<Self> {
        Self::with_mvcc(
            index_relation,
            MvccSatisfies::Snapshot,
            memory_budget,
            target_docs_per_segment,
        )
    }

    pub fn with_mvcc(
        index_relation: &PgRelation,
        mvcc_satisfies: MvccSatisfies,
        memory_budget: usize,
        target_docs_per_segment: Option<usize>,
    ) -> Result<Self> {
        let directory = mvcc_satisfies.clone().directory(index_relation);
        let mut index = Index::open(directory)?;
        let schema = SearchIndexSchema::open(index_relation.oid())?;
        setup_tokenizers(index_relation.oid(), &mut index)?;
        let ctid_field = schema.ctid_field();

        Ok(Self {
            indexrelid: index_relation.oid(),
            ctid_field,
            memory_budget,
            index,
            pending_segment: Default::default(),
            last_flushed_segment_meta: Default::default(),
            max_doc: Default::default(),
            target_docs_per_segment,
            mvcc_satisfies,
        })
    }

    pub fn index_oid(&self) -> pg_sys::Oid {
        self.indexrelid
    }

    pub fn insert(&mut self, mut document: TantivyDocument, ctid: u64) -> Result<()> {
        document.add_u64(self.ctid_field, ctid);

        if self.pending_segment.is_none() {
            self.pending_segment = Some(self.new_segment()?);
        }

        self.pending_segment
            .as_mut()
            .unwrap()
            .add_document(document)?;

        let mem_usage = self.pending_segment.as_ref().unwrap().mem_usage();
        if mem_usage >= self.memory_budget {
            self.finalize_segment()?;
        }

        Ok(())
    }

    pub fn commit(mut self) -> Result<()> {
        self.finalize_segment()
    }

    fn save_metas(&mut self, segments: Vec<SegmentMeta>, current_metas: &IndexMeta) -> Result<()> {
        let new_metas = IndexMeta {
            segments,
            ..current_metas.clone()
        };
        self.index
            .directory()
            .save_metas(&new_metas, &current_metas, &mut ())?;
        Ok(())
    }

    fn new_segment(&mut self) -> Result<PendingSegment> {
        if self.target_docs_per_segment.is_none() || self.last_flushed_segment_meta.is_none() {
            return PendingSegment::new_mvcc(&self.index, self.memory_budget);
        }

        let previous_num_docs = self.last_flushed_segment_meta.as_ref().unwrap().max_doc() as usize;
        let target_docs_per_segment = self.target_docs_per_segment.unwrap();

        if previous_num_docs + self.max_doc.unwrap_or_default() > target_docs_per_segment {
            return PendingSegment::new_mvcc(&self.index, self.memory_budget);
        }

        let mut index = Index::create_in_ram(self.index.schema());
        setup_tokenizers(self.indexrelid, &mut index)?;
        PendingSegment::new_ram(&index, self.memory_budget)
    }

    fn finalize_segment(&mut self) -> Result<()> {
        let Some(pending_segment) = self.pending_segment.take() else {
            // no docs were ever added
            return Ok(());
        };

        let directory_type = pending_segment.directory_type();
        let finalized_segment = pending_segment.finalize()?;

        if self.max_doc.is_none() {
            self.max_doc = Some(finalized_segment.meta().num_docs() as usize);
        }

        let current_metas = self.load_metas()?;
        let mut segments = current_metas.clone().segments;

        match directory_type {
            DirectoryType::Ram => {
                let last_flushed_segment = self
                    .index
                    .segment(self.last_flushed_segment_meta.as_ref().unwrap().clone());
                let last_flushed_segment_id = last_flushed_segment.id();
                let mut merger = SearchIndexMerger::open(self.index.clone())?;
                let merged_segment_meta =
                    merger.merge_into(&[finalized_segment, last_flushed_segment])?;

                if let Some(merged_segment_meta) = merged_segment_meta {
                    segments.retain(|segment| segment.id() != last_flushed_segment_id);
                    segments.push(merged_segment_meta.clone());
                    self.save_metas(segments, &current_metas)?;
                    self.last_flushed_segment_meta = Some(merged_segment_meta);
                }
            }
            DirectoryType::Mvcc => {
                // Add the new segment to the list of segments
                segments.push(finalized_segment.meta().clone());
                self.save_metas(segments, &current_metas)?;
                self.last_flushed_segment_meta = Some(finalized_segment.meta().clone());
            }
        };

        Ok(())
    }

    fn load_metas(&mut self) -> Result<IndexMeta> {
        let index_relation = unsafe { PgRelation::open(self.indexrelid) };
        let directory = self.mvcc_satisfies.clone().directory(&index_relation);
        let index = Index::open(directory)?;
        Ok(index.load_metas()?)
    }
}

pub struct SearchIndexMerger {
    merged_segment_ids: HashSet<SegmentId>,
    index: Index,
    directory: MVCCDirectory,
}

impl SearchIndexMerger {
    pub fn open(index: Index) -> Result<SearchIndexMerger> {
        let directory = index.directory().inner().as_any().downcast_ref::<MVCCDirectory>().unwrap().clone();
        Ok(Self {
            index,
            merged_segment_ids: Default::default(),
            directory,
        })
    }

    pub fn all_entries(&self) -> HashMap<SegmentId, SegmentMetaEntry> {
        self.directory.all_entries()
    }

    pub fn searchable_segment_ids(&mut self) -> tantivy::Result<HashSet<SegmentId>> {
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

    /// Merge any [`Segment`]s into the current index.
    ///
    /// These segments can come from anywhere and can be backed by different Index/Directory types.
    /// For instance, you can merge a RamDirectory-backed segment with a MVCCDirectory-backed segment.
    ///
    /// Unlike merge_segments, this method does not update the metas list because it has no knowledge of
    /// what index the provided segments belong to.
    fn merge_into(&mut self, segments: &[Segment]) -> Result<Option<SegmentMeta>>;
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

    fn merge_into(&mut self, segments: &[Segment]) -> Result<Option<SegmentMeta>> {
        assert!(
            segments
                .iter()
                .all(|segment| !self.merged_segment_ids.contains(&segment.id())),
            "segment was already merged by this merger instance"
        );

        let num_docs = segments
            .iter()
            .map(|segment| segment.meta().num_docs() as u64)
            .sum::<u64>();
        if num_docs == 0 {
            return Ok(None);
        }

        let merged_segment = self.index.new_segment();
        let merger = IndexMerger::open(
            self.index.schema(),
            &segments[..],
            {
                let index = self.index.clone();
                Box::new(move || index.directory().wants_cancel())
            },
            true,
        )?;
        let segment_serializer = SegmentSerializer::for_segment(merged_segment.clone())?;
        let num_docs = merger.write(segment_serializer)?;
        let segment_meta = self.index.new_segment_meta(merged_segment.id(), num_docs);

        self.merged_segment_ids.insert(merged_segment.id());
        Ok(Some(segment_meta))
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
