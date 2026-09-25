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
//! Presence words cover the segment's full min/max heap-block span, including zero words
//! for gaps. A word's position identifies its heap blocks directly; no VM-page directory
//! is needed. Presence, prefix ranks, and document boundaries are ordinary integer columns
//! in `.stats`, written in bounded ColumnarWriter batches with bitpacking or BlockwiseLinearV2.
//! At query time, read the VM first: visible words need no column reads.
//! Builds and merges write directly to `.stats`, without temporary files. Value buffers
//! are bounded; Tantivy's composite footer retains one entry per column chunk.
//! `VisibilityChecker` performs the VM comparison once per eligible segment
//! and snapshot, caches the resulting ranges, and uses them to filter subsequent batches.

use std::io::{self, Write};
use std::ops::Range;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tantivy::columnar::column_values::CodecType;
use tantivy::columnar::{
    Cardinality, Column, ColumnType, ColumnarReader, ColumnarWriter, DynamicColumn,
};
use tantivy::directory::{CompositeFile, CompositeWrite};
use tantivy::index::{Segment, SegmentComponent};
use tantivy::schema::Field;

use crate::api::CTID_FIELD_NAME;
#[cfg(feature = "io_stats")]
use crate::index::reader::io_stats::trace;

pub(super) const METADATA_IDX: usize = 7;
const COLUMNS_IDX: usize = 9;
const COLUMN_COUNT: usize = 3;
const PRESENCE: usize = 0;
const RANK: usize = 1;
const BOUNDARIES: usize = 2;
const CHUNK_SIZE: usize = 32768;
const CODECS: &[CodecType] = &[CodecType::Bitpacked, CodecType::BlockwiseLinearV2];

fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid heap-block columns")
}

#[derive(Serialize, Deserialize)]
pub(super) struct Metadata {
    first_word: u32,
    words: u32,
    blocks: u32,
    docs: u32,
    descending: bool,
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
    let first_word = u32::try_from(values.peek().unwrap() >> 16).map_err(|_| invalid())? / 32;
    let last_word =
        u32::try_from(column.values.get_val(if descending { 0 } else { docs - 1 }) >> 16)
            .map_err(|_| invalid())?
            / 32;
    let mut presence = ChunkWriter::new(field, PRESENCE);
    let mut ranks = ChunkWriter::new(field, RANK);
    let mut boundaries = ChunkWriter::new(field, BOUNDARIES);
    let mut blocks = 0u32;
    let mut processed = 0u32;
    let mut previous = None;

    for word in first_word..=last_word {
        pgrx::check_for_interrupts!();
        ranks.push(blocks, out)?;
        let mut bitmap = 0u32;
        while let Some(&value) = values.peek() {
            let block = u32::try_from(value >> 16).map_err(|_| invalid())?;
            if previous.is_some_and(|last| block < last) {
                return Ok(());
            }
            if block / 32 != word {
                break;
            }
            values.next();
            if previous != Some(block) {
                boundaries.push(processed, out)?;
                bitmap |= 1 << (block % 32);
                blocks += 1;
            }
            previous = Some(block);
            processed += 1;
        }
        presence.push(bitmap, out)?;
    }
    if processed != docs {
        return Err(invalid().into());
    }
    boundaries.push(docs, out)?;
    let metadata = Metadata {
        first_word,
        words: last_word - first_word + 1,
        blocks,
        docs,
        descending,
    };
    out.for_field_with_idx(field, METADATA_IDX)
        .write_all(&postcard::to_allocvec(&metadata).map_err(io::Error::other)?)?;
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
    first_word: u32,
    presence: ChunkReader,
    ranks: ChunkReader,
    boundaries: ChunkReader,
    docs: u32,
    blocks: u32,
    pages_per_vm: u32,
    descending: bool,
}

impl HeapBlockMap {
    /// Uses segment bounds to address bitmap words, leaving the columns unopened.
    pub(super) fn open(
        metadata: Metadata,
        file: Arc<CompositeFile>,
        field: Field,
        docs: u32,
        pages_per_vm: u32,
    ) -> io::Result<Self> {
        let Metadata {
            first_word,
            words,
            blocks,
            docs: stored_docs,
            descending,
        } = metadata;
        if stored_docs != docs
            || blocks == 0
            || blocks > docs
            || words == 0
            || u64::from(first_word) + u64::from(words) > (u64::from(u32::MAX) + 1) / 32
            || pages_per_vm == 0
            || !pages_per_vm.is_multiple_of(32)
        {
            return Err(invalid());
        }
        Ok(Self {
            descending,
            first_word,
            presence: ChunkReader::new(
                file.clone(),
                field,
                PRESENCE,
                words as usize,
                "Visibility Presence",
            ),
            ranks: ChunkReader::new(file.clone(), field, RANK, words as usize, "Visibility Rank"),
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
        let mut row = 0usize;
        while row < self.presence.count {
            pgrx::check_for_interrupts!();
            let first = (self.first_word + row as u32) * 32;
            let words = ((self.pages_per_vm - first % self.pages_per_vm) / 32) as usize;
            let words = words.min(self.presence.count - row);
            let missing = &mut scratch[..words];
            missing.fill(u32::MAX);
            retain_invisible(first, missing);
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
            row += words;
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
