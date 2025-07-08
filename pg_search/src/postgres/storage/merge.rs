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
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::{
    block_number_is_valid, bm25_max_free_space, BM25PageSpecialData, LinkedList, MVCCEntry, PgItem,
};
use crate::postgres::storage::buffer::{BufferManager, BufferMut, PinnedBuffer};
use crate::postgres::storage::{LinkedBytesList, LinkedItemList};
use pgrx::{pg_sys, StringInfo};
use serde::{Deserialize, Serialize};
use std::slice::from_raw_parts;
use tantivy::index::SegmentId;

#[repr(transparent)]
pub struct VacuumSentinel(pub PinnedBuffer);

/// The metadata stored on the [`Metadata`] page
#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct MergeLockData {
    merge_list: pg_sys::BlockNumber,
}

/// Lock the merge process by holding onto an exclusively-locked buffer
#[derive(Debug)]
pub struct MergeLock {
    data: MergeLockData,
    _buffer: BufferMut,
    bman: BufferManager,
}

impl MergeLock {
    /// This is a blocking operation to acquire an exclusive lock on the merge lock buffer
    pub unsafe fn acquire(indexrel: &PgSearchRelation, block_number: pg_sys::BlockNumber) -> Self {
        let mut bman = BufferManager::new(indexrel);
        let mut buffer = bman.get_buffer_mut(block_number);
        let mut page = buffer.page_mut();
        let metadata = page.contents_mut::<MergeLockData>();

        if !block_number_is_valid(metadata.merge_list) {
            metadata.merge_list =
                LinkedItemList::<MergeEntry>::create_with_fsm(indexrel).get_header_blockno();
        }

        MergeLock {
            data: *metadata,
            _buffer: buffer,
            bman,
        }
    }

    pub fn merge_list(&self) -> MergeList {
        MergeList::open(
            LinkedItemList::<MergeEntry>::open(
                self.bman.buffer_access().rel(),
                self.data.merge_list,
            ),
            self.bman.buffer_access().rel(),
        )
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
    indexrel: PgSearchRelation,
    start_block_number: pg_sys::BlockNumber,
    ambulkdelete_sentinel: pg_sys::BlockNumber,
}

impl VacuumList {
    ///
    /// Open a new vacuum list.
    ///
    /// # Arguments
    ///
    /// * `relation_oid` - The OID of the relation to vacuum.
    /// * `start_block_number` - The block number of the first block in the list.
    /// * `ambulkdelete_sentinel` - The block number of the sentinel block. It is the caller's responsibility to ensure this is a valid block number.
    pub fn open(
        indexrel: &PgSearchRelation,
        start_block_number: pg_sys::BlockNumber,
        ambulkdelete_sentinel: pg_sys::BlockNumber,
    ) -> VacuumList {
        Self {
            indexrel: Clone::clone(indexrel),
            start_block_number,
            ambulkdelete_sentinel,
        }
    }

    ///
    /// Write a list of segment ids to the vacuum list. This overwrites any existing content in the list.
    ///
    /// # Arguments
    ///
    /// * `segment_ids` - An iterator of segment ids to write to the list.
    pub fn write_list<'s>(self, segment_ids: impl Iterator<Item = &'s SegmentId>) {
        let mut segment_ids = segment_ids.collect::<Vec<_>>();
        segment_ids.sort();

        let mut bman = BufferManager::new(&self.indexrel);
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
        // Instead of clearing the list, we just return an empty list if ambulkdelete is no longer running.
        if !self.is_ambulkdelete_running() {
            return Default::default();
        }

        let mut segment_ids = HashSet::default();

        let bman = BufferManager::new(&self.indexrel);
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

    pub fn is_ambulkdelete_running(&self) -> bool {
        // an `ambulkdelete()` is running if we can't acquire the sentinel block for cleanup
        // it means ambulkdelete() is holding a pin on that buffer
        let mut bman = BufferManager::new(&self.indexrel);
        bman.get_buffer_for_cleanup_conditional(self.ambulkdelete_sentinel)
            .is_none()
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
    pub unsafe fn segment_ids(&self, indexrel: &PgSearchRelation) -> Vec<SegmentId> {
        let bytes = LinkedBytesList::open(indexrel, self.segment_ids_start_blockno);
        let bytes = bytes.read_all();
        bytes
            .chunks(size_of::<SegmentIdBytes>())
            .map(|entry| {
                SegmentId::from_bytes(entry.try_into().expect("malformed SegmentId entry"))
            })
            .collect()
    }
}

pub struct MergeList {
    entries: LinkedItemList<MergeEntry>,
    bman: BufferManager,
}

impl MergeList {
    pub fn open(entries: LinkedItemList<MergeEntry>, indexrel: &PgSearchRelation) -> Self {
        let bman = BufferManager::new(indexrel);
        Self { entries, bman }
    }

    pub unsafe fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub unsafe fn list(&self) -> Vec<MergeEntry> {
        self.entries.list()
    }

    pub unsafe fn garbage_collect(&mut self) {
        let recycled_entries = self.entries.garbage_collect();

        let indexrel = self.bman.buffer_access().rel().clone();
        self.bman.fsm().extend(
            &mut self.bman,
            recycled_entries.into_iter().flat_map(move |entry| {
                LinkedBytesList::open(&indexrel, entry.segment_ids_start_blockno).used_blocks()
            }),
        );
    }

    pub unsafe fn add_segment_ids<'a>(
        &mut self,
        segment_ids: impl IntoIterator<Item = &'a SegmentId>,
    ) -> anyhow::Result<MergeEntry> {
        assert!(pg_sys::IsTransactionState());

        // write the SegmentIds to disk
        let segment_id_bytes = segment_ids
            .into_iter()
            .flat_map(|segment_id| segment_id.uuid_bytes().iter().copied())
            .collect::<Vec<_>>();
        let segment_ids_list = LinkedBytesList::create_with_fsm(self.bman.buffer_access().rel());
        let segment_ids_start_blockno = segment_ids_list.get_header_blockno();
        segment_ids_list.writer().write(&segment_id_bytes)?;

        // fabricate and write the [`MergeEntry`] itself
        let xid = pg_sys::GetCurrentTransactionId();
        let merge_entry = MergeEntry {
            pid: pg_sys::MyProcPid,
            xmin: xid,
            _unused: pg_sys::InvalidTransactionId,
            segment_ids_start_blockno,
        };

        self.entries.add_items(&[merge_entry], None);
        Ok(merge_entry)
    }

    pub unsafe fn list_segment_ids(&self) -> impl Iterator<Item = SegmentId> + use<'_> {
        Box::new(
            self.entries
                .list()
                .into_iter()
                .flat_map(move |merge_entry| {
                    merge_entry
                        .segment_ids(self.bman.buffer_access().rel())
                        .into_iter()
                }),
        )
    }

    pub unsafe fn remove_entry(&mut self, merge_entry: MergeEntry) -> anyhow::Result<MergeEntry> {
        let removed_entry = self.entries.remove_item(|entry| entry == &merge_entry)?;

        LinkedBytesList::open(
            self.bman.buffer_access().rel(),
            removed_entry.segment_ids_start_blockno,
        )
        .return_to_fsm();
        Ok(removed_entry)
    }
}
