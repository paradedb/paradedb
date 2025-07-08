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

use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::{block_number_is_valid, LinkedList, SegmentMetaEntry};
use crate::postgres::storage::buffer::{
    init_new_buffer, Buffer, BufferManager, BufferMut, PinnedBuffer,
};
use crate::postgres::storage::fsm::FreeSpaceManager;
use crate::postgres::storage::merge::{MergeLock, SegmentIdBytes, VacuumList, VacuumSentinel};
use crate::postgres::storage::{LinkedBytesList, LinkedItemList};
use pgrx::pg_sys;
use tantivy::index::SegmentId;

/// The metadata stored on the [`Metadata`] page
#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct MetaPageData {
    /// This space was once used but no longer is.  As such, it needs to remain dead forever
    #[allow(dead_code)]
    _dead_space_1: [u32; 2],

    /// Contains the [`pg_sys::BlockNumber`] of the active merge list
    active_vacuum_list: pg_sys::BlockNumber,

    /// A block for which is pin is held during `ambulkdelete()`
    ambulkdelete_sentinel: pg_sys::BlockNumber,

    #[allow(dead_code)]
    _dead_space_2: [u32; 1],

    /// The header block for a [`LinkedBytesList<SegmentIdBytes>]`, which are the segment ids created by `CREATE INDEX`
    create_index_list: pg_sys::BlockNumber,

    /// The header block for a [`LinkedItemsList<SegmentMergeEntry>]`
    segment_meta_garbage: pg_sys::BlockNumber,

    /// Merge lock block number
    merge_lock: pg_sys::BlockNumber,

    // these blocks used to be global constants but no longer are
    cleanup_lock: pg_sys::BlockNumber,
    schema_start: pg_sys::BlockNumber,
    settings_start: pg_sys::BlockNumber,
    segment_metas_start: pg_sys::BlockNumber,

    /// The block where our FSM starts
    fsm: pg_sys::BlockNumber,
}

/// Provides read access to the metadata page
/// Because the metadata page does not change after it's initialized in MetaPage::open(),
/// we do not need to hold a share lock for the lifetime of this struct.
pub struct MetaPage {
    data: MetaPageData,
    bman: BufferManager,
}

const METAPAGE: pg_sys::BlockNumber = 0;

impl MetaPage {
    pub unsafe fn init(indexrel: &PgSearchRelation) {
        let mut buffer = init_new_buffer(indexrel);
        assert_eq!(
            buffer.number(),
            0,
            "the MetaPage must be initialized to block 0"
        );
        let mut page = buffer.page_mut();
        let metadata = page.contents_mut::<MetaPageData>();

        unsafe {
            metadata.active_vacuum_list = init_new_buffer(indexrel).number();
            metadata.ambulkdelete_sentinel = init_new_buffer(indexrel).number();
            metadata.segment_meta_garbage =
                LinkedItemList::<SegmentMetaEntry>::create_without_fsm(indexrel);
            metadata.merge_lock = init_new_buffer(indexrel).number();
            metadata.fsm = FreeSpaceManager::create(indexrel);

            metadata.cleanup_lock = init_new_buffer(indexrel).number();
            metadata.schema_start = LinkedBytesList::create_without_fsm(indexrel);
            metadata.settings_start = LinkedBytesList::create_without_fsm(indexrel);
            metadata.segment_metas_start =
                LinkedItemList::<SegmentMetaEntry>::create_without_fsm(indexrel);
        }
    }

    pub fn open(indexrel: &PgSearchRelation) -> Self {
        let mut bman = BufferManager::new(indexrel);
        let buffer = bman.get_buffer(METAPAGE);
        let page = buffer.page();
        let metadata = page.contents::<MetaPageData>();

        // Skip create_index_list because it doesn't need to be initialized yet
        //
        // also skip:
        //      - cleanup_lock
        //      - schema_start
        //      - settings_start
        //      - segment_metas_start
        //
        // These will have either been initialized in `MetaPage::init()` or known to be
        // our old hardcoded values
        let may_need_init = !block_number_is_valid(metadata.active_vacuum_list)
            || !block_number_is_valid(metadata.ambulkdelete_sentinel)
            || !block_number_is_valid(metadata.segment_meta_garbage)
            || !block_number_is_valid(metadata.merge_lock)
            || !block_number_is_valid(metadata.fsm);

        drop(buffer);

        // If any of the fields are not initialized, we need to initialize them
        // We swap our share lock for an exclusive lock
        if may_need_init {
            let mut buffer = bman.get_buffer_mut(METAPAGE);
            let mut page = buffer.page_mut();
            let metadata = page.contents_mut::<MetaPageData>();

            unsafe {
                if !block_number_is_valid(metadata.active_vacuum_list) {
                    metadata.active_vacuum_list = init_new_buffer(indexrel).number();
                }

                if !block_number_is_valid(metadata.ambulkdelete_sentinel) {
                    metadata.ambulkdelete_sentinel = init_new_buffer(indexrel).number();
                }

                if !block_number_is_valid(metadata.segment_meta_garbage) {
                    metadata.segment_meta_garbage =
                        LinkedItemList::<SegmentMetaEntry>::create_without_fsm(indexrel);
                }

                if !block_number_is_valid(metadata.merge_lock) {
                    metadata.merge_lock = init_new_buffer(indexrel).number();
                }

                if !block_number_is_valid(metadata.fsm) {
                    metadata.fsm = FreeSpaceManager::create(indexrel);
                }
            }

            Self {
                data: *metadata,
                bman,
            }
        } else {
            Self {
                data: metadata,
                bman,
            }
        }
    }

    /// Acquires the merge lock.
    pub unsafe fn acquire_merge_lock(&self) -> MergeLock {
        assert!(block_number_is_valid(self.data.merge_lock));
        MergeLock::acquire(self.bman.buffer_access().rel(), self.data.merge_lock)
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
    pub fn segment_metas_garbage(&self) -> Option<LinkedItemList<SegmentMetaEntry>> {
        if !block_number_is_valid(self.data.segment_meta_garbage) {
            return None;
        }

        Some(LinkedItemList::<SegmentMetaEntry>::open(
            self.bman.buffer_access().rel(),
            self.data.segment_meta_garbage,
        ))
    }

    pub fn vacuum_list(&self) -> VacuumList {
        assert!(block_number_is_valid(self.data.active_vacuum_list));
        VacuumList::open(
            self.bman.buffer_access().rel(),
            self.data.active_vacuum_list,
            self.data.ambulkdelete_sentinel,
        )
    }

    pub fn pin_ambulkdelete_sentinel(&mut self) -> VacuumSentinel {
        assert!(block_number_is_valid(self.data.ambulkdelete_sentinel));
        let sentinel = self.bman.pinned_buffer(self.data.ambulkdelete_sentinel);
        VacuumSentinel(sentinel)
    }

    pub unsafe fn create_index_segment_ids(&self) -> Vec<SegmentId> {
        if !block_number_is_valid(self.data.create_index_list) {
            return Vec::new();
        }

        let entries =
            LinkedBytesList::open(self.bman.buffer_access().rel(), self.data.create_index_list);
        let bytes = entries.read_all();
        bytes
            .chunks(size_of::<SegmentIdBytes>())
            .map(|entry| {
                SegmentId::from_bytes(entry.try_into().expect("malformed SegmentId entry"))
            })
            .collect()
    }

    pub fn fsm(&self) -> pg_sys::BlockNumber {
        assert!(block_number_is_valid(self.data.fsm));
        self.data.fsm
    }
}

// legacy hardcoded page support for various index objects
impl MetaPage {
    const LEGACY_CLEANUP_LOCK: pg_sys::BlockNumber = 1;
    const LEGACY_SCHEMA_START: pg_sys::BlockNumber = 2;
    const LEGACY_SETTINGS_START: pg_sys::BlockNumber = 4;
    const LEGACY_SEGMENT_METAS_START: pg_sys::BlockNumber = 6;

    pub fn cleanup_lock_pinned(&self) -> PinnedBuffer {
        let blockno = if self.data.cleanup_lock == 0 {
            Self::LEGACY_CLEANUP_LOCK
        } else {
            self.data.cleanup_lock
        };
        self.bman.pinned_buffer(blockno)
    }

    pub fn cleanup_lock_shared(&self) -> Buffer {
        let blockno = if self.data.cleanup_lock == 0 {
            Self::LEGACY_CLEANUP_LOCK
        } else {
            self.data.cleanup_lock
        };
        self.bman.get_buffer(blockno)
    }

    pub fn cleanup_lock_exclusive(&mut self) -> BufferMut {
        let blockno = if self.data.cleanup_lock == 0 {
            Self::LEGACY_CLEANUP_LOCK
        } else {
            self.data.cleanup_lock
        };
        self.bman.get_buffer_mut(blockno)
    }

    pub fn cleanup_lock_for_cleanup(&mut self) -> BufferMut {
        let blockno = if self.data.cleanup_lock == 0 {
            Self::LEGACY_CLEANUP_LOCK
        } else {
            self.data.cleanup_lock
        };
        self.bman.get_buffer_for_cleanup(blockno)
    }

    pub fn schema_bytes(&self) -> LinkedBytesList {
        let blockno = if self.data.schema_start == 0 {
            Self::LEGACY_SCHEMA_START
        } else {
            self.data.schema_start
        };
        LinkedBytesList::open(self.bman.buffer_access().rel(), blockno)
    }

    pub fn settings_bytes(&self) -> LinkedBytesList {
        let blockno = if self.data.settings_start == 0 {
            Self::LEGACY_SETTINGS_START
        } else {
            self.data.settings_start
        };
        LinkedBytesList::open(self.bman.buffer_access().rel(), blockno)
    }

    pub fn segment_metas(&self) -> LinkedItemList<SegmentMetaEntry> {
        let blockno = if self.data.segment_metas_start == 0 {
            Self::LEGACY_SEGMENT_METAS_START
        } else {
            self.data.segment_metas_start
        };
        LinkedItemList::<SegmentMetaEntry>::open(self.bman.buffer_access().rel(), blockno)
    }
}

// mutable MetaPage operations
impl MetaPage {
    pub fn record_create_index_segment_ids(
        &mut self,
        segment_ids: impl IntoIterator<Item = SegmentId>,
    ) -> anyhow::Result<()> {
        let segment_id_bytes = segment_ids
            .into_iter()
            .flat_map(|segment_id| segment_id.uuid_bytes().to_vec())
            .collect::<Vec<_>>();
        let segment_ids_list = LinkedBytesList::create_with_fsm(self.bman.buffer_access().rel());
        let mut writer = segment_ids_list.writer();
        unsafe {
            writer.write(&segment_id_bytes)?;
        }
        let segment_ids_list = writer.into_inner()?;

        let mut buffer = self.bman.get_buffer_mut(METAPAGE);
        let mut page = buffer.page_mut();
        let metadata = page.contents_mut::<MetaPageData>();
        metadata.create_index_list = segment_ids_list.get_header_blockno();

        Ok(())
    }
}
