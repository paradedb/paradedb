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
use crate::index::writer::index::{CommittedSegment, IndexWriterConfig, SerialIndexWriter};
use crate::launch_parallel_process;
use crate::parallel_worker::mqueue::MessageQueueSender;
use crate::parallel_worker::{
    ParallelProcess, ParallelState, ParallelStateManager, ParallelStateType, ParallelWorker,
    WorkerStyle,
};
use crate::postgres::spinlock::Spinlock;
use crate::postgres::storage::buffer::BufferManager;
use crate::postgres::utils::{categorize_fields, row_to_search_document, CategorizedFieldData};
use crate::schema::{SearchField, SearchIndexSchema};
use pgrx::{check_for_interrupts, pg_guard, pg_sys, PgMemoryContexts, PgRelation};
use std::num::NonZeroUsize;
use std::ptr::{addr_of_mut, NonNull};
use tantivy::TantivyDocument;

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

#[derive(Copy, Clone)]
#[repr(C)]
struct WorkerCoordination {
    mutex: Spinlock,
    nstarted: usize,
    nlaunched: usize,
    target_segment_pool: [usize; MAX_CREATE_INDEX_WORKERS],
    pool_size: usize,
}

impl Default for WorkerCoordination {
    fn default() -> Self {
        Self {
            mutex: Default::default(),
            nstarted: Default::default(),
            nlaunched: Default::default(),
            target_segment_pool: [0; MAX_CREATE_INDEX_WORKERS],
            pool_size: Default::default(),
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
    fn set_target_segment_pool(&mut self, target_segment_pool: Vec<usize>) {
        let _lock = self.mutex.acquire();
        self.pool_size = target_segment_pool.len();
        self.target_segment_pool[..target_segment_pool.len()].copy_from_slice(&target_segment_pool);
    }
    fn claim_target_segment_count(&mut self) -> usize {
        assert!(self.pool_size > 0, "all segments have been claimed");
        let _lock = self.mutex.acquire();
        self.pool_size -= 1;
        self.target_segment_pool[self.pool_size]
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
    segment_ids: HashSet<CommittedSegment>,
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

        // communicate to the group that we've started
        self.coordination.inc_nstarted();

        let (reltuples, segment_ids) = self.do_build()?;
        let response = WorkerResponse {
            reltuples,
            segment_ids,
        };

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
        }
    }

    fn do_build(&mut self) -> anyhow::Result<(f64, HashSet<CommittedSegment>)> {
        unsafe {
            let index_info = pg_sys::BuildIndexInfo(self.indexrel.as_ptr());
            (*index_info).ii_Concurrent = self.config.concurrent;
            let target_segment_count = self.coordination.claim_target_segment_count();
            let nlaunched = self.coordination.nlaunched();
            let per_worker_memory_budget =
                gucs::adjust_maintenance_work_mem(nlaunched).get() / nlaunched;
            let min_docs_per_segment = (estimate_heap_reltuples(&self.heaprel)
                / nlaunched as f64
                / target_segment_count as f64) as usize;
            let mut build_state = WorkerBuildState::new(
                &self.indexrel,
                NonZeroUsize::new(target_segment_count)
                    .expect("target segment count should be non-zero"),
                NonZeroUsize::new(per_worker_memory_budget)
                    .expect("per worker memory budget should be non-zero"),
                min_docs_per_segment,
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

            let segment_ids = build_state.writer.commit()?;
            Ok((reltuples, segment_ids))
        }
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
        target_segment_count: NonZeroUsize,
        per_worker_memory_budget: NonZeroUsize,
        min_docs_per_segment: usize,
    ) -> anyhow::Result<Self> {
        let config = IndexWriterConfig {
            target_segment_count: Some(target_segment_count),
            memory_budget: per_worker_memory_budget,
            min_docs_per_segment: Some(min_docs_per_segment),
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
) -> anyhow::Result<(f64, HashSet<CommittedSegment>)> {
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

        // set target segment pool based on target segment count and number of launched workers
        // for instance, if we have 6 workers and want 8 segments, the target segment pool will be [1, 1, 1, 1, 2, 2]
        let mut target_segment_pool = vec![0; nlaunched_plus_leader];
        let mut remaining_segments = adjusted_target_segment_count(&heaprel);
        let mut i = 0;
        while remaining_segments > 0 {
            target_segment_pool[i] += 1;
            remaining_segments -= 1;
            i += 1;
            if i == target_segment_pool.len() {
                i = 0;
            }
        }
        pgrx::debug1!(
            "build_index: setting target segment pool {:?}",
            target_segment_pool
        );

        // set_nlaunched last, because workers wait for this to be set
        coordination.set_target_segment_pool(target_segment_pool);
        coordination.set_nlaunched(nlaunched_plus_leader);

        let (leader_tuples, leader_segments) = if unsafe { pg_sys::parallel_leader_participation } {
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
            worker.do_build()?
        } else {
            pgrx::debug1!("build_index: leader is not participating");
            (0.0, HashSet::default())
        };

        // wait for the workers to finish by collecting all their response messages
        let mut total_tuples = leader_tuples;
        let mut segment_ids = leader_segments;
        for (_, message) in process {
            check_for_interrupts!();
            let worker_response = serde_json::from_slice::<WorkerResponse>(&message)?;

            total_tuples += worker_response.reltuples;
            segment_ids.extend(worker_response.segment_ids);
        }

        Ok((total_tuples, segment_ids))
    } else {
        pgrx::debug1!("build_index: not doing a parallel build");
        // not doing a parallel build, so directly instantiate a BuildWorker and serially run the
        // whole build here in this connected backend
        let heaprelid = heaprel.oid();
        let indexrelid = indexrel.oid();

        let mut coordination: WorkerCoordination = Default::default();
        coordination.set_target_segment_pool(vec![adjusted_target_segment_count(&heaprel)]);
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

    // ensure that we never have more workers (including the leader) than both the target segment count
    // and max allowed number of workers
    let mut nworkers = maintenance_workers
        .min(target_segment_count)
        .min(MAX_CREATE_INDEX_WORKERS);

    pgrx::debug1!("create_index_nworkers: maintenance_workers: {maintenance_workers}, target_segment_count: {target_segment_count}, nworkers: {nworkers}");

    // round down nworkers such that target_segment_count / nworkers is an integer
    // for instance, if the target is 32 and nworkers is 10, we round down to 8
    // this way, each worker builds the same number of segments, making them more evenly distributed
    for i in (1..nworkers + 1).rev() {
        if target_segment_count % i == 0 {
            nworkers = i;
            break;
        }
    }

    pgrx::debug1!("create_index_nworkers: nworkers rounded down to {nworkers}");

    // if the leader is participating, create one less worker because the leader also counts as a worker
    if unsafe { pg_sys::parallel_leader_participation } {
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
