// Copyright (c) 2023-2026 ParadeDB, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

//! A CTID-sorted segment can map heap blocks directly to document ranges:
//!
//! ```text
//! doc ID:       0  1  2  3  4  5  6
//! heap block:  10 10 10 11 11 15 15
//!
//! heap block:  10 11 12 13 14 15 end
//! boundary:     0  3  5  5  5  5  7
//! ```
//!
//! If the visibility map says block 11 needs checking, its two boundaries give docs
//! [3, 5). Block 12 gives [5, 5): it has no documents. Consecutive dirty blocks need only
//! the two outer boundaries. All-visible blocks need no boundary reads.
//!
//! Boundaries are a standard Tantivy numeric column, written in bounded ColumnarWriter
//! batches after sorting or merging. Each batch is addressed through the `.ctid_map`
//! composite directory and opened lazily with ColumnarReader.

use std::ops::Range;

use anyhow::{Context, bail};
use pgrx::pg_sys::{BlockNumber, InvalidBlockNumber};
use tantivy::DocId;
use tantivy::columnar::column_values::CodecType;
use tantivy::columnar::{
    Cardinality, Column, ColumnType, ColumnarReader, ColumnarWriter, DynamicColumn,
};
use tantivy::directory::error::OpenReadError;
use tantivy::directory::{CompositeFile, CompositeWrite};
use tantivy::index::{Segment, SegmentComponent, SegmentReader};
use tantivy::schema::Field;

use crate::api::CTID_FIELD_NAME;
use crate::index::reader::index::SearchIndexReader;

mod plugin;
pub(crate) use plugin::register;

// Column name shared by the boundary writer and reader.
const BLOCK_BOUNDARIES: &str = "block_boundaries";
// Maximum boundaries per column chunk, bounding writer memory.
const CHUNK_SIZE: usize = 32768;
// Let Tantivy choose between bitpacking and blockwise linear compression.
const CODECS: &[CodecType] = &[CodecType::Bitpacked, CodecType::BlockwiseLinearV2];

/// Builds the boundary column from the final live CTIDs at flush or merge.
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
    let values: Box<dyn Iterator<Item = u64> + '_> = if descending {
        Box::new((0..docs).rev().map(|doc| column.values.get_val(doc)))
    } else {
        column.values.iter()
    };
    let mut values = values.peekable();
    let first_block = u32::try_from(values.peek().unwrap() >> 16)
        .context("heap block number exceeds BlockNumber")?;
    let last_block =
        u32::try_from(column.values.get_val(if descending { 0 } else { docs - 1 }) >> 16)
            .context("heap block number exceeds BlockNumber")?;
    let count = (u64::from(last_block) - u64::from(first_block) + 2) as usize;
    let mut processed = 0u32;
    let mut previous = None;
    for start in (0..count).step_by(CHUNK_SIZE) {
        pgrx::check_for_interrupts!();
        let len = (count - start).min(CHUNK_SIZE);
        let mut writer = ColumnarWriter::default();
        writer.record_column_type(BLOCK_BOUNDARIES, ColumnType::U64, false);
        for row in 0..len {
            let block = u64::from(first_block) + (start + row) as u64;
            while let Some(&value) = values.peek() {
                if value >> 16 >= block {
                    break;
                }
                if previous.is_some_and(|last| value < last) {
                    bail!("invalid heap-block boundaries");
                }
                previous = values.next();
                processed += 1;
            }
            writer.record_numerical(row as u32, BLOCK_BOUNDARIES, u64::from(processed));
        }
        writer.serialize(
            len as u32,
            None,
            CODECS,
            out.for_field_with_idx(field, start / CHUNK_SIZE),
        )?;
    }
    if processed != docs {
        bail!("invalid heap-block boundaries");
    }
    Ok(())
}

pub(crate) struct BlockToDocIdMap {
    first_block: BlockNumber,
    last_block: BlockNumber,
    num_docs: u32,
    file: CompositeFile,
    field: Field,
    values: Option<(usize, Column<u64>)>,
}

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
        if file.open_read_with_idx(field, 0).is_none() {
            return Ok(None);
        }
        let Some(blocks) = SearchIndexReader::block_bounds(segment)? else {
            return Ok(None);
        };
        let first_block = *blocks.start();
        let last_block = *blocks.end();
        let num_docs = segment.max_doc();
        if num_docs == 0 || first_block > last_block || last_block == InvalidBlockNumber {
            bail!("invalid heap-block boundaries");
        }
        Ok(Some(Self {
            first_block,
            last_block,
            num_docs,
            file,
            field,
            values: None,
        }))
    }

    /// Returns the heap-block span covered by the boundary column.
    pub(crate) fn block_range(&self) -> Range<BlockNumber> {
        self.first_block..self.last_block + 1
    }

    /// Given ranges of dirty heap blocks, returns the document ID ranges they map to.
    pub(crate) fn doc_id_ranges_for_blocks(
        &mut self,
        block_ranges: &[Range<BlockNumber>],
    ) -> anyhow::Result<Vec<Range<DocId>>> {
        let mut ranges: Vec<Range<DocId>> = Vec::with_capacity(block_ranges.len());
        let blocks: Vec<_> = block_ranges
            .iter()
            .flat_map(|blocks| [blocks.start, blocks.end])
            .collect();
        let boundaries = self.boundaries(&blocks)?;
        for pair in boundaries.chunks_exact(2) {
            let [start, end] = [pair[0], pair[1]];
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

    /// Given sorted block numbers, returns the starting doc ID of each block.
    fn boundaries(&mut self, mut blocks: &[BlockNumber]) -> anyhow::Result<Vec<DocId>> {
        let mut output = Vec::with_capacity(blocks.len());
        debug_assert!(blocks.is_sorted());
        while let Some(&block) = blocks.first() {
            let index = block
                .checked_sub(self.first_block)
                .context("block precedes the segment block range")?
                as usize;
            let count = (u64::from(self.last_block) - u64::from(self.first_block) + 2) as usize;
            if index >= count {
                bail!("invalid heap-block boundaries");
            }
            let chunk = index / CHUNK_SIZE;
            if self
                .values
                .as_ref()
                .is_none_or(|(current, _)| *current != chunk)
            {
                let file = self
                    .file
                    .open_read_with_idx(self.field, chunk)
                    .context("missing heap-block boundary chunk")?;
                let reader = ColumnarReader::open(file)?;
                let handles = reader.read_columns(BLOCK_BOUNDARIES)?;
                let [handle] = handles.as_slice() else {
                    bail!("invalid heap-block boundaries");
                };
                let DynamicColumn::U64(values) = handle.open()? else {
                    bail!("invalid heap-block boundaries");
                };
                let len = (count - chunk * CHUNK_SIZE).min(CHUNK_SIZE);
                if values.get_cardinality() != Cardinality::Full
                    || values.num_docs() as usize != len
                    || values.max_value() > u64::from(self.num_docs)
                {
                    bail!("invalid heap-block boundaries");
                }
                self.values = Some((chunk, values));
            }
            let (_, values) = self.values.as_ref().unwrap();
            let chunk_start = self.first_block + (chunk * CHUNK_SIZE) as BlockNumber;
            let len =
                blocks.partition_point(|&block| block - chunk_start < CHUNK_SIZE as BlockNumber);
            if blocks[len - 1] - chunk_start >= values.num_docs() {
                bail!("invalid heap-block boundaries");
            }
            output.extend(
                blocks[..len]
                    .iter()
                    .map(|&block| values.values.get_val(block - chunk_start) as DocId),
            );
            blocks = &blocks[len..];
        }
        Ok(output)
    }
}
