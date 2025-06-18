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
use crate::api::HashSet;
use crate::gucs;
use crate::index::mvcc::MVCCDirectory;
use crate::index::writer::index::{
    CommittedSegment, IndexWriterConfig, Mergeable, SearchIndexMerger, SerialIndexWriter,
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
use crate::postgres::storage::merge::MergeList;
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
use tantivy::{Directory, Index, IndexMeta, SegmentMeta, TantivyDocument};

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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum WorkerRole {
    Writer,
    Merger,
}

#[derive(Copy, Clone)]
#[repr(C)]
struct WorkerCoordination {
    mutex: Spinlock,
    nstarted: usize,
    nlaunched: usize,
    roles: [WorkerRole; MAX_CREATE_INDEX_WORKERS],
    nroles: usize,
    nwriters_remaining: usize,
}

impl Default for WorkerCoordination {
    fn default() -> Self {
        Self {
            mutex: Default::default(),
            nstarted: Default::default(),
            nlaunched: Default::default(),
            roles: [WorkerRole::Writer; MAX_CREATE_INDEX_WORKERS],
            nroles: Default::default(),
            nwriters_remaining: Default::default(),
        }
    }
}

impl ParallelStateType for WorkerCoordination {}
impl WorkerCoordination {
    fn inc_nstarted(&mut self) {
        let _lock = self.mutex.acquire();
        self.nstarted += 1;
    }
    fn nstarted(&mut self) -> usize {
        let _lock = self.mutex.acquire();
        self.nstarted
    }

    fn set_nlaunched(&mut self, nlaunched: usize) {
        let _lock = self.mutex.acquire();
        self.nlaunched = nlaunched;
    }
    fn nlaunched(&mut self) -> usize {
        let _lock = self.mutex.acquire();
        self.nlaunched
    }
    fn set_roles(&mut self, roles: Vec<WorkerRole>) {
        let _lock = self.mutex.acquire();
        self.nroles = roles.len();
        self.nwriters_remaining = roles.iter().filter(|r| **r == WorkerRole::Writer).count();
        self.roles[..roles.len()].copy_from_slice(&roles);
    }
    fn claim_role(&mut self) -> WorkerRole {
        let _lock = self.mutex.acquire();
        self.nroles -= 1;
        self.roles[self.nroles]
    }
    fn nwriters_remaining(&mut self) -> usize {
        let _lock = self.mutex.acquire();
        self.nwriters_remaining
    }
    fn dec_nwriters_remaining(&mut self) {
        let _lock = self.mutex.acquire();
        self.nwriters_remaining -= 1;
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

type RelTuples = f64;
#[derive(serde::Serialize, serde::Deserialize)]
enum WorkerResponse {
    WriteFinished(RelTuples),
    MergeFinished,
}

struct BuildWorker<'a> {
    config: WorkerConfig,
    table_scan_desc: Option<NonNull<pg_sys::TableScanDescData>>,
    coordination: &'a mut WorkerCoordination,
    heaprel: PgRelation,
    indexrel: PgRelation,
    merge_group_size: Option<usize>,
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
                merge_group_size: None,
            }
        }
    }

    fn run(mut self, mq_sender: &MessageQueueSender, _worker_number: i32) -> anyhow::Result<()> {
        // wait for the leader to tell us how many total workers have been launched
        while self.coordination.nlaunched() == 0 {
            check_for_interrupts!();
            std::thread::yield_now();
        }

        // communicate to the group that we've started
        self.coordination.inc_nstarted();

        let response = self.do_work()?;
        Ok(mq_sender.send(serde_json::to_vec(&response)?)?)
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
            merge_group_size: None,
        }
    }

    fn do_work(&mut self) -> anyhow::Result<WorkerResponse> {
        if self.coordination.claim_role() == WorkerRole::Writer {
            let reltuples = self.do_build()?;
            self.coordination.dec_nwriters_remaining();
            Ok(WorkerResponse::WriteFinished(reltuples))
        } else {
            self.do_merge()?;
            Ok(WorkerResponse::MergeFinished)
        }
    }

    fn do_build(&mut self) -> anyhow::Result<RelTuples> {
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

            let _ = build_state.writer.commit()?;
            Ok(reltuples)
        }
    }

    fn do_merge(&mut self) -> anyhow::Result<()> {
        // TODO: change this to spin until there's no writers remaining AND there's nothing left to merge
        while self.coordination.nwriters_remaining() > 0 {
            check_for_interrupts!();

            unsafe {
                Self::watch_latch(1000);

                let metadata = unsafe { MetaPage::open(self.indexrel.oid()) };
                let merge_entry = {
                    let merge_lock = unsafe { metadata.acquire_merge_lock() };
                    let mut merge_list = merge_lock.merge_list();
                    let mergeable_entries = mergeable_entries(self.indexrel.oid(), &merge_list);
                    if mergeable_entries.is_empty() {
                        continue;
                    }

                    // use the first segment written to estimate the per-segment doc size
                    let merge_group_size = self.merge_group_size(mergeable_entries[0].max_doc);
                    if merge_group_size <= 1 || mergeable_entries.len() < merge_group_size {
                        continue;
                    }

                    let segment_ids: Vec<_> = mergeable_entries
                        .into_iter()
                        .take(merge_group_size)
                        .map(|entry| entry.segment_id)
                        .collect();
                    merge_list.add_segment_ids(segment_ids.iter())?
                };

                pgrx::debug1!(
                    "do_merge: nwriters_remaining={}, segments to merge: {:?}",
                    self.coordination.nwriters_remaining(),
                    merge_entry
                );

                let directory = MVCCDirectory::mergeable(self.indexrel.oid());
                let index = Index::open(directory.clone())?;
                let metas = index.searchable_segment_metas()?;
                let segment_metas = merge_entry
                    .segment_ids(self.indexrel.oid())
                    .iter()
                    .map(|id| metas.iter().find(|meta| meta.id() == *id).unwrap().clone())
                    .collect::<Vec<_>>();
                let segments = segment_metas
                    .iter()
                    .map(|meta| index.segment(meta.clone()))
                    .collect::<Vec<_>>();
                let mut merger = SearchIndexMerger::open(directory)?;

                if let Some(merged_meta) = merger.merge_into(&segments)? {
                    pgrx::debug1!("do_merge: merged segment: {:?}", merged_meta.id());
                    let merge_lock = metadata.acquire_merge_lock();
                    let mut merge_list = merge_lock.merge_list();
                    merge_list.add_segment_ids(std::iter::once(&merged_meta.id()))?;
                    drop(merge_lock);

                    pgrx::debug1!("do_merge: added to merge list: {:?}", merged_meta.id());
                    let index_meta = index.load_metas()?;
                    let previous_meta = IndexMeta {
                        segments: segment_metas,
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

                pgrx::debug1!("do_merge: garbage collecting");
                garbage_collect_index(&self.indexrel);
            }
        }

        Ok(())
    }

    unsafe fn wait_latch(ms: i64) {
        let events = pg_sys::WL_LATCH_SET as i32
            | pg_sys::WL_TIMEOUT as i32
            | pg_sys::WL_EXIT_ON_PM_DEATH as i32;
        pg_sys::WaitLatch(pg_sys::MyLatch, events, ms, pg_sys::PG_WAIT_EXTENSION);
        pg_sys::ResetLatch(pg_sys::MyLatch);
    }

    fn merge_group_size(&mut self, docs_per_segment: u32) -> usize {
        self.merge_group_size.unwrap_or_else(|| {
            let nsegments = (estimate_heap_reltuples(&self.heaprel) as f64 / docs_per_segment as f64).ceil() as usize;
            let merge_group_size = (nsegments as f64 / adjusted_target_segment_count(&self.heaprel) as f64).ceil() as usize;

            pgrx::debug1!("do_merge: predicting the creation {nsegments} segments, so merge in groups of {merge_group_size}");

            self.merge_group_size = Some(merge_group_size);
            merge_group_size
        })
    }
}

/// Internal state used by each parallel build worker
struct WorkerBuildState {
    writer: SerialIndexWriter,
    categorized_fields: Vec<(SearchField, CategorizedFieldData)>,
    key_field_name: FieldName,
    per_row_context: PgMemoryContexts,
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
            writer,
            categorized_fields,
            key_field_name,
            per_row_context: PgMemoryContexts::new("pg_search ambuild context"),
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

    build_state.per_row_context.switch_to(|_| {
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
            .insert(doc, ctid_u64)
            .unwrap_or_else(|e| panic!("{e}"));
    });
    build_state.per_row_context.reset();
}

/// Build an index.  This is the workhorse behind `CREATE INDEX` and `REINDEX`.
///
/// If the system allows, it will build the index in parallel.  Otherwise the index is built in
/// serially in this connected backend.
pub(super) fn build_index(
    heaprel: PgRelation,
    indexrel: PgRelation,
    concurrent: bool,
) -> anyhow::Result<RelTuples> {
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
        && gucs::adjust_maintenance_work_mem(nworkers).get() / nworkers < 64 * 1024 * 1024
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
        coordination.set_roles(assign_roles(nlaunched_plus_leader));
        coordination.set_nlaunched(nlaunched_plus_leader);

        let response = if unsafe { pg_sys::parallel_leader_participation } {
            // if the leader is to participate too, it's nice for it to wait until all the other workers
            // have indicated that they're running.  Otherwise, it's likely the leader will get ahead
            // of the workers, which doesn't allow for "evenly" distributing the work
            while coordination.nstarted() != nlaunched {
                check_for_interrupts!();
                std::thread::yield_now();
            }

            pgrx::debug1!("build_index: all workers have launched, building in parallel");
            // directly instantiate a worker for the leader and have it do its build
            let mut worker = BuildWorker::new_parallel_worker(*process.state_manager());
            Some(worker.do_work()?)
        } else {
            pgrx::debug1!("build_index: leader is not participating");
            None
        };

        // wait for the workers to finish by collecting all their response messages
        let mut total_tuples = if let Some(WorkerResponse::WriteFinished(reltuples)) = response {
            reltuples
        } else {
            0.0
        };

        for (_, message) in process {
            check_for_interrupts!();
            let worker_response = serde_json::from_slice::<WorkerResponse>(&message)?;

            if let WorkerResponse::WriteFinished(reltuples) = worker_response {
                pgrx::debug1!("build_index: writer finished");
                total_tuples += reltuples;
            }
        }

        Ok(total_tuples)
    } else {
        pgrx::debug1!("build_index: not doing a parallel build");
        // not doing a parallel build, so directly instantiate a BuildWorker and serially run the
        // whole build here in this connected backend
        let heaprelid = heaprel.oid();
        let indexrelid = indexrel.oid();

        let mut coordination: WorkerCoordination = Default::default();
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

fn assign_roles(nworkers: usize) -> Vec<WorkerRole> {
    let nmergers = nworkers / 3;
    let nwriters = nworkers - nmergers;

    let mut roles = vec![WorkerRole::Merger; nmergers];
    roles.extend(vec![WorkerRole::Writer; nwriters]);
    roles
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
    if reltuples <= target_segment_count as RelTuples {
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

fn estimate_heap_reltuples(heap_relation: &PgRelation) -> RelTuples {
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

    reltuples as RelTuples
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
        .map(|entry| entry.clone())
        .collect::<Vec<_>>()
}
