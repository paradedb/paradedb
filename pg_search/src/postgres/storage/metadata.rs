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

use crate::api::version::{Version, parse_version_component};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::{SegmentMetaEntry, block_number_is_valid};
use crate::postgres::storage::buffer::{
    Buffer, BufferManager, BufferMut, PinnedBuffer, init_new_buffer,
};
use crate::postgres::storage::fsm::FreeSpaceManager;
use crate::postgres::storage::merge::{MergeLock, VacuumList, VacuumSentinel};
use crate::postgres::storage::{LinkedBytesList, LinkedItemList};
use pgrx::pg_sys::panic::ErrorReport;
use pgrx::{
    PgLogLevel, PgRelation, PgSqlErrorCode, function_name, iter::TableIterator, name, pg_extern,
    pg_sys,
};
use tantivy::IndexSettings;

/// The metadata stored on the `Metadata` page
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

    /// This used to be for detecting concurrent background merges,
    /// now we use advisory locks
    _dead_space_4: [pg_sys::BlockNumber; 2],

    /// pg_search version that created this index.
    /// PageInit zeroes the page at creation, so bytes past the fields that exist at creation time
    /// are zero forever. So we can reliably say:
    /// All zeros = created before stamping was added.
    created_by_version_major: u16,
    created_by_version_minor: u16,
    created_by_version_patch: u16,

    created_at: pg_sys::TimestampTz,
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

            metadata.created_by_version_major =
                const { parse_version_component(env!("CARGO_PKG_VERSION_MAJOR")) };
            metadata.created_by_version_minor =
                const { parse_version_component(env!("CARGO_PKG_VERSION_MINOR")) };
            metadata.created_by_version_patch =
                const { parse_version_component(env!("CARGO_PKG_VERSION_PATCH")) };

            metadata.created_at = pg_sys::GetCurrentTimestamp();
        }
    }

    pub fn open(indexrel: &PgSearchRelation) -> Self {
        if unsafe { pgrx::pg_sys::HotStandbyActive() } && unsafe { !pg_sys::XLogInsertAllowed() } {
            ErrorReport::new(
                PgSqlErrorCode::ERRCODE_FEATURE_NOT_SUPPORTED,
                "Serving reads from a standby requires write-ahead log (WAL) integration, which is supported on ParadeDB Enterprise, not ParadeDB Community",
                function_name!(),
            )
            .set_detail("ParadeDB Enterprise is commercially licensed and included with ParadeDB Cloud. To self-host ParadeDB Enterprise, contact sales@paradedb.com.")
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
        let may_need_init = !block_number_is_valid(metadata.active_vacuum_list)
            || !block_number_is_valid(metadata.ambulkdelete_sentinel)
            || !block_number_is_valid(metadata.merge_lock)
            || !block_number_is_valid(metadata.v2_fsm)
            || !block_number_is_valid(metadata.segment_meta_garbage);

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

    /// The pg_search version that created this index. Returns `None` for indices created
    /// before version stamping was added (the on-disk fields read as zero).
    #[allow(dead_code)]
    pub fn created_by_version(&self) -> Option<Version> {
        let major = self.data.created_by_version_major;
        let minor = self.data.created_by_version_minor;
        let patch = self.data.created_by_version_patch;

        if major == 0 && minor == 0 && patch == 0 {
            None
        } else {
            Some(Version {
                major,
                minor,
                patch,
            })
        }
    }

    /// The wall-clock time this index was built, as a `TimestampTz`. Returns `None` for indices
    /// created before build-time stamping was added (the on-disk field reads as zero).
    pub fn created_at(&self) -> Option<pg_sys::TimestampTz> {
        let created_at = self.data.created_at;
        if created_at == 0 {
            None
        } else {
            Some(created_at)
        }
    }

    ///
    /// A `LinkedItemList<SegmentMetaEntry>` containing segments which are no longer visible from the
    /// live `segment_metas()` list, and which will be recyclable when no transactions might still
    /// be reading them on physical replicas.
    ///
    /// Deferring recycling avoids readers needing to hold a lock all the way from when
    /// `segment_metas()` is first opened for reading until when they finish consuming the files
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

    pub fn settings(&self) -> tantivy::Result<IndexSettings> {
        let bytes = unsafe { self.settings_bytes().read_all() };
        Ok(serde_json::from_slice(&bytes)?)
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
    use pgrx::datum::TimestampWithTimeZone;
    use pgrx::prelude::*;

    fn vector_metadata_fixture() -> PgSearchRelation {
        Spi::run("CREATE EXTENSION IF NOT EXISTS vector;
            SET paradedb.vector_clustering_threshold = 64;
            CREATE TABLE metadata_vectors(id int PRIMARY KEY, vec vector(1024));
            INSERT INTO metadata_vectors SELECT g, ARRAY(SELECT ((g+i)%17+1)::real
                FROM generate_series(1,1024) i)::vector FROM generate_series(1,2048) g;
            CREATE INDEX metadata_vectors_idx ON metadata_vectors USING paradedb(id, vec vector_l2_ops)
                WITH (vector_fields='{\"vec\":{\"quantization\":{\"layers\":[1,4]}}}', target_segment_count=1);").unwrap();
        PgSearchRelation::open(
            Spi::get_one::<pg_sys::Oid>("SELECT 'metadata_vectors_idx'::regclass::oid")
                .unwrap()
                .unwrap(),
        )
    }

    fn set_single_layer_target(indexrel: &PgSearchRelation) {
        let mut settings: serde_json::Value = serde_json::from_slice(&unsafe {
            MetaPage::open(indexrel).settings_bytes().read_all()
        })
        .unwrap();
        settings["vector_quantization"][0]["layers"]
            .as_array_mut()
            .unwrap()
            .truncate(1);
        settings["vector_quantization"][0]["grids"]
            .as_array_mut()
            .unwrap()
            .retain(|grid| grid["bits"] == 1);
        let header = unsafe { LinkedBytesList::create_without_fsm(indexrel) };
        let mut writer = LinkedBytesList::open(indexrel, header).writer();
        unsafe {
            writer
                .write(&serde_json::to_vec(&settings).unwrap())
                .unwrap();
        }
        writer.finalize_and_write().unwrap();
        {
            let mut bman = BufferManager::new(indexrel);
            let mut buffer = bman.get_buffer_mut(METAPAGE);
            buffer
                .page_mut()
                .contents_mut::<MetaPageData>()
                .settings_start = header;
        }
    }

    fn vector_query() -> String {
        let query = vec!["1"; 1024].join(",");
        format!(
            "SELECT id FROM metadata_vectors WHERE id @@@ pdb.all() ORDER BY vec <-> '[{query}]'::vector LIMIT 5"
        )
    }

    fn expect_reindex(sql: &str) {
        Spi::run(&format!(
            r#"DO $body$
            DECLARE message text; hint text;
            BEGIN
                BEGIN
                    EXECUTE {query};
                    RAISE EXCEPTION 'expected vector format failure';
                EXCEPTION WHEN feature_not_supported THEN
                    GET STACKED DIAGNOSTICS message = MESSAGE_TEXT, hint = PG_EXCEPTION_HINT;
                    ASSERT position('metadata_vectors_idx' in message) > 0, message;
                    ASSERT hint = 'Rebuild index "metadata_vectors_idx" with REINDEX.', hint;
                END;
            END $body$;"#,
            query = quote_literal(sql)
        ))
        .unwrap();
    }

    fn quote_literal(value: &str) -> String {
        format!("'{}'", value.replace("'", "''"))
    }

    fn replace_vector_version(indexrel: &PgSearchRelation, version: u32) {
        use crate::postgres::storage::block::{LinkedListData, SegmentMetaEntryContent};
        let entries = unsafe { MetaPage::open(indexrel).segment_metas().list(None) };
        let file = entries
            .iter()
            .filter(|entry| !entry.is_deleted())
            .find_map(|entry| match entry.content {
                SegmentMetaEntryContent::Immutable(content) => content.vec,
                _ => None,
            })
            .expect("fixture has a vector file");
        let mut bman = BufferManager::new(indexrel);
        let block = bman
            .get_buffer(file.starting_block)
            .page()
            .contents::<LinkedListData>()
            .start_blockno;
        let mut buffer = bman.get_buffer_mut(block);
        *buffer.page_mut().contents_mut::<u32>() = version.to_le();
    }

    // Stored segment layers are independent of the field build target.
    #[pg_test]
    fn vector_metadata_is_independent_of_build_target() {
        let indexrel = vector_metadata_fixture();
        set_single_layer_target(&indexrel);
        assert_eq!(
            Spi::get_one::<bool>(
                "SELECT layers=ARRAY[1] AND bytes_per_row=144 AND settings_version=3
            FROM paradedb.vector_config('metadata_vectors_idx','vec')"
            )
            .unwrap(),
            Some(true)
        );
        assert_eq!(
            Spi::get_one::<bool>(
                "SELECT bool_and(layers=ARRAY[1,4] AND bytes_per_row=668)
            FROM paradedb.vector_info('metadata_vectors_idx','vec')"
            )
            .unwrap(),
            Some(true)
        );
        Spi::run(&vector_query()).unwrap();
        assert_eq!(
            Spi::get_one::<i64>(
                "SELECT count(*) FROM paradedb.vector_estimator_info('metadata_vectors_idx','vec')"
            )
            .unwrap(),
            Some(2)
        );
    }

    #[pg_test]
    fn unsupported_vector_storage_names_index_and_reindex() {
        let indexrel = vector_metadata_fixture();
        Spi::run("SET max_parallel_workers_per_gather=0; SET enable_seqscan=off;").unwrap();
        for version in [4, 3, 99] {
            replace_vector_version(&indexrel, version);
            let before = crate::index::reader::io_stats::vector_read_requests();
            assert_eq!(
                Spi::get_one::<i64>("SELECT count(*) FROM metadata_vectors WHERE id @@@ pdb.all()")
                    .unwrap(),
                Some(2048)
            );
            assert_eq!(
                crate::index::reader::io_stats::vector_read_requests(),
                before,
                "BM25 query read a .vec file at version {version}"
            );
            if version != 4 {
                expect_reindex(&vector_query());
                expect_reindex("SELECT * FROM paradedb.vector_info('metadata_vectors_idx','vec')");
                expect_reindex(
                    "SELECT * FROM paradedb.vector_config('metadata_vectors_idx','vec')",
                );
            }
        }
    }

    #[pg_test]
    fn foreground_vector_merge_reports_reindex() {
        let indexrel = vector_metadata_fixture();
        let oid = indexrel.oid();
        let layer_bytes = unsafe { MetaPage::open(&indexrel).segment_metas().list(None) }
            .iter()
            .map(|entry| entry.byte_size())
            .max()
            .unwrap();
        drop(indexrel);
        Spi::run(&format!("ALTER INDEX metadata_vectors_idx SET (layer_sizes='{layer_bytes} bytes',background_layer_sizes='0',mutable_segment_rows=0)")).unwrap();
        let indexrel = PgSearchRelation::open(oid);
        replace_vector_version(&indexrel, 3);
        expect_reindex(
            "INSERT INTO metadata_vectors SELECT g, ARRAY(SELECT ((g+i)%17+1)::real FROM generate_series(1,1024) i)::vector FROM generate_series(2049,4096) g",
        );
        assert_eq!(
            Spi::get_one::<i64>("SELECT count(*) FROM metadata_vectors").unwrap(),
            Some(2048)
        );
    }

    #[pg_test]
    fn mixed_stored_vector_schedules_reject_estimator_merge() {
        use crate::index::mvcc::MvccSatisfies;
        use crate::index::writer::index::{Mergeable, SearchIndexMerger};
        let indexrel = vector_metadata_fixture();
        let oid = indexrel.oid();
        drop(indexrel);
        Spi::run("ALTER INDEX metadata_vectors_idx SET (layer_sizes='0',background_layer_sizes='0',mutable_segment_rows=0); SET max_parallel_workers_per_gather=0;").unwrap();
        let indexrel = PgSearchRelation::open(oid);
        let first = SearchIndexMerger::open(&indexrel, MvccSatisfies::Mergeable)
            .unwrap()
            .searchable_segment_ids()
            .unwrap();
        set_single_layer_target(&indexrel);
        for start in [2049, 2305] {
            Spi::run(&format!("INSERT INTO metadata_vectors SELECT g, ARRAY(SELECT ((g+i)%17+1)::real FROM generate_series(1,1024) i)::vector FROM generate_series({start},{}) g",start+255)).unwrap();
        }
        unsafe { pg_sys::CommandCounterIncrement() };
        let mut merger = SearchIndexMerger::open(&indexrel, MvccSatisfies::Mergeable).unwrap();
        let new_segments: Vec<_> = merger
            .searchable_segment_ids()
            .unwrap()
            .difference(&first)
            .copied()
            .collect();
        assert_eq!(new_segments.len(), 2);
        // Cluster only the newly inserted rows, retaining the first segment's stored schedule.
        merger.merge_segments(&new_segments).unwrap();
        drop(merger);
        unsafe { pg_sys::CommandCounterIncrement() };
        assert_eq!(Spi::get_one::<bool>("SELECT bool_or(layers=ARRAY[1]) AND bool_or(layers=ARRAY[1,4]) FROM paradedb.vector_info('metadata_vectors_idx','vec')").unwrap(),Some(true));
        Spi::run(&vector_query()).unwrap();
        Spi::run(
            r#"DO $body$
            DECLARE message text;
            BEGIN
                BEGIN
                    PERFORM * FROM paradedb.vector_estimator_info('metadata_vectors_idx','vec');
                    RAISE EXCEPTION 'expected schedule mismatch';
                EXCEPTION WHEN OTHERS THEN
                    GET STACKED DIAGNOSTICS message = MESSAGE_TEXT;
                    ASSERT position('different schedules' in message) > 0, message;
                    ASSERT position('[("SignPlane", 1)]' in message) > 0, message;
                    ASSERT position('[("SignPlane", 1), ("GridPlane", 4)]' in message) > 0, message;
                END;
            END $body$;"#,
        )
        .unwrap();
    }

    #[pg_test]
    fn created_by_version_is_stamped_at_index_build() {
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run("INSERT INTO t (data) VALUES ('hello');").unwrap();
        Spi::run("CREATE INDEX t_idx ON t USING paradedb (id, data);").unwrap();

        let index_oid: pg_sys::Oid =
            Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
                .expect("spi should succeed")
                .unwrap();
        let indexrel = PgSearchRelation::open(index_oid);

        let stamped = MetaPage::open(&indexrel)
            .created_by_version()
            .expect("freshly built index should be version-stamped");

        let expected = Version::new(
            env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
            env!("CARGO_PKG_VERSION_MINOR").parse().unwrap(),
            env!("CARGO_PKG_VERSION_PATCH").parse().unwrap(),
        );
        assert_eq!(stamped, expected);

        // The UDF should surface the same version.
        let via_udf: String = Spi::get_one("SELECT paradedb.index_created_by('t_idx')")
            .expect("spi should succeed")
            .unwrap();
        assert_eq!(via_udf, stamped.to_string());
    }

    #[pg_test]
    fn created_at_is_stamped_at_index_build() {
        // `GetCurrentTimestamp()` is wall-clock time, so bound the build with `clock_timestamp()`
        // (which advances within the transaction) rather than the fixed `now()`.
        let before: TimestampWithTimeZone = Spi::get_one("SELECT clock_timestamp()")
            .expect("spi should succeed")
            .unwrap();

        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run("INSERT INTO t (data) VALUES ('hello');").unwrap();
        Spi::run("CREATE INDEX t_idx ON t USING paradedb (id, data);").unwrap();

        let after: TimestampWithTimeZone = Spi::get_one("SELECT clock_timestamp()")
            .expect("spi should succeed")
            .unwrap();

        let index_oid: pg_sys::Oid =
            Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
                .expect("spi should succeed")
                .unwrap();
        let indexrel = PgSearchRelation::open(index_oid);

        let created_at = TimestampWithTimeZone::try_from(
            MetaPage::open(&indexrel)
                .created_at()
                .expect("freshly built index should be timestamp-stamped"),
        )
        .unwrap();

        assert!(
            created_at >= before && created_at <= after,
            "created_at {created_at:?} should fall within [{before:?}, {after:?}]"
        );

        // The UDF should surface the same instant.
        let via_udf: TimestampWithTimeZone =
            Spi::get_one("SELECT paradedb.index_created_at('t_idx')")
                .expect("spi should succeed")
                .unwrap();
        assert_eq!(via_udf, created_at);
    }
}
