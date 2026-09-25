// Copyright (c) 2023-2026 ParadeDB, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Imagine a segment containing documents 0–6, corresponding to these heap pages:
//!
//! ```text
//! doc ID:             0  1  2  3  4  5  6
//! heap block:        10 10 11 11 11 15 15
//! ```
//!
//! We want to go efficiently from distinct heap blocks to the document IDs that need
//! visibility checks. Getting this mapping from the CTID column alone would require
//! decoding and walking a value for every document, even when many share the same page.
//! CTID sorting keeps each block's documents contiguous, so we can represent them as ranges.
//!
//! First, a page presence bitmap records which heap blocks occur in the segment:
//!
//! ```text
//! heap block:        10 11 12 13 14 15
//! presence:           1  1  0  0  0  1
//! ```
//!
//! We intersect presence with the complement of PostgreSQL's VM all-visible bits to
//! identify the pages that still need visibility checks:
//!
//! ```text
//! heap block:        10 11 12 13 14 15
//! presence:           1  1  0  0  0  1
//! VM all-visible:     1  0  1  1  1  1
//!                         |
//!             presence AND NOT all-visible
//!                         v
//! needs checking:     0  1  0  0  0  0  -> heap block 11
//! ```
//!
//! Now that we know page 11 needs checking, we need to find which document IDs belong to
//! it. Rank gives its position among the present pages, and boundaries translate that
//! position into a document range. We only read boundaries for pages needing checks:
//!
//! ```text
//! present blocks:    [10, 11, 15]
//! rank:                0   1   2
//! boundaries:        [0,  2,  5,  7]
//!
//! heap block 11 -> rank 1 -> [boundaries[1], boundaries[2]) -> doc IDs [2, 5)
//!                                                                  |
//! query matches: [0, 3, 6] -----------------------------------------+
//!                                                                  v
//!                                                      only doc 3 needs checking
//! ```
//!
//! Presence words (or block offsets for sparse regions), prefix ranks, and document
//! boundaries are ordinary integer columns in
//! the segment's `.stats` file. Each bounded chunk uses Tantivy's choice of bitpacking or
//! BlockwiseLinearV2. A directory of occupied VM pages skips large gaps between heap pages.
//! At query time, read the VM first: visible words need no presence, rank, or boundary reads.
//! Builds and merges write bounded ColumnarWriter batches directly into `.stats`, using
//! its composite directory to locate each batch. Only the VM-page directory is spooled.
//! Value buffers are bounded; the composite footer retains one entry per column chunk.
//! `VisibilityChecker` performs the VM comparison once per eligible segment
//! and snapshot, caches the resulting ranges, and uses them to filter subsequent batches.

use std::io::{self, Seek, SeekFrom, Write};
use std::ops::Range;
use std::sync::Arc;

use tantivy::Directory;
use tantivy::columnar::column_values::CodecType;
use tantivy::columnar::{
    Cardinality, Column, ColumnType, ColumnarReader, ColumnarWriter, DynamicColumn,
};
use tantivy::directory::{CompositeFile, CompositeWrite, FileSlice, OwnedBytes};
use tantivy::index::{Segment, SegmentComponent};
use tantivy::schema::Field;

use crate::api::CTID_FIELD_NAME;
#[cfg(feature = "io_stats")]
use crate::index::reader::io_stats::trace;
use crate::postgres::heap::HEAPBLOCKS_PER_PAGE;

pub(super) const DIRECTORY_IDX: usize = 7;
const COLUMNS_IDX: usize = 9;
const COLUMN_COUNT: usize = 3;
const PRESENCE: usize = 0;
const RANK: usize = 1;
const BOUNDARIES: usize = 2;
const HEADER: usize = 24;
const ENTRY: usize = 16;
const SPARSE: u32 = 1 << 31;
const CHUNK_SIZE: usize = 32768;
const CODECS: &[CodecType] = &[CodecType::Bitpacked, CodecType::BlockwiseLinearV2];

fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid heap-block columns")
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

/// Writes presence and boundary entries from the finished CTID column at flush or merge.
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
    if HEAPBLOCKS_PER_PAGE == 0 || !HEAPBLOCKS_PER_PAGE.is_multiple_of(32) {
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
    let directory = segment.index().directory();
    let mut entries = directory.open_temp_file()?;
    let mut presence = ChunkWriter::new(field, PRESENCE);
    let mut ranks = ChunkWriter::new(field, RANK);
    let mut boundaries = ChunkWriter::new(field, BOUNDARIES);
    let mut bitmap = vec![0u32; HEAPBLOCKS_PER_PAGE as usize / 32];
    let mut chunks = 0u32;
    let mut rows = 0u32;
    let mut blocks = 0u32;
    let mut processed = 0u32;
    let mut previous = None;

    while let Some(&value) = values.peek() {
        pgrx::check_for_interrupts!();
        let first = u32::try_from(value >> 16).map_err(|_| invalid())?;
        let vm_page = first / HEAPBLOCKS_PER_PAGE;
        let first_word = (first % HEAPBLOCKS_PER_PAGE / 32) as usize;
        let mut last_word = first_word;
        bitmap.fill(0);
        while let Some(&value) = values.peek() {
            let block = u32::try_from(value >> 16).map_err(|_| invalid())?;
            if previous.is_some_and(|last| block < last) {
                return Ok(());
            }
            if block / HEAPBLOCKS_PER_PAGE != vm_page {
                break;
            }
            if processed.is_multiple_of(8192) {
                pgrx::check_for_interrupts!();
            }
            values.next();
            if previous != Some(block) {
                boundaries.push(processed, out)?;
                last_word = (block % HEAPBLOCKS_PER_PAGE / 32) as usize;
                bitmap[last_word] |= 1 << (block % 32);
            }
            previous = Some(block);
            processed += 1;
        }
        let words = &bitmap[first_word..=last_word];
        let population: u32 = words.iter().map(|word| word.count_ones()).sum();
        let sparse = population as usize <= words.len() * 2;
        let count = if sparse {
            population
        } else {
            words.len() as u32
        };
        for value in [
            first / 32 * 32,
            rows,
            count | if sparse { SPARSE } else { 0 },
            words.len() as u32,
        ] {
            entries.write_all(&value.to_le_bytes())?;
        }
        for (i, &word) in words.iter().enumerate() {
            if sparse {
                let mut bits = word;
                while bits != 0 {
                    presence.push(i as u32 * 32 + bits.trailing_zeros(), out)?;
                    ranks.push(blocks, out)?;
                    blocks += 1;
                    bits &= bits - 1;
                }
            } else {
                presence.push(word, out)?;
                ranks.push(blocks, out)?;
                blocks += word.count_ones();
            }
        }
        rows += count;
        chunks += 1;
    }
    if processed != docs {
        return Err(invalid().into());
    }
    boundaries.push(docs, out)?;

    let writer = out.for_field_with_idx(field, DIRECTORY_IDX);
    for value in [
        u32::from_le_bytes(*b"HBC2"),
        HEAPBLOCKS_PER_PAGE,
        docs,
        blocks,
        chunks,
        u32::from(descending),
    ] {
        writer.write_all(&value.to_le_bytes())?;
    }
    entries.seek(SeekFrom::Start(0))?;
    io::copy(&mut entries, writer)?;
    presence.flush(out)?;
    ranks.flush(out)?;
    boundaries.flush(out)?;
    Ok(())
}

struct ChunkWriter {
    writer: ColumnarWriter,
    rows: u32,
    chunk: usize,
    field: Field,
    column: usize,
}

impl ChunkWriter {
    fn new(field: Field, column: usize) -> Self {
        Self {
            writer: ColumnarWriter::default(),
            rows: 0,
            chunk: 0,
            field,
            column,
        }
    }

    fn push(&mut self, value: u32, out: &mut CompositeWrite) -> io::Result<()> {
        if self.rows == 0 {
            self.writer
                .record_column_type("value", ColumnType::U64, false);
        }
        self.writer
            .record_numerical(self.rows, "value", u64::from(value));
        self.rows += 1;
        if self.rows as usize == CHUNK_SIZE {
            self.flush(out)?;
        }
        Ok(())
    }

    fn flush(&mut self, out: &mut CompositeWrite) -> io::Result<()> {
        if self.rows == 0 {
            return Ok(());
        }
        let index = COLUMNS_IDX + self.chunk * COLUMN_COUNT + self.column;
        self.writer.serialize(
            self.rows,
            None,
            CODECS,
            out.for_field_with_idx(self.field, index),
        )?;
        self.writer = ColumnarWriter::default();
        self.rows = 0;
        self.chunk += 1;
        Ok(())
    }
}

pub(crate) struct HeapBlockMap {
    directory: OwnedBytes,
    presence: ChunkReader,
    ranks: ChunkReader,
    boundaries: ChunkReader,
    docs: u32,
    blocks: u32,
    pages_per_vm: u32,
    descending: bool,
}

impl HeapBlockMap {
    /// Reads the sparse directory while leaving all three columns unopened.
    pub(super) fn open(
        directory: FileSlice,
        file: Arc<CompositeFile>,
        field: Field,
        docs: u32,
        pages_per_vm: u32,
    ) -> io::Result<Self> {
        #[cfg(feature = "io_stats")]
        let _io = trace::external("Visibility Directory");
        let directory = directory.read_bytes()?;
        if directory.len() < HEADER
            || &directory[..4] != b"HBC2"
            || u32_at(&directory, 4) != pages_per_vm
            || u32_at(&directory, 8) != docs
            || pages_per_vm == 0
            || !pages_per_vm.is_multiple_of(32)
            || u32_at(&directory, 20) > 1
        {
            return Err(invalid());
        }
        let chunks = u32_at(&directory, 16) as usize;
        let blocks = u32_at(&directory, 12);
        if HEADER + chunks * ENTRY != directory.len() || blocks == 0 || blocks > docs {
            return Err(invalid());
        }
        let mut rows = 0u32;
        let mut previous_end = 0u64;
        for i in 0..chunks {
            let at = HEADER + i * ENTRY;
            let first = u32_at(&directory, at);
            let encoded_count = u32_at(&directory, at + 8);
            let count = encoded_count & !SPARSE;
            let words = u32_at(&directory, at + 12);
            let end = u64::from(first) + u64::from(words) * 32;
            if !first.is_multiple_of(32)
                || count == 0
                || count > pages_per_vm
                || words == 0
                || words > pages_per_vm / 32
                || (encoded_count & SPARSE == 0 && count != words)
                || u64::from(first) < previous_end
                || end > u64::from(u32::MAX) + 1
                || u64::from(first / pages_per_vm) != (end - 1) / u64::from(pages_per_vm)
                || u32_at(&directory, at + 4) != rows
            {
                return Err(invalid());
            }
            rows = rows.checked_add(count).ok_or_else(invalid)?;
            previous_end = end;
        }
        if rows == 0 || u64::from(blocks) > u64::from(rows) * 32 {
            return Err(invalid());
        }
        Ok(Self {
            descending: u32_at(&directory, 20) != 0,
            directory,
            presence: ChunkReader::new(
                file.clone(),
                field,
                PRESENCE,
                rows as usize,
                "Visibility Presence",
            ),
            ranks: ChunkReader::new(file.clone(), field, RANK, rows as usize, "Visibility Rank"),
            boundaries: ChunkReader::new(
                file,
                field,
                BOUNDARIES,
                blocks as usize + 1,
                "Visibility Boundaries",
            ),
            docs,
            blocks,
            pages_per_vm,
        })
    }

    /// Reads columns only for dirty VM words and coalesces ordinals before decoding boundaries.
    pub(crate) fn missing_ranges(
        &mut self,
        mut retain_invisible: impl FnMut(u32, &mut [u32]),
    ) -> io::Result<Vec<Range<u32>>> {
        let mut ranges: Vec<Range<u32>> = Vec::new();
        let mut scratch = vec![u32::MAX; self.pages_per_vm as usize / 32];
        let mut add = |range: Range<u32>| {
            if let Some(last) = ranges.last_mut().filter(|last| last.end == range.start) {
                last.end = range.end;
            } else {
                ranges.push(range);
            }
        };
        for chunk in 0..u32_at(&self.directory, 16) as usize {
            pgrx::check_for_interrupts!();
            let at = HEADER + chunk * ENTRY;
            let first = u32_at(&self.directory, at);
            let row = u32_at(&self.directory, at + 4) as usize;
            let encoded_count = u32_at(&self.directory, at + 8);
            let count = (encoded_count & !SPARSE) as usize;
            let sparse = encoded_count & SPARSE != 0;
            let words = u32_at(&self.directory, at + 12) as usize;
            let missing = &mut scratch[..words];
            missing.fill(u32::MAX);
            retain_invisible(first, missing);
            if missing.iter().all(|&word| word == 0) {
                continue;
            }
            if missing.iter().all(|&word| word == u32::MAX) {
                let start = self.ranks.get(row)?;
                let end = if sparse {
                    start + count as u32
                } else {
                    let last = row + count - 1;
                    self.ranks.get(last)? + self.presence.get(last)?.count_ones()
                };
                add(start..end);
                continue;
            }
            if sparse {
                let rank = self.ranks.get(row)?;
                for i in 0..count {
                    let block = self.presence.get(row + i)?;
                    if block as usize >= words * 32 {
                        return Err(invalid());
                    }
                    if missing[block as usize / 32] & (1 << (block % 32)) != 0 {
                        add(rank + i as u32..rank + i as u32 + 1);
                    }
                }
                continue;
            }
            for (word, &dirty) in missing.iter().enumerate() {
                if dirty == 0 {
                    continue;
                }
                let present = self.presence.get(row + word)?;
                let mut bad = dirty & present;
                if bad == 0 {
                    continue;
                }
                let rank = self.ranks.get(row + word)?;
                if bad == present {
                    add(rank..rank + present.count_ones());
                    continue;
                }
                while bad != 0 {
                    let bit = bad.trailing_zeros();
                    let ordinal = rank + (present & ((1u32 << bit) - 1)).count_ones();
                    add(ordinal..ordinal + 1);
                    bad &= bad - 1;
                }
            }
        }
        for range in &mut ranges {
            if range.start >= range.end || range.end > self.blocks {
                return Err(invalid());
            }
            let start = self.boundaries.get(range.start as usize)?;
            let end = self.boundaries.get(range.end as usize)?;
            if start >= end || end > self.docs {
                return Err(invalid());
            }
            *range = if self.descending {
                self.docs - end..self.docs - start
            } else {
                start..end
            };
        }
        if self.descending {
            ranges.reverse();
        }
        Ok(ranges)
    }
}

struct ChunkReader {
    file: Arc<CompositeFile>,
    field: Field,
    column: usize,
    count: usize,
    values: Option<(usize, Column<u64>)>,
    _label: &'static str,
}

impl ChunkReader {
    fn new(
        file: Arc<CompositeFile>,
        field: Field,
        column: usize,
        count: usize,
        label: &'static str,
    ) -> Self {
        Self {
            file,
            field,
            column,
            count,
            values: None,
            _label: label,
        }
    }

    fn get(&mut self, index: usize) -> io::Result<u32> {
        if index >= self.count {
            return Err(invalid());
        }
        #[cfg(feature = "io_stats")]
        let _io = trace::external(self._label);
        let chunk = index / CHUNK_SIZE;
        if self
            .values
            .as_ref()
            .is_none_or(|(current, _)| *current != chunk)
        {
            let address = COLUMNS_IDX + chunk * COLUMN_COUNT + self.column;
            let file = self
                .file
                .open_read_with_idx(self.field, address)
                .ok_or_else(invalid)?;
            let reader = ColumnarReader::open(file)?;
            let handles = reader.read_columns("value")?;
            let [handle] = handles.as_slice() else {
                return Err(invalid());
            };
            let DynamicColumn::U64(values) = handle.open()? else {
                return Err(invalid());
            };
            let count = (self.count - chunk * CHUNK_SIZE).min(CHUNK_SIZE);
            if values.get_cardinality() != Cardinality::Full || values.num_docs() as usize != count
            {
                return Err(invalid());
            }
            self.values = Some((chunk, values));
        }
        let (_, values) = self.values.as_ref().unwrap();
        u32::try_from(values.values.get_val((index % CHUNK_SIZE) as u32)).map_err(|_| invalid())
    }
}
