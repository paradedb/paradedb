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
use crate::postgres::storage::buffer::{Buffer, BufferManager, BufferMut, PinnedBuffer};
use crate::postgres::storage::merge::{
    MergeEntry, MergeLock, SegmentIdBytes, VacuumList, VacuumSentinel,
};
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

pub struct MetaPage {
    buffer: Option<Buffer>,
    bman: BufferManager,
}

impl MetaPage {
    fn get_metadata(&self) -> MetaPageData {
        self.buffer
            .as_ref()
            .expect("metapage buffer should have been initialized by now")
            .page()
            .contents::<MetaPageData>()
    }

    pub unsafe fn open(relation_oid: pg_sys::Oid) -> Self {
        let mut bman = BufferManager::new(relation_oid);
        let buffer = bman.get_buffer(METADATA);
        let page = buffer.page();
        let metadata = page.contents::<MetaPageData>();

        let needs_initialization = !block_number_is_initialized(metadata.segment_meta_garbage)
            || !block_number_is_initialized(metadata.merge_list)
            || !block_number_is_initialized(metadata.active_vacuum_list)
            || !block_number_is_initialized(metadata.ambulkdelete_sentinel);

        if needs_initialization {
            drop(buffer);

            let mut buffer = bman.get_buffer_mut(METADATA);
            let mut page = buffer.page_mut();
            let mut metadata = page.contents_mut::<MetaPageData>();

            if !block_number_is_initialized(metadata.segment_meta_garbage) {
                metadata.segment_meta_garbage =
                    LinkedItemList::<SegmentMetaEntry>::create(relation_oid).get_header_blockno();
            }

            if !block_number_is_initialized(metadata.merge_list) {
                metadata.merge_list =
                    LinkedItemList::<MergeEntry>::create(relation_oid).get_header_blockno();
            }

            if !block_number_is_initialized(metadata.active_vacuum_list) {
                metadata.active_vacuum_list = VacuumList::create(relation_oid);
            }

            if !block_number_is_initialized(metadata.ambulkdelete_sentinel) {
                let mut sentinal_buffer = bman.new_buffer();
                sentinal_buffer.init_page();
                metadata.ambulkdelete_sentinel = sentinal_buffer.number();
            }

            if !block_number_is_initialized(metadata.merge_lock) {
                metadata.merge_lock = MergeLock::create(relation_oid);
            }
        }

        Self {
            buffer: Some(bman.get_buffer(METADATA)),
            bman,
        }
    }

    pub unsafe fn acquire_merge_lock(&self) -> MergeLock {
        let metadata = self.get_metadata();
        MergeLock::acquire(self.bman.relation_oid(), metadata.merge_lock)
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
        let metadata = self.get_metadata();
        LinkedItemList::<SegmentMetaEntry>::open(
            self.bman.relation_oid(),
            metadata.segment_meta_garbage,
        )
    }

    pub fn vacuum_list(&self) -> VacuumList {
        let metadata = self.get_metadata();
        VacuumList::open(self.bman.relation_oid(), metadata.active_vacuum_list)
    }

    // TODO: Make this its own struct
    pub fn merge_list(&self) -> LinkedItemList<MergeEntry> {
        let metadata = self.get_metadata();
        LinkedItemList::<MergeEntry>::open(self.bman.relation_oid(), metadata.merge_list)
    }

    pub unsafe fn in_progress_segment_ids(&self) -> impl Iterator<Item = SegmentId> + use<'_> {
        Box::new(
            self.merge_list()
                .list()
                .into_iter()
                .flat_map(move |merge_entry| {
                    merge_entry
                        .segment_ids(self.bman.relation_oid())
                        .into_iter()
                }),
        )
    }

    pub unsafe fn remove_entry(&mut self, merge_entry: MergeEntry) -> anyhow::Result<MergeEntry> {
        let mut entries = self.merge_list();
        let removed_entry = entries.remove_item(|entry| entry == &merge_entry)?;

        LinkedBytesList::open(
            self.bman.relation_oid(),
            removed_entry.segment_ids_start_blockno,
        )
        .return_to_fsm();
        pg_sys::IndexFreeSpaceMapVacuum(self.bman.bm25cache().indexrel());
        Ok(removed_entry)
    }

    pub unsafe fn garbage_collect(&mut self) {
        let mut entries = self.merge_list();
        let recycled_entries = entries.garbage_collect();
        for recycled_entry in recycled_entries {
            LinkedBytesList::open(
                self.bman.relation_oid(),
                recycled_entry.segment_ids_start_blockno,
            )
            .return_to_fsm();
            pg_sys::IndexFreeSpaceMapVacuum(self.bman.bm25cache().indexrel());
        }
    }

    pub fn pin_ambulkdelete_sentinel(&mut self) -> VacuumSentinel {
        let metadata = self.get_metadata();
        let sentinel = self.bman.pinned_buffer(metadata.ambulkdelete_sentinel);
        VacuumSentinel(sentinel)
    }

    pub unsafe fn create_index_segment_ids(&self) -> Vec<SegmentId> {
        let metadata = self.get_metadata();
        if !block_number_is_initialized(metadata.create_index_list) {
            return Vec::new();
        }

        let entries = LinkedBytesList::open(self.bman.relation_oid(), metadata.create_index_list);
        let bytes = entries.read_all();
        bytes
            .chunks(size_of::<SegmentIdBytes>())
            .map(|entry| {
                SegmentId::from_bytes(entry.try_into().expect("malformed SegmentId entry"))
            })
            .collect()
    }

    pub unsafe fn record_in_progress_segment_ids<'a>(
        &mut self,
        segment_ids: impl IntoIterator<Item = &'a SegmentId>,
    ) -> anyhow::Result<MergeEntry> {
        assert!(pg_sys::IsTransactionState());

        // write the SegmentIds to disk
        let segment_id_bytes = segment_ids
            .into_iter()
            .flat_map(|segment_id| segment_id.uuid_bytes().iter().copied())
            .collect::<Vec<_>>();
        let segment_ids_list = LinkedBytesList::create(self.bman.relation_oid());
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

        let metadata = self.get_metadata();
        let mut entries_list =
            LinkedItemList::<MergeEntry>::open(self.bman.relation_oid(), metadata.merge_list);
        entries_list.add_items(&[merge_entry], None);
        Ok(merge_entry)
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
