// Copyright (c) 2023-2026 ParadeDB, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::api::CTID_FIELD_NAME;
use crate::postgres::heap::HEAPBLOCKS_PER_PAGE;
use std::io::{self, Read, Seek, SeekFrom, Write};
use tantivy::Directory;
use tantivy::columnar::column_values::{CodecType, serialize_u64_based_column_values};
use tantivy::columnar::{Cardinality, ColumnarReader, DynamicColumn};
use tantivy::directory::CompositeWrite;
use tantivy::index::{Segment, SegmentComponent};

const PRESENCE_IDX: usize = 3;
const BOUNDARIES_IDX: usize = 6;
const HEADER: usize = 24;
const ENTRY: usize = 16;
const BOUNDARY_HEADER: usize = 8;
const BOUNDARY_CHUNK_SIZE: usize = 32768;
const SPARSE: u32 = 1 << 31;

fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "invalid legacy heap-block presence map",
    )
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
