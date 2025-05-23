// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use crate::api::HashSet;
use crate::postgres::storage::block::{
    bm25_max_free_space, BM25PageSpecialData, MVCCEntry, PgItem,
};
use crate::postgres::storage::buffer::{BufferManager, BufferMut, PinnedBuffer};
use crate::postgres::storage::LinkedBytesList;
use pgrx::{pg_sys, StringInfo};
use serde::{Deserialize, Serialize};
use std::slice::from_raw_parts;
use tantivy::index::SegmentId;

#[repr(transparent)]
pub struct VacuumSentinel(pub PinnedBuffer);

/// Lock the merge process by holding onto an exclusively-locked buffer
#[derive(Debug)]
pub struct MergeLock {
    _buffer: BufferMut,
}

impl MergeLock {
    pub fn create(relation_oid: pg_sys::Oid) -> pg_sys::BlockNumber {
        let mut bman = BufferManager::new(relation_oid);
        let mut start_buffer = bman.new_buffer();
        let mut start_page = start_buffer.init_page();

        let special = start_page.special_mut::<BM25PageSpecialData>();
        special.next_blockno = pg_sys::InvalidBlockNumber;

        start_buffer.number()
    }

    /// This is a blocking operation to acquire the METADATA.
    pub unsafe fn acquire(relation_oid: pg_sys::Oid, block_number: pg_sys::BlockNumber) -> Self {
        let mut bman = BufferManager::new(relation_oid);
        let merge_lock = bman.get_buffer_mut(block_number);
        MergeLock {
            _buffer: merge_lock,
        }
    }
}

pub type SegmentIdBytes = [u8; 16];
#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct VacuumListData {
    segment_ids:
        [SegmentIdBytes; (bm25_max_free_space() - size_of::<u16>()) / size_of::<SegmentIdBytes>()],
    nentries: u16,
}

pub struct VacuumList {
    relation_oid: pg_sys::Oid,
    start_block_number: pg_sys::BlockNumber,
}

impl VacuumList {
    pub fn create(relation_oid: pg_sys::Oid) -> pg_sys::BlockNumber {
        let mut bman = BufferManager::new(relation_oid);
        let mut start_buffer = bman.new_buffer();
        let mut start_page = start_buffer.init_page();

        let special = start_page.special_mut::<BM25PageSpecialData>();
        special.next_blockno = pg_sys::InvalidBlockNumber;

        start_buffer.number()
    }

    pub fn open(relation_oid: pg_sys::Oid, start_block_number: pg_sys::BlockNumber) -> VacuumList {
        Self {
            relation_oid,
            start_block_number,
        }
    }

    pub fn write_list<'s>(self, segment_ids: impl Iterator<Item = &'s SegmentId>) {
        let mut segment_ids = segment_ids.collect::<Vec<_>>();
        segment_ids.sort();

        let mut bman = BufferManager::new(self.relation_oid);
        let mut buffer = bman.get_buffer_mut(self.start_block_number);
        let mut page = buffer.page_mut();
        let mut contents = page.contents_mut::<VacuumListData>();
        contents.nentries = 0;

        for segment_id in segment_ids {
            let segment_id = segment_id.uuid_bytes();
            if contents.nentries as usize >= contents.segment_ids.len() {
                // switch to the next page, either using the one that's already linked
                // or by creating a new page
                if page.next_blockno() != pg_sys::InvalidBlockNumber {
                    // we want to reuse the next block if we have one
                    buffer = bman.get_buffer_mut(page.next_blockno());
                    page = buffer.page_mut();
                } else {
                    // make a new next block and link it in
                    let next_buffer = bman.new_buffer();
                    let special = page.special_mut::<BM25PageSpecialData>();
                    special.next_blockno = next_buffer.number();

                    // switch to it
                    buffer = next_buffer;
                    page = buffer.init_page();
                }

                contents = page.contents_mut::<VacuumListData>();
                contents.nentries = 0;
            }

            contents.segment_ids[contents.nentries as usize].copy_from_slice(segment_id);
            contents.nentries += 1;
        }
    }

    pub fn read_list(&self) -> HashSet<SegmentId> {
        let mut segment_ids = HashSet::default();

        let bman = BufferManager::new(self.relation_oid);
        let mut buffer = bman.get_buffer(self.start_block_number);
        loop {
            let page = buffer.page();
            let contents = page.contents::<VacuumListData>();
            if contents.nentries == 0 {
                // no entries on this page
                break;
            }

            // add all the entries from this page
            segment_ids.extend(
                contents.segment_ids[..contents.nentries as usize]
                    .iter()
                    .map(|bytes| SegmentId::from_bytes(*bytes)),
            );

            if page.next_blockno() == pg_sys::InvalidBlockNumber {
                // we don't have another block
                break;
            }

            buffer = bman.get_buffer(page.next_blockno());
        }

        segment_ids
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MergeEntry {
    pub pid: i32,

    /// The transaction id performing the merge indicated by this [`MergeEntry`]
    pub xmin: pg_sys::TransactionId,

    /// used space where we once stored an `xmax` value
    #[doc(hidden)]
    #[serde(alias = "xmax")]
    pub _unused: pg_sys::TransactionId,

    pub segment_ids_start_blockno: pg_sys::BlockNumber,
}

impl From<PgItem> for MergeEntry {
    fn from(value: PgItem) -> Self {
        let PgItem(item, size) = value;
        let (decoded, _) = bincode::serde::decode_from_slice(
            unsafe { from_raw_parts(item as *const u8, size) },
            bincode::config::legacy(),
        )
        .expect("expected to deserialize valid MergeEntry");
        decoded
    }
}

impl From<MergeEntry> for PgItem {
    fn from(value: MergeEntry) -> Self {
        let mut buf = StringInfo::new();
        let len = bincode::serde::encode_into_std_write(value, &mut buf, bincode::config::legacy())
            .expect("expected to serialize valid MergeEntry");
        PgItem(buf.into_char_ptr() as pg_sys::Item, len as pg_sys::Size)
    }
}

impl MVCCEntry for MergeEntry {
    fn pintest_blockno(&self) -> pg_sys::BlockNumber {
        pg_sys::InvalidBlockNumber
    }

    unsafe fn visible(&self) -> bool {
        true
    }

    unsafe fn recyclable(&self, _: &mut BufferManager) -> bool {
        unsafe {
            self.xmin != pg_sys::InvalidTransactionId
                && !pg_sys::TransactionIdIsInProgress(self.xmin)
        }
    }

    unsafe fn mergeable(&self) -> bool {
        unimplemented!("`MVCCEntry::mergeable()` is not supported for `MergeEntry")
    }
}

impl MergeEntry {
    pub unsafe fn segment_ids(&self, relation_id: pg_sys::Oid) -> Vec<SegmentId> {
        let bytes = LinkedBytesList::open(relation_id, self.segment_ids_start_blockno);
        let bytes = bytes.read_all();
        bytes
            .chunks(size_of::<SegmentIdBytes>())
            .map(|entry| {
                SegmentId::from_bytes(entry.try_into().expect("malformed SegmentId entry"))
            })
            .collect()
    }
}
