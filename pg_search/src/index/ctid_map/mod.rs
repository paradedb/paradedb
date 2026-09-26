// Copyright (c) 2023-2026 ParadeDB, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

//! A CTID-sorted segment can map heap blocks directly to document ranges:
//!
//! ```text
//! doc ID:       0  1  2  3  4  5  6
//! heap block:  10 10 10 11 11 15 15
//!
//! heap block:  10 11 12 13 14 15 end
//! boundary:     0  3  -  -  -  5  7
//! ```
//!
//! Absent blocks are null. Tantivy's nullable column index maps a block to its boundary,
//! or the next present boundary if it is absent. Dirty block 11 therefore gives docs
//! [3, 5), while block 12 gives [5, 5): it has no documents. Consecutive dirty blocks need
//! only the two outer boundaries. All-visible blocks need no boundary reads.
//!
//! Boundaries are a standard Tantivy numeric column, written in bounded ColumnarWriter
//! batches after sorting or merging. Each batch is addressed through the `.ctid_map`
//! composite directory and opened lazily with ColumnarReader. Each batch has a closing
//! boundary so lookups never need to open another batch to resolve a trailing null.

use std::ops::Range;

use anyhow::{Context, bail};
use pgrx::pg_sys::{BlockNumber, InvalidBlockNumber};
use tantivy::DocId;
use tantivy::columnar::column_index::Set;
use tantivy::columnar::column_values::CodecType;
use tantivy::columnar::{
    Cardinality, Column, ColumnIndex, ColumnType, ColumnarReader, ColumnarWriter, DynamicColumn,
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
        let rows = len + usize::from(start + len < count);
        let mut writer = ColumnarWriter::default();
        writer.record_column_type(BLOCK_BOUNDARIES, ColumnType::U64, false);
        for row in 0..rows {
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
            if row == rows - 1 || values.peek().is_some_and(|value| value >> 16 == block) {
                writer.record_numerical(row as u32, BLOCK_BOUNDARIES, u64::from(processed));
            }
        }
        writer.serialize(
            rows as u32,
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
                let rows_with_boundary = len + usize::from((chunk + 1) * CHUNK_SIZE < count);
                let rows = values.num_docs() as usize;
                if values.get_cardinality() == Cardinality::Multivalued
                    || rows != rows_with_boundary
                    || !values.index.has_value(rows as u32 - 1)
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
            output.extend(blocks[..len].iter().map(|&block| {
                let row = block - chunk_start;
                let rank = match &values.index {
                    ColumnIndex::Optional(index) => index.rank(row),
                    _ => row,
                };
                values.values.get_val(rank) as DocId
            }));
            blocks = &blocks[len..];
        }
        Ok(output)
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use std::slice;

    use pgrx::pg_test;
    use tantivy::directory::{RamDirectory, TerminatingWrite};
    use tantivy::index::{IndexSortByField, Order};
    use tantivy::schema::{FAST, Schema};
    use tantivy::{Index, IndexSettings};

    use super::*;

    #[pg_test]
    fn nullable_ctid_map_boundaries() {
        let chunk = CHUNK_SIZE as BlockNumber;
        for blocks in [
            vec![10, 10, 10],
            vec![10, 10, 11, 11, 12],
            vec![10, 10, 11, 11, 11, 15, 15],
            vec![10, 10 + chunk - 1, 10 + chunk, 10 + chunk * 3 + 2],
            vec![10, 10 + chunk - 2],
            (10..10 + chunk + 4)
                .filter(|block| block % 101 != 0)
                .collect(),
            vec![InvalidBlockNumber - 2, InvalidBlockNumber - 1],
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
                for doc in 0..blocks.len() {
                    let block = blocks[if descending {
                        blocks.len() - doc - 1
                    } else {
                        doc
                    }];
                    writer.record_numerical(
                        doc as DocId,
                        CTID_FIELD_NAME,
                        (u64::from(block) << 16) | 1,
                    );
                }
                let mut fast = segment.open_write(SegmentComponent::FastFields).unwrap();
                writer
                    .serialize(blocks.len() as DocId, None, CODECS, &mut fast)
                    .unwrap();
                fast.terminate().unwrap();
                let first_block = blocks[0];
                let last_block = *blocks.last().unwrap();
                let count = (last_block - first_block) as usize + 2;
                let mut output =
                    CompositeWrite::wrap(segment.open_write(plugin::component()).unwrap());
                write(&segment, &mut output).unwrap();
                output.close().unwrap();
                let file =
                    CompositeFile::open(&segment.open_read(plugin::component()).unwrap()).unwrap();
                let mut map = BlockToDocIdMap {
                    first_block,
                    last_block,
                    num_docs: blocks.len() as DocId,
                    file,
                    field,
                    values: None,
                };
                let requested: Vec<_> = (first_block..=last_block + 1).collect();
                let expected: Vec<_> = requested
                    .iter()
                    .map(|block| blocks.partition_point(|present| present < block) as DocId)
                    .collect();
                assert_eq!(map.boundaries(&requested).unwrap(), expected);
                // Exercise cache reuse and queries that start in a different chunk.
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
                for chunk in 0..count.div_ceil(CHUNK_SIZE) {
                    let reader =
                        ColumnarReader::open(map.file.open_read_with_idx(field, chunk).unwrap())
                            .unwrap();
                    let column = reader.read_columns(BLOCK_BOUNDARIES).unwrap()[0]
                        .open()
                        .unwrap();
                    let DynamicColumn::U64(column) = column else {
                        panic!("expected u64 boundaries")
                    };
                    for row in 0..column.num_docs() - 1 {
                        let block = first_block + (chunk * CHUNK_SIZE) as BlockNumber + row;
                        assert_eq!(
                            column.index.has_value(row),
                            blocks.binary_search(&block).is_ok()
                        );
                    }
                }
            }
        }
    }
}
