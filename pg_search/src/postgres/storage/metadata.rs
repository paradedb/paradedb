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
use crate::postgres::storage::block::SegmentMetaEntry;
use crate::postgres::storage::block::{
    bm25_max_free_space, BM25PageSpecialData, LinkedList, MVCCEntry, PgItem, METADATA,
};
use crate::postgres::storage::merge::{MergeEntry, MergeLock, VacuumList, VacuumSentinel, SegmentIdBytes};
use crate::postgres::storage::buffer::{BufferManager, BufferMut, PinnedBuffer};
use crate::postgres::storage::{LinkedBytesList, LinkedItemList};
use pgrx::{pg_sys, StringInfo};
use serde::{Deserialize, Serialize};
use std::slice::from_raw_parts;
use tantivy::index::SegmentId;

/// The metadata stored on the [`Metadata`] page
#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct MetaPageData {
    /// This space was once used but no longer is.  As such, it needs to remain dead forever
    #[allow(dead_code)]
    pub _dead_space: [u32; 2],

    /// Contains the [`pg_sys::BlockNumber`] of the active merge list
    active_vacuum_list: pg_sys::BlockNumber,

    /// A block for which is pin is held during `ambulkdelete()`
    pub ambulkdelete_sentinel: pg_sys::BlockNumber,

    /// The header block for a [`LinkedItemsList<MergeEntry>]`
    pub merge_list: pg_sys::BlockNumber,

    pub create_index_list: pg_sys::BlockNumber,

    /// The header block for a [`LinkedItemsList<SegmentMergeEntry>]`
    segment_meta_garbage: pg_sys::BlockNumber,

    /// Merge lock block number
    merge_lock: pg_sys::BlockNumber,
}

impl MetaPageData {
    pub fn has_been_initialized(&self) -> bool {
        block_number_is_initialized(self.active_vacuum_list)
            && block_number_is_initialized(self.ambulkdelete_sentinel)
            && block_number_is_initialized(self.segment_meta_garbage)
            && block_number_is_initialized(self.merge_lock)
            && block_number_is_initialized(self.merge_list)
    }
}

pub struct MetaPage {
    data: MetaPageData,
    relation_oid: pg_sys::Oid,
    bman: BufferManager,
}

impl MetaPage {
    pub unsafe fn init(relation_oid: pg_sys::Oid) {
        let mut bman = BufferManager::new(relation_oid);
        let mut buffer = bman.get_buffer_mut(METADATA);
        let mut page = buffer.page_mut();
        let metadata = page.contents_mut::<MetaPageData>();

        metadata.active_vacuum_list = pg_sys::InvalidBlockNumber;
        metadata.ambulkdelete_sentinel = pg_sys::InvalidBlockNumber;
        metadata.create_index_list = pg_sys::InvalidBlockNumber;
        metadata.segment_meta_garbage = pg_sys::InvalidBlockNumber;
        metadata.merge_lock = pg_sys::InvalidBlockNumber;
    }

    pub unsafe fn open(relation_oid: pg_sys::Oid) -> Self {
        let mut bman = BufferManager::new(relation_oid);
        let buffer = bman.get_buffer(METADATA);
        let page = buffer.page();
        let metadata = page.contents::<MetaPageData>();

        if !metadata.has_been_initialized() {
            std::mem::drop(buffer);
            let mut buffer = bman.get_buffer_mut(METADATA);
            let mut page = buffer.page_mut();
            let metadata = page.contents_mut::<MetaPageData>();

            if !block_number_is_initialized(metadata.active_vacuum_list) {
                metadata.active_vacuum_list = VacuumList::create(relation_oid);
            }

            if !block_number_is_initialized(metadata.ambulkdelete_sentinel) {
                let mut sentinal_buffer = bman.new_buffer();
                sentinal_buffer.init_page();
                metadata.ambulkdelete_sentinel = sentinal_buffer.number();
            }

            if !block_number_is_initialized(metadata.segment_meta_garbage) {
                let list = LinkedItemList::<SegmentMetaEntry>::create(relation_oid);
                metadata.segment_meta_garbage = list.get_header_blockno();
            }

            // if !block_number_is_initialized(metadata.merge_lock) {
            //     metadata.merge_lock = MergeLock::create(relation_oid);
            // }

            if !block_number_is_initialized(metadata.merge_list) {
                let merge_list = LinkedItemList::<MergeEntry>::create(relation_oid);
                metadata.merge_list = merge_list.get_header_blockno();
            }

            Self {
                data: *metadata,
                relation_oid,
                bman,
            }
        } else {
            Self {
                data: metadata,
                relation_oid,
                bman,
            }
        }
    }

    // pub fn acquire_merge_lock(&self) -> MergeLock {
    //     MergeLock::acquire(self.bman.relation_oid(), self.data.merge_lock)
    // }

    ///
    /// A LinkedItemList<SegmentMetaEntry> containing segments which are no longer visible from the
    /// live `SEGMENT_METAS_START` list, and which will be recyclable when no transactions might still
    /// be reading them on physical replicas.
    ///
    /// Deferring recycling avoids readers needing to hold a lock all the way from when
    /// `SEGMENT_METAS_START` is first opened for reading until when they finish consuming the files
    /// for the segments it references.
    ///
    pub fn segment_metas_garbage(mut self) -> LinkedItemList<SegmentMetaEntry> {
        assert!(block_number_is_initialized(self.data.segment_meta_garbage));
        LinkedItemList::<SegmentMetaEntry>::open(
            self.relation_oid,
            self.data.segment_meta_garbage,
        )
    }

    pub fn vacuum_list(&self, merge_lock: Option<MergeLock>) -> VacuumList {
        VacuumList::open(merge_lock, self.relation_oid, self.data.active_vacuum_list)
    }

    pub fn pin_ambulkdelete_sentinel(&mut self) -> VacuumSentinel {
        assert!(block_number_is_initialized(self.data.ambulkdelete_sentinel));

        let sentinel = self.bman.pinned_buffer(self.data.ambulkdelete_sentinel);
        VacuumSentinel(sentinel)
    }

    pub unsafe fn in_progress_segment_ids(&self) -> impl Iterator<Item = SegmentId> + use<'_> {
        if !block_number_is_initialized(self.data.merge_list) {
            // our merge_list has never been initialized
            let iter: Box<dyn Iterator<Item = SegmentId>> = Box::new(std::iter::empty());
            return iter;
        }

        let entries = LinkedItemList::<MergeEntry>::open(self.relation_oid, self.data.merge_list);
        Box::new(
            entries
                .list()
                .into_iter()
                .flat_map(move |merge_entry| merge_entry.segment_ids(self.relation_oid).into_iter()),
        )
    }

    pub unsafe fn create_index_segment_ids(&self) -> Vec<SegmentId> {
        if self.data.create_index_list == 0
            || self.data.create_index_list == pg_sys::InvalidBlockNumber
        {
            return Vec::new();
        }

        let entries = LinkedBytesList::open(self.relation_oid, self.data.create_index_list);
        let bytes = entries.read_all();
        bytes
            .chunks(size_of::<SegmentIdBytes>())
            .map(|entry| {
                SegmentId::from_bytes(entry.try_into().expect("malformed SegmentId entry"))
            })
            .collect()
    }

    pub unsafe fn in_progress_merge_entries(&self) -> Vec<MergeEntry> {
        assert!(block_number_is_initialized(self.data.merge_list));
        LinkedItemList::<MergeEntry>::open(self.relation_oid, self.data.merge_list).list()
    }

    pub unsafe fn is_merge_in_progress(&self) -> bool {
        assert!(block_number_is_initialized(self.data.merge_list));
        !LinkedItemList::<MergeEntry>::open(self.relation_oid, self.data.merge_list).is_empty()
    }

    pub unsafe fn remove_entry(&mut self, merge_entry: MergeEntry) -> anyhow::Result<MergeEntry> {
        if !block_number_is_initialized(self.data.merge_list) {
            panic!("merge_list should have been initialized by now");
        }

        let mut entries_list = LinkedItemList::<MergeEntry>::open(self.relation_oid, self.data.merge_list);
        let removed_entry = entries_list.remove_item(|entry| entry == &merge_entry)?;

        LinkedBytesList::open(self.relation_oid, removed_entry.segment_ids_start_blockno).return_to_fsm();
        pg_sys::IndexFreeSpaceMapVacuum(self.bman.bm25cache().indexrel());
        Ok(removed_entry)
    }

    pub unsafe fn garbage_collect(&mut self) {
        if !block_number_is_initialized(self.data.merge_list) {
            return;
        }

        let mut entries_list = LinkedItemList::<MergeEntry>::open(self.relation_oid, self.data.merge_list);
        let recycled_entries = entries_list.garbage_collect();
        for recycled_entry in recycled_entries {
            LinkedBytesList::open(self.relation_oid, recycled_entry.segment_ids_start_blockno)
                .return_to_fsm();
            pg_sys::IndexFreeSpaceMapVacuum(self.bman.bm25cache().indexrel());
        }
    }
}

pub struct MetaPageMut {
    buffer: BufferMut,
    bman: BufferManager,
    relation_oid: pg_sys::Oid,
}

impl MetaPageMut {
    pub fn new(relation_oid: pg_sys::Oid) -> Self {
        let mut bman = BufferManager::new(relation_oid);
        let mut buffer = bman.get_buffer_mut(METADATA);
        let page = buffer.page_mut();
        Self {
            buffer,
            bman,
            relation_oid,
        }
    }


    pub unsafe fn record_in_progress_segment_ids<'a>(
        mut self,
        segment_ids: impl IntoIterator<Item = &'a SegmentId>,
    ) -> anyhow::Result<MergeEntry> {
        assert!(pg_sys::IsTransactionState());

        let merge_list_blockno = {
            let mut page = self.buffer.page_mut();
            let metadata = page.contents_mut::<MetaPageData>();

            if !block_number_is_initialized(metadata.merge_list) {
                let merge_list = LinkedItemList::<MergeEntry>::create(self.relation_oid);
                metadata.merge_list = merge_list.get_header_blockno();
            }

            metadata.merge_list
        };

        // write the SegmentIds to disk
        let segment_id_bytes = segment_ids
            .into_iter()
            .flat_map(|segment_id| segment_id.uuid_bytes().iter().copied())
            .collect::<Vec<_>>();
        let segment_ids_list = LinkedBytesList::create(self.relation_oid);
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

        let mut entries_list = LinkedItemList::<MergeEntry>::open(self.relation_oid, merge_list_blockno);
        entries_list.add_items(&[merge_entry], None);
        Ok(merge_entry)
    }

    pub unsafe fn record_create_index_segment_ids<'a>(
        mut self,
        segment_ids: impl IntoIterator<Item = &'a SegmentId>,
    ) -> anyhow::Result<()> {
        let segment_id_bytes = segment_ids
            .into_iter()
            .flat_map(|segment_id| segment_id.uuid_bytes().iter().copied())
            .collect::<Vec<_>>();
        let segment_ids_list = LinkedBytesList::create(self.relation_oid);
        let mut writer = segment_ids_list.writer();
        writer.write(&segment_id_bytes)?;
        let segment_ids_list = writer.into_inner()?;

        let mut page = self.buffer.page_mut();
        let metadata = page.contents_mut::<MetaPageData>();
        metadata.create_index_list = segment_ids_list.get_header_blockno();

        Ok(())
    }
}

#[inline(always)]
fn block_number_is_initialized(block_number: pg_sys::BlockNumber) -> bool {
    block_number != 0 && block_number != pg_sys::InvalidBlockNumber
}
