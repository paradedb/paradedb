// Copyright (c) 2023-2026 ParadeDB, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

//! A CTID-sorted segment maps heap tuple IDs (block, offset) to document IDs using
//! Tantivy `OptionalIndex` bitmaps.
//!
//! In Postgres, an 8 KB heap block supports at most 291 heap tuples:
//! `(BLCKSZ - SizeOfPageHeaderData) / (MAXALIGN(sizeof(HeapTupleHeaderData)) + sizeof(ItemIdData))`
//! = `(8192 - 24) / (24 + 4) = 291`.
//!
//! A single `OptionalIndex` supports up to `u32::MAX` (4,294,967,295) bits, covering up to
//! `4,294,967,295 / 291 = 14,759,337` blocks (~120.9 GB of heap). When a segment's block range
//! exceeds this capacity, the address space is partitioned across multiple `OptionalIndex` chunks
//! in `.ctid_map`.
//!
//! Because documents are indexed in CTID order, the bitmap provides dual O(1) operations
//! without requiring a separate column of document boundaries:
//!
//! - `rank(row_id)` gives document boundaries: the count of tuples strictly preceding `row_id`.
//! - `rank_if_exists(row_id)` returns `Some(DocId)` if a tuple exists in the segment.
//! - `select(doc_id)` returns the exact `(BlockNumber, OffsetNumber)` for a document.

use std::ops::Range;
use std::sync::OnceLock;

use anyhow::{Context, bail};
use pgrx::pg_sys::{BlockNumber, InvalidBlockNumber, OffsetNumber};
use tantivy::DocId;
use tantivy::columnar::column_index::{
    OptionalIndex, OptionalIndexSelectCursor, Set, open_optional_index, serialize_optional_index,
};
use tantivy::columnar::{Cardinality, ColumnarReader, DynamicColumn, Iterable};
use tantivy::directory::error::OpenReadError;
use tantivy::directory::{CompositeFile, CompositeWrite};
use tantivy::index::{Segment, SegmentComponent, SegmentReader};
use tantivy::schema::Field;

use crate::api::CTID_FIELD_NAME;
use crate::index::reader::index::SearchIndexReader;

mod plugin;
pub(crate) use plugin::register;

/// Maximum number of heap tuples physically possible on a standard 8 KB Postgres heap page:
/// `(BLCKSZ - SizeOfPageHeaderData) / (MAXALIGN(sizeof(HeapTupleHeaderData)) + sizeof(ItemIdData))`
/// = `(8192 - 24) / (24 + 4) = 291`.
pub(crate) const OFFSETS_PER_BLOCK: u32 = 291;

/// Maximum number of heap blocks that can be addressed by a single Tantivy `OptionalIndex`.
/// Since `OptionalIndex` uses 32-bit row IDs (`u32::MAX` = 4,294,967,295), this is:
/// `4,294,967,295 / 291 = 14,759,337` blocks (approx 120.9 GB of heap).
pub(crate) const BLOCKS_PER_CHUNK: u32 = u32::MAX / OFFSETS_PER_BLOCK;

/// Number of row IDs allocated per chunk in `OptionalIndex`.
pub(crate) const ROWS_PER_CHUNK: u32 = BLOCKS_PER_CHUNK * OFFSETS_PER_BLOCK; // 4_294_967_067

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ChunkIndex(pub(crate) usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct InChunkRowId(pub(crate) u32);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct TidAddress {
    pub(crate) chunk: ChunkIndex,
    pub(crate) row_id: InChunkRowId,
}

impl TidAddress {
    #[inline]
    pub(crate) fn from_parts(
        first_block: BlockNumber,
        block: BlockNumber,
        offset: OffsetNumber,
    ) -> Option<Self> {
        if offset == 0 || offset as u32 > OFFSETS_PER_BLOCK || block < first_block {
            return None;
        }
        let rel_block = (block - first_block) as u64;
        let chunk = (rel_block / BLOCKS_PER_CHUNK as u64) as usize;
        let in_chunk_block = (rel_block % BLOCKS_PER_CHUNK as u64) as u32;
        let row_id = in_chunk_block * OFFSETS_PER_BLOCK + (offset as u32 - 1);
        Some(Self {
            chunk: ChunkIndex(chunk),
            row_id: InChunkRowId(row_id),
        })
    }

    #[inline]
    pub(crate) fn boundary(first_block: BlockNumber, block: BlockNumber) -> Self {
        if block < first_block {
            return Self {
                chunk: ChunkIndex(0),
                row_id: InChunkRowId(0),
            };
        }
        let rel_block = (block - first_block) as u64;
        let chunk = (rel_block / BLOCKS_PER_CHUNK as u64) as usize;
        let in_chunk_block = (rel_block % BLOCKS_PER_CHUNK as u64) as u32;
        let row_id = in_chunk_block * OFFSETS_PER_BLOCK;
        Self {
            chunk: ChunkIndex(chunk),
            row_id: InChunkRowId(row_id),
        }
    }

    #[inline]
    #[allow(dead_code)]
    pub(crate) fn to_tid(
        first_block: BlockNumber,
        chunk: ChunkIndex,
        row_id: InChunkRowId,
    ) -> (BlockNumber, OffsetNumber) {
        let in_chunk_block = row_id.0 / OFFSETS_PER_BLOCK;
        let offset_idx = row_id.0 % OFFSETS_PER_BLOCK;
        let rel_block = (chunk.0 as u64 * BLOCKS_PER_CHUNK as u64) + in_chunk_block as u64;
        let block = first_block + rel_block as BlockNumber;
        let offset = (offset_idx + 1) as OffsetNumber;
        (block, offset)
    }
}

struct Replayable<I>(I);

impl<T, I> Iterable<T> for Replayable<I>
where
    I: Iterator<Item = T> + Clone,
{
    fn boxed_iter(&self) -> Box<dyn Iterator<Item = T> + '_> {
        Box::new(self.0.clone())
    }
}

/// Builds the TID offset bitmap chunks from live CTIDs at flush or merge.
pub(super) fn write(segment: &Segment, out: &mut CompositeWrite) -> anyhow::Result<()> {
    if !segment
        .index()
        .settings()
        .sort_by_field
        .as_ref()
        .is_some_and(|sort| sort.field == CTID_FIELD_NAME)
    {
        return Ok(());
    }
    let schema = segment.schema();
    let Ok(field) = schema.get_field(CTID_FIELD_NAME) else {
        return Ok(());
    };
    let fast = ColumnarReader::open(segment.open_read(SegmentComponent::FastFields)?)?;
    let handles = fast.read_columns(CTID_FIELD_NAME)?;
    let [handle] = handles.as_slice() else {
        return Ok(());
    };
    let DynamicColumn::U64(column) = handle.open()? else {
        return Ok(());
    };
    let docs = column.num_docs();
    if docs == 0 || column.get_cardinality() != Cardinality::Full {
        return Ok(());
    }
    let descending = column.values.get_val(0) > column.values.get_val(docs - 1);
    let first_block =
        u32::try_from(column.values.get_val(if descending { docs - 1 } else { 0 }) >> 16)
            .context("heap block number exceeds BlockNumber")?;
    let last_block =
        u32::try_from(column.values.get_val(if descending { 0 } else { docs - 1 }) >> 16)
            .context("heap block number exceeds BlockNumber")?;
    if first_block > last_block || last_block == InvalidBlockNumber {
        bail!("invalid heap-block boundaries");
    }

    let total_blocks = u64::from(last_block) - u64::from(first_block) + 1;
    let num_chunks = usize::try_from(total_blocks.div_ceil(BLOCKS_PER_CHUNK as u64))
        .context("too many chunks")?;

    let mut current_doc_start = 0usize;
    for chunk_idx in 0..num_chunks {
        let chunk_end_block_u64 =
            u64::from(first_block) + ((chunk_idx as u64 + 1) * BLOCKS_PER_CHUNK as u64);

        let doc_end = if chunk_idx + 1 == num_chunks {
            docs as usize
        } else {
            let mut left = current_doc_start;
            let mut right = docs as usize;
            while left < right {
                let mid = left + (right - left) / 2;
                let source_doc = if descending {
                    docs as usize - mid - 1
                } else {
                    mid
                };
                if (column.values.get_val(source_doc as u32) >> 16) < chunk_end_block_u64 {
                    left = mid + 1;
                } else {
                    right = mid;
                }
            }
            left
        };

        let doc_start = current_doc_start;
        current_doc_start = doc_end;

        if doc_start >= doc_end {
            continue;
        }

        let chunk_num_rows = if chunk_idx + 1 < num_chunks {
            ROWS_PER_CHUNK
        } else {
            let remaining_blocks = (total_blocks - 1) % (BLOCKS_PER_CHUNK as u64) + 1;
            (remaining_blocks as u32) * OFFSETS_PER_BLOCK
        };

        let column_ref = &column;
        let row_ids = move || {
            (doc_start..doc_end).map(move |doc| {
                if doc.is_multiple_of(8192) {
                    pgrx::check_for_interrupts!();
                }
                let source_doc = if descending {
                    docs as usize - doc - 1
                } else {
                    doc
                };
                let val = column_ref.values.get_val(source_doc as u32);
                let block = (val >> 16) as BlockNumber;
                let offset = val as OffsetNumber;
                let addr = TidAddress::from_parts(first_block, block, offset)
                    .expect("invalid TID address");
                addr.row_id.0
            })
        };

        serialize_optional_index(
            &Replayable(row_ids()),
            chunk_num_rows,
            out.for_field_with_idx(field, chunk_idx),
        )?;
    }

    Ok(())
}

pub(crate) struct BlockToDocIdMap {
    first_block: BlockNumber,
    last_block: BlockNumber,
    num_docs: u32,
    num_chunks: usize,
    #[allow(dead_code)]
    descending: bool,
    file: CompositeFile,
    field: Field,
    chunks: Vec<OnceLock<Option<OptionalIndex>>>,
    docs_before_chunk: OnceLock<Vec<u32>>,
}

#[allow(dead_code)]
pub(crate) type TidOffsetMap = BlockToDocIdMap;

impl BlockToDocIdMap {
    /// Opens the optional component, reusing CTID statistics for its block bounds.
    pub(crate) fn open(segment: &SegmentReader) -> anyhow::Result<Option<Self>> {
        let slice = match segment.open_read(plugin::component()) {
            Ok(slice) => slice,
            Err(OpenReadError::FileDoesNotExist(_)) => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let file = CompositeFile::open(&slice)?;
        let field = segment.schema().get_field(CTID_FIELD_NAME)?;
        let Some(blocks) = SearchIndexReader::block_bounds(segment)? else {
            return Ok(None);
        };
        let first_block = *blocks.start();
        let last_block = *blocks.end();
        let num_docs = segment.max_doc();
        if num_docs == 0 || first_block > last_block || last_block == InvalidBlockNumber {
            bail!("invalid heap-block boundaries");
        }
        let total_blocks = u64::from(last_block) - u64::from(first_block) + 1;
        let num_chunks = usize::try_from(total_blocks.div_ceil(BLOCKS_PER_CHUNK as u64))
            .context("too many chunks")?;

        let has_any = (0..num_chunks).any(|idx| file.open_read_with_idx(field, idx).is_some());
        if !has_any {
            return Ok(None);
        }

        let mut chunks = Vec::with_capacity(num_chunks);
        for _ in 0..num_chunks {
            chunks.push(OnceLock::new());
        }

        Ok(Some(Self {
            first_block,
            last_block,
            num_docs,
            num_chunks,
            descending: false,
            file,
            field,
            chunks,
            docs_before_chunk: OnceLock::new(),
        }))
    }

    /// Configures descending sort order when translating doc IDs.
    #[allow(dead_code)]
    pub(crate) fn with_descending(mut self, descending: bool) -> Self {
        self.descending = descending;
        self
    }

    #[cfg(any(test, feature = "pg_test"))]
    pub(crate) fn for_test(
        first_block: BlockNumber,
        last_block: BlockNumber,
        num_docs: u32,
        file: CompositeFile,
        field: Field,
    ) -> Self {
        let total_blocks = u64::from(last_block) - u64::from(first_block) + 1;
        let num_chunks = usize::try_from(total_blocks.div_ceil(BLOCKS_PER_CHUNK as u64)).unwrap();
        let mut chunks = Vec::with_capacity(num_chunks);
        for _ in 0..num_chunks {
            chunks.push(OnceLock::new());
        }
        Self {
            first_block,
            last_block,
            num_docs,
            num_chunks,
            descending: false,
            file,
            field,
            chunks,
            docs_before_chunk: OnceLock::new(),
        }
    }

    /// Returns the heap-block span covered by the map.
    pub(crate) fn block_range(&self) -> Range<BlockNumber> {
        self.first_block..self.last_block + 1
    }

    /// Lazily loads and returns a reference to the `OptionalIndex` chunk at index `chunk_idx`.
    pub(crate) fn get_chunk(&self, chunk_idx: usize) -> anyhow::Result<Option<&OptionalIndex>> {
        if chunk_idx >= self.num_chunks {
            return Ok(None);
        }
        let opt = self.chunks[chunk_idx].get_or_init(|| {
            self.file
                .open_read_with_idx(self.field, chunk_idx)
                .and_then(|slice| open_optional_index(slice).ok())
        });
        Ok(opt.as_ref())
    }

    /// Computes the prefix sums of documents preceding each chunk.
    pub(crate) fn docs_before_chunk(&self) -> &[u32] {
        self.docs_before_chunk.get_or_init(|| {
            if self.num_chunks == 1 {
                return vec![0];
            }
            let mut docs_before = Vec::with_capacity(self.num_chunks);
            let mut total = 0u32;
            for chunk_idx in 0..self.num_chunks {
                docs_before.push(total);
                if let Ok(Some(chunk)) = self.get_chunk(chunk_idx) {
                    total += chunk.num_non_nulls();
                }
            }
            docs_before
        })
    }

    /// Given ranges of dirty heap blocks, returns the document ID ranges they map to in ascending CTID order.
    pub(crate) fn doc_id_ranges_for_blocks(
        &self,
        block_ranges: &[Range<BlockNumber>],
    ) -> anyhow::Result<Vec<Range<DocId>>> {
        let mut ranges: Vec<Range<DocId>> = Vec::with_capacity(block_ranges.len());
        for block_range in block_ranges {
            let start = self.rank(block_range.start, 1)?;
            let end = self.rank(block_range.end, 1)?;
            if start > end || end > self.num_docs {
                bail!("invalid heap-block boundaries");
            }
            if start == end {
                continue;
            }
            if let Some(last) = ranges.last_mut().filter(|last| last.end == start) {
                last.end = end;
            } else {
                ranges.push(start..end);
            }
        }
        Ok(ranges)
    }

    /// Given sorted block numbers, returns the starting doc ID of each block in ascending CTID order.
    #[allow(dead_code)]
    pub(crate) fn boundaries(&self, blocks: &[BlockNumber]) -> anyhow::Result<Vec<DocId>> {
        debug_assert!(blocks.is_sorted());
        if blocks.is_empty() {
            return Ok(Vec::new());
        }
        blocks.iter().map(|&block| self.rank(block, 1)).collect()
    }

    /// Returns the number of documents strictly preceding `(block, offset)` in ascending CTID order.
    pub(crate) fn rank(&self, block: BlockNumber, offset: OffsetNumber) -> anyhow::Result<DocId> {
        if block < self.first_block {
            return Ok(0);
        }
        if block > self.last_block {
            return Ok(self.num_docs);
        }
        let addr = TidAddress::boundary(self.first_block, block);
        let actual_row_id = if offset > 1 && (offset as u32) <= OFFSETS_PER_BLOCK {
            addr.row_id.0 + (offset as u32 - 1)
        } else {
            addr.row_id.0
        };
        let chunk_idx = addr.chunk.0;
        if chunk_idx >= self.num_chunks {
            return Ok(self.num_docs);
        }
        let docs_before = self.docs_before_chunk()[chunk_idx];
        let in_chunk_rank = match self.get_chunk(chunk_idx)? {
            Some(index) => index.rank(actual_row_id),
            None => 0,
        };
        Ok(docs_before + in_chunk_rank)
    }

    /// Checks whether a tuple ID exists in this segment.
    #[allow(dead_code)]
    pub(crate) fn contains_tid(
        &self,
        block: BlockNumber,
        offset: OffsetNumber,
    ) -> anyhow::Result<bool> {
        let Some(addr) = TidAddress::from_parts(self.first_block, block, offset) else {
            return Ok(false);
        };
        if addr.chunk.0 >= self.num_chunks {
            return Ok(false);
        }
        let Some(chunk) = self.get_chunk(addr.chunk.0)? else {
            return Ok(false);
        };
        Ok(chunk.contains(addr.row_id.0))
    }

    /// Looks up the document ID for a tuple ID, if present in this segment.
    #[allow(dead_code)]
    pub(crate) fn doc_id_for_tid(
        &self,
        block: BlockNumber,
        offset: OffsetNumber,
    ) -> anyhow::Result<Option<DocId>> {
        let Some(addr) = TidAddress::from_parts(self.first_block, block, offset) else {
            return Ok(None);
        };
        if addr.chunk.0 >= self.num_chunks {
            return Ok(None);
        }
        let Some(chunk) = self.get_chunk(addr.chunk.0)? else {
            return Ok(None);
        };
        let Some(in_chunk_rank) = chunk.rank_if_exists(addr.row_id.0) else {
            return Ok(None);
        };
        let rank = self.docs_before_chunk()[addr.chunk.0] + in_chunk_rank;
        let doc = if self.descending {
            self.num_docs - 1 - rank
        } else {
            rank
        };
        Ok(Some(doc))
    }

    /// Returns the tuple ID `(block, offset)` corresponding to a document ID.
    #[allow(dead_code)]
    pub(crate) fn tid_for_doc_id(&self, doc: DocId) -> anyhow::Result<(BlockNumber, OffsetNumber)> {
        if doc >= self.num_docs {
            bail!(
                "doc ID {doc} out of bounds for segment with {} docs",
                self.num_docs
            );
        }
        let rank = if self.descending {
            self.num_docs - 1 - doc
        } else {
            doc
        };
        let docs_before = self.docs_before_chunk();
        let chunk_idx = if self.num_chunks == 1 {
            0
        } else {
            docs_before
                .partition_point(|&before| before <= rank)
                .checked_sub(1)
                .context("invalid rank")?
        };
        let in_chunk_rank = rank - docs_before[chunk_idx];
        let chunk = self
            .get_chunk(chunk_idx)?
            .context("chunk unexpectedly empty")?;
        let in_chunk_row_id = chunk.select(in_chunk_rank);
        Ok(TidAddress::to_tid(
            self.first_block,
            ChunkIndex(chunk_idx),
            InChunkRowId(in_chunk_row_id),
        ))
    }

    /// Returns a cursor over this map that preserves select state across sequential lookups.
    pub(crate) fn cursor(&self) -> BlockToDocIdMapCursor<'_> {
        BlockToDocIdMapCursor::new(self)
    }

    /// Translates a slice of document IDs to `(BlockNumber, OffsetNumber)` tuples in batch.
    #[allow(dead_code)]
    pub(crate) fn tids_for_doc_ids(
        &self,
        doc_ids: &[DocId],
        out_tids: &mut [(BlockNumber, OffsetNumber)],
    ) -> anyhow::Result<()> {
        self.cursor().tids_for_doc_ids(doc_ids, out_tids)
    }

    /// Translates a slice of document IDs to raw packed CTIDs (`(block << 16) | offset`) in batch.
    #[allow(dead_code)]
    pub(crate) fn tids_for_doc_ids_as_u64s(
        &self,
        doc_ids: &[DocId],
        out_ctids: &mut [Option<u64>],
    ) -> anyhow::Result<()> {
        self.cursor().tids_for_doc_ids_as_u64s(doc_ids, out_ctids)
    }
}

/// A cursor over a [`BlockToDocIdMap`] that maintains internal chunk select state across sequential lookups.
pub(crate) struct BlockToDocIdMapCursor<'a> {
    map: &'a BlockToDocIdMap,
    docs_before: &'a [u32],
    current_chunk_idx: usize,
    chunk_cursor: Option<OptionalIndexSelectCursor<'a>>,
}

impl<'a> BlockToDocIdMapCursor<'a> {
    pub(crate) fn new(map: &'a BlockToDocIdMap) -> Self {
        Self {
            map,
            docs_before: map.docs_before_chunk(),
            current_chunk_idx: usize::MAX,
            chunk_cursor: None,
        }
    }

    fn ensure_chunk(&mut self, chunk_idx: usize) -> anyhow::Result<()> {
        if self.current_chunk_idx != chunk_idx {
            let Some(chunk) = self.map.get_chunk(chunk_idx)? else {
                bail!("ctid map chunk {chunk_idx} missing");
            };
            self.chunk_cursor = Some(chunk.select_cursor());
            self.current_chunk_idx = chunk_idx;
        }
        Ok(())
    }

    /// Translates a slice of document IDs to `(BlockNumber, OffsetNumber)` tuples.
    pub(crate) fn tids_for_doc_ids(
        &mut self,
        doc_ids: &[DocId],
        out_tids: &mut [(BlockNumber, OffsetNumber)],
    ) -> anyhow::Result<()> {
        assert_eq!(doc_ids.len(), out_tids.len());
        if doc_ids.is_empty() {
            return Ok(());
        }

        let first_block = self.map.first_block;
        let descending = self.map.descending;
        let num_docs = self.map.num_docs;
        let docs_before = self.docs_before;

        if self.map.num_chunks == 1 {
            self.ensure_chunk(0)?;
            let cursor = self.chunk_cursor.as_mut().unwrap();
            if descending {
                for (out, &doc) in out_tids.iter_mut().rev().zip(doc_ids.iter().rev()) {
                    let rank = num_docs - 1 - doc;
                    let row_id = cursor.select(rank);
                    *out = TidAddress::to_tid(first_block, ChunkIndex(0), InChunkRowId(row_id));
                }
            } else {
                for (out, &doc) in out_tids.iter_mut().zip(doc_ids) {
                    let row_id = cursor.select(doc);
                    *out = TidAddress::to_tid(first_block, ChunkIndex(0), InChunkRowId(row_id));
                }
            }
            return Ok(());
        }

        let mut i = 0;
        while i < doc_ids.len() {
            let doc = doc_ids[i];
            let rank = if descending { num_docs - 1 - doc } else { doc };
            let chunk_idx = docs_before
                .partition_point(|&before| before <= rank)
                .checked_sub(1)
                .context("invalid rank")?;
            let chunk_next_before = docs_before.get(chunk_idx + 1).copied().unwrap_or(num_docs);

            let chunk_start = i;
            let mut chunk_end = i + 1;
            while chunk_end < doc_ids.len() {
                let d = doc_ids[chunk_end];
                let r = if descending { num_docs - 1 - d } else { d };
                if r >= docs_before[chunk_idx] && r < chunk_next_before {
                    chunk_end += 1;
                } else {
                    break;
                }
            }

            self.ensure_chunk(chunk_idx)?;
            let cursor = self.chunk_cursor.as_mut().unwrap();
            let chunk_doc_ids = &doc_ids[chunk_start..chunk_end];
            let chunk_out = &mut out_tids[chunk_start..chunk_end];
            let chunk_docs_before = docs_before[chunk_idx];
            if descending {
                for (out, &d) in chunk_out.iter_mut().rev().zip(chunk_doc_ids.iter().rev()) {
                    let r = num_docs - 1 - d - chunk_docs_before;
                    let row_id = cursor.select(r);
                    *out = TidAddress::to_tid(
                        first_block,
                        ChunkIndex(chunk_idx),
                        InChunkRowId(row_id),
                    );
                }
            } else {
                for (out, &d) in chunk_out.iter_mut().zip(chunk_doc_ids) {
                    let r = d - chunk_docs_before;
                    let row_id = cursor.select(r);
                    *out = TidAddress::to_tid(
                        first_block,
                        ChunkIndex(chunk_idx),
                        InChunkRowId(row_id),
                    );
                }
            }

            i = chunk_end;
        }

        Ok(())
    }

    /// Translates a slice of document IDs to raw packed CTIDs (`(block << 16) | offset`) in batch.
    pub(crate) fn tids_for_doc_ids_as_u64s(
        &mut self,
        doc_ids: &[DocId],
        out_ctids: &mut [Option<u64>],
    ) -> anyhow::Result<()> {
        assert_eq!(doc_ids.len(), out_ctids.len());
        if doc_ids.is_empty() {
            return Ok(());
        }

        let first_block = self.map.first_block;
        let descending = self.map.descending;
        let num_docs = self.map.num_docs;
        let docs_before = self.docs_before;

        if self.map.num_chunks == 1 {
            self.ensure_chunk(0)?;
            let cursor = self.chunk_cursor.as_mut().unwrap();
            if descending {
                for (out, &doc) in out_ctids.iter_mut().rev().zip(doc_ids.iter().rev()) {
                    let rank = num_docs - 1 - doc;
                    let row_id = cursor.select(rank);
                    let (block, offset) =
                        TidAddress::to_tid(first_block, ChunkIndex(0), InChunkRowId(row_id));
                    *out = Some(((block as u64) << 16) | (offset as u64));
                }
            } else {
                for (out, &doc) in out_ctids.iter_mut().zip(doc_ids) {
                    let row_id = cursor.select(doc);
                    let (block, offset) =
                        TidAddress::to_tid(first_block, ChunkIndex(0), InChunkRowId(row_id));
                    *out = Some(((block as u64) << 16) | (offset as u64));
                }
            }
            return Ok(());
        }

        let mut i = 0;
        while i < doc_ids.len() {
            let doc = doc_ids[i];
            let rank = if descending { num_docs - 1 - doc } else { doc };
            let chunk_idx = docs_before
                .partition_point(|&before| before <= rank)
                .checked_sub(1)
                .context("invalid rank")?;
            let chunk_next_before = docs_before.get(chunk_idx + 1).copied().unwrap_or(num_docs);

            let chunk_start = i;
            let mut chunk_end = i + 1;
            while chunk_end < doc_ids.len() {
                let d = doc_ids[chunk_end];
                let r = if descending { num_docs - 1 - d } else { d };
                if r >= docs_before[chunk_idx] && r < chunk_next_before {
                    chunk_end += 1;
                } else {
                    break;
                }
            }

            self.ensure_chunk(chunk_idx)?;
            let cursor = self.chunk_cursor.as_mut().unwrap();
            let chunk_doc_ids = &doc_ids[chunk_start..chunk_end];
            let chunk_out = &mut out_ctids[chunk_start..chunk_end];
            let chunk_docs_before = docs_before[chunk_idx];
            if descending {
                for (out, &d) in chunk_out.iter_mut().rev().zip(chunk_doc_ids.iter().rev()) {
                    let r = num_docs - 1 - d - chunk_docs_before;
                    let row_id = cursor.select(r);
                    let (block, offset) = TidAddress::to_tid(
                        first_block,
                        ChunkIndex(chunk_idx),
                        InChunkRowId(row_id),
                    );
                    *out = Some(((block as u64) << 16) | (offset as u64));
                }
            } else {
                for (out, &d) in chunk_out.iter_mut().zip(chunk_doc_ids) {
                    let r = d - chunk_docs_before;
                    let row_id = cursor.select(r);
                    let (block, offset) = TidAddress::to_tid(
                        first_block,
                        ChunkIndex(chunk_idx),
                        InChunkRowId(row_id),
                    );
                    *out = Some(((block as u64) << 16) | (offset as u64));
                }
            }

            i = chunk_end;
        }

        Ok(())
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use std::slice;

    use pgrx::pg_test;
    use tantivy::columnar::column_values::CodecType;
    use tantivy::columnar::{ColumnType, ColumnarWriter};
    use tantivy::directory::{CompositeFile, CompositeWrite, RamDirectory, TerminatingWrite};
    use tantivy::index::{IndexSortByField, Order, SegmentComponent};
    use tantivy::schema::{FAST, Schema};
    use tantivy::{Index, IndexSettings};

    use super::*;

    #[pg_test]
    fn nullable_ctid_map_boundaries() {
        let block_size = 65_536;
        for blocks in [
            vec![10, 10, 10],
            vec![10, 10, 11, 11, 12],
            (10..1035).collect(),
            vec![10, 10, 11, 11, 11, 15, 15],
            vec![
                10,
                10 + block_size - 1,
                10 + block_size,
                10 + block_size * 3 + 2,
            ],
            vec![10, 10 + block_size - 2],
            (10..10 + block_size + 4)
                .filter(|block| block % 101 != 0)
                .collect(),
            vec![InvalidBlockNumber - 2, InvalidBlockNumber - 1],
            vec![0, InvalidBlockNumber - 1],
        ] {
            for descending in [false, true] {
                let mut schema = Schema::builder();
                let field = schema.add_u64_field(CTID_FIELD_NAME, FAST);
                let index = Index::create(
                    RamDirectory::create(),
                    schema.build(),
                    IndexSettings {
                        sort_by_field: Some(IndexSortByField {
                            field: CTID_FIELD_NAME.into(),
                            order: if descending { Order::Desc } else { Order::Asc },
                        }),
                        ..IndexSettings::default()
                    },
                )
                .unwrap();
                let segment = index.new_segment();
                let mut writer = ColumnarWriter::default();
                writer.record_column_type(CTID_FIELD_NAME, ColumnType::U64, false);

                let mut tids: Vec<(BlockNumber, OffsetNumber)> = Vec::with_capacity(blocks.len());
                let mut current_block = None;
                let mut current_offset = 1;
                for &block in &blocks {
                    if current_block == Some(block) {
                        current_offset += 1;
                    } else {
                        current_block = Some(block);
                        current_offset = 1;
                    }
                    tids.push((block, current_offset));
                }

                for doc in 0..blocks.len() {
                    let (block, offset) = tids[if descending {
                        blocks.len() - doc - 1
                    } else {
                        doc
                    }];
                    writer.record_numerical(
                        doc as DocId,
                        CTID_FIELD_NAME,
                        (u64::from(block) << 16) | u64::from(offset),
                    );
                }
                let mut fast = segment.open_write(SegmentComponent::FastFields).unwrap();
                writer
                    .serialize(
                        blocks.len() as DocId,
                        None,
                        &[CodecType::BlockwiseLinearV2],
                        &mut fast,
                    )
                    .unwrap();
                fast.terminate().unwrap();
                let first_block = blocks[0];
                let last_block = *blocks.last().unwrap();
                let mut output =
                    CompositeWrite::wrap(segment.open_write(plugin::component()).unwrap());
                write(&segment, &mut output).unwrap();
                output.close().unwrap();
                let file =
                    CompositeFile::open(&segment.open_read(plugin::component()).unwrap()).unwrap();
                let map = BlockToDocIdMap::for_test(
                    first_block,
                    last_block,
                    blocks.len() as DocId,
                    file,
                    field,
                )
                .with_descending(descending);

                let requested: Vec<BlockNumber> =
                    if u64::from(last_block) - u64::from(first_block) > 100_000 {
                        vec![first_block, first_block + 1, last_block, last_block + 1]
                    } else {
                        (first_block..=last_block + 1).collect()
                    };
                let expected: Vec<_> = requested
                    .iter()
                    .map(|&block| blocks.partition_point(|&present| present < block) as DocId)
                    .collect();
                assert_eq!(map.boundaries(&requested).unwrap(), expected);

                for &block in requested.iter().rev() {
                    let start = blocks.partition_point(|&present| present < block) as DocId;
                    assert_eq!(map.boundaries(&[block]).unwrap(), [start]);
                }
                assert_eq!(
                    map.doc_id_ranges_for_blocks(slice::from_ref(&(first_block..last_block + 1)))
                        .unwrap()
                        .as_slice(),
                    slice::from_ref(&(0..blocks.len() as DocId))
                );

                for (i, &(block, offset)) in tids.iter().enumerate() {
                    let expected_doc = if descending { blocks.len() - 1 - i } else { i } as DocId;
                    assert_eq!(
                        map.doc_id_for_tid(block, offset).unwrap(),
                        Some(expected_doc)
                    );
                    assert!(map.contains_tid(block, offset).unwrap());
                    assert_eq!(map.tid_for_doc_id(expected_doc).unwrap(), (block, offset));
                }
                assert_eq!(map.doc_id_for_tid(first_block, 290).unwrap(), None);
                assert!(!map.contains_tid(first_block, 290).unwrap());

                let mut batch_tids = vec![(0, 0); blocks.len()];
                let doc_ids: Vec<DocId> = (0..blocks.len() as DocId).collect();
                map.tids_for_doc_ids(&doc_ids, &mut batch_tids).unwrap();
                for (doc, &tid) in doc_ids.iter().zip(&batch_tids) {
                    assert_eq!(map.tid_for_doc_id(*doc).unwrap(), tid);
                }
                let mut batch_ctids = vec![None; blocks.len()];
                map.tids_for_doc_ids_as_u64s(&doc_ids, &mut batch_ctids)
                    .unwrap();
                for (&tid, &maybe_ctid) in batch_tids.iter().zip(&batch_ctids) {
                    let expected_ctid = (u64::from(tid.0) << 16) | u64::from(tid.1);
                    assert_eq!(maybe_ctid, Some(expected_ctid));
                }
            }
        }
    }

    #[pg_test]
    fn multi_chunk_boundaries_and_lookups() {
        let blocks = [10, 10 + BLOCKS_PER_CHUNK, 10 + 2 * BLOCKS_PER_CHUNK];
        for descending in [false, true] {
            let mut schema = Schema::builder();
            let field = schema.add_u64_field(CTID_FIELD_NAME, FAST);
            let index = Index::create(
                RamDirectory::create(),
                schema.build(),
                IndexSettings {
                    sort_by_field: Some(IndexSortByField {
                        field: CTID_FIELD_NAME.into(),
                        order: if descending { Order::Desc } else { Order::Asc },
                    }),
                    ..IndexSettings::default()
                },
            )
            .unwrap();
            let segment = index.new_segment();
            let mut writer = ColumnarWriter::default();
            writer.record_column_type(CTID_FIELD_NAME, ColumnType::U64, false);

            let tids: [(BlockNumber, OffsetNumber); 3] =
                [(blocks[0], 1), (blocks[1], 1), (blocks[2], 1)];

            for doc in 0..blocks.len() {
                let (block, offset) = tids[if descending {
                    blocks.len() - doc - 1
                } else {
                    doc
                }];
                writer.record_numerical(
                    doc as DocId,
                    CTID_FIELD_NAME,
                    (u64::from(block) << 16) | u64::from(offset),
                );
            }
            let mut fast = segment.open_write(SegmentComponent::FastFields).unwrap();
            writer
                .serialize(
                    blocks.len() as DocId,
                    None,
                    &[CodecType::BlockwiseLinearV2],
                    &mut fast,
                )
                .unwrap();
            fast.terminate().unwrap();
            let first_block = blocks[0];
            let last_block = *blocks.last().unwrap();
            let mut output = CompositeWrite::wrap(segment.open_write(plugin::component()).unwrap());
            write(&segment, &mut output).unwrap();
            output.close().unwrap();
            let file =
                CompositeFile::open(&segment.open_read(plugin::component()).unwrap()).unwrap();
            let map = BlockToDocIdMap::for_test(
                first_block,
                last_block,
                blocks.len() as DocId,
                file,
                field,
            )
            .with_descending(descending);

            assert_eq!(map.num_chunks, 3);
            let requested = vec![
                first_block,
                first_block + 1,
                blocks[1],
                blocks[1] + 1,
                last_block,
                last_block + 1,
            ];
            let expected: Vec<_> = requested
                .iter()
                .map(|&block| blocks.partition_point(|&present| present < block) as DocId)
                .collect();
            assert_eq!(map.boundaries(&requested).unwrap(), expected);

            for (i, &(block, offset)) in tids.iter().enumerate() {
                let expected_doc = if descending { blocks.len() - 1 - i } else { i } as DocId;
                assert_eq!(
                    map.doc_id_for_tid(block, offset).unwrap(),
                    Some(expected_doc)
                );
                assert!(map.contains_tid(block, offset).unwrap());
                assert_eq!(map.tid_for_doc_id(expected_doc).unwrap(), (block, offset));
            }

            let doc_ids: Vec<DocId> = (0..blocks.len() as DocId).collect();
            let mut expected_tids = tids;
            if descending {
                expected_tids.reverse();
            }
            let mut resolved_tids = vec![(0, 0); doc_ids.len()];
            map.tids_for_doc_ids(&doc_ids, &mut resolved_tids).unwrap();
            assert_eq!(resolved_tids, expected_tids);

            let expected_u64s: Vec<Option<u64>> = expected_tids
                .iter()
                .map(|&(b, o)| Some(((b as u64) << 16) | (o as u64)))
                .collect();
            let mut resolved_u64s = vec![None; doc_ids.len()];
            map.tids_for_doc_ids_as_u64s(&doc_ids, &mut resolved_u64s)
                .unwrap();
            assert_eq!(resolved_u64s, expected_u64s);
        }
    }
}
