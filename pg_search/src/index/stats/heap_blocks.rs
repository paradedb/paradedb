// Copyright (c) 2023-2026 ParadeDB, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Visibility checking for a segment sorted by CTID:
//!
//! ```text
//! doc ID:             0  1  2  3  4  5  6
//! heap block:        10 10 11 11 11 15 15
//!
//! heap block:        10 11 12 13 14 15
//! presence:           1  1  0  0  0  1
//! VM all-visible:     1  0  1  1  1  1
//! needs checking:     0  1  0  0  0  0
//!                        |
//!                        v
//! rank(block 11) = 1 -> boundaries [0, 2, 5, 7] -> doc IDs [2, 5)
//!
//! query matches [0, 3, 6] -> only doc 3 needs a visibility check
//! ```
//!
//! CTID sorting keeps each heap block's documents together. Three structures let us move
//! from heap blocks to document ranges without walking every document:
//!
//! - **Presence** records which heap blocks occur in the segment. It uses a bitmap for
//!   dense blocks and a short list for sparse blocks, grouped by PostgreSQL visibility-map
//!   (VM) page so we can check all blocks covered by that page together.
//! - **Rank** gives a block's position among the present blocks. In the example, block 11
//!   is the second present block, so its rank is 1. Bitmap checkpoints and popcounts make
//!   this lookup cheap; sparse lists already provide the position.
//! - **Boundaries** map that rank to a contiguous document range. They are compressed in
//!   independently readable groups, so looking up one range doesn't require decoding
//!   earlier boundaries.
//!
//! Presence and boundaries are written into the segment's `.stats` file at flush and merge.
//! On the first eligible query batch, `VisibilityChecker` compares presence with the VM
//! and reads boundaries only for blocks that aren't all-visible. It caches the resulting
//! document ranges for its snapshot, then checks only query matches within those ranges.
//! If every present block is all-visible, no boundary reads or per-document visibility
//! checks are needed. Segments without this metadata use the existing visibility path.

use std::io::{self, Write};
use std::ops::Range;

use tantivy::HasLen;
use tantivy::columnar::{Cardinality, ColumnarReader, DynamicColumn};
use tantivy::directory::{CompositeWrite, FileSlice, OwnedBytes};
use tantivy::index::{Segment, SegmentComponent};

use crate::api::CTID_FIELD_NAME;
use crate::postgres::heap::HEAPBLOCKS_PER_PAGE;

pub(super) const PRESENCE_IDX: usize = 3;
pub(super) const BOUNDARIES_IDX: usize = 4;
const HEADER: usize = 24;
const ENTRY: usize = 16;
const SPARSE: u32 = 1 << 31;
const BOUNDARY_GROUP: usize = 128;

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
    let mut runs: Vec<(u32, u32)> = Vec::new();
    for (doc, value) in column.values.iter().enumerate() {
        if doc.is_multiple_of(8192) {
            pgrx::check_for_interrupts!();
        }
        let block = u32::try_from(value >> 16).map_err(|_| invalid())?;
        if let Some((last, count)) = runs.last_mut() {
            if block == *last {
                *count += 1;
                continue;
            }
            if (block < *last) != descending {
                return Ok(());
            }
        }
        runs.push((block, 1));
    }
    if descending {
        runs.reverse();
    }
    let (presence, boundaries) = encode(&runs, descending, docs, HEAPBLOCKS_PER_PAGE)?;
    out.for_field_with_idx(field, PRESENCE_IDX)
        .write_all(&presence)?;
    out.for_field_with_idx(field, BOUNDARIES_IDX)
        .write_all(&boundaries)?;
    Ok(())
}

/// Encodes ascending heap-block runs into VM-page payloads and packed document boundaries.
fn encode(
    runs: &[(u32, u32)],
    descending: bool,
    docs: u32,
    pages_per_vm: u32,
) -> io::Result<(Vec<u8>, Vec<u8>)> {
    let mut directory = Vec::new();
    let mut payload = Vec::new();
    let mut ordinal = 0;
    while ordinal < runs.len() {
        let vm_page = runs[ordinal].0 / pages_per_vm;
        let end = ordinal
            + runs[ordinal..].partition_point(|&(block, _)| block / pages_per_vm == vm_page);
        let count = end - ordinal;
        let base = vm_page * pages_per_vm;
        let first_word = (runs[ordinal].0 - base) / 32;
        let last_word = (runs[end - 1].0 - base) / 32;
        let words = (last_word - first_word + 1) as usize;
        let sparse = count * 2 <= 4 + words.div_ceil(8) * 2 + words * 4;
        directory.push((
            vm_page,
            ordinal as u32,
            u32::try_from(payload.len()).map_err(|_| invalid())?,
            count as u32 | if sparse { SPARSE } else { 0 },
        ));
        if sparse {
            for &(block, _) in &runs[ordinal..end] {
                payload.extend_from_slice(&((block - base) as u16).to_le_bytes());
            }
        } else {
            payload.extend_from_slice(&(first_word as u16).to_le_bytes());
            payload.extend_from_slice(&(words as u16).to_le_bytes());
            let mut bitmap = vec![0u32; words];
            for &(block, _) in &runs[ordinal..end] {
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
        ordinal = end;
    }
    let mut presence = Vec::new();
    for value in [
        u32::from_le_bytes(*b"HBP1"),
        pages_per_vm,
        docs,
        runs.len() as u32,
        directory.len() as u32,
        u32::from(descending),
    ] {
        presence.extend_from_slice(&value.to_le_bytes());
    }
    let payload_start = HEADER + directory.len() * ENTRY;
    for (vm_page, rank, offset, count) in directory {
        for value in [
            vm_page,
            rank,
            offset
                .checked_add(payload_start as u32)
                .ok_or_else(invalid)?,
            count,
        ] {
            presence.extend_from_slice(&value.to_le_bytes());
        }
    }
    presence.extend_from_slice(&payload);

    let mut starts = Vec::with_capacity(runs.len() + 1);
    let mut start = 0u32;
    for &(_, count) in runs {
        starts.push(start);
        start = start.checked_add(count).ok_or_else(invalid)?;
    }
    if start != docs {
        return Err(invalid());
    }
    starts.push(start);
    let groups = starts.len().div_ceil(BOUNDARY_GROUP);
    let mut boundaries = vec![0; groups * 8];
    for (group, values) in starts.chunks(BOUNDARY_GROUP).enumerate() {
        let offset = boundaries.len() as u64;
        boundaries[group * 8..group * 8 + 8].copy_from_slice(&offset.to_le_bytes());
        let base = values[0];
        let bits = 32 - (values[values.len() - 1] - base).leading_zeros();
        boundaries.extend_from_slice(&base.to_le_bytes());
        boundaries.extend_from_slice(&bits.to_le_bytes());
        let start = boundaries.len();
        boundaries.resize(start + (values.len() * bits as usize).div_ceil(8) + 8, 0);
        for (i, value) in values.iter().enumerate() {
            let bit = i * bits as usize;
            let offset = start + bit / 8;
            let packed = u64::from(*value - base) << (bit % 8);
            for (dst, src) in boundaries[offset..offset + 8]
                .iter_mut()
                .zip(packed.to_le_bytes())
            {
                *dst |= src;
            }
        }
    }
    Ok((presence, boundaries))
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
    directory: ReadWindow,
    values: ReadWindow,
    count: usize,
}

impl BoundaryReader {
    /// Creates a lazy boundary reader with separate directory and packed-value windows.
    fn new(file: FileSlice, count: usize) -> Self {
        Self {
            directory: ReadWindow::new(file.clone()),
            values: ReadWindow::new(file),
            count,
        }
    }

    /// Reads one boundary directly through its group offset, base, and packed delta.
    fn get(&mut self, index: usize) -> io::Result<u32> {
        if index >= self.count {
            return Err(invalid());
        }
        let group = index / BOUNDARY_GROUP;
        let offset = u64::from_le_bytes(self.directory.read(group * 8, 8)?.try_into().unwrap());
        let offset = usize::try_from(offset).map_err(|_| invalid())?;
        if offset < self.count.div_ceil(BOUNDARY_GROUP) * 8 {
            return Err(invalid());
        }
        let header = self.values.read(offset, 8)?;
        let base = u32_at(header, 0);
        let bits = u32_at(header, 4) as usize;
        if bits > 32 {
            return Err(invalid());
        }
        let bit = index % BOUNDARY_GROUP * bits;
        let word = u64::from_le_bytes(
            self.values
                .read(offset + 8 + bit / 8, 8)?
                .try_into()
                .unwrap(),
        );
        let delta = (word >> (bit % 8)) & ((1u64 << bits) - 1);
        base.checked_add(delta as u32).ok_or_else(invalid)
    }
}

struct ReadWindow {
    file: FileSlice,
    offset: usize,
    bytes: OwnedBytes,
}

impl ReadWindow {
    /// Creates an empty read window without fetching file bytes.
    fn new(file: FileSlice) -> Self {
        Self {
            file,
            offset: 0,
            bytes: OwnedBytes::empty(),
        }
    }

    /// Reads a checked byte range, reusing or refilling an aligned file window.
    fn read(&mut self, offset: usize, len: usize) -> io::Result<&[u8]> {
        let end = offset.checked_add(len).ok_or_else(invalid)?;
        if end > self.file.len() {
            return Err(invalid());
        }
        if offset < self.offset || end > self.offset + self.bytes.len() {
            self.offset = offset / 4096 * 4096;
            self.bytes = self
                .file
                .slice(self.offset..(self.offset + 8192).max(end).min(self.file.len()))
                .read_bytes()?;
        }
        Ok(&self.bytes[offset - self.offset..end - self.offset])
    }
}
