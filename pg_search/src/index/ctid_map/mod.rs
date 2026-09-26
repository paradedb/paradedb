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
//! Boundaries are one nullable Tantivy column in `.ctid_map`. Restartable iterators scan
//! and deduplicate the final CTIDs without buffering the column. BlockwiseLinearV2
//! compresses values in 512-value blocks and reads them lazily; the nullable index is
//! loaded on the first boundary lookup. A final boundary marks the end of the documents.

use std::iter;
use std::ops::Range;

use anyhow::{Context, bail};
use pgrx::pg_sys::{BlockNumber, InvalidBlockNumber};
use tantivy::DocId;
use tantivy::columnar::column_index::{SerializableColumnIndex, SerializableOptionalIndex, Set};
use tantivy::columnar::column_values::CodecType;
use tantivy::columnar::{
    Cardinality, Column, ColumnIndex, ColumnarReader, DynamicColumn, Iterable, Version,
    open_column_u64, serialize_column_mappable_to_u64,
};
use tantivy::directory::error::OpenReadError;
use tantivy::directory::{CompositeFile, CompositeWrite, FileSlice};
use tantivy::index::{Segment, SegmentComponent, SegmentReader};

use crate::api::CTID_FIELD_NAME;
use crate::index::reader::index::SearchIndexReader;

mod plugin;
pub(crate) use plugin::register;

struct Replayable<I>(I);

impl<T, I> Iterable<T> for Replayable<I>
where
    I: Iterator<Item = T> + Clone,
{
    /// Restarts the scan for Tantivy's statistics and serialization passes.
    fn boxed_iter(&self) -> Box<dyn Iterator<Item = T> + '_> {
        Box::new(self.0.clone())
    }
}

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
    let first_block =
        u32::try_from(column.values.get_val(if descending { docs - 1 } else { 0 }) >> 16)
            .context("heap block number exceeds BlockNumber")?;
    let last_block =
        u32::try_from(column.values.get_val(if descending { 0 } else { docs - 1 }) >> 16)
            .context("heap block number exceeds BlockNumber")?;
    if first_block > last_block || last_block == InvalidBlockNumber {
        bail!("invalid heap-block boundaries");
    }
    // Tantivy column row IDs cannot represent the entire BlockNumber address space.
    let Ok(count) = u32::try_from(u64::from(last_block) - u64::from(first_block) + 2) else {
        return Ok(());
    };
    let column = &column;
    let boundaries = || {
        let mut previous = None;
        (0..docs)
            .filter_map(move |doc| {
                if doc.is_multiple_of(8192) {
                    pgrx::check_for_interrupts!();
                }
                let source_doc = if descending { docs - doc - 1 } else { doc };
                let block = (column.values.get_val(source_doc) >> 16) as BlockNumber;
                let boundary = (previous != Some(block)).then_some((block, doc));
                previous = Some(block);
                boundary
            })
            .chain(iter::once((last_block + 1, docs)))
    };
    let mut previous = None;
    let mut present = 0;
    for (block, _) in boundaries() {
        if previous.is_some_and(|last| block <= last) {
            bail!("invalid heap-block boundaries");
        }
        previous = Some(block);
        present += 1;
    }
    let index = if present == count {
        SerializableColumnIndex::Full
    } else {
        SerializableColumnIndex::Optional(SerializableOptionalIndex {
            non_null_row_ids: Box::new(Replayable(
                boundaries().map(|(block, _)| block - first_block),
            )),
            num_rows: count,
        })
    };
    serialize_column_mappable_to_u64(
        index,
        &Replayable(boundaries().map(|(_, doc)| u64::from(doc))),
        &[CodecType::BlockwiseLinearV2],
        out.for_field(field),
    )?;
    Ok(())
}

pub(crate) struct BlockToDocIdMap {
    first_block: BlockNumber,
    last_block: BlockNumber,
    num_docs: u32,
    file: FileSlice,
    values: Option<Column<u64>>,
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
        let Some(file) = file.open_read(field) else {
            return Ok(None);
        };
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
    fn boundaries(&mut self, blocks: &[BlockNumber]) -> anyhow::Result<Vec<DocId>> {
        debug_assert!(blocks.is_sorted());
        if blocks.is_empty() {
            return Ok(Vec::new());
        }
        if self.values.is_none() {
            let values = open_column_u64(self.file.clone(), Version::V2)?;
            let count = u64::from(self.last_block) - u64::from(self.first_block) + 2;
            if values.get_cardinality() == Cardinality::Multivalued
                || u64::from(values.num_docs()) != count
                || !values.index.has_value(values.num_docs() - 1)
                || values.max_value() > u64::from(self.num_docs)
            {
                bail!("invalid heap-block boundaries");
            }
            self.values = Some(values);
        }
        let values = self.values.as_ref().unwrap();
        blocks
            .iter()
            .map(|&block| {
                let row = block
                    .checked_sub(self.first_block)
                    .context("block precedes the segment block range")?;
                if row >= values.num_docs() {
                    bail!("invalid heap-block boundaries");
                }
                let rank = match &values.index {
                    ColumnIndex::Optional(index) => index.rank(row),
                    _ => row,
                };
                Ok(values.values.get_val(rank) as DocId)
            })
            .collect()
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use std::slice;

    use pgrx::pg_test;
    use tantivy::columnar::{ColumnType, ColumnarWriter};
    use tantivy::directory::{RamDirectory, TerminatingWrite};
    use tantivy::index::{IndexSortByField, Order};
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
                if u64::from(last_block) - u64::from(first_block) + 2 > u64::from(u32::MAX) {
                    assert!(file.open_read(field).is_none());
                    continue;
                }
                let mut map = BlockToDocIdMap {
                    first_block,
                    last_block,
                    num_docs: blocks.len() as DocId,
                    file: file.open_read(field).unwrap(),
                    values: None,
                };
                let requested: Vec<_> = (first_block..=last_block + 1).collect();
                let expected: Vec<_> = requested
                    .iter()
                    .map(|block| blocks.partition_point(|present| present < block) as DocId)
                    .collect();
                assert_eq!(map.boundaries(&requested).unwrap(), expected);
                // Exercise cache reuse and lookups in reverse order.
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
                let column = map.values.as_ref().unwrap();
                let distinct = blocks.windows(2).filter(|pair| pair[0] != pair[1]).count() + 1;
                assert_eq!(
                    column.get_cardinality(),
                    if distinct == (last_block - first_block + 1) as usize {
                        Cardinality::Full
                    } else {
                        Cardinality::Optional
                    }
                );
                for row in 0..column.num_docs() - 1 {
                    let block = first_block + row;
                    assert_eq!(
                        column.index.has_value(row),
                        blocks.binary_search(&block).is_ok()
                    );
                }
            }
        }
    }
}
