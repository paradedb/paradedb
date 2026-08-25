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

use crate::api::version::Version;
use crate::gucs;
use crate::index::kdtree::KdTree;
use crate::index::mvcc::MvccSatisfies;
use crate::index::stats::{LogicalBounds, LogicalBoundsByField};
use crate::index::writer::index::{
    DiskSpaceGuard, IndexWriterConfig, Mergeable, SearchIndexMerger, SerialIndexWriter,
};
use crate::launch_parallel_process;
use crate::parallel_worker::builder::ParallelProcessLauncher;
use crate::parallel_worker::mqueue::MessageQueueSender;
use crate::parallel_worker::{
    ParallelProcess, ParallelState, ParallelStateManager, ParallelStateType, ParallelWorker,
    WorkerStyle, chunk_range,
};
use crate::postgres::build_partitioning::plan_partition_boundaries;
use crate::postgres::composite::CompositeSlotValues;
use crate::postgres::heap::{ExpressionState, HeapDocFetcher, HeapFetchState, PrefetchWindow};
use crate::postgres::locks::Spinlock;
use crate::postgres::merge::garbage_collect_index;
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::postgres::ps_status::{
    COMMITTING, FINALIZING, GARBAGE_COLLECTING, INDEXING, MERGING, set_ps_display_remove_suffix,
    set_ps_display_suffix,
};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::buffer::BufferManager;
use crate::postgres::storage::metadata::MetaPage;
use crate::postgres::tuplesort::Int8Sorter;
use crate::postgres::utils::{
    collect_composites_for_unpacking, get_field_value, row_to_search_document,
    scalar_datum_to_tantivy_value, unwrap_alias_datum,
};
use crate::schema::{CategorizedFieldData, SearchField};
use pgrx::pg_sys::panic::ErrorReport;
use pgrx::{
    PgLogLevel, PgMemoryContexts, PgSqlErrorCode, PgTupleDesc, check_for_interrupts, function_name,
    pg_guard, pg_sys,
};
use std::ffi::CString;
use std::num::NonZeroUsize;
use std::os::raw::c_int;
use std::ptr::{NonNull, addr_of_mut};
use std::sync::{Arc, OnceLock};
use std::time::Instant;
use tantivy::index::SegmentId;
use tantivy::{SegmentMeta, TantivyDocument};

const TUPLES_DONE_BATCH_SIZE: usize = 5;
/// General, immutable configuration used for the workers
#[derive(Copy, Clone, Debug)]
#[repr(C)]
struct WorkerConfig {
    heaprelid: pg_sys::Oid,
    indexrelid: pg_sys::Oid,
    concurrent: bool,
    current_xid: pg_sys::FullTransactionId,
    need_wal: bool,
    next_xid: pg_sys::FullTransactionId,
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
    ntuples_done: usize,
    nsegments_written: usize,
    /// Participants whose partitioned-build scan is over and whose spill files are closed.
    nspilled: usize,
}

impl ParallelStateType for WorkerCoordination {}
impl ParallelStateType for pg_sys::SharedFileSet {}
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
    fn add_tuples_done(&mut self, count: usize) {
        let _lock = self.mutex.acquire();
        self.ntuples_done += count;
    }
    fn tuples_done(&mut self) -> usize {
        let _lock = self.mutex.acquire();
        self.ntuples_done
    }
    fn add_segments_written(&mut self) -> usize {
        let _lock = self.mutex.acquire();
        self.nsegments_written += 1;
        self.nsegments_written
    }
    fn nsegments_written(&mut self) -> usize {
        let _lock = self.mutex.acquire();
        self.nsegments_written
    }
    fn add_spilled(&mut self) {
        let _lock = self.mutex.acquire();
        self.nspilled += 1;
    }
    fn nspilled(&mut self) -> usize {
        let _lock = self.mutex.acquire();
        self.nspilled
    }
}

/// The parallel process for setting up a parallel index build
struct ParallelBuild {
    config: WorkerConfig,
    scandesc: ScanDesc,
    coordination: WorkerCoordination,
    /// The leader's global partition boundaries, serialized with `postcard`. Empty when the
    /// index has no `partition_by`.
    partitioning: Vec<u8>,
    /// The fileset holding a partitioned build's per-partition ctid spills. Only reserved here: the
    /// leader initializes it in place once the DSM is mapped, since its cleanup hooks onto the
    /// segment. Untouched without `partition_by`.
    fileset: pg_sys::SharedFileSet,
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
        snapshot: pg_sys::Snapshot,
        config: WorkerConfig,
        partitioning: Vec<u8>,
    ) -> Self {
        let scandesc = unsafe {
            let size = size_of::<pg_sys::ParallelTableScanDescData>()
                + pg_sys::table_parallelscan_estimate(heaprel.as_ptr(), snapshot) as usize;
            let scandesc = pg_sys::palloc0(size).cast();
            pg_sys::table_parallelscan_initialize(heaprel.as_ptr(), scandesc, snapshot);
            (size, scandesc)
        };
        Self {
            config,
            scandesc,
            coordination: Default::default(),
            partitioning,
            fileset: Default::default(),
        }
    }
}

/// Index of the [`ParallelBuild::fileset`] entry in the parallel state.
const FILESET_STATE_IDX: usize = 4;

impl ParallelProcess for ParallelBuild {
    fn state_values(&self) -> Vec<&dyn ParallelState> {
        vec![
            &self.config,
            &self.scandesc,
            &self.coordination,
            &self.partitioning,
            &self.fileset,
        ]
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct WorkerResponse {
    reltuples: f64,
    nmerges: usize,
}

fn deserialize_partitioning(bytes: &[u8]) -> Option<KdTree> {
    if bytes.is_empty() {
        return None;
    }
    Some(
        postcard::from_bytes(bytes)
            .unwrap_or_else(|e| panic!("could not deserialize partition boundaries: {e}")),
    )
}

struct BuildWorker<'a> {
    config: WorkerConfig,
    table_scan_desc: Option<NonNull<pg_sys::TableScanDescData>>,
    coordination: &'a mut WorkerCoordination,
    heaprel: PgSearchRelation,
    indexrel: PgSearchRelation,
    /// Global partition boundaries computed by the leader; `None` without `partition_by`.
    partitioning: Option<KdTree>,
    /// The spill fileset of a parallel partitioned build, in the DSM. Null on the serial path.
    fileset: *mut pg_sys::SharedFileSet,
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
            .expect("ParallelTableScanDesc should not be NULL");
        let coordination = state_manager
            .object::<WorkerCoordination>(2)
            .expect("should be able to get WorkerCoordination")
            .expect("WorkerCoordination should not be NULL");
        let partitioning = state_manager
            .slice::<u8>(3)
            .expect("should be able to get partitioning bytes")
            .expect("partitioning bytes should not be NULL");
        let partitioning = deserialize_partitioning(partitioning);
        let fileset = state_manager
            .object::<pg_sys::SharedFileSet>(FILESET_STATE_IDX)
            .expect("should be able to get the spill fileset")
            .expect("spill fileset should not be NULL");

        unsafe {
            // A worker attaches so that the last process out deletes the spill files. On an
            // error the leader detaches before its workers have exited, and a worker still
            // in its scan could recreate a partition file after the leader's cleanup ran. The
            // participating leader is the fileset's creator and holds the first reference.
            if partitioning.is_some() && pg_sys::ParallelWorkerNumber >= 0 {
                pg_sys::SharedFileSetAttach(fileset, state_manager.dsm_segment());
            }

            let (heap_lock, index_lock) = if !config.concurrent {
                (pg_sys::ShareLock, pg_sys::AccessExclusiveLock)
            } else {
                (pg_sys::ShareUpdateExclusiveLock, pg_sys::RowExclusiveLock)
            };

            let heaprel =
                PgSearchRelation::with_lock(config.heaprelid, heap_lock as pg_sys::LOCKMODE);
            let mut indexrel =
                PgSearchRelation::with_lock(config.indexrelid, index_lock as pg_sys::LOCKMODE);
            let table_scan_desc = pg_sys::table_beginscan_parallel(heaprel.as_ptr(), scandesc);

            indexrel.set_is_create_index();
            indexrel.set_need_wal(config.need_wal);

            Self {
                config: *config,
                table_scan_desc: NonNull::new(table_scan_desc),
                coordination,
                heaprel,
                indexrel,
                partitioning,
                fileset,
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

        let (reltuples, nmerges) = self.do_build(worker_number, false)?;
        Ok(mq_sender.send(serde_json::to_vec(&WorkerResponse { reltuples, nmerges })?)?)
    }
}

impl<'a> BuildWorker<'a> {
    fn new(
        heaprel: &PgSearchRelation,
        indexrel: &PgSearchRelation,
        config: WorkerConfig,
        coordination: &'a mut WorkerCoordination,
        partitioning: Option<KdTree>,
        fileset: *mut pg_sys::SharedFileSet,
    ) -> Self {
        Self {
            config,
            table_scan_desc: None,
            heaprel: Clone::clone(heaprel),
            indexrel: Clone::clone(indexrel),
            coordination,
            partitioning,
            fileset,
        }
    }

    fn do_build(&mut self, worker_number: i32, is_leader: bool) -> anyhow::Result<(f64, usize)> {
        unsafe {
            let index_info = self.indexrel.index_info();
            (*index_info).ii_Concurrent = self.config.concurrent;
            let nlaunched = self.coordination.nlaunched();
            let per_worker_memory_budget =
                gucs::adjust_maintenance_work_mem(nlaunched).get() / nlaunched;
            let target_segment_count =
                plan::adjusted_target_segment_count(&self.heaprel, &self.indexrel);
            let (_, worker_segment_target) =
                chunk_range(target_segment_count, nlaunched, worker_number as usize);
            // Workers are numbered from 0 and a participating leader runs as one more, so the
            // leader takes the last participant slot.
            let participant = if is_leader {
                nlaunched - 1
            } else {
                worker_number as usize
            };

            pgrx::debug1!(
                "build_worker {worker_number}: target_segment_count: {target_segment_count}, nlaunched: {nlaunched}, worker_segment_target: {worker_segment_target}"
            );

            // `build_index` plans boundaries only for a non-concurrent build, so this is `None`
            // under CONCURRENTLY and the worker takes the regular per-row path.
            let partitioning = self.partitioning.take();
            if let Some(partitioning) = &partitioning {
                pgrx::debug1!("build_worker {worker_number}: partition boundaries: {partitioning}");
            }

            let mut build_state = WorkerBuildState::new(
                &self.heaprel,
                &self.indexrel,
                NonZeroUsize::new(per_worker_memory_budget)
                    .expect("per worker memory budget should be non-zero"),
                self.config.current_xid,
                self.config.next_xid,
                worker_segment_target.max(1),
                target_segment_count,
                self.coordination,
                worker_number,
                is_leader,
                partitioning,
                PartitionSpillFiles::new(self.fileset, participant, nlaunched),
            )?;

            set_ps_display_suffix(INDEXING.as_ptr());
            let scan = Instant::now();
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

            pgrx::debug1!(
                "build_worker {worker_number}: scanned {reltuples} tuples in {:.3}s",
                scan.elapsed().as_secs_f64()
            );

            if build_state.partitioning.is_some() {
                build_state.drain_partitioned()?;
            } else {
                build_state.commit()?;
            }
            Ok((reltuples as f64, build_state.nmerges))
        }
    }
}

/// One spill record: a ctid as 8 bytes.
const CTID_RECORD_LEN: usize = 8;

fn encode_ctid_record(ctid: u64) -> [u8; CTID_RECORD_LEN] {
    ctid.to_be_bytes()
}

fn decode_ctid_record(bytes: &[u8]) -> u64 {
    u64::from_be_bytes(
        bytes
            .try_into()
            .expect("a ctid record should be CTID_RECORD_LEN bytes"),
    )
}

/// The kd-tree box of `partition`, keyed by field name, the way the `.stats` component records
/// it.
fn partition_box(tree: &KdTree, partition: usize) -> Option<Arc<LogicalBoundsByField>> {
    let bounds = tree.partition_bounds(partition)?;
    Some(Arc::new(
        tree.dims()
            .iter()
            .zip(bounds)
            .map(|(dim, (lower, upper))| (dim.as_ref().to_string(), LogicalBounds { lower, upper }))
            .collect(),
    ))
}

/// `open(2)` flag for `BufFileOpenFileSet`, and `whence` for `BufFileSeek`. pgrx binds no
/// `fcntl.h` constants; both are 0 on every platform Postgres runs on.
const O_RDONLY: c_int = 0;
const SEEK_SET: c_int = 0;

/// Where a participant's per-partition ctid spill files live.
enum SpillFileset {
    /// A serial build: temporary files this backend rewinds and reads back itself. Each is
    /// deleted on close, or by the resource owner should the build error out.
    Local,
    /// A parallel build: a fileset in the DSM that every participant writes into and the partition
    /// owners read out of. Its files go away when the leader detaches from the segment, on
    /// success or on error.
    Shared(*mut pg_sys::SharedFileSet),
}

/// The per-partition ctid spill of one build participant. Phase 1 appends each scanned row's ctid to
/// its partition's file; phase 2 hands a partition's owner every participant's file for that partition.
///
/// Every open `BufFile` carries a `BLCKSZ` buffer outside the worker budget, so phase 1 holds
/// `8KB x partitions` per participant; the bound on `target_segment_count` keeps that small.
struct PartitionSpillFiles {
    fileset: SpillFileset,
    participant: usize,
    nparticipants: usize,
    /// This participant's write handle per partition, created on the partition's first row.
    writers: Vec<Option<*mut pg_sys::BufFile>>,
    /// Bytes this participant appended per partition. A local temp file has no fileset, and
    /// `BufFileSize` asserts on one before PG18, so the serial path sizes its files from here.
    written: Vec<usize>,
}

impl PartitionSpillFiles {
    /// `fileset` is the DSM entry of a parallel build, or null for a serial one.
    fn new(fileset: *mut pg_sys::SharedFileSet, participant: usize, nparticipants: usize) -> Self {
        let fileset = if fileset.is_null() {
            SpillFileset::Local
        } else {
            SpillFileset::Shared(fileset)
        };
        Self {
            fileset,
            participant,
            nparticipants,
            writers: Vec::new(),
            written: Vec::new(),
        }
    }

    fn with_partitions(mut self, npartitions: usize) -> Self {
        self.writers = vec![None; npartitions];
        self.written = vec![0; npartitions];
        self
    }

    /// The name of `participant`'s file for `partition` inside the shared fileset.
    fn file_name(participant: usize, partition: usize) -> CString {
        CString::new(format!("p{partition}.{participant}"))
            .expect("spill file name should have no NUL")
    }

    unsafe fn append(&mut self, partition: usize, ctid: u64) {
        let writer = match self.writers[partition] {
            Some(writer) => writer,
            None => {
                let writer = match self.fileset {
                    SpillFileset::Local => pg_sys::BufFileCreateTemp(false),
                    SpillFileset::Shared(fileset) => pg_sys::BufFileCreateFileSet(
                        &raw mut (*fileset).fs,
                        Self::file_name(self.participant, partition).as_ptr(),
                    ),
                };
                self.writers[partition] = Some(writer);
                writer
            }
        };
        let mut record = encode_ctid_record(ctid);
        pg_sys::BufFileWrite(writer, record.as_mut_ptr().cast(), CTID_RECORD_LEN);
        self.written[partition] += CTID_RECORD_LEN;
    }

    /// End phase 1. Shared files are closed so that peers can open them (the fileset keeps them
    /// around); local ones stay open for [`Self::gather`] to rewind.
    unsafe fn finish_writes(&mut self) {
        if let SpillFileset::Shared(_) = self.fileset {
            for writer in self.writers.iter_mut() {
                if let Some(writer) = writer.take() {
                    pg_sys::BufFileClose(writer);
                }
            }
        }
    }

    /// Every participant's spill file for `partition`, positioned at its start, with its size in
    /// bytes.
    unsafe fn readers(&mut self, partition: usize) -> Vec<(*mut pg_sys::BufFile, usize)> {
        match self.fileset {
            SpillFileset::Local => {
                let Some(file) = self.writers[partition].take() else {
                    return Vec::new();
                };
                if pg_sys::BufFileSeek(file, 0, 0, SEEK_SET) != 0 {
                    pgrx::error!("could not rewind the spill file of partition {partition}");
                }
                vec![(file, self.written[partition])]
            }
            SpillFileset::Shared(fileset) => (0..self.nparticipants)
                .filter_map(|participant| {
                    let file = pg_sys::BufFileOpenFileSet(
                        &raw mut (*fileset).fs,
                        Self::file_name(participant, partition).as_ptr(),
                        O_RDONLY,
                        true,
                    );
                    (!file.is_null()).then(|| (file, pg_sys::BufFileSize(file).max(0) as usize))
                })
                .collect(),
        }
    }

    /// Collect `partition`'s ctids from every participant into an `int8` tuplesort on `budget`
    /// bytes, or `None` when no participant saw a row of the partition. A packed ctid is 48
    /// bits, so it orders the same as an `int8`.
    unsafe fn gather(
        &mut self,
        partition: usize,
        budget: usize,
    ) -> anyhow::Result<Option<Int8Sorter>> {
        let readers = self.readers(partition);
        if readers.is_empty() {
            return Ok(None);
        }
        let total_bytes = readers.iter().map(|&(_, bytes)| bytes).sum::<usize>();
        pgrx::debug1!(
            "partition {partition}: {} of {} participants spilled rows, {total_bytes} bytes of ctids",
            readers.len(),
            self.nparticipants
        );
        let mut sorter = Int8Sorter::new(budget);
        for (file, _) in readers {
            read_ctid_records(file, |record| sorter.put(decode_ctid_record(record) as i64))?;
            pg_sys::BufFileClose(file);
        }
        sorter.performsort();
        Ok(Some(sorter))
    }
}

/// Stream `file` from its current position to the end, one fixed-length ctid record at a time.
unsafe fn read_ctid_records(
    file: *mut pg_sys::BufFile,
    mut sink: impl FnMut(&[u8]),
) -> anyhow::Result<()> {
    let mut buf = [0u8; 1024 * CTID_RECORD_LEN];
    loop {
        let nread = pg_sys::BufFileRead(file, buf.as_mut_ptr().cast(), buf.len());
        if nread == 0 {
            return Ok(());
        }
        if !nread.is_multiple_of(CTID_RECORD_LEN) {
            anyhow::bail!("partition spill file holds a partial ctid record ({nread} bytes read)");
        }
        buf[..nread]
            .chunks_exact(CTID_RECORD_LEN)
            .for_each(&mut sink);
    }
}

/// The share of a worker's budget the partitioned drain gives the sort of one partition's ctids. It
/// never pushes the writer under the floor that `adjust_maintenance_work_mem` promised; at the
/// floor the ctids just sort on disk.
fn gather_budget(per_worker_budget: NonZeroUsize) -> usize {
    let budget = per_worker_budget.get();
    (budget / 8)
        .min(budget.saturating_sub(gucs::limits::MEMORY_BUDGET_NUM_BYTES_MIN))
        .max(1)
}

/// Phase-1 state for a partitioned build: rows are not indexed during the scan. Each row is
/// routed to a partition by the leader's kd-tree boundaries and its ctid appended to the
/// partition's spill file. Phase 2 assigns every partition to one participant, which drains it from all
/// participants' files, so a partition ends up as one segment however many workers scanned it.
struct PartitionSpill {
    tree: KdTree,
    /// The `partition_by` fields in `tree.dims()` order, resolved to the index's categorized
    /// fields so the callback can project each scanned row onto them.
    dim_fields: Vec<(SearchField, CategorizedFieldData)>,
    files: PartitionSpillFiles,
    /// A key's encoding can depend on the version that created the index, so routing has to
    /// convert values the way the document build does.
    created_by_version: Option<Version>,
}

impl PartitionSpill {
    fn new(
        tree: KdTree,
        categorized_fields: &[(SearchField, CategorizedFieldData)],
        files: PartitionSpillFiles,
        created_by_version: Option<Version>,
    ) -> anyhow::Result<Self> {
        let dim_fields = tree
            .dims()
            .iter()
            .map(|dim| {
                categorized_fields
                    .iter()
                    .find(|(field, _)| field.field_name() == dim)
                    .cloned()
                    .ok_or_else(|| {
                        anyhow::anyhow!("partition_by field `{dim}` is not an indexed field")
                    })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        let files = files.with_partitions(tree.partition_count());
        Ok(Self {
            tree,
            dim_fields,
            files,
            created_by_version,
        })
    }

    /// Route one scanned row to its partition by projecting it onto the `partition_by` dimensions and
    /// consulting the kd-tree. `unpacked_composites` must be built over the full field set so a
    /// composite `partition_by` field resolves the same way the document build would.
    unsafe fn route(
        &self,
        values: *mut pg_sys::Datum,
        isnull: *mut bool,
        unpacked_composites: &CompositeSlotValues,
    ) -> anyhow::Result<usize> {
        let point = self
            .dim_fields
            .iter()
            .map(|(field, categorized)| {
                let (datum, is_null) = get_field_value(
                    &categorized.source,
                    categorized.attno,
                    values,
                    isnull,
                    unpacked_composites,
                );
                if is_null {
                    return Ok(PdbOwnedValue::Null);
                }
                let datum = unwrap_alias_datum(datum, categorized.pg_type);
                Ok(scalar_datum_to_tantivy_value(
                    datum,
                    field.field_type(),
                    categorized.base_oid,
                    self.created_by_version,
                )?
                .0)
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(self.tree.route(&point))
    }
}

/// Internal state used by each parallel build worker
struct WorkerBuildState<'a> {
    writer: Option<SerialIndexWriter>,
    categorized_fields: Vec<(SearchField, CategorizedFieldData)>,
    per_row_context: PgMemoryContexts,
    current_xid: pg_sys::FullTransactionId,
    next_xid: pg_sys::FullTransactionId,
    indexrel: PgSearchRelation,
    heaprel: PgSearchRelation,
    index_created_by_version: Option<Version>,
    // the following statistics are used to determine when and what to merge:
    //
    // 1. how many segments does this worker expect to make, assuming no merges?
    estimated_nsegments: OnceLock<usize>,
    //
    // 2. how many segments is this worker supposed to make? (assigned by the leader)
    worker_segment_target: usize,
    //
    // 2b. how many segments are all workers together supposed to make?
    target_segment_count: usize,
    //
    // 3. how many merges has this worker done so far? (incrementing counter)
    nmerges: usize,
    //
    // 4. how many workers are there in total? (including the leader) - utilizing the `nlaunched` field in WorkerCoordination
    coordination: &'a mut WorkerCoordination, // passing in `WorkerCoordination` to have a shared view of `ntuples_done`, used for reporting `tuples_done` in progress monitoring view
    //
    // 5. unmerged segment metas that this worker has created so far
    unmerged_metas: Vec<SegmentMeta>,
    local_tuple_done_count: usize, // worker-local number of tuples done - used to updated the shared `ntuples_done` in `coordination`
    is_leader: bool,
    // the first flushed segment's on-disk size, once observed. Segments are memory-budget
    // bound, so one sample is representative; the partitioned drain re-applies it to each
    // per-partition writer's fresh disk guard.
    sampled_segment_bytes: Option<u64>,
    // For a partitioned build: phase-1 routing and spill state. `None` on the regular path.
    partitioning: Option<PartitionSpill>,
    // The budget of the sort that orders one partition's ctids on the partitioned drain.
    gather_budget: usize,
    // The writer config, kept to reopen the per-partition writers on the partitioned drain.
    writer_config: IndexWriterConfig,
    worker_number: i32,
    // This worker's slot among the build's participants, and how many there are. A partitioned
    // drain hands each participant its own share of the partitions.
    participant: usize,
    nparticipants: usize,
}

impl<'a> WorkerBuildState<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        heaprel: &PgSearchRelation,
        indexrel: &PgSearchRelation,
        per_worker_memory_budget: NonZeroUsize,
        current_xid: pg_sys::FullTransactionId,
        next_xid: pg_sys::FullTransactionId,
        worker_segment_target: usize,
        target_segment_count: usize,
        coordination: &'a mut WorkerCoordination,
        worker_number: i32,
        is_leader: bool,
        partitioning: Option<KdTree>,
        spill_files: PartitionSpillFiles,
    ) -> anyhow::Result<Self> {
        // If we're making more than one segment, do an early cutoff based on doc
        // count in case the memory budget is so high that all the docs fit into one
        // segment. For a single-segment target, leave it unbounded (memory-driven).
        // Any vector-specific doc cap is applied in `SerialIndexWriter::open`.
        //
        // Partitioned builds cut segments at partition boundaries instead, so their writers stay
        // memory-driven on their share of the worker budget. (A vector schema's doc cap in
        // `SerialIndexWriter::open` still applies; an overfull partition then flushes in capped
        // segments that `finish_partition` merges back down in bounded passes.)
        let max_docs_per_segment = if partitioning.is_none() && worker_segment_target > 1 {
            Some(
                plan::estimate_heap_reltuples(heaprel) as u32
                    / coordination.nlaunched as u32
                    / worker_segment_target as u32,
            )
        } else {
            None
        };
        // A partitioned drain holds one partition's ctids (in memory, or in a tuplesort that spills
        // past the same bound) next to the live writer's arena, so the ctids take a slice of the
        // worker budget and the writer gets the rest: the worker's peak stays at the share of
        // `maintenance_work_mem` that `adjust_maintenance_work_mem` sized. Phase 1 holds only
        // the spill files' write buffers.
        let gather_budget = if partitioning.is_some() {
            gather_budget(per_worker_memory_budget)
        } else {
            0
        };
        let per_writer_budget = NonZeroUsize::new(per_worker_memory_budget.get() - gather_budget)
            .expect("writer budget should be non-zero");
        let config = IndexWriterConfig {
            memory_budget: per_writer_budget,
            max_docs_per_segment,
        };
        // Abort the build early if it is projected not to fit on the tablespace volume.
        let disk_guard = indexrel
            .is_create_index()
            .then(|| DiskSpaceGuard::new(indexrel));
        let writer = SerialIndexWriter::open(indexrel, config.clone(), worker_number)?
            .with_disk_guard(disk_guard);
        let schema = writer.schema();
        let categorized_fields = schema.categorized_fields().clone();
        let created_by_version = indexrel.created_by_version();

        let (participant, nparticipants) = (spill_files.participant, spill_files.nparticipants);
        let partitioning = partitioning
            .map(|tree| {
                PartitionSpill::new(tree, &categorized_fields, spill_files, created_by_version)
            })
            .transpose()?;

        Ok(Self {
            writer: Some(writer),
            categorized_fields,
            per_row_context: PgMemoryContexts::new("pg_search ambuild context"),
            indexrel: indexrel.clone(),
            heaprel: heaprel.clone(),
            index_created_by_version: created_by_version,
            current_xid,
            next_xid,
            worker_segment_target,
            target_segment_count,
            coordination,
            estimated_nsegments: OnceLock::new(),
            nmerges: Default::default(),
            unmerged_metas: Default::default(),
            local_tuple_done_count: 0,
            is_leader,
            sampled_segment_bytes: None,
            partitioning,
            gather_budget,
            writer_config: config,
            worker_number,
            participant,
            nparticipants,
        })
    }

    /// After this worker flushes a segment, refresh its disk guard with a build-wide view: bump
    /// the shared count of segments written across all workers, and tell the guard how many
    /// segments the whole build still has to write. On the first flush, also hand the guard the
    /// segment's on-disk size (segments are memory-budget bound, so one sample is representative).
    fn on_segment_flushed(&mut self, segment_id: SegmentId) {
        let written = self.coordination.add_segments_written();
        let remaining = self.target_segment_count.saturating_sub(written);

        let mut segment_bytes = None;
        if self.sampled_segment_bytes.is_none() {
            unsafe {
                MetaPage::open(&self.indexrel)
                    .segment_metas()
                    .for_each(|_, entry| {
                        if entry.segment_id() == segment_id {
                            segment_bytes = Some(entry.byte_size());
                        }
                    });
            }
            self.sampled_segment_bytes = segment_bytes;
        }

        let Some(writer) = self.writer.as_mut() else {
            return;
        };
        writer.set_remaining_segments(remaining);
        if let Some(bytes) = segment_bytes {
            writer.set_segment_byte_size(bytes);
        }
    }

    fn commit(&mut self) -> anyhow::Result<()> {
        unsafe {
            set_ps_display_suffix(FINALIZING.as_ptr());
        }
        let writer = self.writer.take().expect("writer should be set");
        if let Some((segment_meta, _)) = writer.commit()? {
            self.unmerged_metas.push(segment_meta);
        }
        self.try_merge(true)?;

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
                    self.worker_segment_target.saturating_sub(self.nmerges),
                );

                if chunk_size <= 1 || self.unmerged_metas.len() < chunk_size {
                    pgrx::debug1!(
                        "try_merge: skipping merge because chunk_size: {chunk_size}, unmerged_metas: {:?}",
                        self.unmerged_metas.len()
                    );
                    return Ok(());
                }

                if self.nmerges == self.worker_segment_target - 1 {
                    pgrx::debug1!(
                        "try_merge: skipping merge because this is not the last merge, and we can only do one more"
                    );
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

        pgrx::debug1!("try_merge: last merge {is_last_merge}");
        self.merge_now(&segment_ids_to_merge)?;
        Ok(())
    }

    /// Merge the given segments into a single segment and garbage collect the index, returning
    /// the reclaimed space to the fsm. Split out of [`Self::try_merge`] so the partitioned
    /// build can merge a partition's segments without the chunk accounting. Returns the merged
    /// segment's meta, if the merge produced one.
    fn merge_now(
        &mut self,
        segment_ids_to_merge: &[SegmentId],
    ) -> anyhow::Result<Option<SegmentMeta>> {
        pgrx::debug1!(
            "do_merge: about to merge {} segments: {:?}",
            segment_ids_to_merge.len(),
            segment_ids_to_merge
        );
        let mut merger = SearchIndexMerger::open(&self.indexrel, MvccSatisfies::Mergeable)?;
        unsafe { set_ps_display_suffix(MERGING.as_ptr()) };
        let merged = merger.merge_segments(segment_ids_to_merge)?;

        // garbage collect the index, returning to the fsm
        pgrx::debug1!("do_merge: garbage collecting");
        unsafe {
            set_ps_display_suffix(GARBAGE_COLLECTING.as_ptr());
        };
        unsafe { garbage_collect_index(&self.indexrel, self.current_xid, self.next_xid) };

        self.nmerges += 1;

        Ok(merged)
    }

    /// Estimates how many segments this worker will make if no merging happens.
    ///
    /// This is used to determine how many segments to merge down in chunks.
    fn estimated_nsegments(&self, docs_per_segment: u32) -> usize {
        *self.estimated_nsegments.get_or_init(|| {
            let reltuples = plan::estimate_heap_reltuples(&self.heaprel);
            let reltuples_per_worker = reltuples / self.coordination.nlaunched as f64;
            let nsegments = (reltuples_per_worker / docs_per_segment as f64).ceil() as usize;
            pgrx::debug1!("estimated that this worker will make {nsegments} segments, based on reltuples: {reltuples}, nlaunched: {}, reltuples_per_worker: {reltuples_per_worker}, docs_per_segment: {docs_per_segment}", self.coordination.nlaunched);
            nsegments
        })
    }

    /// Phase-1 handling for a partitioned build: route the scanned row to its partition and append
    /// its ctid to the partition's spill file. Composites are unpacked over the full field set so a
    /// composite `partition_by` field resolves the same way the document build would.
    unsafe fn route_and_spill(
        &mut self,
        values: *mut pg_sys::Datum,
        isnull: *mut bool,
        ctid: u64,
    ) -> anyhow::Result<()> {
        let categorized_fields = &self.categorized_fields;
        let partitioning = self
            .partitioning
            .as_mut()
            .expect("route_and_spill: partitioning should be set");
        // Routing pallocs per row (composite unpacking, detoast, numeric conversion) and the
        // callback's context lives for the whole scan, so run it in the per-row context and
        // reset after, like the document branch. The append stays outside: a partition's spill file
        // is created on its first row and has to outlive the reset.
        let pid = self.per_row_context.switch_to(|_| {
            let unpacked_composites =
                CompositeSlotValues::from_composites(collect_composites_for_unpacking(
                    categorized_fields.iter().map(|(_, cat)| cat),
                    values,
                    isnull,
                ));
            partitioning.route(values, isnull, &unpacked_composites)
        });
        self.per_row_context.reset();
        partitioning.files.append(pid?, ctid);
        Ok(())
    }

    /// Bump the worker-local tuple count and periodically fold it into the shared progress
    /// counter (`pg_stat_progress_create_index.tuples_done`).
    fn count_tuple_done(&mut self) {
        self.local_tuple_done_count += 1;

        if self
            .local_tuple_done_count
            .is_multiple_of(TUPLES_DONE_BATCH_SIZE)
        {
            self.coordination.add_tuples_done(TUPLES_DONE_BATCH_SIZE);

            if self.is_leader {
                unsafe {
                    pg_sys::pgstat_progress_update_param(
                        pg_sys::PROGRESS_CREATEIDX_TUPLES_DONE as i32,
                        self.coordination.tuples_done() as i64,
                    );
                }
            }
        }
    }

    /// Phase 2 of a partitioned build. Every participant appended its rows' ctids to per-partition
    /// spill files during the scan; once all of them are done, each participant takes its share
    /// of the partitions and drains them one at a time: a partition's ctids from all participants, sorted,
    /// so that re-fetching walks the heap in order under a reused buffer pin, with the blocks
    /// ahead prefetched. Only one writer is alive at a time, and each partition finalizes into its own
    /// segment, whichever participants scanned its rows.
    fn drain_partitioned(&mut self) -> anyhow::Result<()> {
        let PartitionSpill {
            tree, mut files, ..
        } = self
            .partitioning
            .take()
            .expect("drain_partitioned: partitioning state should be set");
        unsafe { files.finish_writes() };
        let barrier = Instant::now();
        self.wait_for_spills();
        pgrx::debug1!(
            "build_worker {}: partitioned drain starting after a {:.3}s wait for the other participants",
            self.worker_number,
            barrier.elapsed().as_secs_f64()
        );
        let drain = Instant::now();

        let heaprel = self.heaprel.clone();
        let indexrel = self.indexrel.clone();
        let heap_fetch_state = HeapFetchState::new(&heaprel);
        let expression_state = ExpressionState::new(&indexrel);
        let heaptupdesc = unsafe { PgTupleDesc::from_pg_unchecked(heaprel.rd_att) };
        let categorized_fields = self.categorized_fields.clone();
        let mut fetcher = HeapDocFetcher::new(
            &heap_fetch_state,
            &expression_state,
            &heaprel,
            &heaptupdesc,
            &categorized_fields,
            self.index_created_by_version,
            // Like any CREATE INDEX, the build must index every live row so that the
            // segments can serve future snapshots: fetch with maintenance semantics.
            false,
        )
        // The scan callback spilled HOT chain root ctids; index each chain's live tail,
        // as the inline callback would have.
        .with_root_ctids();

        let prefetch_distance = unsafe { pg_sys::maintenance_io_concurrency }.max(0) as usize;
        let (first_partition, owned_partitions) =
            chunk_range(tree.partition_count(), self.nparticipants, self.participant);
        for partition in first_partition..first_partition + owned_partitions {
            check_for_interrupts!();
            let Some(mut sorter) = (unsafe { files.gather(partition, self.gather_budget)? }) else {
                continue;
            };
            if self.writer.is_none() {
                self.open_partition_writer()?;
            }
            // Every segment of the partition carries its box, so a reader can prune on it without
            // opening the segment.
            self.writer
                .as_mut()
                .expect("drain_partitioned: writer should be set")
                .set_logical_bounds(partition_box(&tree, partition));
            let mut partition_metas: Vec<SegmentMeta> = Vec::new();
            let mut ctids = PrefetchWindow::new(
                &heaprel,
                std::iter::from_fn(|| sorter.next_sorted().map(|ctid| ctid as u64)),
                prefetch_distance,
            );
            for ctid in &mut ctids {
                check_for_interrupts!();

                // The doc's palloc traffic (detoast copies, expression evaluation) lands in the
                // per-row context; the insert runs outside the closure because both the writer
                // and the context live on `self`, and the doc's values are Rust-owned by then
                // anyway.
                let doc = unsafe { self.per_row_context.switch_to(|_| fetcher.fetch_doc(ctid)) };
                if let Some(doc) = doc {
                    let segment_meta = self
                        .writer
                        .as_mut()
                        .expect("drain_partitioned: writer should be set")
                        .insert(doc, ctid, || unsafe {
                            set_ps_display_suffix(COMMITTING.as_ptr())
                        })?;
                    if let Some(segment_meta) = segment_meta {
                        self.on_segment_flushed(segment_meta.id());
                        partition_metas.push(segment_meta);
                        unsafe { set_ps_display_suffix(INDEXING.as_ptr()) };
                    }
                }
                unsafe { self.per_row_context.reset() };
                self.count_tuple_done();
            }
            // The sort is done with; release it before the partition's commit and merge stack on
            // top of the writer.
            drop(ctids);
            drop(sorter);
            self.finish_partition(&mut partition_metas)?;
            // A partition merge leaves the status at garbage collecting while the drain goes on
            // indexing the next partition.
            unsafe { set_ps_display_suffix(INDEXING.as_ptr()) };
        }
        pgrx::debug1!(
            "build_worker {}: drained {owned_partitions} partitions in {:.3}s",
            self.worker_number,
            drain.elapsed().as_secs_f64()
        );

        unsafe { set_ps_display_remove_suffix() };
        Ok(())
    }

    /// The phase-1 barrier: a partition's spill is complete only once every participant's scan is
    /// over, so no participant drains before the last one gets there.
    fn wait_for_spills(&mut self) {
        let nparticipants = self.nparticipants;
        self.coordination.add_spilled();
        while self.coordination.nspilled() < nparticipants {
            check_for_interrupts!();
            // A short latch wait rather than a sleep: it shows up as a wait event, and a
            // parallel message (a peer's error) or postmaster death ends it at once.
            unsafe {
                pg_sys::WaitLatch(
                    pg_sys::MyLatch,
                    (pg_sys::WL_LATCH_SET | pg_sys::WL_TIMEOUT | pg_sys::WL_EXIT_ON_PM_DEATH)
                        as c_int,
                    10,
                    pg_sys::PG_WAIT_EXTENSION,
                );
                pg_sys::ResetLatch(pg_sys::MyLatch);
            }
        }
    }

    /// Finalize the current partition: commit the writer's pending segment and, if the partition's rows
    /// exceeded the writer budget and flushed multiple segments, merge them down to one in a
    /// single merge, as the regular path does for a worker's last merge. Merges stay scoped to
    /// a single partition within a single worker; never across workers.
    fn finish_partition(&mut self, partition_metas: &mut Vec<SegmentMeta>) -> anyhow::Result<()> {
        let writer = self
            .writer
            .take()
            .expect("finish_partition: writer should be set");
        if let Some((segment_meta, _)) = writer.commit()? {
            self.on_segment_flushed(segment_meta.id());
            partition_metas.push(segment_meta);
        }

        if partition_metas.len() > 1 {
            let ids = partition_metas
                .iter()
                .map(|meta| meta.id())
                .collect::<Vec<_>>();
            self.merge_now(&ids)?;
        }
        partition_metas.clear();
        Ok(())
    }

    /// Open a fresh writer for the next partition.
    fn open_partition_writer(&mut self) -> anyhow::Result<()> {
        let disk_guard = self
            .indexrel
            .is_create_index()
            .then(|| DiskSpaceGuard::new(&self.indexrel));
        let mut writer = SerialIndexWriter::open(
            &self.indexrel,
            self.writer_config.clone(),
            self.worker_number,
        )?
        .with_disk_guard(disk_guard);
        // A fresh guard knows neither the segment size, sampled once per build, nor how many
        // segments the whole build still has to write; without both it never projects and its
        // check is a no-op.
        if let Some(bytes) = self.sampled_segment_bytes {
            writer.set_segment_byte_size(bytes);
        }
        let written = self.coordination.nsegments_written();
        writer.set_remaining_segments(self.target_segment_count.saturating_sub(written));
        self.writer = Some(writer);
        Ok(())
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

    // Partitioned build, phase 1: rows are not indexed during the scan. Route the row to its
    // partition and append its ctid to the partition's spill file; the phase-2 drain re-fetches and
    // indexes rows partition by partition. tuples_done is reported from that drain, which does this row's
    // actual indexing work; scan progress is visible through blocks_done.
    if build_state.partitioning.is_some() {
        build_state
            .route_and_spill(values, isnull, ctid_u64)
            .unwrap_or_else(|e| panic!("could not route row for partitioned build: {e}"));
        return;
    }

    let segment_meta = build_state.per_row_context.switch_to(|_| {
        let mut doc = TantivyDocument::new();

        // Unpack all composites upfront
        let unpacked_composites =
            CompositeSlotValues::from_composites(collect_composites_for_unpacking(
                build_state.categorized_fields.iter().map(|(_, cat)| cat),
                values,
                isnull,
            ));

        row_to_search_document(
            build_state
                .categorized_fields
                .iter()
                .map(|(field, categorized)| {
                    let (datum, is_null) = get_field_value(
                        &categorized.source,
                        categorized.attno,
                        values,
                        isnull,
                        &unpacked_composites,
                    );
                    (datum, is_null, field, categorized)
                }),
            &mut doc,
            build_state.index_created_by_version,
        )
        .unwrap_or_else(|e| panic!("{e}"));

        build_state
            .writer
            .as_mut()
            .expect("build_callback: writer should be set")
            .insert(doc, ctid_u64, || set_ps_display_suffix(COMMITTING.as_ptr()))
            .unwrap_or_else(|e| panic!("{e}"))
    });
    build_state.per_row_context.reset();

    build_state.count_tuple_done();

    if let Some(segment_meta) = segment_meta {
        build_state.on_segment_flushed(segment_meta.id());
        build_state.unmerged_metas.push(segment_meta);
        build_state
            .try_merge(false)
            .unwrap_or_else(|e| panic!("{e}"));
        set_ps_display_suffix(INDEXING.as_ptr());
    }
}

/// Build an index.  This is the workhorse behind `CREATE INDEX` and `REINDEX`.
///
/// If the system allows, it will build the index in parallel.  Otherwise the index is built in
/// serially in this connected backend.
pub(super) fn build_index(
    heaprel: PgSearchRelation,
    indexrel: PgSearchRelation,
    concurrent: bool,
) -> anyhow::Result<f64> {
    struct SnapshotDropper(pg_sys::Snapshot);
    crate::impl_safe_drop!(SnapshotDropper, |self| {
        unsafe {
            let snapshot = self.0;
            // if it's an mvcc snapshot we must unregister it
            if (*snapshot).snapshot_type == pg_sys::SnapshotType::SNAPSHOT_MVCC
                || (*snapshot).snapshot_type == pg_sys::SnapshotType::SNAPSHOT_HISTORIC_MVCC
            {
                pg_sys::UnregisterSnapshot(snapshot);
            }
        }
    });

    let snapshot = SnapshotDropper(unsafe {
        if concurrent {
            pg_sys::RegisterSnapshot(pg_sys::GetTransactionSnapshot())
        } else {
            &raw mut pg_sys::SnapshotAnyData
        }
    });

    let config = WorkerConfig {
        heaprelid: heaprel.oid(),
        indexrelid: indexrel.oid(),
        concurrent,
        current_xid: unsafe { pg_sys::GetCurrentFullTransactionId() },
        need_wal: indexrel.need_wal(),
        next_xid: unsafe { pg_sys::ReadNextFullTransactionId() },
    };

    // Boundaries are fixed before any worker starts, so every worker cuts on the same ones.
    // Partitioned builds cover only non-concurrent CREATE INDEX for now: a concurrent build's
    // deferred re-fetch could index row versions newer than its registered snapshot, so under
    // CONCURRENTLY skip the planning entirely: no heap sample, and no tree in the DSM for
    // workers to deserialize and drop.
    // TODO(M3): the target segment count doubles as the partition count for now; rename the
    // reloption to `partition_count` once partitioned storage lands.
    let nworkers = plan::create_index_nworkers(&heaprel, &indexrel);
    // An index that declares `partition_by` is always built partitioned, however the target and
    // the workers come out; the partitioned path handles a single partition too.
    let partitioning = if concurrent {
        None
    } else {
        plan_partition_boundaries(
            &heaprel,
            &indexrel,
            snapshot.0,
            plan::adjusted_target_segment_count(&heaprel, &indexrel),
        )?
    };
    if let Some(partitioning) = &partitioning {
        pgrx::debug1!(
            "build_index: {} partition boundaries:\n{}",
            partitioning.partition_count(),
            partitioning.bounds_listing()
        );
    }
    let partitioning_bytes = match &partitioning {
        Some(partitioning) => postcard::to_allocvec(partitioning)?,
        None => Vec::new(),
    };

    let process = ParallelBuild::new(&heaprel, snapshot.0, config, partitioning_bytes);
    let is_partitioned = partitioning.is_some();
    pgrx::debug1!("build_index: asked for {nworkers} workers");

    // This is updating `tuples_total` in the `pg_stat_progress_create_index` view - `tuples_done` is incremented in `build_callback`
    unsafe {
        pg_sys::pgstat_progress_update_param(
            pg_sys::PROGRESS_CREATEIDX_TUPLES_TOTAL as i32,
            plan::estimate_heap_reltuples(&heaprel) as i64,
        );
    }

    let total_tuples = if let Some(mut process) = launch_parallel_process!(
        ParallelBuild<BuildWorker>,
        process,
        WorkerStyle::Maintenance,
        nworkers,
        1024,
        |launcher: &ParallelProcessLauncher| {
            if !is_partitioned {
                return;
            }
            // The spill fileset is initialized in the DSM itself: its cleanup is a detach
            // callback on the segment, keyed on this very address, so a copy would not do.
            let fileset = launcher
                .state_manager()
                .object::<pg_sys::SharedFileSet>(FILESET_STATE_IDX)
                .expect("should be able to get the spill fileset")
                .expect("spill fileset should not be NULL");
            unsafe { pg_sys::SharedFileSetInit(fileset, launcher.dsm_segment()) };
        }
    ) {
        let nlaunched = process.launched_workers();
        pgrx::debug1!("build_index: launched {nlaunched} workers (not including leader)");
        let coordination = process
            .state_manager_mut()
            .object::<WorkerCoordination>(2)
            .expect("process coordination")
            .expect("process coordination should not be NULL");

        // account for the leader in the coordination
        let mut nlaunched_plus_leader = nlaunched;
        let leader_participating = unsafe { pg_sys::parallel_leader_participation };
        if leader_participating {
            nlaunched_plus_leader += 1;
        }

        // set_nlaunched last, because workers wait for this to be set
        coordination.set_nlaunched(nlaunched_plus_leader);
        pgrx::debug1!("build_index: has {nlaunched_plus_leader} workers (including leader)");

        let (mut total_tuples, mut total_merges) = if leader_participating {
            // if the leader is to participate too, it's nice for it to wait until all the other workers
            // have indicated that they're running.  Otherwise, it's likely the leader will get ahead
            // of the workers, which doesn't allow for "evenly" distributing the work
            while coordination.nstarted() != nlaunched {
                check_for_interrupts!();
                std::thread::yield_now();
            }

            // directly instantiate a worker for the leader and have it do its build
            let mut worker = BuildWorker::new_parallel_worker(*process.state_manager());
            worker.do_build(nlaunched_plus_leader as i32, true)?
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
        let mut coordination: WorkerCoordination = Default::default();
        coordination.set_nlaunched(1);

        let mut worker = BuildWorker::new(
            &heaprel,
            &indexrel,
            config,
            &mut coordination,
            partitioning,
            std::ptr::null_mut(),
        );

        let (total_tuples, total_merges) = worker.do_build(1, true)?;
        pgrx::debug1!("build_index: total_tuples: {total_tuples}, total_merges: {total_merges}");
        total_tuples
    };

    unsafe { set_ps_display_remove_suffix() };
    Ok(total_tuples)
}

mod plan {
    use super::*;

    pub(super) const MAX_VECTOR_BUILD_WORKERS: usize = 4;

    /// Determine the number of workers to use for a given CREATE INDEX/REINDEX statement.
    ///
    /// The number of workers is determined by max_parallel_maintenance_workers. However, if max_parallel_maintenance_workers
    /// is greater than available parallelism, we use available parallelism.
    ///
    /// If the leader is participating, we subtract 1 from the number of workers because the leader also counts as a worker.
    pub(super) fn create_index_nworkers(
        heaprel: &PgSearchRelation,
        indexrel: &PgSearchRelation,
    ) -> usize {
        // We don't want a parallel build to happen if we're creating a single segment
        let target_segment_count = plan::adjusted_target_segment_count(heaprel, indexrel);
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

        // For vector builds, beyond 4 workers there's no improvement to index build time:
        // clustering dominates and parallelizes internally (rayon + BLAS threads), so
        // parallelizing the heap scan/segment flushing isn't the bottleneck. Constrain to 4
        // because fewer concurrent merges means less memory.
        let maintenance_workers = if indexrel
            .schema()
            .is_ok_and(|schema| schema.has_vector_field())
        {
            maintenance_workers.min(MAX_VECTOR_BUILD_WORKERS)
        } else {
            maintenance_workers
        };

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
    pub(super) fn adjusted_target_segment_count(
        heaprel: &PgSearchRelation,
        indexrel: &PgSearchRelation,
    ) -> usize {
        // If there are fewer rows than number of CPUs, use 1 worker
        let reltuples = plan::estimate_heap_reltuples(heaprel);
        let target_segment_count = indexrel.options().target_segment_count();
        if reltuples <= target_segment_count as f64 {
            pgrx::debug1!(
                "number of reltuples ({reltuples}) is less than target segment count ({target_segment_count}), creating a single segment"
            );
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

    // TODO: Convert to use RowEstimate.
    pub(super) fn estimate_heap_reltuples(heap_relation: &PgSearchRelation) -> f64 {
        let mut reltuples = unsafe { (*heap_relation.rd_rel).reltuples };

        // if the reltuples estimate is not available, estimate the number of tuples in the heap
        // by multiplying the number of pages by the max offset number of the first page
        if reltuples <= 0.0 {
            let npages = unsafe {
                pg_sys::RelationGetNumberOfBlocksInFork(
                    heap_relation.as_ptr(),
                    heap_relation.fork_number(),
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
                heap_relation.fork_number(),
            )
        };

        npages as usize * pg_sys::BLCKSZ as usize
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx::prelude::*;

    fn setup_parallel_build_large_table() {
        Spi::run(
            r#"
            DROP TABLE IF EXISTS parallel_build_large;
            CREATE TABLE parallel_build_large (
                id SERIAL PRIMARY KEY,
                name TEXT
            );
            INSERT INTO parallel_build_large (name)
            SELECT 'lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.'
            FROM generate_series(1, 35000);
            "#,
        )
        .expect("failed to setup parallel_build_large table");
    }

    fn cleanup_parallel_build_large_table() {
        Spi::run("DROP TABLE IF EXISTS parallel_build_large;")
            .expect("failed to cleanup parallel_build_large table");
    }

    /// Tests that parallel index building fails with insufficient memory.
    #[pg_test]
    #[should_panic(expected = "maintenance_work_mem")]
    fn test_parallel_build_large_insufficient_memory() {
        setup_parallel_build_large_table();

        Spi::run("SET max_parallel_workers = 8;").unwrap();
        Spi::run("SET maintenance_work_mem = '64MB';").unwrap();
        Spi::run("SET max_parallel_maintenance_workers = 8;").unwrap();

        // This should panic with a "maintenance_work_mem is not high enough" error
        Spi::run(
            "CREATE INDEX parallel_build_large_idx ON parallel_build_large USING paradedb (id, name) WITH (key_field = 'id', target_segment_count = 16);",
        ).unwrap();
    }

    /// Tests parallel index building with various configurations.
    ///
    /// This test creates 35,000 rows and tests 32 different configuration
    /// combinations of maintenance_work_mem, workers, leader participation,
    /// and target segment count.
    #[pg_test]
    fn test_parallel_build_large_configurations() {
        setup_parallel_build_large_table();

        Spi::run("SET max_parallel_workers = 8;").unwrap();

        let maintenance_work_mem = ["2GB", "128MB"];
        let maintenance_workers = [6, 2];
        let leader_participation = [true, false];
        let target_segments = [4, 32];

        for mwm in &maintenance_work_mem {
            for mw in &maintenance_workers {
                for lp in &leader_participation {
                    for ts in &target_segments {
                        // Set configuration
                        Spi::run(&format!("SET max_parallel_maintenance_workers = {};", mw))
                            .unwrap();
                        Spi::run(&format!("SET parallel_leader_participation = {};", lp)).unwrap();
                        Spi::run(&format!("SET maintenance_work_mem = '{}';", mwm)).unwrap();

                        // Create index
                        Spi::run(&format!(
                            "CREATE INDEX parallel_build_large_idx ON parallel_build_large USING paradedb (id, name) WITH (key_field = 'id', target_segment_count = {});",
                            ts
                        ))
                        .unwrap_or_else(|e| {
                            panic!(
                                "CREATE INDEX failed with workers={}, leader={}, mem={}, segments={}: {:?}",
                                mw, lp, mwm, ts, e
                            )
                        });

                        // Verify segment count
                        let count: i64 = Spi::get_one(
                            "SELECT COUNT(*)::bigint FROM paradedb.index_info('parallel_build_large_idx');",
                        )
                        .unwrap()
                        .unwrap();

                        if *ts == 4 {
                            assert_eq!(
                                count, 4,
                                "Expected 4 segments with workers={}, leader={}, mem={}, segments={}, got {}",
                                mw, lp, mwm, ts, count
                            );
                        } else if *ts == 32 {
                            assert!(
                                (28..=33).contains(&count),
                                "Expected 28-33 segments with workers={}, leader={}, mem={}, segments={}, got {}",
                                mw,
                                lp,
                                mwm,
                                ts,
                                count
                            );
                        }

                        // Verify total document count
                        let num_docs: i64 = Spi::get_one(
                            "SELECT COALESCE(SUM(num_docs), 0)::bigint FROM paradedb.index_info('parallel_build_large_idx');",
                        )
                        .unwrap()
                        .unwrap();

                        assert_eq!(
                            num_docs, 35000,
                            "Expected 35000 docs with workers={}, leader={}, mem={}, segments={}, got {}",
                            mw, lp, mwm, ts, num_docs
                        );

                        // Drop index for next iteration
                        Spi::run("DROP INDEX parallel_build_large_idx;").unwrap();
                    }
                }
            }
        }

        cleanup_parallel_build_large_table();
    }

    /// A row count that is an exact multiple of the vector doc-count cap leaves the writer with
    /// no pending segment at commit; the final merge must still run.
    #[pg_test]
    fn test_single_segment_build_exact_doc_cap_multiple() {
        let nrows = crate::index::writer::index::DEFAULT_MAX_DOCS_PER_SEGMENT * 5;
        Spi::run("CREATE EXTENSION IF NOT EXISTS vector;").unwrap();
        Spi::run(&format!(
            r#"
            CREATE TABLE exact_cap_multiple (id SERIAL8, emb vector(8));
            INSERT INTO exact_cap_multiple (emb)
            SELECT ('[' || array_to_string(array(SELECT random() FROM generate_series(1, 8)), ',') || ']')::vector
            FROM generate_series(1, {nrows});
            "#
        ))
        .unwrap();
        Spi::run(
            "CREATE INDEX exact_cap_multiple_idx ON exact_cap_multiple USING paradedb (id, emb vector_cosine_ops) WITH (key_field = 'id', target_segment_count = 1);",
        )
        .unwrap();

        let count: i64 = Spi::get_one(
            "SELECT COUNT(*)::bigint FROM paradedb.index_info('exact_cap_multiple_idx');",
        )
        .unwrap()
        .unwrap();
        assert_eq!(count, 1, "expected a single segment, got {count}");
    }

    /// An index with `partition_by` routes each row to a kd-tree partition and builds one segment
    /// per partition, serial or parallel. `tenant_id` is scattered across the heap so every
    /// participant's scan slice holds rows of every partition and each partition gathers from all of
    /// them.
    #[pg_test]
    fn test_partitioned_build_segments_and_docs() {
        // The heap must exceed the 15MB floor in `adjusted_target_segment_count`, or the target
        // collapses to a single partition. A ~900-byte name over 20k rows clears it while staying
        // under the TOAST threshold. `tenant_id` is scattered across the heap.
        Spi::run(
            r#"
            CREATE TABLE partitioned_build (id BIGSERIAL PRIMARY KEY, tenant_id BIGINT, name TEXT);
            INSERT INTO partitioned_build (tenant_id, name)
            SELECT (i * 7919) % 100, 'lorem ipsum ' || i || ' ' || repeat('padding word here ', 50)
            FROM generate_series(1, 20000) i;
            "#,
        )
        .unwrap();

        // Serial build: a single worker sees every partition, so it emits exactly one segment per
        // partition (the kd-tree's partition count).
        Spi::run("SET max_parallel_maintenance_workers = 0;").unwrap();
        Spi::run(
            "CREATE INDEX partitioned_build_idx ON partitioned_build USING paradedb (id, tenant_id, name) WITH (key_field = 'id', partition_by = 'tenant_id', target_segment_count = 8);",
        )
        .unwrap();

        let count: i64 = Spi::get_one(
            "SELECT COUNT(*)::bigint FROM paradedb.index_info('partitioned_build_idx');",
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            count, 8,
            "serial partitioned build: one segment per partition"
        );

        let num_docs: i64 = Spi::get_one(
            "SELECT COALESCE(SUM(num_docs), 0)::bigint FROM paradedb.index_info('partitioned_build_idx');",
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            num_docs, 20000,
            "serial partitioned build indexes every row"
        );

        let matches: i64 = Spi::get_one(
            "SELECT COUNT(*)::bigint FROM partitioned_build WHERE partitioned_build @@@ 'name:lorem';",
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            matches, 20000,
            "search through the serial partitioned index"
        );

        // Parallel builds: every worker's scan slice spans every partition, but each partition is drained
        // by one participant, so the segment count stays at the partition count whether the leader
        // takes a share of the partitions or not.
        Spi::run("SET max_parallel_workers = 8;").unwrap();
        Spi::run("SET max_parallel_maintenance_workers = 2;").unwrap();
        Spi::run("SET maintenance_work_mem = '128MB';").unwrap();
        for leader_participation in ["false", "true"] {
            Spi::run("DROP INDEX partitioned_build_idx;").unwrap();
            Spi::run(&format!(
                "SET parallel_leader_participation = {leader_participation};"
            ))
            .unwrap();
            Spi::run(
                "CREATE INDEX partitioned_build_idx ON partitioned_build USING paradedb (id, tenant_id, name) WITH (key_field = 'id', partition_by = 'tenant_id', target_segment_count = 8);",
            )
            .unwrap();

            let count: i64 = Spi::get_one(
                "SELECT COUNT(*)::bigint FROM paradedb.index_info('partitioned_build_idx');",
            )
            .unwrap()
            .unwrap();
            assert_eq!(
                count, 8,
                "parallel partitioned build (leader participation {leader_participation}): one segment per partition"
            );

            let num_docs: i64 = Spi::get_one(
                "SELECT COALESCE(SUM(num_docs), 0)::bigint FROM paradedb.index_info('partitioned_build_idx');",
            )
            .unwrap()
            .unwrap();
            assert_eq!(
                num_docs, 20000,
                "parallel partitioned build indexes every row"
            );

            let matches: i64 = Spi::get_one(
                "SELECT COUNT(*)::bigint FROM partitioned_build WHERE partitioned_build @@@ 'name:lorem';",
            )
            .unwrap()
            .unwrap();
            assert_eq!(
                matches, 20000,
                "search through the parallel partitioned index"
            );
        }

        Spi::run("DROP TABLE partitioned_build;").unwrap();
    }

    /// A partition whose rows all sit in one heap block is scanned by a single participant, so every
    /// other participant has no spill file for it, and its owner (fixed by `chunk_range`, not
    /// by who scanned it) gathers it from that one file. The rest of the table is scattered
    /// so the other partitions still gather from everyone.
    #[pg_test]
    fn test_partitioned_build_partition_seen_by_one_participant() {
        Spi::run(
            r#"
            CREATE TABLE partitioned_lone_partition (id BIGSERIAL PRIMARY KEY, tenant_id BIGINT, name TEXT);
            INSERT INTO partitioned_lone_partition (tenant_id, name)
            SELECT (i * 7919) % 7, 'lorem ipsum ' || i || ' ' || repeat('padding word here ', 50)
            FROM generate_series(1, 20000) i;
            INSERT INTO partitioned_lone_partition (tenant_id, name)
            SELECT 7, 'lone ' || i FROM generate_series(1, 5) i;
            "#,
        )
        .unwrap();
        Spi::run("SET max_parallel_workers = 8;").unwrap();
        Spi::run("SET max_parallel_maintenance_workers = 3;").unwrap();
        Spi::run("SET maintenance_work_mem = '128MB';").unwrap();
        for leader_participation in ["false", "true"] {
            Spi::run("DROP INDEX IF EXISTS partitioned_lone_partition_idx;").unwrap();
            Spi::run(&format!(
                "SET parallel_leader_participation = {leader_participation};"
            ))
            .unwrap();
            Spi::run(
                "CREATE INDEX partitioned_lone_partition_idx ON partitioned_lone_partition USING paradedb (id, tenant_id, name) WITH (key_field = 'id', partition_by = 'tenant_id', target_segment_count = 8);",
            )
            .unwrap();

            let count: i64 = Spi::get_one(
                "SELECT COUNT(*)::bigint FROM paradedb.index_info('partitioned_lone_partition_idx');",
            )
            .unwrap()
            .unwrap();
            assert_eq!(
                count, 8,
                "one segment per partition (leader participation {leader_participation})"
            );

            let num_docs: i64 = Spi::get_one(
                "SELECT COALESCE(SUM(num_docs), 0)::bigint FROM paradedb.index_info('partitioned_lone_partition_idx');",
            )
            .unwrap()
            .unwrap();
            assert_eq!(num_docs, 20005, "every row is indexed once");

            let lone: i64 = Spi::get_one(
                "SELECT COUNT(*)::bigint FROM partitioned_lone_partition WHERE tenant_id = 7 AND partitioned_lone_partition @@@ 'name:lone';",
            )
            .unwrap()
            .unwrap();
            assert_eq!(lone, 5, "the lone partition's rows are searchable");
        }

        Spi::run("DROP TABLE partitioned_lone_partition;").unwrap();
    }

    /// A row deleted by the building transaction itself is still indexed, as any index build
    /// does for a pre-existing snapshot that may need it: the drain must not mistake the
    /// deletion for a HOT chain it has to walk past. The regular path is the reference.
    #[pg_test]
    fn test_partitioned_build_indexes_rows_deleted_in_same_transaction() {
        Spi::run(
            r#"
            CREATE TABLE partitioned_deleted (id BIGSERIAL PRIMARY KEY, tenant_id BIGINT, name TEXT);
            INSERT INTO partitioned_deleted (tenant_id, name)
            SELECT (i * 7919) % 100, 'lorem ipsum ' || i || ' ' || repeat('padding word here ', 50)
            FROM generate_series(1, 20000) i;
            DELETE FROM partitioned_deleted WHERE id % 100 = 0;
            "#,
        )
        .unwrap();
        Spi::run("SET max_parallel_maintenance_workers = 0;").unwrap();

        let num_docs = |index: &str| -> i64 {
            Spi::get_one(&format!(
                "SELECT COALESCE(SUM(num_docs), 0)::bigint FROM paradedb.index_info('{index}');"
            ))
            .unwrap()
            .unwrap()
        };

        Spi::run(
            "CREATE INDEX partitioned_deleted_idx ON partitioned_deleted USING paradedb (id, tenant_id, name) WITH (key_field = 'id', target_segment_count = 8);",
        )
        .unwrap();
        let regular = num_docs("partitioned_deleted_idx");
        Spi::run("DROP INDEX partitioned_deleted_idx;").unwrap();

        Spi::run(
            "CREATE INDEX partitioned_deleted_idx ON partitioned_deleted USING paradedb (id, tenant_id, name) WITH (key_field = 'id', partition_by = 'tenant_id', target_segment_count = 8);",
        )
        .unwrap();
        let partitioned = num_docs("partitioned_deleted_idx");

        assert_eq!(
            regular, 20000,
            "the regular build indexes the deleted rows too"
        );
        assert_eq!(
            partitioned, regular,
            "the partitioned build indexes what the regular one does"
        );

        let visible: i64 = Spi::get_one(
            "SELECT COUNT(*)::bigint FROM partitioned_deleted WHERE partitioned_deleted @@@ 'name:lorem';",
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            visible, 19800,
            "the deleted rows stay invisible to this transaction"
        );

        Spi::run("DROP TABLE partitioned_deleted;").unwrap();
    }

    /// The partition count is bounded where the target is validated, so a target above the
    /// reloption's maximum never reaches the build.
    #[pg_test(error = "value 10000 out of bounds for option \"target_segment_count\"")]
    fn test_partitioned_build_rejects_a_target_above_the_bound() {
        Spi::run("CREATE TABLE partitioned_capped (id BIGSERIAL PRIMARY KEY, name TEXT);").unwrap();
        Spi::run(
            "CREATE INDEX partitioned_capped_idx ON partitioned_capped USING paradedb (id, name) WITH (key_field = 'id', partition_by = 'id', target_segment_count = 10000);",
        )
        .unwrap();
    }

    /// A partition whose ctids outgrow the drain's in-memory share of the worker budget is sorted
    /// through a spilling tuplesort instead. With `maintenance_work_mem` at the 15MB floor that
    /// share is under 2MB, so a partition of 300k ctids (2.4MB of records) takes that path; the
    /// narrow rows still put the heap past the floor that keeps the target at one partition.
    #[pg_test]
    fn test_partitioned_build_spills_large_partition_ctids() {
        Spi::run(
            r#"
            CREATE TABLE partitioned_spill (id BIGSERIAL PRIMARY KEY, tenant_id BIGINT);
            INSERT INTO partitioned_spill (tenant_id)
            SELECT i % 2 FROM generate_series(1, 600000) i;
            "#,
        )
        .unwrap();
        Spi::run("SET max_parallel_maintenance_workers = 0;").unwrap();
        Spi::run("SET maintenance_work_mem = '15MB';").unwrap();
        Spi::run(
            "CREATE INDEX partitioned_spill_idx ON partitioned_spill USING paradedb (id, tenant_id) WITH (key_field = 'id', partition_by = 'tenant_id', target_segment_count = 2);",
        )
        .unwrap();

        let count: i64 = Spi::get_one(
            "SELECT COUNT(*)::bigint FROM paradedb.index_info('partitioned_spill_idx');",
        )
        .unwrap()
        .unwrap();
        assert_eq!(count, 2, "one segment per partition");

        let num_docs: i64 = Spi::get_one(
            "SELECT COALESCE(SUM(num_docs), 0)::bigint FROM paradedb.index_info('partitioned_spill_idx');",
        )
        .unwrap()
        .unwrap();
        assert_eq!(num_docs, 600000, "every row is indexed once");

        let odd: i64 = Spi::get_one(
            "SELECT COUNT(*)::bigint FROM partitioned_spill WHERE tenant_id = 1 AND id @@@ pdb.all();",
        )
        .unwrap()
        .unwrap();
        assert_eq!(odd, 300000, "the search sees each partition's rows");

        Spi::run("DROP TABLE partitioned_spill;").unwrap();
    }

    /// A HOT update superseding a row in the same transaction as CREATE INDEX leaves the chain
    /// root DELETE_IN_PROGRESS at drain time. The partitioned drain re-fetches by root ctid and
    /// must walk the chain to the live tail, indexing the value the inline build callback
    /// delivered, not the superseded version's. The whole test runs in one transaction (pgrx
    /// wraps each test in one), which is exactly the same-transaction scenario; a low fillfactor
    /// keeps the update on-page (HOT).
    #[pg_test]
    fn test_partitioned_build_indexes_live_hot_member() {
        Spi::run(
            r#"
            CREATE TABLE partitioned_hot (id BIGSERIAL PRIMARY KEY, tenant_id BIGINT, name TEXT)
                WITH (fillfactor = 20);
            INSERT INTO partitioned_hot (tenant_id, name)
            SELECT (i * 7919) % 8, 'filler ' || i FROM generate_series(1, 200) i;
            INSERT INTO partitioned_hot (tenant_id, name) VALUES (3, 'stalemarker');
            UPDATE partitioned_hot SET name = 'freshmarker' WHERE name = 'stalemarker';
            CREATE INDEX partitioned_hot_idx ON partitioned_hot USING paradedb (id, tenant_id, name) WITH (key_field = 'id', partition_by = 'tenant_id', target_segment_count = 2);
            "#,
        )
        .unwrap();

        let fresh: i64 = Spi::get_one(
            "SELECT COUNT(*)::bigint FROM partitioned_hot WHERE partitioned_hot @@@ 'name:freshmarker';",
        )
        .unwrap()
        .unwrap();
        assert_eq!(fresh, 1, "the live row version must be indexed");

        let stale: i64 = Spi::get_one(
            "SELECT COUNT(*)::bigint FROM partitioned_hot WHERE partitioned_hot @@@ 'name:stalemarker';",
        )
        .unwrap()
        .unwrap();
        assert_eq!(stale, 0, "the superseded row version must not be indexed");
    }

    /// A partition whose rows outgrow the writer budget flushes multiple segments mid-partition;
    /// finish_partition must merge them back down so the layout stays one segment per partition.
    #[pg_test]
    fn test_partitioned_build_merges_overfull_partitions() {
        // High-entropy text (every md5 distinct) so the writer's arena genuinely outgrows the
        // small budget within each partition. Values stay under the TOAST threshold so the heap keeps
        // its bytes in the main fork.
        Spi::run(
            r#"
            CREATE TABLE partitioned_overfull (id BIGSERIAL PRIMARY KEY, tenant_id BIGINT, name TEXT);
            INSERT INTO partitioned_overfull (tenant_id, name)
            SELECT (i * 7919) % 4,
                   (SELECT string_agg(md5((i * 32 + j)::text), ' ') FROM generate_series(1, 32) j)
            FROM generate_series(1, 24000) i;
            "#,
        )
        .unwrap();
        Spi::run("SET max_parallel_maintenance_workers = 0;").unwrap();
        Spi::run("SET maintenance_work_mem = '16MB';").unwrap();
        Spi::run(
            "CREATE INDEX partitioned_overfull_idx ON partitioned_overfull USING paradedb (id, tenant_id, name) WITH (key_field = 'id', partition_by = 'tenant_id', target_segment_count = 2);",
        )
        .unwrap();

        let count: i64 = Spi::get_one(
            "SELECT COUNT(*)::bigint FROM paradedb.index_info('partitioned_overfull_idx');",
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            count, 2,
            "overfull partitions must merge down to one segment each"
        );

        let num_docs: i64 = Spi::get_one(
            "SELECT COALESCE(SUM(num_docs), 0)::bigint FROM paradedb.index_info('partitioned_overfull_idx');",
        )
        .unwrap()
        .unwrap();
        assert_eq!(num_docs, 24000, "merged partitions must keep every row");
    }

    /// The partitioned build must return the same rows a non-partitioned build would, serial
    /// and parallel: the other tests check totals, which would not catch a row misrouted or
    /// dropped inside a partition. `partition_by` spans two fields, one of them text, so the
    /// multi-dimensional projection and a detoastable dimension go through the routing spill.
    /// `tenant_id` is scattered across the heap so each worker's slice spans every partition.
    /// [`test_partitioned_build_partition_seen_by_one_participant`] covers the other extreme.
    #[pg_test]
    fn test_partitioned_build_result_parity() {
        // ~950-byte rows over 20k rows clear the 15MB floor in `adjusted_target_segment_count`
        // so the target of 8 yields real partitions, while staying under the TOAST threshold.
        Spi::run(
            r#"
            CREATE TABLE partitioned_parity (id BIGSERIAL PRIMARY KEY, tenant_id BIGINT, message TEXT);
            INSERT INTO partitioned_parity (tenant_id, message)
            SELECT (i * 7919) % 16,
                   'doc ' || i || ' ' || (ARRAY['alpha', 'beta', 'gamma'])[1 + i % 3]
                       || ' ' || repeat('padding word here ', 50)
            FROM generate_series(1, 20000) i;
            "#,
        )
        .unwrap();

        let ids_for = |query: &str| -> String {
            Spi::get_one::<String>(&format!(
                "SELECT COALESCE(string_agg(id::text, ',' ORDER BY id), '') \
                 FROM partitioned_parity WHERE partitioned_parity @@@ '{query}';"
            ))
            .unwrap()
            .unwrap()
        };

        // Ground truth: a non-partitioned index over the same rows.
        Spi::run("SET max_parallel_maintenance_workers = 0;").unwrap();
        Spi::run(
            "CREATE INDEX partitioned_parity_plain ON partitioned_parity USING paradedb (id, tenant_id, message) WITH (key_field = 'id');",
        )
        .unwrap();
        let expected_alpha = ids_for("message:alpha");
        let expected_beta = ids_for("message:beta");
        let expected_all = ids_for("message:doc");
        assert!(
            !expected_alpha.is_empty(),
            "baseline should match some rows"
        );
        Spi::run("DROP INDEX partitioned_parity_plain;").unwrap();

        let assert_parity = |label: &str| {
            assert_eq!(
                ids_for("message:alpha"),
                expected_alpha,
                "{label}: alpha rows differ"
            );
            assert_eq!(
                ids_for("message:beta"),
                expected_beta,
                "{label}: beta rows differ"
            );
            assert_eq!(
                ids_for("message:doc"),
                expected_all,
                "{label}: full set differs"
            );
        };
        let create_partitioned = "CREATE INDEX partitioned_parity_idx ON partitioned_parity USING paradedb (id, tenant_id, message) WITH (key_field = 'id', partition_by = 'tenant_id, message', target_segment_count = 8);";

        // Serial build.
        Spi::run(create_partitioned).unwrap();
        assert_parity("serial");
        Spi::run("DROP INDEX partitioned_parity_idx;").unwrap();

        // Parallel build over the same rows: the result set must not change.
        Spi::run("SET max_parallel_workers = 8;").unwrap();
        Spi::run("SET max_parallel_maintenance_workers = 4;").unwrap();
        Spi::run("SET parallel_leader_participation = false;").unwrap();
        Spi::run("SET maintenance_work_mem = '128MB';").unwrap();
        Spi::run(create_partitioned).unwrap();
        assert_parity("parallel");

        Spi::run("DROP TABLE partitioned_parity;").unwrap();
    }

    /// A partition holding a few dozen segments must merge down to one. A vector schema gets there
    /// cheaply: its writer caps every segment at `DEFAULT_MAX_DOCS_PER_SEGMENT` docs whatever
    /// the memory budget, and a single-partition tree puts all of them in one partition. The vectors are
    /// seeded: the clusterer's behavior depends on the data, and a merge test must not change
    /// its input from run to run.
    #[pg_test]
    fn test_partitioned_build_merges_many_capped_segments() {
        let nrows = crate::index::writer::index::DEFAULT_MAX_DOCS_PER_SEGMENT as usize * 33;
        Spi::run("CREATE EXTENSION IF NOT EXISTS vector;").unwrap();
        Spi::run(&format!(
            r#"
            CREATE TABLE partitioned_passes (id SERIAL8, tenant_id BIGINT, emb vector(8));
            SELECT setseed(0.42);
            INSERT INTO partitioned_passes (tenant_id, emb)
            SELECT i % 4,
                   ('[' || array_to_string(array(SELECT random() FROM generate_series(1, 8)), ',') || ']')::vector
            FROM generate_series(1, {nrows}) i;
            "#
        ))
        .unwrap();
        Spi::run("SET max_parallel_maintenance_workers = 0;").unwrap();
        Spi::run(
            "CREATE INDEX partitioned_passes_idx ON partitioned_passes USING paradedb (id, tenant_id, emb vector_cosine_ops) WITH (key_field = 'id', partition_by = 'tenant_id', target_segment_count = 1);",
        )
        .unwrap();

        let count: i64 = Spi::get_one(
            "SELECT COUNT(*)::bigint FROM paradedb.index_info('partitioned_passes_idx');",
        )
        .unwrap()
        .unwrap();
        assert_eq!(count, 1, "the partition must merge down to one segment");

        let num_docs: i64 = Spi::get_one(
            "SELECT COALESCE(SUM(num_docs), 0)::bigint FROM paradedb.index_info('partitioned_passes_idx');",
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            num_docs, nrows as i64,
            "the merged partition must keep every row"
        );
    }
}
