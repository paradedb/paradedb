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
    chunk_range, ParallelProcess, ParallelState, ParallelStateManager, ParallelStateType,
    ParallelWorker, WorkerStyle,
};
use crate::postgres::insert::garbage_collect_index;
use crate::postgres::ps_status::{
    set_ps_display_remove_suffix, set_ps_display_suffix, INDEXING, MERGING,
};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::spinlock::Spinlock;
use crate::postgres::storage::buffer::BufferManager;
use crate::postgres::utils::{categorize_fields, row_to_search_document, CategorizedFieldData};
use crate::schema::{SearchField, SearchIndexSchema};
use pgrx::pg_sys::panic::ErrorReport;
use pgrx::{
    check_for_interrupts, function_name, pg_guard, pg_sys, PgLogLevel, PgMemoryContexts,
    PgSqlErrorCode,
};
use std::num::NonZeroUsize;
use std::ptr::{addr_of_mut, NonNull};
use std::sync::OnceLock;
use tantivy::{SegmentMeta, TantivyDocument};

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
    nlaunched: usize,
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
        heaprel: &PgSearchRelation,
        indexrel: &PgSearchRelation,
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
    nmerges: usize,
}

struct BuildWorker<'a> {
    config: WorkerConfig,
    table_scan_desc: Option<NonNull<pg_sys::TableScanDescData>>,
    coordination: &'a mut WorkerCoordination,
    heaprel: crate::postgres::rel::PgSearchRelation,
    indexrel: crate::postgres::rel::PgSearchRelation,
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

            let heaprel =
                (PgSearchRelation::with_lock(config.heaprelid, heap_lock as pg_sys::LOCKMODE));
            let indexrel =
                (PgSearchRelation::with_lock(config.indexrelid, index_lock as pg_sys::LOCKMODE));
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

    fn run(mut self, mq_sender: &MessageQueueSender, worker_number: i32) -> anyhow::Result<()> {
        // wait for the leader to tell us how many total workers have been launched
        while self.coordination.nlaunched() == 0 {
            check_for_interrupts!();
            std::thread::yield_now();
        }

        // communicate to the group that we've started
        self.coordination.inc_nstarted();

        let (reltuples, nmerges) = self.do_build(worker_number)?;
        Ok(mq_sender.send(serde_json::to_vec(&WorkerResponse { reltuples, nmerges })?)?)
    }
}

impl<'a> BuildWorker<'a> {
    fn new(
        heaprel: &crate::postgres::rel::PgSearchRelation,
        indexrel: &crate::postgres::rel::PgSearchRelation,
        config: WorkerConfig,
        coordination: &'a mut WorkerCoordination,
    ) -> Self {
        Self {
            config,
            table_scan_desc: None,
            heaprel: Clone::clone(heaprel),
            indexrel: Clone::clone(indexrel),
            coordination,
        }
    }

    fn do_build(&mut self, worker_number: i32) -> anyhow::Result<(f64, usize)> {
        unsafe {
            let index_info = pg_sys::BuildIndexInfo(self.indexrel.as_ptr());
            (*index_info).ii_Concurrent = self.config.concurrent;
            let nlaunched = self.coordination.nlaunched();
            let per_worker_memory_budget =
                gucs::adjust_maintenance_work_mem(nlaunched).get() / nlaunched;
            let target_segment_count = plan::adjusted_target_segment_count(&self.heaprel);
            let (_, worker_segment_target) =
                chunk_range(target_segment_count, nlaunched, worker_number as usize);

            pgrx::debug1!("build_worker {worker_number}: target_segment_count: {target_segment_count}, nlaunched: {nlaunched}, worker_segment_target: {worker_segment_target}");

            let mut build_state = WorkerBuildState::new(
                &self.heaprel,
                &self.indexrel,
                NonZeroUsize::new(per_worker_memory_budget)
                    .expect("per worker memory budget should be non-zero"),
                worker_segment_target.max(1),
                nlaunched,
                worker_number,
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
            Ok((reltuples as f64, build_state.nmerges))
        }
    }
}

/// Internal state used by each parallel build worker
struct WorkerBuildState {
    writer: Option<SerialIndexWriter>,
    categorized_fields: Vec<(SearchField, CategorizedFieldData)>,
    key_field_name: FieldName,
    per_row_context: PgMemoryContexts,
    indexrel: crate::postgres::rel::PgSearchRelation,
    heaprel: crate::postgres::rel::PgSearchRelation,
    // the following statistics are used to determine when and what to merge:
    //
    // 1. how many segments does this worker expect to make, assuming no merges?
    estimated_nsegments: OnceLock<usize>,
    //
    // 2. how many segments is this worker supposed to make? (assigned by the leader)
    worker_segment_target: usize,
    //
    // 3. how many merges has this worker done so far? (incrementing counter)
    nmerges: usize,
    //
    // 4. how many workers are there in total? (including the leader)
    nlaunched: usize,
    //
    // 5. unmerged segment metas that this worker has created so far
    unmerged_metas: Vec<SegmentMeta>,
}

impl WorkerBuildState {
    pub fn new(
        heaprel: &crate::postgres::rel::PgSearchRelation,
        indexrel: &crate::postgres::rel::PgSearchRelation,
        per_worker_memory_budget: NonZeroUsize,
        worker_segment_target: usize,
        nlaunched: usize,
        worker_number: i32,
    ) -> anyhow::Result<Self> {
        // if we're making more than one segment, do an early cutoff based on doc count in case
        // the memory budget is so high that all the docs fit into one segment
        let max_docs_per_segment = if worker_segment_target > 1 {
            Some(
                plan::estimate_heap_reltuples(heaprel) as u32
                    / nlaunched as u32
                    / worker_segment_target as u32,
            )
        } else {
            None
        };
        let config = IndexWriterConfig {
            memory_budget: per_worker_memory_budget,
            max_docs_per_segment,
        };
        let writer = SerialIndexWriter::open(indexrel, config, worker_number)?;
        let schema = SearchIndexSchema::open(indexrel)?;
        let categorized_fields = categorize_fields(indexrel, &schema);
        let key_field_name = schema.key_field().field_name();
        Ok(Self {
            writer: Some(writer),
            categorized_fields,
            key_field_name,
            per_row_context: PgMemoryContexts::new("pg_search ambuild context"),
            indexrel: Clone::clone(indexrel),
            heaprel: Clone::clone(heaprel),
            worker_segment_target,
            nlaunched,
            estimated_nsegments: OnceLock::new(),
            nmerges: Default::default(),
            unmerged_metas: Default::default(),
        })
    }

    fn commit(&mut self) -> anyhow::Result<()> {
        let writer = self.writer.take().expect("writer should be set");
        if let Some((segment_meta, _)) = writer.commit()? {
            self.unmerged_metas.push(segment_meta);
            self.try_merge(true)?;
        }

        unsafe { set_ps_display_remove_suffix() };
        Ok(())
    }

    /// Based on our calculated chunk size, merge down a chunk of segments into a single segment
    /// if we have created at least that many segments.
    fn try_merge(&mut self, is_last_merge: bool) -> anyhow::Result<()> {
        // which segments should me merge together? if there's not enough, return early
        let segment_ids_to_merge = {
            if self.unmerged_metas.is_empty() {
                return Ok(());
            }

            let chunk_size = if !is_last_merge {
                // calculate the chunk size for this merge iteration
                //
                // chunk_range gives us chunks with the larger ones at the front
                // we want the larger ones at the back, because the smallest "straggler" segment will be written last
                //
                // for instance, imagine we have 3 segments of size [100, 100, 5]
                // we would want the chunks to be [1,2] (merging together [100, 5]) and not [2,1] (merging together [100, 100])
                let (_, chunk_size) = chunk_range(
                    self.estimated_nsegments(self.unmerged_metas[0].max_doc()),
                    self.worker_segment_target,
                    // this achieves the effect of reversing the chunks
                    (self.worker_segment_target - self.nmerges).max(0),
                );

                if chunk_size <= 1 || self.unmerged_metas.len() < chunk_size {
                    pgrx::debug1!(
                        "try_merge: skipping merge because chunk_size: {chunk_size}, unmerged_metas: {:?}",
                        self.unmerged_metas.len()
                    );
                    return Ok(());
                }

                if self.nmerges == self.worker_segment_target - 1 {
                    pgrx::debug1!("try_merge: skipping merge because this is not the last merge, and we can only do one more");
                    return Ok(());
                }

                chunk_size
            } else {
                // if it's the last merge, ignore the chunk size and instead solve the following equation for chunk_size:
                //
                // worker_segment_target = self.nmerges + unmerged_segments_len - chunk_size + 1
                //
                // this guarantees we hit the target segment count exactly, assuming we haven't already exceeded it
                // convert to i32 because it's possible that chunk_size comes out negative
                let chunk_size: i32 = self.nmerges as i32 - self.worker_segment_target as i32
                    + self.unmerged_metas.len() as i32
                    + 1;
                if chunk_size <= 1 || self.unmerged_metas.len() < chunk_size as usize {
                    return Ok(());
                }

                chunk_size as usize
            };

            self.unmerged_metas.sort_by_key(|entry| entry.max_doc());
            self.unmerged_metas
                .drain(..chunk_size)
                .map(|entry| entry.id())
                .collect::<Vec<_>>()
        };

        unsafe { set_ps_display_suffix(MERGING.as_ptr()) };

        // do the merge
        pgrx::debug1!(
            "do_merge: last merge {}, about to merge {} segments: {:?}",
            is_last_merge,
            segment_ids_to_merge.len(),
            segment_ids_to_merge
        );
        let directory = MVCCDirectory::mergeable(&self.indexrel);
        let mut merger = SearchIndexMerger::open(directory)?;
        merger.merge_segments(&segment_ids_to_merge)?;

        // garbage collect the index, returning to the fsm
        pgrx::debug1!("do_merge: garbage collecting");
        unsafe { garbage_collect_index(&self.indexrel) };

        self.nmerges += 1;

        Ok(())
    }

    /// Estimates how many segments this worker will make if no merging happens.
    ///
    /// This is used to determine how many segments to merge down in chunks.
    fn estimated_nsegments(&self, docs_per_segment: u32) -> usize {
        *self.estimated_nsegments.get_or_init(|| {
            let reltuples = plan::estimate_heap_reltuples(&self.heaprel);
            let reltuples_per_worker = reltuples / self.nlaunched as f64;
            let nsegments = (reltuples_per_worker / docs_per_segment as f64).ceil() as usize;
            pgrx::debug1!("estimated that this worker will make {nsegments} segments, based on reltuples: {reltuples}, nlaunched: {}, reltuples_per_worker: {reltuples_per_worker}, docs_per_segment: {docs_per_segment}", self.nlaunched);
            nsegments
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
    set_ps_display_suffix(INDEXING.as_ptr());

    let build_state = &mut *state.cast::<WorkerBuildState>();
    let ctid_u64 = crate::postgres::utils::item_pointer_to_u64(*ctid);

    let segment_meta = build_state.per_row_context.switch_to(|_| {
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

    if let Some(segment_meta) = segment_meta {
        build_state.unmerged_metas.push(segment_meta);
        build_state
            .try_merge(false)
            .unwrap_or_else(|e| panic!("{e}"));
    }
}

/// Build an index.  This is the workhorse behind `CREATE INDEX` and `REINDEX`.
///
/// If the system allows, it will build the index in parallel.  Otherwise the index is built in
/// serially in this connected backend.
pub(super) fn build_index(
    heaprel: crate::postgres::rel::PgSearchRelation,
    indexrel: crate::postgres::rel::PgSearchRelation,
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
    let nworkers = plan::create_index_nworkers(&heaprel);
    pgrx::debug1!("build_index: asked for {nworkers} workers");

    let total_tuples = if let Some(mut process) = launch_parallel_process!(
        ParallelBuild<BuildWorker>,
        process,
        WorkerStyle::Maintenance,
        nworkers,
        1024
    ) {
        let nlaunched = process.launched_workers();
        pgrx::debug1!("build_index: launched {nworkers} workers (not including leader)");
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
        coordination.set_nlaunched(nlaunched_plus_leader);
        pgrx::debug1!("build_index: has {nlaunched_plus_leader} workers (including leader)");

        let (mut total_tuples, mut total_merges) =
            if unsafe { pg_sys::parallel_leader_participation } {
                // if the leader is to participate too, it's nice for it to wait until all the other workers
                // have indicated that they're running.  Otherwise, it's likely the leader will get ahead
                // of the workers, which doesn't allow for "evenly" distributing the work
                while coordination.nstarted() != nlaunched {
                    check_for_interrupts!();
                    std::thread::yield_now();
                }

                // directly instantiate a worker for the leader and have it do its build
                let mut worker = BuildWorker::new_parallel_worker(*process.state_manager());
                worker.do_build(nlaunched_plus_leader as i32)?
            } else {
                pgrx::debug1!("build_index: leader is not participating");
                (0.0, 0)
            };

        // wait for the workers to finish by collecting all their response messages
        for (_, message) in process {
            check_for_interrupts!();
            let worker_response = serde_json::from_slice::<WorkerResponse>(&message)?;
            total_tuples += worker_response.reltuples;
            total_merges += worker_response.nmerges;
        }

        pgrx::debug1!("build_index: total_tuples: {total_tuples}, total_merges: {total_merges}");
        total_tuples
    } else {
        pgrx::debug1!("build_index: not doing a parallel build");
        // not doing a parallel build, so directly instantiate a BuildWorker and serially run the
        // whole build here in this connected backend
        let heaprelid = heaprel.oid();
        let indexrelid = indexrel.oid();

        let mut coordination: WorkerCoordination = Default::default();
        coordination.set_nlaunched(1);

        let mut worker = BuildWorker::new(
            &heaprel,
            &indexrel,
            WorkerConfig {
                heaprelid,
                indexrelid,
                concurrent,
            },
            &mut coordination,
        );

        let (total_tuples, total_merges) = worker.do_build(1)?;
        pgrx::debug1!("build_index: total_tuples: {total_tuples}, total_merges: {total_merges}");
        total_tuples
    };

    unsafe { set_ps_display_remove_suffix() };
    Ok(total_tuples)
}

mod plan {
    use super::*;
    /// Determine the number of workers to use for a given CREATE INDEX/REINDEX statement.
    ///
    /// The number of workers is determined by max_parallel_maintenance_workers. However, if max_parallel_maintenance_workers
    /// is greater than available parallelism, we use available parallelism.
    ///
    /// If the leader is participating, we subtract 1 from the number of workers because the leader also counts as a worker.
    pub(super) fn create_index_nworkers(heaprel: &crate::postgres::rel::PgSearchRelation) -> usize {
        // We don't want a parallel build to happen if we're creating a single segment
        let target_segment_count = plan::adjusted_target_segment_count(heaprel);
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

        // must also be less than max_parallel_workers and max_worker_processes
        let maintenance_workers = maintenance_workers
            .min(unsafe { pg_sys::max_parallel_workers as usize })
            .min(unsafe { pg_sys::max_worker_processes as usize });

        if maintenance_workers < 3 {
            ErrorReport::new(
                PgSqlErrorCode::ERRCODE_INSUFFICIENT_RESOURCES,
                format!("only {maintenance_workers} parallel workers were available for index build"),
                function_name!(),
            )
            .set_detail("for large tables, increasing the number of workers can reduce the time it takes to build the index")
            .set_hint("`SET max_parallel_maintenance_workers = <number>`")
            .report(PgLogLevel::WARNING);
        }

        if maintenance_workers == 0 {
            return 0;
        }

        // Ensure that we never have more workers (including the leader) than the max allowed number of workers.
        //
        // We also want nworkers to be at most 1/2 of the target segment count. To illustrate why:
        //
        // Imagine we have 8 workers, a target segment count of 8, and a table size such that each worker produces 4 segments.
        // In this scenario, each worker would do one big merge of all 4 segments at the very end, which means none of the
        // merges would be able to reuse the FSM.
        //
        // On the other hand, imagine we have only 4 workers, over the same table and target segment count.
        // In this scenario, each worker would target 2 segments, meaning it would do 2 merges -- once when it's about halfway done
        // and once at the end. The merge at the end would be able to use the free space created by the first merge.
        let max_workers = target_segment_count.div_ceil(2);
        let mut nworkers = maintenance_workers.min(max_workers);

        if unsafe { pg_sys::parallel_leader_participation } && nworkers == max_workers {
            nworkers -= 1;
        }

        nworkers
    }

    /// If we determine that the table is very small, we should just create a single segment
    pub(super) fn adjusted_target_segment_count(heaprel: &PgSearchRelation) -> usize {
        // If there are fewer rows than number of CPUs, use 1 worker
        let reltuples = plan::estimate_heap_reltuples(heaprel);
        let target_segment_count = gucs::target_segment_count();
        if reltuples <= target_segment_count as f64 {
            pgrx::debug1!("number of reltuples ({reltuples}) is less than target segment count ({target_segment_count}), creating a single segment");
            return 1;
        }

        // If the entire heap fits inside the smallest allowed Tantivy segment memory budget of 15MB, use 1 worker
        let byte_size = plan::estimate_heap_byte_size(heaprel);
        if byte_size <= 15 * 1024 * 1024 {
            pgrx::debug1!(
                "heap byte size ({byte_size}) is less than 15MB, creating a single segment"
            );
            return 1;
        }

        target_segment_count
    }

    pub(super) fn estimate_heap_reltuples(heap_relation: &PgSearchRelation) -> f64 {
        let mut reltuples = unsafe { (*heap_relation.rd_rel).reltuples };

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

            let bman = BufferManager::new(heap_relation);
            let buffer = bman.get_buffer(0);
            let page = buffer.page();
            let max_offset = page.max_offset_number();
            reltuples = npages as f32 * max_offset as f32;
        }

        reltuples as f64
    }

    pub(super) fn estimate_heap_byte_size(heap_relation: &PgSearchRelation) -> usize {
        let npages = unsafe {
            pg_sys::RelationGetNumberOfBlocksInFork(
                heap_relation.as_ptr(),
                pg_sys::ForkNumber::MAIN_FORKNUM,
            )
        };

        npages as usize * pg_sys::BLCKSZ as usize
    }
}
