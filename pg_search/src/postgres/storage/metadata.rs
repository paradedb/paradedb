// Copyright (C) 2023-2026 ParadeDB, Inc.
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
use crate::postgres::storage::block::{block_number_is_valid, SegmentMetaEntry};
use crate::postgres::storage::buffer::{
    init_new_buffer, Buffer, BufferManager, BufferMut, ImmutablePage, PinnedBuffer,
};
use crate::postgres::storage::fsm::FreeSpaceManager;
use crate::postgres::storage::merge::{MergeLock, VacuumList, VacuumSentinel};
use crate::postgres::storage::{LinkedBytesList, LinkedItemList};
use pgrx::pg_sys::panic::ErrorReport;
use pgrx::{
    function_name, iter::TableIterator, name, pg_extern, pg_sys, PgLogLevel, PgRelation,
    PgSqlErrorCode,
};

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
    #[doc(hidden)]
    _dead_space_2: [u32; 2],

    /// This used to be the header block for a [`LinkedItemsList<SegmentMergeEntry>]`
    #[allow(dead_code)]
    #[doc(hidden)]
    _dead_space_3: pg_sys::BlockNumber,

    /// Merge lock block number
    merge_lock: pg_sys::BlockNumber,

    // these blocks used to be global constants but no longer are
    cleanup_lock: pg_sys::BlockNumber,
    schema_start: pg_sys::BlockNumber,
    settings_start: pg_sys::BlockNumber,
    segment_metas_start: pg_sys::BlockNumber,

    /// The block where our old v1 FSM starts
    v1_fsm: pg_sys::BlockNumber,

    /// The header block for a [`LinkedItemsList<SegmentMergeEntry>]`
    segment_meta_garbage: pg_sys::BlockNumber,
    ambulkdelete_epoch: u32,

    /// The block where our current, v2, FSM starts
    v2_fsm: pg_sys::BlockNumber,

    /// Allow up to 2 concurrent background merges
    /// If one of these blocks is pinned, that means a background merge is running
    bgmerger: [pg_sys::BlockNumber; 2],
}

/// Provides read access to the metadata page
/// Because the metadata page does not change after it's initialized in MetaPage::open(),
// (with the exception of the `ambulkdelete_epoch` field, see comment below)
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
            metadata.merge_lock = init_new_buffer(indexrel).number();
            metadata.v2_fsm = crate::postgres::storage::fsm::v2::V2FSM::create(indexrel);
            metadata.segment_meta_garbage =
                LinkedItemList::<SegmentMetaEntry>::create_without_fsm(indexrel);

            metadata.cleanup_lock = init_new_buffer(indexrel).number();
            metadata.schema_start = LinkedBytesList::create_without_fsm(indexrel);
            metadata.settings_start = LinkedBytesList::create_without_fsm(indexrel);
            metadata.segment_metas_start =
                LinkedItemList::<SegmentMetaEntry>::create_without_fsm(indexrel);
            metadata.bgmerger = std::array::from_fn(|_| init_new_buffer(indexrel).number());
        }
    }

    pub fn open(indexrel: &PgSearchRelation) -> Self {
        if unsafe { pgrx::pg_sys::HotStandbyActive() } && unsafe { !pg_sys::XLogInsertAllowed() } {
            ErrorReport::new(
                PgSqlErrorCode::ERRCODE_FEATURE_NOT_SUPPORTED,
                "Serving reads from a standby requires write-ahead log (WAL) integration, which is supported on ParadeDB Enterprise, not ParadeDB Community",
                function_name!(),
            )
            .set_detail("Please contact ParadeDB for access to ParadeDB Enterprise")
            .report(PgLogLevel::ERROR);
        }

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
        let bgmerger = metadata.bgmerger;
        let may_need_init = !block_number_is_valid(metadata.active_vacuum_list)
            || !block_number_is_valid(metadata.ambulkdelete_sentinel)
            || !block_number_is_valid(metadata.merge_lock)
            || !block_number_is_valid(metadata.v2_fsm)
            || !block_number_is_valid(metadata.segment_meta_garbage)
            || bgmerger
                .iter()
                .any(|&blockno| !block_number_is_valid(blockno));

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

                if !block_number_is_valid(metadata.merge_lock) {
                    metadata.merge_lock = init_new_buffer(indexrel).number();
                }

                if !block_number_is_valid(metadata.v2_fsm) {
                    metadata.v2_fsm = crate::postgres::storage::fsm::v2::V2FSM::create(indexrel);

                    if block_number_is_valid(metadata.v1_fsm) {
                        // convert the v1_fsm to v2
                        let v1_fsm =
                            crate::postgres::storage::fsm::v1::V1FSM::open(metadata.v1_fsm);
                        let v2_fsm =
                            crate::postgres::storage::fsm::v2::V2FSM::open(metadata.v2_fsm);

                        crate::postgres::storage::fsm::convert_v1_to_v2(&mut bman, v1_fsm, v2_fsm);

                        // the v1_fsm is no longer valid
                        metadata.v1_fsm = pg_sys::InvalidBlockNumber;
                    }
                }

                if !block_number_is_valid(metadata.segment_meta_garbage) {
                    metadata.segment_meta_garbage =
                        LinkedItemList::<SegmentMetaEntry>::create_without_fsm(indexrel);
                }

                for i in 0..2 {
                    if !block_number_is_valid(metadata.bgmerger[i]) {
                        metadata.bgmerger[i] = init_new_buffer(indexrel).number();
                    }
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

    pub fn fsm(&self) -> pg_sys::BlockNumber {
        assert!(block_number_is_valid(self.data.v2_fsm));
        self.data.v2_fsm
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

    pub fn bgmerger(&self) -> BgMergerPage {
        BgMergerPage {
            bman: self.bman.clone(),
            blocknos: self.data.bgmerger,
        }
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

    // Note that this value is read when not under a share lock, so there's no guarantee that it hasn't
    // been updated and this value is stale
    pub fn ambulkdelete_epoch(&self) -> u32 {
        self.data.ambulkdelete_epoch
    }

    pub fn increment_ambulkdelete_epoch(&mut self) {
        let mut buffer = self.bman.get_buffer_mut(METAPAGE);
        let mut page = buffer.page_mut();
        let metadata = page.contents_mut::<MetaPageData>();
        metadata.ambulkdelete_epoch = metadata.ambulkdelete_epoch.wrapping_add(1);
    }
}

// We at most allow 2 concurrent background merges
// The first background merge is for "small" merges, the second is for "large" merges
// This makes it so that a long-running large merge doesn't block smaller merges from happening
// We arbitrarily say that a merge is "large" if the largest layer size is greater than
// or equal to this threshold
const LARGE_MERGE_THRESHOLD: u64 = 100 * 1024 * 1024; // 100mb

pub struct BgMergerPage {
    bman: BufferManager,
    blocknos: [pg_sys::BlockNumber; 2],
}

impl BgMergerPage {
    pub fn can_start(&mut self, largest_layer_size: u64) -> Option<pg_sys::BlockNumber> {
        let blockno = if largest_layer_size >= LARGE_MERGE_THRESHOLD {
            1
        } else {
            0
        };

        assert!(block_number_is_valid(self.blocknos[blockno]));

        let buffer = self
            .bman
            .get_buffer_for_cleanup_conditional(self.blocknos[blockno]);
        buffer.map(|_| self.blocknos[blockno])
    }

    pub fn try_starting(&mut self, blockno: pg_sys::BlockNumber) -> Option<ImmutablePage> {
        assert!(blockno == self.blocknos[0] || blockno == self.blocknos[1]);

        let buffer = self.bman.get_buffer_for_cleanup_conditional(blockno);
        buffer.map(|buffer| buffer.into_immutable_page())
    }
}

#[allow(unused_variables)]
#[pg_extern]
unsafe fn reset_bgworker_state(index: PgRelation) {
    pgrx::warning!("reset_bgworker_state has been deprecated");
}

#[allow(unused_variables)]
#[pg_extern]
unsafe fn bgmerger_state(
    index: PgRelation,
) -> TableIterator<'static, (name!(pid, i32), name!(state, String))> {
    pgrx::warning!("bgmerger_state has been deprecated");
    TableIterator::new(std::iter::empty())
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use pgrx::prelude::*;

    fn create_index() -> pg_sys::Oid {
        Spi::run("SET client_min_messages = 'debug1';").unwrap();
        Spi::run("CREATE TABLE IF NOT EXISTS t (id SERIAL, data TEXT);").unwrap();
        Spi::run("INSERT INTO t (data) VALUES ('test');").unwrap();
        Spi::run("CREATE INDEX t_idx ON t USING bm25(id, data) WITH (key_field = 'id')").unwrap();
        Spi::get_one::<pg_sys::Oid>(
            "SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';",
        )
        .expect("spi should succeed")
        .unwrap()
    }

    #[pg_test]
    fn test_bgmerger_max_concurrent_merges() {
        let index_oid = create_index();
        let index = PgSearchRelation::open(index_oid);
        let metadata = MetaPage::open(&index);
        let mut bgmerger = metadata.bgmerger();

        let pin1 = bgmerger.try_starting(bgmerger.blocknos[1]);
        assert!(pin1.is_some());

        let pin2 = bgmerger.try_starting(bgmerger.blocknos[1]);
        assert!(pin2.is_none());

        let pin3 = bgmerger.try_starting(bgmerger.blocknos[0]);
        assert!(pin3.is_some());

        let pin4 = bgmerger.try_starting(bgmerger.blocknos[0]);
        assert!(pin4.is_none());

        // drop one pin, should be able to start another
        drop(pin1.unwrap());
        let pin5 = bgmerger.try_starting(bgmerger.blocknos[1]);
        assert!(pin5.is_some());

        // drop one pin, should be able to start another
        drop(pin3.unwrap());
        let pin6 = bgmerger.try_starting(bgmerger.blocknos[0]);
        assert!(pin6.is_some());

        // drop the rest
        drop(pin5.unwrap());
        drop(pin6.unwrap());
    }
}
