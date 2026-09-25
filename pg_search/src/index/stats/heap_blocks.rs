// Copyright (c) 2023-2026 ParadeDB, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

//! A CTID-sorted segment can map heap pages directly to document ranges:
//!
//! ```text
//! doc ID:       0  1  2  3  4  5  6
//! heap block:  10 10 10 11 11 15 15
//!
//! heap block:  10 11 12 13 14 15 end
//! boundary:     0  3  5  5  5  5  7
//! ```
//!
//! If the visibility map says page 11 needs checking, its two boundaries give docs
//! [3, 5). Page 12 gives [5, 5): it has no documents. Consecutive dirty pages need only
//! the two outer boundaries. All-visible pages need no boundary reads.
//!
//! Boundaries are a standard Tantivy numeric column, written in bounded ColumnarWriter
//! batches after sorting or merging. Each batch is addressed through the `.stats`
//! composite directory and opened lazily with ColumnarReader. There is no presence
//! bitmap, rank structure, custom column encoding, or temporary file. Value buffers are
//! bounded; the composite footer retains one entry per batch.

use std::io;
use std::ops::{Range, RangeInclusive};
use std::sync::Arc;

use tantivy::columnar::column_values::CodecType;
use tantivy::columnar::{
    Cardinality, Column, ColumnType, ColumnarReader, ColumnarWriter, DynamicColumn,
};
use tantivy::directory::{CompositeFile, CompositeWrite};
use tantivy::index::{Segment, SegmentComponent};
use tantivy::schema::Field;

use crate::api::CTID_FIELD_NAME;

pub(super) const COLUMNS_IDX: usize = 3;
const CHUNK_SIZE: usize = 32768;
const RANGES_PER_BATCH: usize = 128;
const CODECS: &[CodecType] = &[CodecType::Bitpacked, CodecType::BlockwiseLinearV2];

fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid heap-block boundaries")
}

/// Builds the boundary column from the final live CTIDs at flush or merge.
pub(super) fn write(segment: &Segment, out: &mut CompositeWrite) -> tantivy::Result<()> {
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
    let first_block = u32::try_from(values.peek().unwrap() >> 16).map_err(|_| invalid())?;
    let last_block =
        u32::try_from(column.values.get_val(if descending { 0 } else { docs - 1 }) >> 16)
            .map_err(|_| invalid())?;
    let count = (u64::from(last_block) - u64::from(first_block) + 2) as usize;
    let mut processed = 0u32;
    let mut previous = None;
    for start in (0..count).step_by(CHUNK_SIZE) {
        pgrx::check_for_interrupts!();
        let len = (count - start).min(CHUNK_SIZE);
        let mut writer = ColumnarWriter::default();
        writer.record_column_type("boundary", ColumnType::U64, false);
        for row in 0..len {
            let block = u64::from(first_block) + (start + row) as u64;
            while let Some(&value) = values.peek() {
                if value >> 16 >= block {
                    break;
                }
                if previous.is_some_and(|last| value < last) {
                    return Err(invalid().into());
                }
                previous = values.next();
                processed += 1;
            }
            writer.record_numerical(row as u32, "boundary", u64::from(processed));
        }
        writer.serialize(
            len as u32,
            None,
            CODECS,
            out.for_field_with_idx(field, COLUMNS_IDX + start / CHUNK_SIZE),
        )?;
    }
    if processed != docs {
        return Err(invalid().into());
    }
    Ok(())
}

pub(crate) struct HeapBlockMap {
    first_block: u32,
    last_block: u32,
    docs: u32,
    descending: bool,
    file: Arc<CompositeFile>,
    field: Field,
    pages_per_vm: u32,
    values: Option<(usize, Column<u64>)>,
}

impl HeapBlockMap {
    /// Uses the CTID column bounds without reading the boundary column.
    pub(super) fn open(
        ctids: RangeInclusive<u64>,
        docs: u32,
        descending: bool,
        file: Arc<CompositeFile>,
        field: Field,
        pages_per_vm: u32,
    ) -> io::Result<Self> {
        let first_block = u32::try_from(*ctids.start() >> 16).map_err(|_| invalid())?;
        let last_block = u32::try_from(*ctids.end() >> 16).map_err(|_| invalid())?;
        if docs == 0
            || first_block > last_block
            || pages_per_vm == 0
            || !pages_per_vm.is_multiple_of(32)
        {
            return Err(invalid());
        }
        Ok(Self {
            first_block,
            last_block,
            docs,
            descending,
            file,
            field,
            pages_per_vm,
            values: None,
        })
    }

    /// Coalesces dirty VM pages, then reads two boundaries per range.
    pub(crate) fn missing_ranges(
        &mut self,
        mut retain_invisible: impl FnMut(u32, &mut [u32]),
    ) -> io::Result<Vec<Range<u32>>> {
        let first = u64::from(self.first_block);
        let end = u64::from(self.last_block) + 1;
        let mut block = first / 32 * 32;
        let mut scratch = vec![u32::MAX; self.pages_per_vm as usize / 32];
        let mut pending: Option<Range<u64>> = None;
        let mut ranges = Vec::new();
        let mut pages = Vec::with_capacity(RANGES_PER_BATCH);
        while block < end {
            pgrx::check_for_interrupts!();
            let span = u64::from(self.pages_per_vm) - block % u64::from(self.pages_per_vm);
            let words = (end - block).min(span).div_ceil(32) as usize;
            let missing = &mut scratch[..words];
            missing.fill(u32::MAX);
            retain_invisible(block as u32, missing);
            for (word, &bits) in missing.iter().enumerate() {
                let mut bits = bits;
                while bits != 0 {
                    let bit = bits.trailing_zeros();
                    let len = (bits >> bit).trailing_ones();
                    bits &= !((u32::MAX >> (32 - len)) << bit);
                    let start = block + word as u64 * 32 + u64::from(bit);
                    let range = start.max(first)..(start + u64::from(len)).min(end);
                    if range.is_empty() {
                        continue;
                    }
                    if let Some(last) = pending.as_mut().filter(|last| last.end == range.start) {
                        last.end = range.end;
                    } else {
                        if let Some(previous) = pending.replace(range) {
                            pages.push(previous);
                            if pages.len() == RANGES_PER_BATCH {
                                self.append_ranges(&pages, &mut ranges)?;
                                pages.clear();
                            }
                        }
                    }
                }
            }
            block += words as u64 * 32;
        }
        if let Some(last) = pending {
            pages.push(last);
        }
        self.append_ranges(&pages, &mut ranges)?;
        if self.descending {
            for range in &mut ranges {
                *range = self.docs - range.end..self.docs - range.start;
            }
            ranges.reverse();
        }
        Ok(ranges)
    }

    /// Batch-decodes dirty page endpoints and coalesces their document ranges.
    fn append_ranges(
        &mut self,
        pages: &[Range<u64>],
        ranges: &mut Vec<Range<u32>>,
    ) -> io::Result<()> {
        let blocks: Vec<_> = pages
            .iter()
            .flat_map(|page| [page.start, page.end])
            .collect();
        let mut boundaries = vec![0; blocks.len()];
        self.boundaries(&blocks, &mut boundaries)?;
        for pair in boundaries.chunks_exact(2) {
            let [start, end] = [pair[0], pair[1]];
            if start > end || end > self.docs {
                return Err(invalid());
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
        Ok(())
    }

    /// Reads sorted boundaries in batches, retaining only the current column chunk.
    fn boundaries(&mut self, mut blocks: &[u64], mut output: &mut [u32]) -> io::Result<()> {
        debug_assert_eq!(blocks.len(), output.len());
        debug_assert!(blocks.is_sorted());
        while let Some(&block) = blocks.first() {
            let index = block
                .checked_sub(u64::from(self.first_block))
                .ok_or_else(invalid)? as usize;
            let count = (u64::from(self.last_block) - u64::from(self.first_block) + 2) as usize;
            if index >= count {
                return Err(invalid());
            }
            let chunk = index / CHUNK_SIZE;
            if self
                .values
                .as_ref()
                .is_none_or(|(current, _)| *current != chunk)
            {
                let file = self
                    .file
                    .open_read_with_idx(self.field, COLUMNS_IDX + chunk)
                    .ok_or_else(invalid)?;
                let reader = ColumnarReader::open(file)?;
                let handles = reader.read_columns("boundary")?;
                let [handle] = handles.as_slice() else {
                    return Err(invalid());
                };
                let DynamicColumn::U64(values) = handle.open()? else {
                    return Err(invalid());
                };
                let len = (count - chunk * CHUNK_SIZE).min(CHUNK_SIZE);
                if values.get_cardinality() != Cardinality::Full
                    || values.num_docs() as usize != len
                    || values.max_value() > u64::from(self.docs)
                {
                    return Err(invalid());
                }
                self.values = Some((chunk, values));
            }
            let (_, values) = self.values.as_ref().unwrap();
            let chunk_start = u64::from(self.first_block) + (chunk * CHUNK_SIZE) as u64;
            let len = blocks.partition_point(|&block| block < chunk_start + CHUNK_SIZE as u64);
            let positions: Vec<_> = blocks[..len]
                .iter()
                .map(|&block| (block - chunk_start) as u32)
                .collect();
            if positions
                .last()
                .is_some_and(|&position| position >= values.num_docs())
            {
                return Err(invalid());
            }
            let (batch, rest) = output.split_at_mut(len);
            values.u32_vals(&positions, batch);
            blocks = &blocks[len..];
            output = rest;
        }
        Ok(())
    }
}
