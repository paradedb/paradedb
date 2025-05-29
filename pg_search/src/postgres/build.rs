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

use crate::index::mvcc::{MVCCDirectory, MvccSatisfies};
use crate::index::reader::index::SearchIndexReader;
use crate::index::writer::index::SerialSegmentWriter;
use crate::index::{get_index_schema, WriterResources};
use crate::postgres::storage::block::{
    SegmentMetaEntry, CLEANUP_LOCK, METADATA, SCHEMA_START, SEGMENT_METAS_START, SETTINGS_START,
};
use crate::postgres::storage::buffer::BufferManager;
use crate::postgres::storage::metadata::MetaPageMut;
use crate::postgres::storage::{LinkedBytesList, LinkedItemList};
use crate::postgres::utils::{
    categorize_fields, item_pointer_to_u64, row_to_search_document, CategorizedFieldData,
};
use crate::schema::document::SearchDocument;
use crate::schema::SearchField;
use anyhow::Result;
use pgrx::*;
use std::ffi::CStr;
use std::os::raw::c_void;
use std::time::Instant;
use tantivy::{Index, IndexSettings};

const PARALLEL_KEY_SHARED_STATE: u64 = 0xA000000000000001;
const LEADER_PARTICIPATES: bool = true;

// For now just pass the count on the build callback state
struct BuildState {
    reltuples: usize,
    per_row_context: PgMemoryContexts,
    start: Instant,
    writer: Option<SerialSegmentWriter>,
    categorized_fields: Vec<(SearchField, CategorizedFieldData)>,
    key_field_name: String,
    heap_relation: pg_sys::Relation,
    index_relation: pg_sys::Relation,
    leader: *mut ParallelBuildLeader,
}

struct ParallelBuildSharedState {
    heap_oid: pg_sys::Oid,
    index_oid: pg_sys::Oid,
    is_concurrent: bool,
    workers_done: pg_sys::ConditionVariable,
    mutex: pg_sys::slock_t,
    n_participants_done: i32,
    reltuples: f64,
}

struct ParallelBuildLeader {
    parallel_context: *mut pg_sys::ParallelContext,
    n_participant_tuple_sorts: i32,
    shared: *mut ParallelBuildSharedState,
    snapshot: pg_sys::Snapshot,
}

impl BuildState {
    fn new(relation_oid: pg_sys::Oid) -> Self {
        let index_relation = unsafe { PgRelation::open(relation_oid) };
        let (_, memory_budget) = WriterResources::CreateIndex.resources();
        let writer = SerialSegmentWriter::open(&index_relation, memory_budget)
            .expect("build state: should be able to open writer");

        let tupdesc = unsafe { PgTupleDesc::from_pg_unchecked(index_relation.rd_att) };
        let schema = get_index_schema(&index_relation)
            .expect("build state: should be able to get index schema");
        let categorized_fields = categorize_fields(&tupdesc, &schema);
        let key_field_name = schema.key_field().name.0;

        BuildState {
            reltuples: 0,
            per_row_context: PgMemoryContexts::new("pg_search ambuild context"),
            start: Instant::now(),
            writer: Some(writer),
            categorized_fields,
            key_field_name,
            index_relation: index_relation.as_ptr(),
            heap_relation: index_relation
                .heap_relation()
                .expect("build state: index must have a heap relation")
                .as_ptr(),
            leader: std::ptr::null_mut(),
        }
    }
}

#[pg_guard]
pub extern "C-unwind" fn ambuild(
    heaprel: pg_sys::Relation,
    indexrel: pg_sys::Relation,
    index_info: *mut pg_sys::IndexInfo,
) -> *mut pg_sys::IndexBuildResult {
    let heap_relation = unsafe { PgRelation::from_pg(heaprel) };
    let index_relation = unsafe { PgRelation::from_pg(indexrel) };
    let index_oid = index_relation.oid();

    unsafe { init_fixed_buffers(&index_relation) };

    // ensure we only allow one `USING bm25` index on this relation, accounting for a REINDEX
    // and accounting for CONCURRENTLY.
    unsafe {
        let index_tuple = &(*index_relation.rd_index);
        let is_reindex = !index_tuple.indisvalid;
        let is_concurrent = (*index_info).ii_Concurrent;

        if !is_reindex {
            for existing_index in heap_relation.indices(pg_sys::AccessShareLock as _) {
                if existing_index.oid() == index_oid {
                    // the index we're about to build already exists on the table.
                    continue;
                }

                if is_bm25_index(&existing_index) && !is_concurrent {
                    panic!("a relation may only have one `USING bm25` index");
                }
            }
        }
    }

    // create the index
    create_index(&index_relation).expect("failed to create index");

    // populate the index
    let nworkers = unsafe { compute_nworkers(&heap_relation, &index_relation) };
    let mut build_state = BuildState::new(index_relation.oid());

    pgrx::info!("nworkers: {}", nworkers);

    if nworkers > 0 {
        unsafe {
            begin_parallel_index_build(&mut build_state, (*index_info).ii_Concurrent, nworkers);
            parallel_heap_scan(&mut build_state);
        }
    } else {
        do_heap_scan(
            index_info,
            &heap_relation,
            &index_relation,
            &mut build_state,
        );
    };

    if !build_state.leader.is_null() && nworkers > 0 {
        unsafe { end_parallel_index_build(build_state.leader) };
    }

    unsafe { pg_sys::FlushRelationBuffers(indexrel) };

    let mut result = unsafe { PgBox::<pg_sys::IndexBuildResult>::alloc0() };
    result.heap_tuples = build_state.reltuples as f64;
    result.index_tuples = build_state.reltuples as f64;
    result.into_pg()
}

#[pg_guard]
pub extern "C-unwind" fn ambuildempty(_index_relation: pg_sys::Relation) {}

fn do_heap_scan<'a>(
    index_info: *mut pg_sys::IndexInfo,
    heap_relation: &'a PgRelation,
    index_relation: &'a PgRelation,
    build_state: &'a mut BuildState,
) {
    unsafe {
        pg_sys::IndexBuildHeapScan(
            heap_relation.as_ptr(),
            index_relation.as_ptr(),
            index_info,
            Some(build_callback),
            build_state,
        );

        build_state
            .writer
            .take()
            .expect("do_heap_scan: writer should exist by now")
            .commit()
            .unwrap_or_else(|e| panic!("failed to commit new tantivy index: {e}"));

        // store number of segments created in metadata
        let reader = SearchIndexReader::open(index_relation, MvccSatisfies::Snapshot)
            .expect("do_heap_scan: should be able to open a SearchIndexReader");

        // record the segment ids created in the merge lock
        let metadata = MetaPageMut::new(index_relation.oid());
        metadata
            .record_create_index_segment_ids(reader.segment_ids().iter())
            .expect("do_heap_scan: should be able to record segment ids in merge lock");
    }
}

#[pg_guard]
unsafe extern "C-unwind" fn build_callback(
    indexrel: pg_sys::Relation,
    ctid: pg_sys::ItemPointer,
    values: *mut pg_sys::Datum,
    isnull: *mut bool,
    _tuple_is_alive: bool,
    state: *mut c_void,
) {
    check_for_interrupts!();
    let build_state = (state as *mut BuildState)
        .as_mut()
        .expect("BuildState pointer should not be null");

    let categorized_fields = &build_state.categorized_fields;
    let key_field_name = &build_state.key_field_name;
    let writer = &mut build_state
        .writer
        .as_mut()
        .expect("build_callback:writer should exist by now");
    // In the block below, we switch to the memory context we've defined on our build
    // state, resetting it before and after. We do this because we're looking up a
    // PgTupleDesc... which is supposed to free the corresponding Postgres memory when it
    // is dropped. However, in practice, we're not seeing the memory get freed, which is
    // causing huge memory usage when building large indexes.
    //
    // By running in our own memory context, we can force the memory to be freed with
    // the call to reset().
    unsafe {
        build_state.per_row_context.switch_to(|cxt| {
            let mut search_document = SearchDocument { doc: tantivy::TantivyDocument::new() };

            row_to_search_document(
                values,
                isnull,
                key_field_name,
                categorized_fields,
                &mut search_document,
            )
                .unwrap_or_else(|err| {
                    panic!(
                        "error creating index entries for index '{}': {err}",
                        CStr::from_ptr((*(*indexrel).rd_rel).relname.data.as_ptr()).to_string_lossy()
                    );
                });

            writer
                .insert(search_document, item_pointer_to_u64(*ctid))
                .unwrap_or_else(|err| {
                    panic!("error inserting document during build callback.  See Postgres log for more information: {err:?}")
                });
            cxt.reset();
        });

        // important to count the number of items we've indexed for proper statistics updates,
        // especially after CREATE INDEX has finished
        build_state.reltuples += 1;

        if crate::gucs::log_create_index_progress() && build_state.reltuples % 100_000 == 0 {
            let secs = build_state.start.elapsed().as_secs_f64();
            let rate = build_state.reltuples as f64 / secs;
            pgrx::log!(
                "processed {} rows in {secs:.2} seconds ({rate:.2} per second)",
                build_state.reltuples,
            );
        }
    }
}

pub fn is_bm25_index(indexrel: &PgRelation) -> bool {
    indexrel.rd_amhandler == bm25_amhandler_oid().unwrap_or_default()
}

fn bm25_amhandler_oid() -> Option<pg_sys::Oid> {
    unsafe {
        let name = pg_sys::Datum::from(c"bm25".as_ptr());
        let pg_am_entry = pg_sys::SearchSysCache1(pg_sys::SysCacheIdentifier::AMNAME as _, name);
        if pg_am_entry.is_null() {
            return None;
        }

        let mut is_null = false;
        let datum = pg_sys::SysCacheGetAttr(
            pg_sys::SysCacheIdentifier::AMNAME as _,
            pg_am_entry,
            pg_sys::Anum_pg_am_amhandler as _,
            &mut is_null,
        );
        let oid = pg_sys::Oid::from_datum(datum, is_null);
        pg_sys::ReleaseSysCache(pg_am_entry);
        oid
    }
}

unsafe fn init_fixed_buffers(index_relation: &PgRelation) {
    let relation_oid = index_relation.oid();
    let mut bman = BufferManager::new(relation_oid);

    // Init merge lock buffer
    let mut merge_lock = bman.new_buffer();
    assert_eq!(merge_lock.number(), METADATA);
    merge_lock.init_page();

    // Init cleanup lock buffer
    let mut cleanup_lock = bman.new_buffer();
    assert_eq!(cleanup_lock.number(), CLEANUP_LOCK);
    cleanup_lock.init_page();

    // initialize all the other required buffers
    let schema = LinkedBytesList::create(relation_oid);
    let settings = LinkedBytesList::create(relation_oid);
    let segment_metas = LinkedItemList::<SegmentMetaEntry>::create(relation_oid);

    assert_eq!(schema.header_blockno, SCHEMA_START);
    assert_eq!(settings.header_blockno, SETTINGS_START);
    assert_eq!(segment_metas.header_blockno, SEGMENT_METAS_START);
}

fn create_index(index_relation: &PgRelation) -> Result<()> {
    let schema = get_index_schema(index_relation)?;
    let directory = MVCCDirectory::snapshot(index_relation.oid());
    let settings = IndexSettings {
        docstore_compress_dedicated_thread: false,
        ..IndexSettings::default()
    };
    Index::create(directory, schema.into(), settings)?;
    Ok(())
}

unsafe fn compute_nworkers(heap_relation: &PgRelation, index_relation: &PgRelation) -> i32 {
    if !crate::gucs::enable_parallel_index_build() {
        return 0;
    }

    if pg_sys::plan_create_index_workers(heap_relation.oid(), index_relation.oid()) == 0 {
        return 0;
    }

    let nworkers = relation_get_parallel_workers(heap_relation.as_ptr(), -1);
    if nworkers != -1 {
        return nworkers.min(pg_sys::max_parallel_maintenance_workers);
    }

    pg_sys::max_parallel_maintenance_workers
}

unsafe fn relation_get_parallel_workers(relation: pg_sys::Relation, default: i32) -> i32 {
    if !(*relation).rd_options.is_null() {
        (*(*relation).rd_options.cast::<pg_sys::StdRdOptions>()).parallel_workers
    } else {
        default
    }
}

unsafe fn begin_parallel_index_build(
    build_state: &mut BuildState,
    is_concurrent: bool,
    parallel_workers: i32,
) {
    pg_sys::EnterParallelMode();
    assert!(parallel_workers > 0);

    let parallel_context = pg_sys::CreateParallelContext(
        c"pg_search".as_ptr(),
        c"parallel_build_main".as_ptr(),
        parallel_workers,
    );
    let snapshot = if !is_concurrent {
        &raw mut pg_sys::SnapshotAnyData
    } else {
        pg_sys::RegisterSnapshot(pg_sys::GetTransactionSnapshot())
    };

    let est_shared = parallel_estimate_shmem(build_state.heap_relation, snapshot);
    shm_toc_estimate_chunk(&mut (*parallel_context).estimator, est_shared);
    shm_toc_estimate_keys(&mut (*parallel_context).estimator, 1);

    // TODO: Add maintenance_work_mem and PARALLEL_KEY_QUERY_TEXT to shm_toc_estimate_chunk

    // Exit parallel mode if no DSM segment is created
    pg_sys::InitializeParallelDSM(parallel_context);
    if (*parallel_context).seg.is_null() {
        if is_mvcc_snapshot(snapshot) {
            pg_sys::UnregisterSnapshot(snapshot);
        }
        pg_sys::DestroyParallelContext(parallel_context);
        pg_sys::ExitParallelMode();
        return;
    }

    let shared_state = pg_sys::shm_toc_allocate((*parallel_context).toc, est_shared)
        as *mut ParallelBuildSharedState;
    (*shared_state).heap_oid = (*build_state.heap_relation).rd_id;
    (*shared_state).index_oid = (*build_state.index_relation).rd_id;
    (*shared_state).is_concurrent = is_concurrent;

    pg_sys::ConditionVariableInit(&mut (*shared_state).workers_done);
    pg_sys::SpinLockInit(&mut (*shared_state).mutex);
    pg_sys::table_parallelscan_initialize(
        build_state.heap_relation,
        parallel_table_scan_from_shared_state(shared_state),
        snapshot,
    );

    pg_sys::shm_toc_insert(
        (*parallel_context).toc,
        PARALLEL_KEY_SHARED_STATE,
        shared_state as *mut c_void,
    );

    pg_sys::LaunchParallelWorkers(parallel_context);

    let leader =
        pg_sys::palloc0(std::mem::size_of::<ParallelBuildLeader>()) as *mut ParallelBuildLeader;
    (*leader).parallel_context = parallel_context;
    (*leader).n_participant_tuple_sorts = (*parallel_context).nworkers_launched;
    (*leader).shared = shared_state;
    (*leader).snapshot = snapshot;

    if LEADER_PARTICIPATES {
        (*leader).n_participant_tuple_sorts += 1;
    }

    if (*parallel_context).nworkers_launched == 0 {
        end_parallel_index_build(leader);
        return;
    }

    build_state.leader = leader;

    // Leader joins the parallel build
    if LEADER_PARTICIPATES {
        parallel_scan_and_insert(
            build_state.heap_relation,
            build_state.index_relation,
            shared_state,
            true,
        );
    }

    pgrx::log!("waiting for workers to attach");
    pg_sys::WaitForParallelWorkersToAttach(parallel_context);
    pgrx::log!("workers attached");
}

#[no_mangle]
unsafe extern "C-unwind" fn parallel_build_main(
    _seg: *mut pg_sys::dsm_segment,
    toc: *mut pg_sys::shm_toc,
) {
    let shared_state = pg_sys::shm_toc_lookup(toc, PARALLEL_KEY_SHARED_STATE, false)
        as *mut ParallelBuildSharedState;
    let (heap_lock, index_lock) = if !(*shared_state).is_concurrent {
        (pg_sys::ShareLock, pg_sys::AccessExclusiveLock)
    } else {
        (pg_sys::ShareUpdateExclusiveLock, pg_sys::RowExclusiveLock)
    };

    let heap = pg_sys::table_open((*shared_state).heap_oid, heap_lock as i32);
    let index = pg_sys::index_open((*shared_state).index_oid, index_lock as i32);

    parallel_scan_and_insert(heap, index, shared_state, false);

    pg_sys::index_close(index, index_lock as i32);
    pg_sys::table_close(heap, heap_lock as i32);
}

unsafe fn parallel_scan_and_insert(
    heap: pg_sys::Relation,
    index: pg_sys::Relation,
    shared_state: *mut ParallelBuildSharedState,
    progress: bool,
) {
    let index_info = pg_sys::BuildIndexInfo(index);
    (*index_info).ii_Concurrent = (*shared_state).is_concurrent;

    let mut build_state = BuildState::new((*shared_state).index_oid);
    let scan =
        pg_sys::table_beginscan_parallel(heap, parallel_table_scan_from_shared_state(shared_state));
    let reltuples = pg_sys::table_index_build_scan(
        heap,
        index,
        index_info,
        true,
        progress,
        Some(build_callback),
        &mut build_state as *mut _ as *mut c_void,
        scan,
    );

    pg_sys::SpinLockAcquire(&mut (*shared_state).mutex);
    (*shared_state).n_participants_done += 1;
    (*shared_state).reltuples += reltuples;
    pg_sys::SpinLockRelease(&mut (*shared_state).mutex);

    pg_sys::ConditionVariableSignal(&mut (*shared_state).workers_done);
}

unsafe fn parallel_heap_scan(build_state: *mut BuildState) {
    let leader = (*build_state).leader;
    let shared_state = (*leader).shared;
    let n_participant_tuple_sorts = (*leader).n_participant_tuple_sorts;

    loop {
        pg_sys::SpinLockAcquire(&mut (*shared_state).mutex);
        if (*shared_state).n_participants_done == n_participant_tuple_sorts {
            (*build_state).reltuples = (*shared_state).reltuples as usize;
            pg_sys::SpinLockRelease(&mut (*shared_state).mutex);
            break;
        }
        pgrx::log!(
            "n_participants_done: {} need {}",
            (*shared_state).n_participants_done,
            n_participant_tuple_sorts
        );
        pg_sys::SpinLockRelease(&mut (*shared_state).mutex);
        pg_sys::ConditionVariableSleep(
            &mut (*shared_state).workers_done,
            pg_sys::WaitEventIPC::WAIT_EVENT_PARALLEL_CREATE_INDEX_SCAN,
        );
    }

    pg_sys::ConditionVariableCancelSleep();
}

unsafe fn end_parallel_index_build(leader: *mut ParallelBuildLeader) {
    pg_sys::WaitForParallelWorkersToFinish((*leader).parallel_context);

    if is_mvcc_snapshot((*leader).snapshot) {
        pg_sys::UnregisterSnapshot((*leader).snapshot);
    }

    pg_sys::DestroyParallelContext((*leader).parallel_context);
    pg_sys::ExitParallelMode();
}

unsafe fn parallel_estimate_shmem(
    relation: pg_sys::Relation,
    snapshot: pg_sys::Snapshot,
) -> pg_sys::Size {
    pg_sys::add_size(
        pg_sys::table_parallelscan_estimate(relation, snapshot),
        buffer_align(std::mem::size_of::<BuildState>()),
    )
}

unsafe fn buffer_align(len: usize) -> usize {
    pg_sys::TYPEALIGN(pg_sys::ALIGNOF_BUFFER as usize, len)
}

unsafe fn shm_toc_estimate_chunk(e: *mut pg_sys::shm_toc_estimator, sz: usize) {
    (*e).space_for_chunks = pg_sys::add_size((*e).space_for_chunks, buffer_align(sz));
}

unsafe fn shm_toc_estimate_keys(e: *mut pg_sys::shm_toc_estimator, cnt: usize) {
    (*e).number_of_keys = pg_sys::add_size((*e).number_of_keys, cnt);
}

unsafe fn parallel_table_scan_from_shared_state(
    shared: *mut ParallelBuildSharedState,
) -> pg_sys::ParallelTableScanDesc {
    let offset = buffer_align(std::mem::size_of::<ParallelBuildSharedState>());
    (shared as *mut u8).add(offset) as pg_sys::ParallelTableScanDesc
}

unsafe fn is_mvcc_snapshot(snapshot: pg_sys::Snapshot) -> bool {
    (*snapshot).snapshot_type == pg_sys::SnapshotType::SNAPSHOT_MVCC
        || (*snapshot).snapshot_type == pg_sys::SnapshotType::SNAPSHOT_HISTORIC_MVCC
}
