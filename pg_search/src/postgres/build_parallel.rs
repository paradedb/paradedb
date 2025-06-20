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

use crate::api::FieldName;
use crate::gucs;
use crate::index::mvcc::MVCCDirectory;
use crate::index::writer::index::{
    IndexWriterConfig, Mergeable, SearchIndexMerger, SerialIndexWriter,
};
use crate::launch_parallel_process;
use crate::parallel_worker::mqueue::MessageQueueSender;
use crate::parallel_worker::{
    ParallelProcess, ParallelState, ParallelStateManager, ParallelStateType, ParallelWorker,
    WorkerStyle,
};
use crate::postgres::insert::garbage_collect_index;
use crate::postgres::spinlock::Spinlock;
use crate::postgres::storage::block::{SegmentMetaEntry, SEGMENT_METAS_START};
use crate::postgres::storage::buffer::BufferManager;
use crate::postgres::storage::merge::{MergeEntry, MergeList};
use crate::postgres::storage::metadata::MetaPage;
use crate::postgres::storage::LinkedItemList;
use crate::postgres::utils::{categorize_fields, row_to_search_document, CategorizedFieldData};
use crate::schema::{SearchField, SearchIndexSchema};
use pgrx::pg_sys::panic::ErrorReport;
use pgrx::{
    check_for_interrupts, function_name, pg_guard, pg_sys, PgLogLevel, PgMemoryContexts,
    PgRelation, PgSqlErrorCode,
};
use std::num::NonZeroUsize;
use std::ptr::{addr_of_mut, NonNull};
use std::sync::OnceLock;
use tantivy::{Directory, Index, IndexMeta, TantivyDocument};

// target_segment_pool in WorkerCoordination requires a fixed size array, so we have to limit the
// number of workers to 512, which is okay because max_parallel_maintenance_workers is typically much smaller
const MAX_CREATE_INDEX_WORKERS: usize = 512;

/// General, immutable configuration used for the workers
#[derive(Copy, Clone)]
#[repr(C)]
struct WorkerConfig {
    heaprelid: pg_sys::Oid,
    indexrelid: pg_sys::Oid,
    concurrent: bool,
}
impl ParallelStateType for WorkerConfig {}

/// Type alias that holds a pointer to a [`pg_sys::ParallelTableScanDescData`] which is over-allocated,
/// so the [`usize`] field tells us how big it really is, in bytes
type ScanDesc = (usize, *mut pg_sys::ParallelTableScanDescData);
impl ParallelStateType for pg_sys::ParallelTableScanDescData {}

#[derive(Copy, Clone, Default)]
#[repr(C)]
struct WorkerCoordination {
    mutex: Spinlock,
    nlaunched: usize,
    nwriters_remaining: usize,
}

impl ParallelStateType for WorkerCoordination {}
impl WorkerCoordination {
    fn set_nlaunched(&mut self, nlaunched: usize) {
        let _lock = self.mutex.acquire();
        self.nlaunched = nlaunched;
    }
    fn nlaunched(&mut self) -> usize {
        let _lock = self.mutex.acquire();
        self.nlaunched
    }
    fn set_nwriters_remaining(&mut self, nwriters_remaining: usize) {
        let _lock = self.mutex.acquire();
        self.nwriters_remaining = nwriters_remaining;
    }
    fn dec_nwriters_remaining(&mut self) -> usize {
        let _lock = self.mutex.acquire();
        self.nwriters_remaining -= 1;
        self.nwriters_remaining
    }
}

/// The parallel process for setting up a parallel index build
struct ParallelBuild {
    config: WorkerConfig,
    scandesc: ScanDesc,
    coordination: WorkerCoordination,
}

impl ParallelState for ScanDesc {
    fn type_name(&self) -> &'static str {
        std::any::type_name::<pg_sys::ParallelTableScanDescData>()
    }

    fn size_of(&self) -> usize {
        self.0
    }

    fn as_bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.1 as *const _ as *const u8, self.size_of()) }
    }
}

impl ParallelBuild {
    fn new(
        heaprel: &PgRelation,
        indexrel: &PgRelation,
        snapshot: pg_sys::Snapshot,
        concurrent: bool,
    ) -> Self {
        let scandesc = unsafe {
            let size = size_of::<pg_sys::ParallelTableScanDescData>()
                + pg_sys::table_parallelscan_estimate(heaprel.as_ptr(), snapshot) as usize;
            let scandesc = pg_sys::palloc0(size).cast();
            pg_sys::table_parallelscan_initialize(heaprel.as_ptr(), scandesc, snapshot);
            (size, scandesc)
        };
        Self {
            config: WorkerConfig {
                heaprelid: heaprel.oid(),
                indexrelid: indexrel.oid(),
                concurrent,
            },
            scandesc,
            coordination: Default::default(),
        }
    }
}

impl ParallelProcess for ParallelBuild {
    fn state_values(&self) -> Vec<&dyn ParallelState> {
        vec![&self.config, &self.scandesc, &self.coordination]
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct WorkerResponse {
    reltuples: f64,
}

struct BuildWorker<'a> {
    config: WorkerConfig,
    table_scan_desc: Option<NonNull<pg_sys::TableScanDescData>>,
    coordination: &'a mut WorkerCoordination,
    heaprel: PgRelation,
    indexrel: PgRelation,
}

impl ParallelWorker for BuildWorker<'_> {
    fn new_parallel_worker(state_manager: ParallelStateManager) -> Self
    where
        Self: Sized,
    {
        let config = state_manager
            .object::<WorkerConfig>(0)
            .expect("should be able to get ParallelBuildConfig from state manager")
            .expect("ParallelBuildConfig should not be NULL");
        let scandesc = state_manager
            .object::<pg_sys::ParallelTableScanDescData>(1)
            .expect("should be able to get ParallelTableScanDesc")
            .expect("ParallelTableDescDesc should not be NULL");
        let coordination = state_manager
            .object::<WorkerCoordination>(2)
            .expect("should be able to get ProcessCoordination")
            .expect("ProcessCoordination should not be NULL");

        unsafe {
            let (heap_lock, index_lock) = if !config.concurrent {
                (pg_sys::ShareLock, pg_sys::AccessExclusiveLock)
            } else {
                (pg_sys::ShareUpdateExclusiveLock, pg_sys::RowExclusiveLock)
            };

            let heaprel = PgRelation::with_lock(config.heaprelid, heap_lock as pg_sys::LOCKMODE);
            let indexrel = PgRelation::with_lock(config.indexrelid, index_lock as pg_sys::LOCKMODE);
            let table_scan_desc = pg_sys::table_beginscan_parallel(heaprel.as_ptr(), scandesc);

            Self {
                config: *config,
                table_scan_desc: NonNull::new(table_scan_desc),
                coordination,
                heaprel,
                indexrel,
            }
        }
    }

    fn run(mut self, mq_sender: &MessageQueueSender, _worker_number: i32) -> anyhow::Result<()> {
        // wait for the leader to tell us how many total workers have been launched
        while self.coordination.nlaunched() == 0 {
            check_for_interrupts!();
            std::thread::yield_now();
        }

        let reltuples = self.do_build()?;
        Ok(mq_sender.send(serde_json::to_vec(&WorkerResponse { reltuples })?)?)
    }
}

impl<'a> BuildWorker<'a> {
    fn new(
        heaprel: PgRelation,
        indexrel: PgRelation,
        config: WorkerConfig,
        coordination: &'a mut WorkerCoordination,
    ) -> Self {
        Self {
            config,
            table_scan_desc: None,
            heaprel,
            indexrel,
            coordination,
        }
    }

    fn do_build(&mut self) -> anyhow::Result<f64> {
        unsafe {
            let index_info = pg_sys::BuildIndexInfo(self.indexrel.as_ptr());
            (*index_info).ii_Concurrent = self.config.concurrent;
            let nlaunched = self.coordination.nlaunched();
            let per_worker_memory_budget =
                gucs::adjust_maintenance_work_mem(nlaunched).get() / nlaunched;
            let mut build_state = WorkerBuildState::new(
                &self.indexrel,
                NonZeroUsize::new(per_worker_memory_budget)
                    .expect("per worker memory budget should be non-zero"),
            )?;

            let reltuples = pg_sys::table_index_build_scan(
                self.heaprel.as_ptr(),
                self.indexrel.as_ptr(),
                index_info,
                true,
                true,
                Some(build_callback),
                addr_of_mut!(build_state).cast(),
                self.table_scan_desc
                    .as_ref()
                    .map(|x| x.as_ptr())
                    .unwrap_or(std::ptr::null_mut()),
            );

            build_state.commit()?;
            let nwriters_remaining = self.coordination.dec_nwriters_remaining();

            pgrx::debug1!("do_build: nwriters_remaining: {}", nwriters_remaining);
            build_state.do_merge(nwriters_remaining == 0)?;

            Ok(reltuples as f64)
        }
    }
}

/// Internal state used by each parallel build worker
struct WorkerBuildState {
    writer: Option<SerialIndexWriter>,
    categorized_fields: Vec<(SearchField, CategorizedFieldData)>,
    key_field_name: FieldName,
    per_row_context: PgMemoryContexts,
    merge_group_size: OnceLock<usize>,
    indexrel: PgRelation,
    heaprel: PgRelation,
}

impl WorkerBuildState {
    pub fn new(
        indexrel: &PgRelation,
        per_worker_memory_budget: NonZeroUsize,
    ) -> anyhow::Result<Self> {
        let config = IndexWriterConfig {
            memory_budget: per_worker_memory_budget,
        };
        let writer = SerialIndexWriter::open(indexrel, config)?;
        let schema = SearchIndexSchema::open(indexrel.oid())?;
        let tupdesc = indexrel.tuple_desc();
        let categorized_fields = categorize_fields(&tupdesc, &schema);
        let key_field_name = schema.key_field().field_name();
        Ok(Self {
            writer: Some(writer),
            categorized_fields,
            key_field_name,
            per_row_context: PgMemoryContexts::new("pg_search ambuild context"),
            merge_group_size: OnceLock::new(),
            indexrel: indexrel.clone(),
            heaprel: indexrel.heap_relation().unwrap().clone(),
        })
    }

    fn commit(&mut self) -> anyhow::Result<()> {
        let writer = self.writer.take().expect("writer should be set");
        let _ = writer.commit()?;
        Ok(())
    }

    unsafe fn do_merge(&mut self, is_last_worker: bool) -> anyhow::Result<()> {
        let merge_entry = {
            let metadata = MetaPage::open(self.indexrel.oid());
            let merge_lock = metadata.acquire_merge_lock();
            let mut merge_list = merge_lock.merge_list();
            let mergeable_entries = mergeable_entries(self.indexrel.oid(), &merge_list);

            // check if there's enough segments to merge
            if mergeable_entries.is_empty() {
                return Ok(());
            }

            let merge_group_size = self.merge_group_size(mergeable_entries[0].max_doc);
            if merge_group_size <= 1 {
                return Ok(());
            }

            if !is_last_worker && mergeable_entries.len() < merge_group_size {
                return Ok(());
            }

            pgrx::debug1!(
                "do_merge: {:?} waiting to be merged",
                mergeable_entries.len()
            );

            // if it's the last worker, ignore the merge group size and merge down all the segments
            let nmerge = if is_last_worker {
                mergeable_entries.len()
            } else {
                merge_group_size
            };
            let segment_ids: Vec<_> = mergeable_entries
                .into_iter()
                .take(nmerge)
                .map(|entry| entry.segment_id)
                .collect();
            merge_list.add_segment_ids(segment_ids.iter())?
        };

        // do the merge
        pgrx::debug1!(
            "do_merge: segments to merge: {:?}",
            merge_entry.segment_ids(self.indexrel.oid()),
        );
        merge_down_entry(self.indexrel.oid(), &merge_entry)?;

        // garbage collect the index, returning to the fsm
        pgrx::debug1!("do_merge: garbage collecting");
        garbage_collect_index(&self.indexrel);

        Ok(())
    }

    fn merge_group_size(&mut self, docs_per_segment: u32) -> usize {
        *self.merge_group_size.get_or_init(|| {
            let nsegments = (estimate_heap_reltuples(&self.heaprel)
                / docs_per_segment as f64)
                .ceil() as usize;
            let target_segment_count = adjusted_target_segment_count(&self.heaprel);
            let merge_group_size = (nsegments as f64 / target_segment_count as f64).ceil() as usize;

            pgrx::debug1!(
                "predicting {nsegments} total segments, targeting {target_segment_count}, merge in groups of {merge_group_size}"
            );

            merge_group_size
        })
    }
}

#[pg_guard]
unsafe extern "C-unwind" fn build_callback(
    _indexrel: pg_sys::Relation,
    ctid: pg_sys::ItemPointer,
    values: *mut pg_sys::Datum,
    isnull: *mut bool,
    _tuple_is_alive: bool,
    state: *mut std::os::raw::c_void,
) {
    check_for_interrupts!();

    let build_state = &mut *state.cast::<WorkerBuildState>();
    let ctid_u64 = crate::postgres::utils::item_pointer_to_u64(*ctid);

    let did_finalize = build_state.per_row_context.switch_to(|_| {
        let mut doc = TantivyDocument::new();
        row_to_search_document(
            values,
            isnull,
            &build_state.key_field_name,
            &build_state.categorized_fields,
            &mut doc,
        )
        .unwrap_or_else(|e| panic!("{e}"));

        build_state
            .writer
            .as_mut()
            .expect("build_callback: writer should be set")
            .insert(doc, ctid_u64)
            .unwrap_or_else(|e| panic!("{e}"))
    });
    build_state.per_row_context.reset();

    if did_finalize {
        build_state
            .do_merge(false)
            .unwrap_or_else(|e| panic!("{e}"));
    }
}

/// Build an index.  This is the workhorse behind `CREATE INDEX` and `REINDEX`.
///
/// If the system allows, it will build the index in parallel.  Otherwise the index is built in
/// serially in this connected backend.
pub(super) fn build_index(
    heaprel: PgRelation,
    indexrel: PgRelation,
    concurrent: bool,
) -> anyhow::Result<f64> {
    struct SnapshotDropper(pg_sys::Snapshot);
    impl Drop for SnapshotDropper {
        fn drop(&mut self) {
            unsafe {
                let snapshot = self.0;
                // if it's an mvcc snapshot we must unregister it
                if (*snapshot).snapshot_type == pg_sys::SnapshotType::SNAPSHOT_MVCC
                    || (*snapshot).snapshot_type == pg_sys::SnapshotType::SNAPSHOT_HISTORIC_MVCC
                {
                    pg_sys::UnregisterSnapshot(snapshot);
                }
            }
        }
    }

    let snapshot = SnapshotDropper(unsafe {
        if concurrent {
            pg_sys::RegisterSnapshot(pg_sys::GetTransactionSnapshot())
        } else {
            &raw mut pg_sys::SnapshotAnyData
        }
    });

    let process = ParallelBuild::new(&heaprel, &indexrel, snapshot.0, concurrent);
    let nworkers = create_index_nworkers(&heaprel);
    pgrx::debug1!("build_index: asked for {nworkers} workers");

    if adjusted_target_segment_count(&heaprel) > 1
        && gucs::adjust_maintenance_work_mem(nworkers).get() / nworkers < 15 * 1024 * 1024
    {
        ErrorReport::new(
            PgSqlErrorCode::ERRCODE_INSUFFICIENT_RESOURCES,
            "maintenance_work_mem may be too low",
            function_name!(),
        )
        .set_detail("this can significantly increase the time it takes to build the index")
        .set_hint("increase maintenance_work_mem")
        .report(PgLogLevel::WARNING);
    }

    if let Some(mut process) = launch_parallel_process!(
        ParallelBuild<BuildWorker>,
        process,
        WorkerStyle::Maintenance,
        nworkers,
        1024
    ) {
        let nlaunched = process.launched_workers();
        pgrx::debug1!("build_index: launched {nworkers} workers");
        let coordination = process
            .state_manager_mut()
            .object::<WorkerCoordination>(2)
            .expect("process coordination")
            .expect("process coordination should not be NULL");

        // account for the leader in the coordination
        let mut nlaunched_plus_leader = nlaunched;
        if unsafe { pg_sys::parallel_leader_participation } {
            nlaunched_plus_leader += 1;
        }

        // set_nlaunched last, because workers wait for this to be set
        coordination.set_nwriters_remaining(nlaunched_plus_leader);
        coordination.set_nlaunched(nlaunched_plus_leader);

        let mut total_tuples = if unsafe { pg_sys::parallel_leader_participation } {
            // directly instantiate a worker for the leader and have it do its build
            let mut worker = BuildWorker::new_parallel_worker(*process.state_manager());
            worker.do_build()?
        } else {
            pgrx::debug1!("build_index: leader is not participating");
            0.0
        };

        // wait for the workers to finish by collecting all their response messages
        for (_, message) in process {
            check_for_interrupts!();
            let worker_response = serde_json::from_slice::<WorkerResponse>(&message)?;
            total_tuples += worker_response.reltuples;
        }

        Ok(total_tuples)
    } else {
        pgrx::debug1!("build_index: not doing a parallel build");
        // not doing a parallel build, so directly instantiate a BuildWorker and serially run the
        // whole build here in this connected backend
        let heaprelid = heaprel.oid();
        let indexrelid = indexrel.oid();

        let mut coordination: WorkerCoordination = Default::default();
        coordination.set_nwriters_remaining(1);
        coordination.set_nlaunched(1);

        let mut worker = BuildWorker::new(
            heaprel,
            indexrel,
            WorkerConfig {
                heaprelid,
                indexrelid,
                concurrent,
            },
            &mut coordination,
        );

        worker.do_build()
    }
}

/// Determine the number of workers to use for a given CREATE INDEX/REINDEX statement.
///
/// The number of workers is determined by max_parallel_maintenance_workers. However, if max_parallel_maintenance_workers
/// is greater than available parallelism, we use available parallelism.
///
/// If the leader is participating, we subtract 1 from the number of workers because the leader also counts as a worker.
fn create_index_nworkers(heaprel: &PgRelation) -> usize {
    // We don't want a parallel build to happen if we're creating a single segment
    let target_segment_count = adjusted_target_segment_count(heaprel);
    if target_segment_count == 1 {
        return 0;
    }

    // NB: we _could_ use pg_sys::plan_create_index_workers(), or on v17+ accept IndexIndex::ii_ParallelWorkers,
    // but doing either of these would prohibit the user from having direct control over the number of
    // workers used for a given CREATE INDEX/REINDEX statement.  Internal discussions led to that
    // being more important that us trying to be "smart"
    let maintenance_workers = unsafe {
        if !heaprel.rd_options.is_null() {
            let options = heaprel.rd_options.cast::<pg_sys::StdRdOptions>();
            if (*options).parallel_workers <= 0 {
                pg_sys::max_parallel_maintenance_workers as usize
            } else {
                (*options).parallel_workers as usize
            }
        } else {
            pg_sys::max_parallel_maintenance_workers as usize
        }
    };

    if maintenance_workers == 0 {
        return 0;
    }

    // ensure that we never have more workers (including the leader) than the max allowed number of workers
    let mut nworkers = maintenance_workers.min(MAX_CREATE_INDEX_WORKERS);
    if unsafe { pg_sys::parallel_leader_participation } && nworkers == MAX_CREATE_INDEX_WORKERS {
        nworkers -= 1;
    }
    nworkers
}

/// If we determine that the table is very small, we should just create a single segment
fn adjusted_target_segment_count(heaprel: &PgRelation) -> usize {
    // If there are fewer rows than number of CPUs, use 1 worker
    let reltuples = estimate_heap_reltuples(heaprel);
    let target_segment_count = gucs::target_segment_count();
    if reltuples <= target_segment_count as f64 {
        pgrx::debug1!("number of reltuples ({reltuples}) is less than target segment count ({target_segment_count}), creating a single segment");
        return 1;
    }

    // If the entire heap fits inside the smallest allowed Tantivy segment memory budget of 15MB, use 1 worker
    let byte_size = estimate_heap_byte_size(heaprel);
    if byte_size <= 15 * 1024 * 1024 {
        pgrx::debug1!("heap byte size ({byte_size}) is less than 15MB, creating a single segment");
        return 1;
    }

    target_segment_count
}

fn estimate_heap_reltuples(heap_relation: &PgRelation) -> f64 {
    let mut reltuples = heap_relation.reltuples().unwrap_or_default();

    // if the reltuples estimate is not available, estimate the number of tuples in the heap
    // by multiplying the number of pages by the max offset number of the first page
    if reltuples <= 0.0 {
        let npages = unsafe {
            pg_sys::RelationGetNumberOfBlocksInFork(
                heap_relation.as_ptr(),
                pg_sys::ForkNumber::MAIN_FORKNUM,
            )
        };

        if npages == 0 {
            // the tuple count actually is 0
            return 0.0;
        }

        let bman = BufferManager::new(heap_relation.oid());
        let buffer = bman.get_buffer(0);
        let page = buffer.page();
        let max_offset = page.max_offset_number();
        reltuples = npages as f32 * max_offset as f32;
    }

    reltuples as f64
}

fn estimate_heap_byte_size(heap_relation: &PgRelation) -> usize {
    let npages = unsafe {
        pg_sys::RelationGetNumberOfBlocksInFork(
            heap_relation.as_ptr(),
            pg_sys::ForkNumber::MAIN_FORKNUM,
        )
    };

    npages as usize * pg_sys::BLCKSZ as usize
}

fn mergeable_entries(index_oid: pg_sys::Oid, merge_list: &MergeList) -> Vec<SegmentMetaEntry> {
    let segment_components =
        LinkedItemList::<SegmentMetaEntry>::open(index_oid, SEGMENT_METAS_START);
    let all_entries = unsafe { segment_components.list() };

    let non_mergeable_segments = unsafe { merge_list.list_segment_ids().collect::<Vec<_>>() };
    all_entries
        .iter()
        .filter(|entry| {
            if non_mergeable_segments.contains(&entry.segment_id) {
                return false;
            }
            if entry.xmax == pg_sys::FrozenTransactionId {
                return false;
            }
            true
        })
        .copied()
        .collect::<Vec<_>>()
}

/// Merge down the segments inside a single [`MergeEntry`],
/// then update merge list to prevent them from being merged again,
/// and finally update the index metas to reflect the merge
unsafe fn merge_down_entry(index_oid: pg_sys::Oid, merge_entry: &MergeEntry) -> anyhow::Result<()> {
    let directory = MVCCDirectory::mergeable(index_oid);
    let index = Index::open(directory.clone())?;
    let mut merger = SearchIndexMerger::open(directory)?;
    let searchable_metas = index.searchable_segment_metas()?;
    let metas_to_merge = merge_entry
        .segment_ids(index_oid)
        .iter()
        .map(|id| {
            searchable_metas
                .iter()
                .find(|meta| meta.id() == *id)
                .unwrap()
                .clone()
        })
        .collect::<Vec<_>>();
    let segments = metas_to_merge
        .iter()
        .map(|meta| index.segment(meta.clone()))
        .collect::<Vec<_>>();

    if let Some(merged_meta) = merger.merge_into(&segments)? {
        pgrx::debug1!("do_merge: merged segment: {:?}", merged_meta.id());

        // update the merge list first, under the merge lock
        {
            let metadata = MetaPage::open(index_oid);
            let merge_lock = metadata.acquire_merge_lock();
            let mut merge_list = merge_lock.merge_list();
            merge_list.add_segment_ids(std::iter::once(&merged_meta.id()))?;
        }

        // then update the index metas
        let index_meta = index.load_metas()?;
        let previous_meta = IndexMeta {
            segments: metas_to_merge,
            ..index_meta.clone()
        };
        let new_meta = IndexMeta {
            segments: vec![merged_meta],
            ..index_meta.clone()
        };
        index
            .directory()
            .save_metas(&new_meta, &previous_meta, &mut ())?;
    }

    Ok(())
}
