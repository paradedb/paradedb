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
use tantivy::index::SegmentId;
use tantivy::indexer::{AddOperation, SegmentWriter};
use tantivy::schema::Field;
use tantivy::{Directory, Index, IndexMeta, IndexWriter, Segment, SegmentMeta, TantivyDocument};
use thiserror::Error;

use crate::index::mvcc::{MVCCDirectory, MvccSatisfies};
use crate::index::setup_tokenizers;
use crate::postgres::storage::block::SegmentMetaEntry;
use crate::{postgres::types::TantivyValueError, schema::SearchIndexSchema};

/// Unlike Tantivy's IndexWriter, the SerialIndexWriter does not spin up any threads.
/// Everything happens in the foreground, making it ideal for Postgres.
pub struct SerialIndexWriter {
    indexrelid: pg_sys::Oid,
    ctid_field: Field,
    current_segment: Option<(Segment, SegmentWriter)>,
    memory_budget: usize,
    index: Index,
    new_metas: Vec<SegmentMeta>,
}

impl SerialIndexWriter {
    pub fn open(index_relation: &PgRelation, memory_budget: usize) -> Result<Self> {
        Self::with_mvcc(index_relation, MvccSatisfies::Snapshot, memory_budget)
    }

    pub fn with_mvcc(
        index_relation: &PgRelation,
        mvcc_satisfies: MvccSatisfies,
        memory_budget: usize,
    ) -> Result<Self> {
        let directory = mvcc_satisfies.directory(index_relation);
        let mut index = Index::open(directory)?;
        let schema = SearchIndexSchema::open(index_relation.oid())?;
        setup_tokenizers(index_relation.oid(), &mut index)?;
        let ctid_field = schema.ctid_field();

        Ok(Self {
            indexrelid: index_relation.oid(),
            ctid_field,
            memory_budget,
            index,
            current_segment: Default::default(),
            new_metas: Default::default(),
        })
    }

    pub fn index_oid(&self) -> pg_sys::Oid {
        self.indexrelid
    }

    pub fn insert(&mut self, mut document: TantivyDocument, ctid: u64) -> Result<()> {
        document.add_u64(self.ctid_field, ctid);

        if self.current_segment.is_none() {
            let new_segment = self.index.new_segment();
            let new_writer = SegmentWriter::for_segment(self.memory_budget, new_segment.clone())?;
            self.current_segment = Some((new_segment, new_writer));
        }

        self.current_segment
            .as_mut()
            .unwrap()
            .1
            .add_document(AddOperation {
                opstamp: 0,
                document,
            })?;

        if self.memory_budget <= self.current_segment.as_ref().unwrap().1.mem_usage() {
            self.finalize_segment()?;
        }

        Ok(())
    }

    pub fn commit(mut self) -> Result<Vec<SegmentId>> {
        self.finalize_segment()?;

        // Save new metas
        let current_metas = self.index.load_metas()?;
        let current_segments = current_metas.clone().segments;
        let segment_ids = current_segments
            .iter()
            .map(|meta| meta.id())
            .collect::<Vec<_>>();
        let new_metas = IndexMeta {
            segments: [self.new_metas, current_segments].concat(),
            ..current_metas.clone()
        };
        self.index
            .directory()
            .save_metas(&new_metas, &current_metas, &mut ())?;
        Ok(segment_ids)
    }

    fn finalize_segment(&mut self) -> Result<()> {
        let Some((segment, writer)) = self.current_segment.take() else {
            // no docs were ever added
            return Ok(());
        };

        let max_doc = writer.max_doc();
        writer.finalize()?;
        let segment = segment.with_max_doc(max_doc);
        self.new_metas.push(segment.meta().clone());
        Ok(())
    }
}

pub struct SearchIndexMerger {
    directory: MVCCDirectory,
    writer: IndexWriter,
    merged_segment_ids: HashSet<SegmentId>,
}

impl SearchIndexMerger {
    pub fn open(relation_id: pg_sys::Oid) -> Result<SearchIndexMerger> {
        let directory = MVCCDirectory::mergeable(relation_id);
        let index = Index::open(directory.clone())?;
        let writer = index.writer(15 * 1024 * 1024)?;

        Ok(Self {
            directory,
            writer,
            merged_segment_ids: Default::default(),
        })
    }

    pub fn all_entries(&self) -> HashMap<SegmentId, SegmentMetaEntry> {
        self.directory.all_entries()
    }

    pub fn segment_ids(&mut self) -> tantivy::Result<HashSet<SegmentId>> {
        Ok(self
            .writer
            .index()
            .searchable_segment_ids()?
            .into_iter()
            .collect())
    }

    /// Only keep pins on the specified segments, releasing pins on all other segments.
    pub fn adjust_pins<'a>(
        mut self,
        segment_ids: impl Iterator<Item = &'a SegmentId>,
    ) -> tantivy::Result<impl Mergeable> {
        let keep = segment_ids.cloned().collect::<HashSet<_>>();
        let current = self.segment_ids()?;
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

        let new_segment = self.writer.merge_foreground(segment_ids, true)?;
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
