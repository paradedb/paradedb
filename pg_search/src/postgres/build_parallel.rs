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
use crate::index::writer::index::SerialIndexWriter;
use crate::index::WriterResources;
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
use std::ptr::{addr_of_mut, NonNull};
use tantivy::TantivyDocument;

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
    nstarted: usize,
}
impl ParallelStateType for WorkerCoordination {}
impl WorkerCoordination {
    fn inc_nlaunched(&mut self) {
        let _lock = self.mutex.acquire();
        self.nstarted += 1;
    }
    fn nlaunched(&mut self) -> usize {
        let _lock = self.mutex.acquire();
        self.nstarted
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
        self.coordination.inc_nlaunched();
        let reltuples = self.do_build()?;
        let response = WorkerResponse { reltuples };

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

    fn do_build(&mut self) -> anyhow::Result<f64> {
        unsafe {
            let index_info = pg_sys::BuildIndexInfo(self.indexrel.as_ptr());
            (*index_info).ii_Concurrent = self.config.concurrent;

            let mut build_state = WorkerBuildState::new(&self.indexrel)?;

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

            build_state.writer.commit()?;
            Ok(reltuples)
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
    pub fn new(indexrel: &PgRelation) -> anyhow::Result<Self> {
        let (parallelism, memory_budget) = WriterResources::CreateIndex.resources();
        let memory_budget = memory_budget / parallelism;
        let writer = SerialIndexWriter::open(
            indexrel,
            memory_budget,
            Some(Self::target_docs_per_segment(indexrel)),
        )?;
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

    fn target_docs_per_segment(indexrel: &PgRelation) -> usize {
        let desired_segment_count = std::thread::available_parallelism()
            .expect("your computer should have at least one core");
        let heap_relation = indexrel.heap_relation().unwrap();
        let mut reltuples = heap_relation.reltuples().unwrap_or_default();

        // If the reltuples estimate is not available, estimate the number of tuples in the heap
        // by multiplying the number of pages by the max offset number of the first page
        if reltuples <= 0.0 {
            let bman = BufferManager::new(heap_relation.oid());
            let buffer = bman.get_buffer(0);
            let page = buffer.page();
            let max_offset = page.max_offset_number();
            let npages = unsafe {
                pg_sys::RelationGetNumberOfBlocksInFork(
                    heap_relation.as_ptr(),
                    pg_sys::ForkNumber::MAIN_FORKNUM,
                )
            };
            reltuples = npages as f32 * max_offset as f32;
        }

        (reltuples as f64 / desired_segment_count.get() as f64).ceil() as usize
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
    let nworkers = calculate_nworkers(&heaprel);

    if let Some(mut process) = launch_parallel_process!(
        ParallelBuild<BuildWorker>,
        process,
        WorkerStyle::Maintenance,
        nworkers,
        1024
    ) {
        let leader_tuples = if unsafe { pg_sys::parallel_leader_participation } {
            // if the leader is to participate too, it's nice for it to wait until all the other workers
            // have indicated that they're running.  Otherwise, it's likely the leader will get ahead
            // of the workers, which doesn't allow for "evenly" distributing the work
            let nlaunched = process.launched_workers();
            let coordination = process
                .state_manager_mut()
                .object::<WorkerCoordination>(2)
                .expect("process coordination")
                .expect("process coordination should not be NULL");

            while coordination.nlaunched() != nlaunched {
                check_for_interrupts!();
                std::thread::yield_now();
            }

            // directly instantiate a worker for the leader and have it do its build
            let mut worker = BuildWorker::new_parallel_worker(*process.state_manager());
            worker.do_build()?
        } else {
            0.0
        };

        // wait for the workers to finish by collecting all their response messages
        let mut total_tuples = leader_tuples;
        for (_, message) in process {
            check_for_interrupts!();
            let worker_response = serde_json::from_slice::<WorkerResponse>(&message)?;

            total_tuples += worker_response.reltuples;
        }

        Ok(total_tuples)
    } else {
        // not doing a parallel build, so directly instantiate a BuildWorker and serially run the
        // whole build here in this connected backend
        let heaprelid = heaprel.oid();
        let indexrelid = indexrel.oid();
        let mut coordination = Default::default();
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

fn calculate_nworkers(heaprel: &PgRelation) -> usize {
    // NB: we _could_ use pg_sys::plan_create_index_workers(), or on v17+ accept IndexIndex::ii_ParallelWorkers,
    // but doing either of these would prohibit the user from having direct control over the number of
    // workers used for a given CREATE INDEX/REINDEX statement.  Internal discussions led to that
    // being more important that us trying to be "smart"
    unsafe {
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
    }
}
