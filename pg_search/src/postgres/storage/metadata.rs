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

use crate::postgres::storage::block::{BM25PageSpecialData, SegmentMetaEntry};
use crate::postgres::storage::block::{LinkedList, METADATA};
use crate::postgres::storage::buffer::{BufferManager, BufferMut};
use crate::postgres::storage::merge::{
    MergeEntry, MergeList, MergeLock, SegmentIdBytes, VacuumList, VacuumSentinel,
};
use crate::postgres::storage::{LinkedBytesList, LinkedItemList};
use pgrx::pg_sys;
use tantivy::index::SegmentId;

/// The metadata stored on the [`Metadata`] page
#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct MetaPageData {
    /// This space was once used but no longer is.  As such, it needs to remain dead forever
    #[allow(dead_code)]
    _dead_space: [u32; 2],

    /// Contains the [`pg_sys::BlockNumber`] of the active merge list
    active_vacuum_list: pg_sys::BlockNumber,

    /// A block for which is pin is held during `ambulkdelete()`
    ambulkdelete_sentinel: pg_sys::BlockNumber,

    /// The header block for a [`LinkedItemsList<MergeEntry>]`
    merge_list: pg_sys::BlockNumber,

    /// The header block for a [`LinkedBytesList<SegmentIdBytes>]`, which are the segment ids created by `CREATE INDEX`
    create_index_list: pg_sys::BlockNumber,

    /// The header block for a [`LinkedItemsList<SegmentMergeEntry>]`
    segment_meta_garbage: pg_sys::BlockNumber,

    /// Merge lock block number
    merge_lock: pg_sys::BlockNumber,
}

/// Provides read access to the metadata page
/// Because the metadata page does not change after it's initialized in MetaPage::open(),
/// we do not need to hold a share lock for the lifetime of this struct.
pub struct MetaPage {
    data: MetaPageData,
    bman: BufferManager,
}

impl MetaPage {
    pub unsafe fn open(relation_oid: pg_sys::Oid) -> Self {
        let mut bman = BufferManager::new(relation_oid);
        let buffer = bman.get_buffer(METADATA);
        let page = buffer.page();
        let metadata = page.contents::<MetaPageData>();

        // Skip create_index_list because it doesn't need to be initialized yet
        let may_need_init = !block_number_is_initialized(metadata.active_vacuum_list)
            || !block_number_is_initialized(metadata.ambulkdelete_sentinel)
            || !block_number_is_initialized(metadata.merge_list)
            || !block_number_is_initialized(metadata.segment_meta_garbage)
            || !block_number_is_initialized(metadata.merge_lock);

        drop(buffer);

        // If any of the fields are not initialized, we need to initialize them
        // We swap our share lock for an exclusive lock
        if may_need_init {
            let mut buffer = bman.get_buffer_mut(METADATA);
            let mut page = buffer.page_mut();
            let metadata = page.contents_mut::<MetaPageData>();

            if !block_number_is_initialized(metadata.active_vacuum_list) {
                metadata.active_vacuum_list = new_buffer_and_init_page(relation_oid);
            }

            if !block_number_is_initialized(metadata.ambulkdelete_sentinel) {
                metadata.ambulkdelete_sentinel = new_buffer_and_init_page(relation_oid);
            }

            if !block_number_is_initialized(metadata.merge_list) {
                metadata.merge_list =
                    LinkedItemList::<MergeEntry>::create(relation_oid).get_header_blockno();
            }

            if !block_number_is_initialized(metadata.segment_meta_garbage) {
                metadata.segment_meta_garbage =
                    LinkedItemList::<SegmentMetaEntry>::create(relation_oid).get_header_blockno();
            }

            if !block_number_is_initialized(metadata.merge_lock) {
                metadata.merge_lock = new_buffer_and_init_page(relation_oid);
            }
        }

        Self {
            data: bman.get_buffer(METADATA).page().contents::<MetaPageData>(),
            bman,
        }
    }

    /// Acquires the merge lock.
    pub unsafe fn acquire_merge_lock(&self) -> MergeLock {
        MergeLock::acquire(self.bman.relation_oid(), self.data.merge_lock)
    }

    ///
    /// A LinkedItemList<SegmentMetaEntry> containing segments which are no longer visible from the
    /// live `SEGMENT_METAS_START` list, and which will be recyclable when no transactions might still
    /// be reading them on physical replicas.
    ///
    /// Deferring recycling avoids readers needing to hold a lock all the way from when
    /// `SEGMENT_METAS_START` is first opened for reading until when they finish consuming the files
    /// for the segments it references.
    ///
    pub fn segment_metas_garbage(&self) -> LinkedItemList<SegmentMetaEntry> {
        LinkedItemList::<SegmentMetaEntry>::open(
            self.bman.relation_oid(),
            self.data.segment_meta_garbage,
        )
    }

    pub fn vacuum_list(&self) -> VacuumList {
        VacuumList::open(
            self.bman.relation_oid(),
            self.data.active_vacuum_list,
            self.data.ambulkdelete_sentinel,
        )
    }

    pub fn merge_list(&self) -> MergeList {
        MergeList::open(
            LinkedItemList::<MergeEntry>::open(self.bman.relation_oid(), self.data.merge_list),
            self.bman.relation_oid(),
        )
    }

    pub fn pin_ambulkdelete_sentinel(&mut self) -> VacuumSentinel {
        let sentinel = self.bman.pinned_buffer(self.data.ambulkdelete_sentinel);
        VacuumSentinel(sentinel)
    }

    pub unsafe fn create_index_segment_ids(&self) -> Vec<SegmentId> {
        if !block_number_is_initialized(self.data.create_index_list) {
            return Vec::new();
        }

        let entries = LinkedBytesList::open(self.bman.relation_oid(), self.data.create_index_list);
        let bytes = entries.read_all();
        bytes
            .chunks(size_of::<SegmentIdBytes>())
            .map(|entry| {
                SegmentId::from_bytes(entry.try_into().expect("malformed SegmentId entry"))
            })
            .collect()
    }
}

/// For actions that dirty the metadata page -- takes an exclusive lock on the metadata page
/// and holds it until `MetaPageMut` is dropped.
pub struct MetaPageMut {
    buffer: BufferMut,
    bman: BufferManager,
}

impl MetaPageMut {
    pub fn new(relation_oid: pg_sys::Oid) -> Self {
        let mut bman = BufferManager::new(relation_oid);
        let buffer = bman.get_buffer_mut(METADATA);
        Self { buffer, bman }
    }

    pub unsafe fn record_create_index_segment_ids<'a>(
        mut self,
        segment_ids: impl IntoIterator<Item = &'a SegmentId>,
    ) -> anyhow::Result<()> {
        let segment_id_bytes = segment_ids
            .into_iter()
            .flat_map(|segment_id| segment_id.uuid_bytes().iter().copied())
            .collect::<Vec<_>>();
        let segment_ids_list = LinkedBytesList::create(self.bman.relation_oid());
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

#[inline(always)]
fn new_buffer_and_init_page(relation_oid: pg_sys::Oid) -> pg_sys::BlockNumber {
    let mut bman = BufferManager::new(relation_oid);
    let mut start_buffer = bman.new_buffer();
    let mut start_page = start_buffer.init_page();

    let special = start_page.special_mut::<BM25PageSpecialData>();
    special.next_blockno = pg_sys::InvalidBlockNumber;

    start_buffer.number()
}
