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
//! In addition to filtering visibility checks, `HeapBlockMap` serves as a run-length
//! decoder mapping document IDs to heap block numbers, allowing `TidReader` to bypass
//! reading the `tid_block` columnar fast field for immutable CTID-sorted segments.

use std::io::{self, Read, Seek, SeekFrom, Write};
use std::ops::Range;
use std::sync::Arc;

use tantivy::columnar::column_values::{
    CodecType, load_u64_based_column_values, serialize_u64_based_column_values,
};
use tantivy::columnar::{Cardinality, ColumnValues, ColumnarReader, DynamicColumn};
use tantivy::directory::{CompositeWrite, FileSlice, OwnedBytes};
use tantivy::index::{Segment, SegmentComponent};
use tantivy::{Directory, DocId, HasLen};

use crate::api::{CTID_FIELD_NAME, TID_BLOCK_FIELD_NAME, TID_OFFSET_FIELD_NAME};
use crate::postgres::heap::HEAPBLOCKS_PER_PAGE;
use crate::postgres::utils::TidBlock;

pub(super) const PRESENCE_IDX: usize = 3;
pub(super) const BOUNDARIES_IDX: usize = 6;
const HEADER: usize = 24;
const ENTRY: usize = 16;
const BOUNDARY_HEADER: usize = 8;
const BOUNDARY_CHUNK_SIZE: usize = 32768;
const SPARSE: u32 = 1 << 31;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MissingBlockRange {
    pub doc_range: Range<DocId>,
    pub block: TidBlock,
}

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
    let settings = segment.index().settings();
    let is_ctid_sorted = match settings.sort_by_fields() {
        [sort] if sort.field == CTID_FIELD_NAME => true,
        [block, offset]
            if block.field == TID_BLOCK_FIELD_NAME
                && offset.field == TID_OFFSET_FIELD_NAME
                && block.order == offset.order =>
        {
            true
        }
        _ => false,
    };
    if !is_ctid_sorted {
        return Ok(());
    }
    if HEAPBLOCKS_PER_PAGE > u16::MAX as u32 || !HEAPBLOCKS_PER_PAGE.is_multiple_of(32) {
        return Ok(());
    }
    let schema = segment.schema();
    let (field, is_split) = if let Ok(block_field) = schema.get_field(TID_BLOCK_FIELD_NAME) {
        (block_field, true)
    } else if let Ok(ctid_field) = schema.get_field(CTID_FIELD_NAME) {
        (ctid_field, false)
    } else {
        return Ok(());
    };
    let fast = ColumnarReader::open(segment.open_read(SegmentComponent::FastFields)?)?;
    let field_name = if is_split {
        TID_BLOCK_FIELD_NAME
    } else {
        CTID_FIELD_NAME
    };
    let handles = fast.read_columns(field_name)?;
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

    let get_block = |value: u64| -> io::Result<u32> {
        if is_split {
            u32::try_from(value).map_err(|_| invalid())
        } else {
            u32::try_from(value >> 16).map_err(|_| invalid())
        }
    };

    // Spool one VM page at a time, keeping presence densely packed apart from boundaries.
    while let Some(&value) = values.peek() {
        pgrx::check_for_interrupts!();
        let block = get_block(value)?;
        let vm_page = block / HEAPBLOCKS_PER_PAGE;
        present_blocks.clear();
        while let Some(&value) = values.peek() {
            let block = get_block(value)?;
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

#[derive(Debug)]
pub struct HeapBlockMap {
    presence: OwnedBytes,
    boundaries: BoundaryReader,
    docs: u32,
    blocks: u32,
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
            blocks,
            pages_per_vm,
        })
    }

    /// Filters presence through the VM callback and returns ranges needing checks with their block numbers,
    /// ordered by document ID.
    pub(crate) fn missing_ranges(
        &mut self,
        mut retain_invisible: impl FnMut(u32, &mut [u32]),
    ) -> io::Result<Vec<MissingBlockRange>> {
        let mut ranges: Vec<MissingBlockRange> = Vec::new();
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
            let mut add = |rank: u32, block: u32| -> io::Result<()> {
                let start = self.boundaries.get(rank as usize)?;
                let end = self.boundaries.get(rank as usize + 1)?;
                if start >= end || end > self.docs {
                    return Err(invalid());
                }
                let doc_range = if self.descending {
                    self.docs - end..self.docs - start
                } else {
                    start..end
                };
                ranges.push(MissingBlockRange { doc_range, block });
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
                        let block = base + word * 32 + bit;
                        let rank =
                            ordinal + first as u32 + (present & ((1u32 << bit) - 1)).count_ones();
                        add(rank, block)?;
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
                        let block = base + (first_word + word) as u32 * 32 + bit;
                        add(rank + (present & ((1u32 << bit) - 1)).count_ones(), block)?;
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

    /// Returns the minimum block number present in this map.
    #[inline(always)]
    pub(crate) fn min_block(&self) -> io::Result<u32> {
        self.block_at_rank(0)
    }

    /// Returns the maximum block number present in this map.
    #[inline(always)]
    pub(crate) fn max_block(&self) -> io::Result<u32> {
        self.block_at_rank(self.blocks - 1)
    }

    /// Returns the heap block number at the given presence rank.
    pub(crate) fn block_at_rank(&self, target_rank: u32) -> io::Result<u32> {
        if target_rank >= self.blocks {
            return Err(invalid());
        }
        let chunks = u32_at(&self.presence, 16) as usize;
        let mut low = 1;
        let mut high = chunks;
        while low < high {
            let mid = low + (high - low) / 2;
            if u32_at(&self.presence, HEADER + mid * ENTRY + 4) <= target_rank {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        let chunk_idx = low - 1;

        let at = HEADER + chunk_idx * ENTRY;
        let page = u32_at(&self.presence, at);
        let base = page * self.pages_per_vm;
        let chunk_rank = u32_at(&self.presence, at + 4);
        let count_field = u32_at(&self.presence, at + 12);
        let count = count_field & !SPARSE;
        let is_sparse = (count_field & SPARSE) != 0;
        let offset = u32_at(&self.presence, at + 8) as usize;

        if target_rank < chunk_rank || target_rank >= chunk_rank + count {
            return Err(invalid());
        }
        let rel_rank = target_rank - chunk_rank;

        if is_sparse {
            let block_offset = u16_at(&self.presence, offset + rel_rank as usize * 2);
            Ok(base + block_offset as u32)
        } else {
            let bytes = &self.presence[offset..];
            let first_word = usize::from(u16_at(bytes, 0));
            let words = usize::from(u16_at(bytes, 2));
            let ranks_bytes = words.div_ceil(8) * 2;
            let num_checkpoints = words.div_ceil(8);
            let mut cp_low = 1;
            let mut cp_high = num_checkpoints;
            while cp_low < cp_high {
                let mid = cp_low + (cp_high - cp_low) / 2;
                if usize::from(u16_at(bytes, 4 + mid * 2)) <= rel_rank as usize {
                    cp_low = mid + 1;
                } else {
                    cp_high = mid;
                }
            }
            let cp = cp_low - 1;

            let mut current_rank = usize::from(u16_at(bytes, 4 + cp * 2));
            let start_word = cp * 8;
            let end_word = (start_word + 8).min(words);

            for word_idx in start_word..end_word {
                let word_val = u32_at(bytes, 4 + ranks_bytes + word_idx * 4);
                let ones = word_val.count_ones() as usize;
                if current_rank + ones > rel_rank as usize {
                    let needed = rel_rank as usize - current_rank;
                    let mut w = word_val;
                    for _ in 0..needed {
                        w &= w - 1;
                    }
                    let bit = w.trailing_zeros();
                    let block = base + (first_word + word_idx) as u32 * 32 + bit;
                    return Ok(block);
                }
                current_rank += ones;
            }
            Err(invalid())
        }
    }

    /// Returns the presence rank for a document ID.
    pub(crate) fn rank_of_doc(&mut self, doc: DocId) -> io::Result<u32> {
        if doc >= self.docs {
            return Err(invalid());
        }
        let processed = if self.descending {
            self.docs - 1 - doc
        } else {
            doc
        };

        let mut low = 1;
        let mut high = self.blocks;
        while low < high {
            let mid = low + (high - low) / 2;
            if self.boundaries.get(mid as usize)? <= processed {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        let rank = low - 1;
        Ok(rank)
    }

    /// Returns the heap block number for a document ID.
    pub(crate) fn block_of_doc(&mut self, doc: DocId) -> io::Result<u32> {
        let rank = self.rank_of_doc(doc)?;
        self.block_at_rank(rank)
    }

    /// Populates block numbers for a sorted slice of document IDs.
    pub(crate) fn blocks_of_docs(
        &mut self,
        docs: &[DocId],
        output: &mut [TidBlock],
    ) -> io::Result<()> {
        if docs.is_empty() {
            return Ok(());
        }
        if docs.len() != output.len() {
            return Err(invalid());
        }
        let mut current_rank = 0u32;
        let mut current_end = self.boundaries.get(1)?;
        let mut current_block = self.block_at_rank(0)?;

        if !self.descending {
            for (i, &doc) in docs.iter().enumerate() {
                if doc >= self.docs {
                    return Err(invalid());
                }
                if doc >= current_end {
                    if current_rank + 1 < self.blocks {
                        let next_end = self.boundaries.get((current_rank + 2) as usize)?;
                        if doc < next_end {
                            current_rank += 1;
                            current_end = next_end;
                            current_block = self.block_at_rank(current_rank)?;
                            output[i] = current_block;
                            continue;
                        }
                    }
                    let mut low = current_rank + 2;
                    let mut high = self.blocks;
                    while low < high {
                        let mid = low + (high - low) / 2;
                        if self.boundaries.get(mid as usize)? <= doc {
                            low = mid + 1;
                        } else {
                            high = mid;
                        }
                    }
                    current_rank = low - 1;
                    current_end = self.boundaries.get((current_rank + 1) as usize)?;
                    current_block = self.block_at_rank(current_rank)?;
                }
                output[i] = current_block;
            }
        } else {
            for i in (0..docs.len()).rev() {
                let doc = docs[i];
                if doc >= self.docs {
                    return Err(invalid());
                }
                let processed = self.docs - 1 - doc;
                if processed >= current_end {
                    if current_rank + 1 < self.blocks {
                        let next_end = self.boundaries.get((current_rank + 2) as usize)?;
                        if processed < next_end {
                            current_rank += 1;
                            current_end = next_end;
                            current_block = self.block_at_rank(current_rank)?;
                            output[i] = current_block;
                            continue;
                        }
                    }
                    let mut low = current_rank + 2;
                    let mut high = self.blocks;
                    while low < high {
                        let mid = low + (high - low) / 2;
                        if self.boundaries.get(mid as usize)? <= processed {
                            low = mid + 1;
                        } else {
                            high = mid;
                        }
                    }
                    current_rank = low - 1;
                    current_end = self.boundaries.get((current_rank + 1) as usize)?;
                    current_block = self.block_at_rank(current_rank)?;
                }
                output[i] = current_block;
            }
        }
        Ok(())
    }

    /// Populates block numbers as `Option<u64>` for a sorted slice of document IDs.
    pub(crate) fn blocks_of_docs_u64(
        &mut self,
        docs: &[DocId],
        output: &mut [Option<u64>],
    ) -> io::Result<()> {
        if docs.is_empty() {
            return Ok(());
        }
        if docs.len() != output.len() {
            return Err(invalid());
        }
        let mut current_rank = 0u32;
        let mut current_end = self.boundaries.get(1)?;
        let mut current_block = self.block_at_rank(0)?;

        if !self.descending {
            for (i, &doc) in docs.iter().enumerate() {
                if doc >= self.docs {
                    output[i] = None;
                    continue;
                }
                if doc >= current_end {
                    if current_rank + 1 < self.blocks {
                        let next_end = self.boundaries.get((current_rank + 2) as usize)?;
                        if doc < next_end {
                            current_rank += 1;
                            current_end = next_end;
                            current_block = self.block_at_rank(current_rank)?;
                            output[i] = Some(current_block as u64);
                            continue;
                        }
                    }
                    let mut low = current_rank + 2;
                    let mut high = self.blocks;
                    while low < high {
                        let mid = low + (high - low) / 2;
                        if self.boundaries.get(mid as usize)? <= doc {
                            low = mid + 1;
                        } else {
                            high = mid;
                        }
                    }
                    current_rank = low - 1;
                    current_end = self.boundaries.get((current_rank + 1) as usize)?;
                    current_block = self.block_at_rank(current_rank)?;
                }
                output[i] = Some(current_block as u64);
            }
        } else {
            for i in (0..docs.len()).rev() {
                let doc = docs[i];
                if doc >= self.docs {
                    output[i] = None;
                    continue;
                }
                let processed = self.docs - 1 - doc;
                if processed >= current_end {
                    if current_rank + 1 < self.blocks {
                        let next_end = self.boundaries.get((current_rank + 2) as usize)?;
                        if processed < next_end {
                            current_rank += 1;
                            current_end = next_end;
                            current_block = self.block_at_rank(current_rank)?;
                            output[i] = Some(current_block as u64);
                            continue;
                        }
                    }
                    let mut low = current_rank + 2;
                    let mut high = self.blocks;
                    while low < high {
                        let mid = low + (high - low) / 2;
                        if self.boundaries.get(mid as usize)? <= processed {
                            low = mid + 1;
                        } else {
                            high = mid;
                        }
                    }
                    current_rank = low - 1;
                    current_end = self.boundaries.get((current_rank + 1) as usize)?;
                    current_block = self.block_at_rank(current_rank)?;
                }
                output[i] = Some(current_block as u64);
            }
        }
        Ok(())
    }
}

const BOUNDARY_CACHE_CAPACITY: usize = 16;

struct BoundaryReader {
    file: FileSlice,
    count: usize,
    header_checked: bool,
    cache: Vec<(usize, Arc<dyn ColumnValues<u32>>)>,
}

impl std::fmt::Debug for BoundaryReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoundaryReader")
            .field("count", &self.count)
            .field(
                "cached_chunks",
                &self
                    .cache
                    .iter()
                    .map(|(chunk, _)| *chunk)
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl BoundaryReader {
    /// Defers reading the boundary directory and columns until a heap block needs checks.
    fn new(file: FileSlice, count: usize) -> Self {
        Self {
            file,
            count,
            header_checked: false,
            cache: Vec::with_capacity(BOUNDARY_CACHE_CAPACITY),
        }
    }

    /// Reads a boundary by ordinal, caching decoded chunks in an LRU cache.
    fn get(&mut self, index: usize) -> io::Result<u32> {
        if index >= self.count {
            return Err(invalid());
        }
        let chunk = index / BOUNDARY_CHUNK_SIZE;
        if let Some(pos) = self.cache.iter().position(|(c, _)| *c == chunk) {
            if pos > 0 {
                let entry = self.cache.remove(pos);
                self.cache.insert(0, entry);
            }
            return Ok(self.cache[0]
                .1
                .get_val((index % BOUNDARY_CHUNK_SIZE) as u32));
        }
        let chunks = self.count.div_ceil(BOUNDARY_CHUNK_SIZE);
        let payload_start = BOUNDARY_HEADER + (chunks + 1) * 8;
        if !self.header_checked {
            if self.file.len() < payload_start {
                return Err(invalid());
            }
            let header = self.file.slice(..BOUNDARY_HEADER).read_bytes()?;
            if &header[..4] != b"HBB1" || u32_at(&header, 4) as usize != chunks {
                return Err(invalid());
            }
            self.header_checked = true;
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
        if self.cache.len() == BOUNDARY_CACHE_CAPACITY {
            self.cache.pop();
        }
        self.cache.insert(0, (chunk, Arc::clone(&values)));
        Ok(values.get_val((index % BOUNDARY_CHUNK_SIZE) as u32))
    }
}
