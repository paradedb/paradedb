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
//! Presence is stored compactly as bitmaps or sparse lists, with rank checkpoints for
//! bitmaps and independently readable, compressed boundaries. These live in the segment's
//! `.stats` file. Builds and merges spool one VM page at a time to temporary files, keeping
//! presence densely packed and compressing boundaries in bounded chunks.
//! `VisibilityChecker` performs the VM comparison once per eligible segment
//! and snapshot, caches the resulting ranges, and uses them to filter subsequent batches.

use std::io::{self, Read, Seek, SeekFrom, Write};
use std::ops::Range;
use std::sync::Arc;

use tantivy::columnar::column_values::{
    CodecType, load_u64_based_column_values, serialize_u64_based_column_values,
};
use tantivy::columnar::{Cardinality, ColumnValues, ColumnarReader, DynamicColumn};
use tantivy::directory::{CompositeWrite, FileSlice, OwnedBytes};
use tantivy::index::{Segment, SegmentComponent};
use tantivy::{Directory, HasLen};

use crate::api::CTID_FIELD_NAME;
use crate::postgres::heap::HEAPBLOCKS_PER_PAGE;

pub(super) const PRESENCE_IDX: usize = 3;
pub(super) const BOUNDARIES_IDX: usize = 6;
const HEADER: usize = 24;
const ENTRY: usize = 16;
const BOUNDARY_HEADER: usize = 8;
const BOUNDARY_CHUNK_SIZE: usize = 32768;
const SPARSE: u32 = 1 << 31;

fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "invalid heap-block presence map",
    )
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap())
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
    if HEAPBLOCKS_PER_PAGE > u16::MAX as u32 || !HEAPBLOCKS_PER_PAGE.is_multiple_of(32) {
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
    let mut presence = directory.open_temp_file()?;
    let mut boundaries = directory.open_temp_file()?;
    let mut boundary_offsets = directory.open_temp_file()?;
    let mut starts = Vec::with_capacity(BOUNDARY_CHUNK_SIZE);
    let mut boundary_bytes = 0u64;
    let mut flush_boundaries = |starts: &mut Vec<u32>| -> io::Result<()> {
        let mut encoded = Vec::new();
        serialize_u64_based_column_values(
            &starts.as_slice(),
            &[CodecType::BlockwiseLinearV2],
            &mut encoded,
        )?;
        boundary_offsets.write_all(&boundary_bytes.to_le_bytes())?;
        boundaries.write_all(&encoded)?;
        boundary_bytes += encoded.len() as u64;
        starts.clear();
        Ok(())
    };
    let mut present_blocks = Vec::with_capacity(HEAPBLOCKS_PER_PAGE as usize);
    let mut chunks = 0u32;
    let mut blocks = 0u32;
    let mut processed = 0u32;
    let mut presence_bytes = 0u32;
    let mut previous = None;

    // Spool one VM page at a time, keeping presence densely packed apart from boundaries.
    while let Some(&value) = values.peek() {
        pgrx::check_for_interrupts!();
        let block = u32::try_from(value >> 16).map_err(|_| invalid())?;
        let vm_page = block / HEAPBLOCKS_PER_PAGE;
        present_blocks.clear();
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
            if present_blocks.last() != Some(&block) {
                present_blocks.push(block);
                starts.push(processed);
                if starts.len() == BOUNDARY_CHUNK_SIZE {
                    flush_boundaries(&mut starts)?;
                }
            }
            previous = Some(block);
            processed += 1;
        }
        let (payload, sparse) = encode_presence(&present_blocks, HEAPBLOCKS_PER_PAGE);
        for value in [
            vm_page,
            blocks,
            presence_bytes,
            present_blocks.len() as u32 | if sparse { SPARSE } else { 0 },
        ] {
            entries.write_all(&value.to_le_bytes())?;
        }
        presence.write_all(&payload)?;
        presence_bytes = presence_bytes
            .checked_add(payload.len() as u32)
            .ok_or_else(invalid)?;
        blocks += present_blocks.len() as u32;
        chunks += 1;
    }
    if processed != docs {
        return Err(invalid().into());
    }

    starts.push(docs);
    flush_boundaries(&mut starts)?;

    let writer = out.for_field_with_idx(field, PRESENCE_IDX);
    for value in [
        u32::from_le_bytes(*b"HBP1"),
        HEAPBLOCKS_PER_PAGE,
        docs,
        blocks,
        chunks,
        u32::from(descending),
    ] {
        writer.write_all(&value.to_le_bytes())?;
    }
    let payload_start = HEADER as u32 + chunks * ENTRY as u32;
    payload_start
        .checked_add(presence_bytes)
        .ok_or_else(invalid)?;
    entries.seek(SeekFrom::Start(0))?;
    let mut entry = [0u8; ENTRY];
    for _ in 0..chunks {
        entries.read_exact(&mut entry)?;
        let offset = payload_start + u32_at(&entry, 8);
        entry[8..12].copy_from_slice(&offset.to_le_bytes());
        writer.write_all(&entry)?;
    }
    presence.seek(SeekFrom::Start(0))?;
    io::copy(&mut presence, writer)?;

    let writer = out.for_field_with_idx(field, BOUNDARIES_IDX);
    writer.write_all(b"HBB1")?;
    let boundary_chunks = (u64::from(blocks) + 1).div_ceil(BOUNDARY_CHUNK_SIZE as u64) as u32;
    writer.write_all(&boundary_chunks.to_le_bytes())?;
    let payload_start = BOUNDARY_HEADER as u64 + (u64::from(boundary_chunks) + 1) * 8;
    boundary_offsets.seek(SeekFrom::Start(0))?;
    let mut offset = [0u8; 8];
    for _ in 0..boundary_chunks {
        boundary_offsets.read_exact(&mut offset)?;
        let offset = payload_start + u64::from_le_bytes(offset);
        writer.write_all(&offset.to_le_bytes())?;
    }
    writer.write_all(&(payload_start + boundary_bytes).to_le_bytes())?;
    boundaries.seek(SeekFrom::Start(0))?;
    io::copy(&mut boundaries, writer)?;
    Ok(())
}

/// Encodes one VM page's presence as a sparse list or a bitmap with rank checkpoints.
fn encode_presence(blocks: &[u32], pages_per_vm: u32) -> (Vec<u8>, bool) {
    let base = blocks[0] / pages_per_vm * pages_per_vm;
    let first_word = (blocks[0] - base) / 32;
    let last_word = (blocks[blocks.len() - 1] - base) / 32;
    let words = (last_word - first_word + 1) as usize;
    let sparse = blocks.len() * 2 <= 4 + words.div_ceil(8) * 2 + words * 4;
    let mut payload = Vec::new();
    if sparse {
        for &block in blocks {
            payload.extend_from_slice(&((block - base) as u16).to_le_bytes());
        }
    } else {
        payload.extend_from_slice(&(first_word as u16).to_le_bytes());
        payload.extend_from_slice(&(words as u16).to_le_bytes());
        let mut bitmap = vec![0u32; words];
        for &block in blocks {
            let offset = block - base;
            bitmap[(offset / 32 - first_word) as usize] |= 1 << (offset % 32);
        }
        let mut rank = 0u16;
        for (i, &word) in bitmap.iter().enumerate() {
            if i.is_multiple_of(8) {
                payload.extend_from_slice(&rank.to_le_bytes());
            }
            rank += word.count_ones() as u16;
        }
        for word in bitmap {
            payload.extend_from_slice(&word.to_le_bytes());
        }
    }

    (payload, sparse)
}

pub(crate) struct HeapBlockMap {
    presence: OwnedBytes,
    boundaries: BoundaryReader,
    docs: u32,
    pages_per_vm: u32,
    descending: bool,
}

impl HeapBlockMap {
    /// Loads and validates presence metadata while deferring boundary reads until needed.
    pub(super) fn open(
        presence: FileSlice,
        boundaries: FileSlice,
        docs: u32,
        pages_per_vm: u32,
    ) -> io::Result<Self> {
        let presence = presence.read_bytes()?;
        if presence.len() < HEADER
            || &presence[..4] != b"HBP1"
            || u32_at(&presence, 4) != pages_per_vm
            || u32_at(&presence, 8) != docs
            || pages_per_vm == 0
            || pages_per_vm > u16::MAX as u32
            || !pages_per_vm.is_multiple_of(32)
            || u32_at(&presence, 20) > 1
        {
            return Err(invalid());
        }
        let chunks = u32_at(&presence, 16) as usize;
        let blocks = u32_at(&presence, 12);
        if HEADER + chunks * ENTRY > presence.len() || blocks > docs {
            return Err(invalid());
        }
        let mut previous = None;
        let mut rank = 0u32;
        for i in 0..chunks {
            let at = HEADER + i * ENTRY;
            let page = u32_at(&presence, at);
            let count = u32_at(&presence, at + 12) & !SPARSE;
            let offset = u32_at(&presence, at + 8) as usize;
            let end = if i + 1 < chunks {
                u32_at(&presence, at + ENTRY + 8) as usize
            } else {
                presence.len()
            };
            if previous.is_some_and(|prev| prev >= page)
                || u32_at(&presence, at + 4) != rank
                || count == 0
                || count > pages_per_vm
                || offset < HEADER + chunks * ENTRY
                || offset > end
                || end > presence.len()
                || u64::from(page) * u64::from(pages_per_vm) > u64::from(u32::MAX)
            {
                return Err(invalid());
            }
            let bytes = &presence[offset..end];
            if u32_at(&presence, at + 12) & SPARSE != 0 {
                if bytes.len() != count as usize * 2 {
                    return Err(invalid());
                }
                let mut prev = None;
                for value in bytes.chunks_exact(2) {
                    let block = u16_at(value, 0);
                    if u32::from(block) >= pages_per_vm
                        || u64::from(page) * u64::from(pages_per_vm) + u64::from(block)
                            > u64::from(u32::MAX)
                        || prev.is_some_and(|p| p >= block)
                    {
                        return Err(invalid());
                    }
                    prev = Some(block);
                }
            } else {
                if bytes.len() < 4 {
                    return Err(invalid());
                }
                let words = u16_at(bytes, 2) as usize;
                let ranks = words.div_ceil(8) * 2;
                if words == 0
                    || usize::from(u16_at(bytes, 0)) + words > pages_per_vm as usize / 32
                    || u64::from(page) * u64::from(pages_per_vm)
                        + (u64::from(u16_at(bytes, 0)) + words as u64) * 32
                        - 1
                        > u64::from(u32::MAX)
                    || bytes.len() != 4 + ranks + words * 4
                {
                    return Err(invalid());
                }
                let mut population = 0u32;
                for word in 0..words {
                    if word.is_multiple_of(8)
                        && u32::from(u16_at(bytes, 4 + word / 8 * 2)) != population
                    {
                        return Err(invalid());
                    }
                    population += u32_at(bytes, 4 + ranks + word * 4).count_ones();
                }
                if population != count {
                    return Err(invalid());
                }
            }
            rank = rank.checked_add(count).ok_or_else(invalid)?;
            previous = Some(page);
        }
        if rank != blocks || blocks == 0 {
            return Err(invalid());
        }
        Ok(Self {
            descending: u32_at(&presence, 20) != 0,
            presence,
            boundaries: BoundaryReader::new(boundaries, blocks as usize + 1),
            docs,
            pages_per_vm,
        })
    }

    /// Filters presence through the VM callback and returns coalesced ranges in document order.
    pub(crate) fn missing_ranges(
        &mut self,
        mut retain_invisible: impl FnMut(u32, &mut [u32]),
    ) -> io::Result<Vec<Range<u32>>> {
        let mut ranges: Vec<Range<u32>> = Vec::new();
        let mut scratch = vec![0u32; self.pages_per_vm as usize / 32];
        for chunk in 0..u32_at(&self.presence, 16) as usize {
            pgrx::check_for_interrupts!();
            let at = HEADER + chunk * ENTRY;
            let base = u32_at(&self.presence, at) * self.pages_per_vm;
            let ordinal = u32_at(&self.presence, at + 4);
            let offset = u32_at(&self.presence, at + 8) as usize;
            let count = u32_at(&self.presence, at + 12);
            let bytes = &self.presence[offset..];
            let (first_word, words) = if count & SPARSE != 0 {
                let count = (count & !SPARSE) as usize;
                let first = usize::from(u16_at(bytes, 0)) / 32;
                let last = usize::from(u16_at(bytes, (count - 1) * 2)) / 32;
                scratch[..=last - first].fill(0);
                for i in 0..count {
                    let block = usize::from(u16_at(bytes, i * 2));
                    scratch[block / 32 - first] |= 1 << (block % 32);
                }
                (first, last - first + 1)
            } else {
                let first = usize::from(u16_at(bytes, 0));
                let words = usize::from(u16_at(bytes, 2));
                let bitmap = 4 + words.div_ceil(8) * 2;
                for (i, mask) in scratch[..words].iter_mut().enumerate() {
                    *mask = u32_at(bytes, bitmap + i * 4);
                }
                (first, words)
            };
            let missing = &mut scratch[..words];
            retain_invisible(base + first_word as u32 * 32, missing);
            if missing.iter().all(|&mask| mask == 0) {
                continue;
            }
            let mut add = |rank: u32| -> io::Result<()> {
                let start = self.boundaries.get(rank as usize)?;
                let end = self.boundaries.get(rank as usize + 1)?;
                if start >= end || end > self.docs {
                    return Err(invalid());
                }
                let range = if self.descending {
                    self.docs - end..self.docs - start
                } else {
                    start..end
                };
                if let Some(last) = ranges.last_mut() {
                    if last.end == range.start {
                        last.end = range.end;
                        return Ok(());
                    }
                    if range.end == last.start {
                        last.start = range.start;
                        return Ok(());
                    }
                }
                ranges.push(range);
                Ok(())
            };
            if count & SPARSE != 0 {
                let count = (count & !SPARSE) as usize;
                let mut i = 0;
                while i < count {
                    let first = i;
                    let word = u32::from(u16_at(bytes, i * 2)) / 32;
                    let mut present = 0u32;
                    while i < count && u32::from(u16_at(bytes, i * 2)) / 32 == word {
                        present |= 1 << (u16_at(bytes, i * 2) % 32);
                        i += 1;
                    }
                    let mut bad = missing[word as usize - first_word] & present;
                    while bad != 0 {
                        let bit = bad.trailing_zeros();
                        add(ordinal + first as u32 + (present & ((1u32 << bit) - 1)).count_ones())?;
                        bad &= bad - 1;
                    }
                }
            } else {
                let bitmap = 4 + words.div_ceil(8) * 2;
                for (word, &bad) in missing.iter().enumerate() {
                    if bad == 0 {
                        continue;
                    }
                    let present = u32_at(bytes, bitmap + word * 4);
                    let mut bad = bad & present;
                    if bad == 0 {
                        continue;
                    }
                    let mut rank = ordinal + u32::from(u16_at(bytes, 4 + word / 8 * 2));
                    for previous in word / 8 * 8..word {
                        rank += u32_at(bytes, bitmap + previous * 4).count_ones();
                    }
                    while bad != 0 {
                        let bit = bad.trailing_zeros();
                        add(rank + (present & ((1u32 << bit) - 1)).count_ones())?;
                        bad &= bad - 1;
                    }
                }
            }
        }
        if self.descending {
            ranges.reverse();
        }
        Ok(ranges)
    }
}

struct BoundaryReader {
    file: FileSlice,
    count: usize,
    values: Option<(usize, Arc<dyn ColumnValues<u32>>)>,
}

impl BoundaryReader {
    /// Defers reading the boundary directory and columns until a heap block needs checks.
    fn new(file: FileSlice, count: usize) -> Self {
        Self {
            file,
            count,
            values: None,
        }
    }

    /// Reads a boundary by ordinal, retaining at most one fixed-size compressed column.
    fn get(&mut self, index: usize) -> io::Result<u32> {
        if index >= self.count {
            return Err(invalid());
        }
        let chunk = index / BOUNDARY_CHUNK_SIZE;
        if let Some((current, values)) = &self.values
            && *current == chunk
        {
            return Ok(values.get_val((index % BOUNDARY_CHUNK_SIZE) as u32));
        }
        let chunks = self.count.div_ceil(BOUNDARY_CHUNK_SIZE);
        let payload_start = BOUNDARY_HEADER + (chunks + 1) * 8;
        if self.values.is_none() {
            if self.file.len() < payload_start {
                return Err(invalid());
            }
            let header = self.file.slice(..BOUNDARY_HEADER).read_bytes()?;
            if &header[..4] != b"HBB1" || u32_at(&header, 4) as usize != chunks {
                return Err(invalid());
            }
        }
        let at = BOUNDARY_HEADER + chunk * 8;
        let offsets = self.file.slice(at..at + 16).read_bytes()?;
        let start = usize::try_from(u64::from_le_bytes(offsets[..8].try_into().unwrap()))
            .map_err(|_| invalid())?;
        let end = usize::try_from(u64::from_le_bytes(offsets[8..].try_into().unwrap()))
            .map_err(|_| invalid())?;
        if start < payload_start || start >= end || end > self.file.len() {
            return Err(invalid());
        }
        let values = load_u64_based_column_values::<u32>(self.file.slice(start..end))?;
        let count = (self.count - chunk * BOUNDARY_CHUNK_SIZE).min(BOUNDARY_CHUNK_SIZE);
        if values.num_vals() as usize != count {
            return Err(invalid());
        }
        self.values = Some((chunk, values));
        let (_, values) = self.values.as_ref().unwrap();
        Ok(values.get_val((index % BOUNDARY_CHUNK_SIZE) as u32))
    }
}
