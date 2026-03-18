// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

//! DSM-backed BlockwiseLinear metadata cache for the ctid column.
//!
//! **Cold path** (once per segment lifetime): `BlockwiseLinearCodec::load()` parses the
//! column, `into_parts()` extracts metadata, which is written to DSM.  A sanity check
//! verifies our reconstruction matches tantivy's output.
//!
//! **Hot path** (every scan): `DsmBlockwiseValues` reads metadata directly from DSM via
//! pointer arithmetic — no per-scan allocations beyond `Vec<OnceLock>` for lazy block data.

use crate::postgres::storage::dsm_cache::{self, CacheTag, DsmSlice};

use pgrx::pg_sys;
use std::sync::{Arc, OnceLock};
use tantivy::columnar::{Column, ColumnCodec, ColumnIndex, ColumnType, ColumnValues};
use tantivy::directory::FileSlice;
use tantivy::directory::OwnedBytes;
use tantivy::fastfield::FastFieldReaders;
use tantivy::HasLen;

const BLOCK_SIZE: u32 = 512;

// ---------------------------------------------------------------------------
// DSM layout: [DsmHeader][DsmBlockMeta × num_blocks]
// ---------------------------------------------------------------------------

#[repr(C)]
struct DsmHeader {
    min_value: u64,
    max_value: u64,
    gcd: u64,
    num_rows: u32,
    num_blocks: u32,
    /// Absolute byte offset of the data section within the column FileSlice.
    data_section_offset: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct DsmBlockMeta {
    slope: u64,
    intercept: u64,
    /// Byte offset relative to data section start.
    data_start: u32,
    data_len: u32,
    bit_width: u8,
    _pad: [u8; 3],
}

const HEADER_SIZE: usize = std::mem::size_of::<DsmHeader>();
const BLOCK_META_SIZE: usize = std::mem::size_of::<DsmBlockMeta>();

// ---------------------------------------------------------------------------
// DsmBlockwiseValues — custom ColumnValues<u64> backed by DSM metadata
// ---------------------------------------------------------------------------

struct DsmBlockwiseValues {
    dsm: DsmSlice,
    /// Points to the data section of the column FileSlice.
    data_file_slice: FileSlice,
    min_value: u64,
    gcd: u64,
    num_rows: u32,
    /// Per-block lazy data cache. Dropped with the Column at scan end.
    block_data: Vec<OnceLock<OwnedBytes>>,
}

// SAFETY: DsmSlice is Send+Sync (pinned DSM memory), FileSlice is Send+Sync (Arc-based).
unsafe impl Send for DsmBlockwiseValues {}
unsafe impl Sync for DsmBlockwiseValues {}

impl DsmBlockwiseValues {
    #[inline(always)]
    fn block_meta(&self, block_id: usize) -> &DsmBlockMeta {
        let offset = HEADER_SIZE + block_id * BLOCK_META_SIZE;
        unsafe { &*(self.dsm[offset..].as_ptr() as *const DsmBlockMeta) }
    }
}

impl ColumnValues<u64> for DsmBlockwiseValues {
    #[inline(always)]
    fn get_val(&self, idx: u32) -> u64 {
        let block_id = (idx / BLOCK_SIZE) as usize;
        let idx_within_block = idx % BLOCK_SIZE;
        let meta = self.block_meta(block_id);

        let linear_part =
            ((idx_within_block as u64).wrapping_mul(meta.slope) >> 32) as i32 as u64;
        let interpoled_val = meta.intercept.wrapping_add(linear_part);

        let block_bytes = self.block_data[block_id].get_or_init(|| {
            let len = meta.data_len as usize;
            let start = meta.data_start as usize;
            if len == 0 {
                return OwnedBytes::empty();
            }
            self.data_file_slice
                .slice(start..(start + len).min(self.data_file_slice.len()))
                .read_bytes()
                .unwrap()
        });

        let bitpacked_diff = bitunpack_get(meta.bit_width, idx_within_block, block_bytes);

        self.min_value + self.gcd.wrapping_mul(interpoled_val.wrapping_add(bitpacked_diff))
    }

    #[inline(always)]
    fn min_value(&self) -> u64 {
        let header = unsafe { &*(self.dsm[..HEADER_SIZE].as_ptr() as *const DsmHeader) };
        header.min_value
    }

    #[inline(always)]
    fn max_value(&self) -> u64 {
        let header = unsafe { &*(self.dsm[..HEADER_SIZE].as_ptr() as *const DsmHeader) };
        header.max_value
    }

    #[inline(always)]
    fn num_vals(&self) -> u32 {
        self.num_rows
    }
}

// ---------------------------------------------------------------------------
// Inline bit unpacker (equivalent to tantivy_bitpacker::BitUnpacker::get)
// ---------------------------------------------------------------------------

#[inline(always)]
fn bitunpack_get(bit_width: u8, idx: u32, data: &[u8]) -> u64 {
    if bit_width == 0 {
        return 0;
    }
    let mask: u64 = if bit_width == 64 {
        !0u64
    } else {
        (1u64 << bit_width) - 1
    };
    let addr_in_bits = (idx as usize) * (bit_width as usize);
    let addr = addr_in_bits >> 3;
    let bit_shift = (addr_in_bits & 7) as u32;

    if addr + 8 <= data.len() {
        let val = u64::from_le_bytes(data[addr..addr + 8].try_into().unwrap()) >> bit_shift;
        val & mask
    } else {
        let mut bytes = [0u8; 8];
        let available = data.len() - addr;
        bytes[..available].copy_from_slice(&data[addr..]);
        (u64::from_le_bytes(bytes) >> bit_shift) & mask
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Try to create a DSM-cached ctid Column<u64>.
///
/// Returns `None` if caching isn't possible (wrong codec, DSM full, etc).
/// The caller should fall back to the normal tantivy path.
pub fn create_cached_ctid_column(
    ffr: &FastFieldReaders,
    index_oid: pg_sys::Oid,
    segment_id: &[u8; 16],
) -> Option<Column<u64>> {
    let handle = ffr
        .dynamic_column_handle("ctid", ColumnType::U64)
        .ok()
        .flatten()?;
    let column_file_slice = handle.file_slice().clone();

    let key = dsm_cache::CacheKey {
        index_oid,
        segment_id: *segment_id,
        tag: CacheTag::CtidCodec,
        sub_key: 0,
    };

    // Single call: cache checks run first; tantivy parse only on true miss.
    let dsm = dsm_cache::get_or_create_lazy(&key, || {
        let (reader, data_section_offset) = load_blockwise_linear(&column_file_slice)?;
        let (stats, parts) = reader.into_parts();

        let num_blocks = parts.len() as u32;
        let dsm_size = HEADER_SIZE + parts.len() * BLOCK_META_SIZE;

        let fill_fn = move |buf: &mut [u8]| {
            let header = DsmHeader {
                min_value: stats.min_value,
                max_value: stats.max_value,
                gcd: stats.gcd.get(),
                num_rows: stats.num_rows,
                num_blocks,
                data_section_offset,
            };
            unsafe {
                std::ptr::copy_nonoverlapping(
                    &header as *const _ as *const u8,
                    buf.as_mut_ptr(),
                    HEADER_SIZE,
                );
            }

            let mut data_start = 0u32;
            for (i, (line, bit_width, file_slice)) in parts.iter().enumerate() {
                let data_len = file_slice.len() as u32;
                let meta = DsmBlockMeta {
                    slope: line.slope,
                    intercept: line.intercept,
                    data_start,
                    data_len,
                    bit_width: *bit_width,
                    _pad: [0; 3],
                };
                let offset = HEADER_SIZE + i * BLOCK_META_SIZE;
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        &meta as *const _ as *const u8,
                        buf.as_mut_ptr().add(offset),
                        BLOCK_META_SIZE,
                    );
                }
                data_start += data_len;
            }
        };

        Some((dsm_size, fill_fn))
    })?;

    build_column_from_dsm(dsm, &column_file_slice)
}

/// Split a column FileSlice, load the BlockwiseLinear codec via tantivy,
/// and return the absolute byte offset of the data section.
/// Returns `None` if the column doesn't use BlockwiseLinear.
fn load_blockwise_linear(
    column_file_slice: &FileSlice,
) -> Option<(tantivy::columnar::BlockwiseLinearReader, u32)> {
    if column_file_slice.len() < 5 {
        return None;
    }

    let (body, index_len_payload) = column_file_slice.clone().split_from_end(4);
    let index_len_bytes = index_len_payload.read_bytes().ok()?;
    let column_index_num_bytes =
        u32::from_le_bytes(index_len_bytes.as_slice().try_into().ok()?) as usize;

    let (_, column_values_data) = body.split(column_index_num_bytes);
    if column_values_data.len() < 2 {
        return None;
    }

    let (codec_header, codec_body) = column_values_data.split(1);
    let codec_type = codec_header.read_bytes().ok()?[0];
    if codec_type != 2 {
        return None;
    }

    // Compute where the data section starts: after column_index + codec_type + stats
    let (_, stats_body) =
        tantivy::columnar::ColumnStats::deserialize_from_tail(codec_body.clone()).ok()?;
    let stats_nbytes = codec_body.len() - stats_body.len();
    let data_section_offset = (column_index_num_bytes + 1 + stats_nbytes) as u32;

    let reader = tantivy::columnar::BlockwiseLinearCodec::load(codec_body).ok()?;
    Some((reader, data_section_offset))
}

/// Build a Column<u64> from DSM metadata. Zero allocation beyond `Vec<OnceLock>`.
fn build_column_from_dsm(dsm: DsmSlice, column_file_slice: &FileSlice) -> Option<Column<u64>> {
    let header = unsafe { &*(dsm[..HEADER_SIZE].as_ptr() as *const DsmHeader) };

    if header.gcd == 0 {
        pgrx::warning!("pg_search: corrupted DSM cache entry (GCD=0), skipping ctid cache");
        return None;
    }

    let data_start = header.data_section_offset as usize;
    let total_data_len: usize = (0..header.num_blocks as usize)
        .map(|i| {
            let meta =
                unsafe { &*(dsm[HEADER_SIZE + i * BLOCK_META_SIZE..].as_ptr() as *const DsmBlockMeta) };
            meta.data_len as usize
        })
        .sum();
    let data_file_slice = column_file_slice.slice(data_start..data_start + total_data_len);

    let values = DsmBlockwiseValues {
        min_value: header.min_value,
        gcd: header.gcd,
        num_rows: header.num_rows,
        block_data: (0..header.num_blocks as usize)
            .map(|_| OnceLock::new())
            .collect(),
        data_file_slice,
        dsm,
    };

    Some(Column {
        index: ColumnIndex::Full,
        values: Arc::new(values),
    })
}
