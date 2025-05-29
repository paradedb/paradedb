use crate::api::aggregate::mvcc_collector::MVCCFilterCollector;
use crate::api::aggregate::vischeck::TSVisibilityChecker;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::launch_parallel_process;
use crate::postgres::parallel_worker::mqueue::MessageQueueSender;
use crate::postgres::parallel_worker::{ParallelProcess, ParallelState, ParallelWorker};
use crate::postgres::spinlock::Spinlock;
use crate::query::SearchQueryInput;
use pgrx::{check_for_interrupts, default, pg_extern, pg_sys, Json, JsonB, PgRelation};
use rustc_hash::FxHashSet;
use std::error::Error;
use std::ffi::c_void;
use std::ptr::addr_of;
use tantivy::aggregation::agg_req::Aggregations;
use tantivy::aggregation::intermediate_agg_result::IntermediateAggregationResults;
use tantivy::aggregation::{AggregationLimitsGuard, DistributedAggregationCollector};
use tantivy::collector::Collector;
use tantivy::index::SegmentId;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct ParallelAggregationState {
    indexrelid: pg_sys::Oid,
    total_segments: usize,
    solve_mvcc: bool,

    memory_limit: u64,
    bucket_limit: u32,

    // (start,length) lookups into `payload`, all measured in bytes
    agg_req_offset: (usize, usize),
    query_offset: (usize, usize),
    segment_ids_offset: (usize, usize),

    // these require the Spinlock mutex for atomic access (read and write)
    mutex: Spinlock,
    nlaunched: usize,
    remaining_segments: usize,
    payload: [u8; 0], // allocated after the struct by Postgres' shm system
}

impl ParallelState for ParallelAggregationState {}

impl ParallelAggregationState {
    fn set_launched_workers(&mut self, nlaunched: usize) {
        let _lock = self.mutex.acquire();
        self.nlaunched = nlaunched;
    }

    fn launched_workers(&mut self) -> usize {
        let _lock = self.mutex.acquire();
        self.nlaunched
    }

    fn checkout_segment(&mut self) -> Option<SegmentId> {
        let segment_ids = unsafe {
            let payload = self.payload.as_ptr();
            std::slice::from_raw_parts(
                payload.add(self.segment_ids_offset.0).cast::<SegmentId>(),
                self.total_segments,
            )
        };

        let _lock = self.mutex.acquire();
        if self.remaining_segments == 0 {
            return None;
        }
        self.remaining_segments -= 1;
        segment_ids.get(self.remaining_segments).copied()
    }

    fn total_segments(&mut self) -> usize {
        self.total_segments
    }

    fn solve_mvcc(&self) -> bool {
        self.solve_mvcc
    }

    fn agg_req_bytes(&self) -> &[u8] {
        unsafe {
            let payload = self.payload.as_ptr();
            std::slice::from_raw_parts(payload.add(self.agg_req_offset.0), self.agg_req_offset.1)
        }
    }

    fn query_bytes(&self) -> &[u8] {
        unsafe {
            let payload = self.payload.as_ptr();
            std::slice::from_raw_parts(payload.add(self.query_offset.0), self.query_offset.1)
        }
    }
}

struct ParallelAggregation {
    state: ParallelAggregationState,
    query_bytes: Vec<u8>,
    agg_req_bytes: Vec<u8>,
    segment_ids: Vec<SegmentId>,
}

impl ParallelProcess for ParallelAggregation {
    fn size_of_state(&self) -> usize {
        size_of::<ParallelAggregationState>()
            + self.agg_req_bytes.len()
            + self.query_bytes.len()
            + self.segment_ids.len() * size_of::<SegmentId>()
    }

    unsafe fn init_shm_state(&self, dest: *mut c_void) {
        unsafe {
            std::ptr::copy_nonoverlapping(
                addr_of!(self.state) as *const c_void,
                dest,
                size_of::<ParallelAggregationState>(),
            );

            let shm_state = dest as *mut ParallelAggregationState;
            let payload = (*shm_state).payload.as_mut_ptr();

            let agg_req_offset = (0, self.agg_req_bytes.len());
            std::ptr::copy_nonoverlapping(
                self.agg_req_bytes.as_ptr(),
                payload.add(agg_req_offset.0),
                agg_req_offset.1,
            );

            let query_offset = (agg_req_offset.0 + agg_req_offset.1, self.query_bytes.len());
            std::ptr::copy_nonoverlapping(
                self.query_bytes.as_ptr(),
                payload.add(query_offset.0),
                query_offset.1,
            );

            let segment_ids_offset = (
                query_offset.0 + query_offset.1,
                self.segment_ids.len() * size_of::<SegmentId>(),
            );
            std::ptr::copy_nonoverlapping(
                self.segment_ids.as_ptr(),
                payload.add(segment_ids_offset.0).cast(),
                self.segment_ids.len(),
            );

            (*shm_state).agg_req_offset = agg_req_offset;
            (*shm_state).query_offset = query_offset;
            (*shm_state).segment_ids_offset = segment_ids_offset;
        }
    }
}

impl ParallelAggregation {
    pub fn new(
        indexrelid: pg_sys::Oid,
        query: &SearchQueryInput,
        aggregations: &Aggregations,
        solve_mvcc: bool,
        memory_limit: u64,
        bucket_limit: u32,
        segment_ids: Vec<SegmentId>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            state: ParallelAggregationState {
                mutex: Spinlock::new(),
                nlaunched: 0,
                total_segments: segment_ids.len(),
                solve_mvcc,

                memory_limit,
                bucket_limit,
                indexrelid,
                remaining_segments: segment_ids.len(),

                agg_req_offset: (0, 0),
                query_offset: (0, 0),
                segment_ids_offset: (0, 0),
                payload: [],
            },
            agg_req_bytes: serde_json::to_vec(aggregations)?,
            query_bytes: serde_json::to_vec(query)?,
            segment_ids,
        })
    }
}

struct ParallelAggregationWorker;

impl ParallelAggregationWorker {
    fn checkout_segments(
        state: &mut ParallelAggregationState,
        worker_number: i32,
    ) -> FxHashSet<SegmentId> {
        /*
            // thanks, Daniel Lemire:  https://x.com/lemire/status/1925609310274400509

            // N is the total number of elements
            // M is the number of chunks
            // i is the index of the chunk (0-indexed)
            std::pair<size_t, size_t> get_chunk_range_simple(size_t N, size_t M, size_t i) {
                // Calculate the quotient and remainder
                size_t quotient = N / M;
                size_t remainder = N % M;
                size_t start = quotient * i + (i < remainder ? i : remainder);
                size_t length = quotient + (i < remainder ? 1 : 0);
                return {start, length};
            }
        */
        fn chunk_range(n: usize, m: usize, i: usize) -> (usize, usize) {
            let quotient = n / m;
            let remainder = n % m;
            let start = quotient * i + (if i < remainder { i } else { remainder });
            let length = quotient + if i < remainder { 1 } else { 0 };
            (start, length)
        }

        let worker_number = worker_number
            + if unsafe { pg_sys::parallel_leader_participation } {
                1
            } else {
                0
            };

        let nworkers = state.launched_workers();
        let nsegments = state.total_segments();

        let mut segment_ids = FxHashSet::default();
        let (_, many_segments) = chunk_range(nsegments, nworkers, worker_number as usize);
        while let Some(segment_id) = state.checkout_segment() {
            segment_ids.insert(segment_id);

            if segment_ids.len() == many_segments {
                // we have all the segments we need
                break;
            }
        }
        segment_ids
    }

    fn execute_aggregate(
        state: &mut ParallelAggregationState,
        worker_number: i32,
    ) -> anyhow::Result<IntermediateAggregationResults> {
        let segment_ids = ParallelAggregationWorker::checkout_segments(state, worker_number);
        let query = serde_json::from_slice::<SearchQueryInput>(state.query_bytes())?;
        let agg_req = serde_json::from_slice::<Aggregations>(state.agg_req_bytes())?;

        let indexrel =
            unsafe { PgRelation::with_lock(state.indexrelid, pg_sys::AccessShareLock as _) };
        let reader =
            SearchIndexReader::open(&indexrel, MvccSatisfies::ParallelWorker(segment_ids))?;

        let base_collector = DistributedAggregationCollector::from_aggs(
            agg_req,
            AggregationLimitsGuard::new(Some(state.memory_limit), Some(state.bucket_limit)),
        );

        let intermediate_results = if state.solve_mvcc() {
            let heaprel = indexrel
                .heap_relation()
                .expect("index should belong to a heap relation");
            let mvcc_collector = MVCCFilterCollector::new(
                base_collector,
                TSVisibilityChecker::with_rel_and_snap(heaprel.as_ptr(), unsafe {
                    pg_sys::GetActiveSnapshot()
                }),
            );
            reader.collect(&query, mvcc_collector, false)
        } else {
            reader.collect(&query, base_collector, false)
        };
        Ok(intermediate_results)
    }
}

impl ParallelWorker for ParallelAggregationWorker {
    type State = ParallelAggregationState;

    fn run(
        state: &mut Self::State,
        mq_sender: &MessageQueueSender,
        worker_number: i32,
    ) -> anyhow::Result<()> {
        // wait for all workers to launch
        while state.launched_workers() == 0 {
            check_for_interrupts!();
            std::thread::yield_now();
        }

        let intermediate_results = Self::execute_aggregate(state, worker_number)?;

        let bytes = serde_json::to_vec(&intermediate_results)?;
        Ok(mq_sender.send(bytes)?)
    }
}

#[pg_extern]
pub fn aggregate(
    indexrel: PgRelation,
    query: SearchQueryInput,
    agg: Json,
    solve_mvcc: default!(bool, true),
    memory_limit: default!(i64, 500000000),
    bucket_limit: default!(i64, 65000),
) -> Result<JsonB, Box<dyn Error>> {
    unsafe {
        let reader = SearchIndexReader::open(&indexrel, MvccSatisfies::Snapshot)?;
        let agg_req = serde_json::from_value(agg.0)?;
        let process = ParallelAggregation::new(
            indexrel.oid(),
            &query,
            &agg_req,
            solve_mvcc,
            memory_limit.try_into()?,
            bucket_limit.try_into()?,
            reader.segment_ids(),
        )?;

        let nworkers = pg_sys::max_parallel_workers_per_gather as usize;
        let mut process = launch_parallel_process!(
            ParallelAggregation<ParallelAggregationState, ParallelAggregationWorker>,
            process,
            nworkers,
            16384)
        .expect("should be able to launch parallel process");

        // signal our workers with the number of workers actually launched
        // they need this before they can begin checking out the correct segment counts
        let mut nlaunched = process.launched_workers();
        if pg_sys::parallel_leader_participation {
            nlaunched += 1;
        }
        process.shm_state_mut().set_launched_workers(nlaunched);

        // leader participation
        let mut agg_results = Vec::with_capacity(nworkers);
        if pg_sys::parallel_leader_participation {
            agg_results.push(Ok(ParallelAggregationWorker::execute_aggregate(
                process.shm_state_mut(),
                -1, // leader worker number is negative one
            )?));
        }

        // wait for workers to finish, collecting their intermediate aggregate results
        for (_worker_number, message) in process {
            let worker_results =
                serde_json::from_slice::<IntermediateAggregationResults>(&message)?;

            agg_results.push(Ok(worker_results));
        }

        // have tantivy finalize the intermediate results from each worker
        let merged = {
            let collector = DistributedAggregationCollector::from_aggs(
                agg_req.clone(),
                AggregationLimitsGuard::new(
                    Some(memory_limit.try_into()?),
                    Some(bucket_limit.try_into()?),
                ),
            );
            collector.merge_fruits(agg_results)?.into_final_result(
                agg_req,
                AggregationLimitsGuard::new(
                    Some(memory_limit.try_into()?),
                    Some(bucket_limit.try_into()?),
                ),
            )?
        };

        Ok(JsonB(serde_json::to_value(merged)?))
    }
}

pub mod mvcc_collector {
    use parking_lot::Mutex;
    use std::sync::Arc;
    use tantivy::collector::{Collector, SegmentCollector};

    use crate::api::aggregate::vischeck::TSVisibilityChecker;
    use crate::index::fast_fields_helper::FFType;
    use tantivy::{DocId, Score, SegmentOrdinal, SegmentReader};

    pub struct MVCCFilterCollector<C: Collector> {
        inner: C,
        lock: Arc<Mutex<TSVisibilityChecker>>,
    }

    unsafe impl<C: Collector> Send for MVCCFilterCollector<C> {}
    unsafe impl<C: Collector> Sync for MVCCFilterCollector<C> {}

    impl<C: Collector> Collector for MVCCFilterCollector<C> {
        type Fruit = C::Fruit;
        type Child = MVCCFilterSegmentCollector<C::Child>;

        fn for_segment(
            &self,
            segment_local_id: SegmentOrdinal,
            segment: &SegmentReader,
        ) -> tantivy::Result<Self::Child> {
            Ok(MVCCFilterSegmentCollector {
                inner: self.inner.for_segment(segment_local_id, segment)?,
                lock: self.lock.clone(),
                ctid_ff: FFType::new(segment.fast_fields(), "ctid"),
            })
        }

        fn requires_scoring(&self) -> bool {
            self.inner.requires_scoring()
        }

        fn merge_fruits(
            &self,
            segment_fruits: Vec<<Self::Child as SegmentCollector>::Fruit>,
        ) -> tantivy::Result<Self::Fruit> {
            self.inner.merge_fruits(segment_fruits)
        }
    }

    #[allow(clippy::arc_with_non_send_sync)]
    impl<C: Collector> MVCCFilterCollector<C> {
        pub fn new(wrapped: C, vischeck: TSVisibilityChecker) -> Self {
            Self {
                inner: wrapped,
                lock: Arc::new(Mutex::new(vischeck)),
            }
        }
    }

    pub struct MVCCFilterSegmentCollector<SC: SegmentCollector> {
        inner: SC,
        lock: Arc<Mutex<TSVisibilityChecker>>,
        ctid_ff: FFType,
    }
    unsafe impl<C: SegmentCollector> Send for MVCCFilterSegmentCollector<C> {}
    unsafe impl<C: SegmentCollector> Sync for MVCCFilterSegmentCollector<C> {}

    impl<SC: SegmentCollector> SegmentCollector for MVCCFilterSegmentCollector<SC> {
        type Fruit = SC::Fruit;

        fn collect(&mut self, doc: DocId, score: Score) {
            let ctid = self.ctid_ff.as_u64(doc).expect("ctid should be present");
            if self.lock.lock().is_visible(ctid) {
                self.inner.collect(doc, score);
            }
        }

        fn collect_block(&mut self, docs: &[DocId]) {
            let ctids = docs
                .iter()
                .map(|doc_id| {
                    self.ctid_ff
                        .as_u64(*doc_id)
                        .expect("ctid should be present")
                })
                .collect::<Vec<_>>();
            let mut filtered = Vec::with_capacity(docs.len());

            let mut vischeck = self.lock.lock();
            for (doc, ctid) in docs.iter().zip(ctids.iter()) {
                if vischeck.is_visible(*ctid) {
                    filtered.push(*doc);
                }
            }
            drop(vischeck);

            self.inner.collect_block(&filtered);
        }

        fn harvest(self) -> Self::Fruit {
            self.inner.harvest()
        }
    }
}

mod vischeck {
    use crate::postgres::utils;
    use pgrx::itemptr::item_pointer_get_block_number;
    use pgrx::pg_sys;

    pub struct TSVisibilityChecker {
        scan: *mut pg_sys::IndexFetchTableData,
        slot: *mut pg_sys::TupleTableSlot,
        snapshot: pg_sys::Snapshot,
        tid: pg_sys::ItemPointerData,
        vmbuf: pg_sys::Buffer,
    }

    impl Clone for TSVisibilityChecker {
        fn clone(&self) -> Self {
            unsafe { Self::with_rel_and_snap((*self.scan).rel, self.snapshot) }
        }
    }

    impl Drop for TSVisibilityChecker {
        fn drop(&mut self) {
            unsafe {
                if !pg_sys::IsTransactionState() {
                    // we are not in a transaction, so we can't do things like release buffers and close relations
                    return;
                }

                pg_sys::table_index_fetch_end(self.scan);
                ExecClearTuple(self.slot);
                if self.vmbuf != pg_sys::InvalidBuffer as pg_sys::Buffer {
                    pg_sys::ReleaseBuffer(self.vmbuf);
                }
            }
        }
    }

    #[allow(improper_ctypes)]
    extern "C" {
        #[link_name = "ExecClearTuple__pgrx_cshim"]
        fn ExecClearTuple(slot: *mut pg_sys::TupleTableSlot) -> *mut pg_sys::TupleTableSlot;
        #[link_name = "table_index_fetch_tuple__pgrx_cshim"]
        fn table_index_fetch_tuple(
            scan: *mut pg_sys::IndexFetchTableData,
            tid: pg_sys::ItemPointer,
            snapshot: pg_sys::Snapshot,
            slot: *mut pg_sys::TupleTableSlot,
            call_again: *mut bool,
            all_dead: *mut bool,
        ) -> bool;
        fn visibilitymap_get_status(
            rel: pg_sys::Relation,
            heapBlk: pg_sys::BlockNumber,
            vmbuf: *mut pg_sys::Buffer,
        ) -> u8;
    }

    impl TSVisibilityChecker {
        /// Construct a new [`VisibilityChecker`] that can validate ctid visibility against the specified
        /// `relation` and `snapshot`
        pub fn with_rel_and_snap(heaprel: pg_sys::Relation, snapshot: pg_sys::Snapshot) -> Self {
            unsafe {
                Self {
                    scan: pg_sys::table_index_fetch_begin(heaprel),
                    slot: pg_sys::MakeTupleTableSlot(
                        pg_sys::CreateTupleDesc(0, std::ptr::null_mut()),
                        &pg_sys::TTSOpsBufferHeapTuple,
                    ),
                    snapshot,
                    tid: pg_sys::ItemPointerData::default(),
                    vmbuf: pg_sys::InvalidBuffer as _,
                }
            }
        }

        pub fn is_visible(&mut self, ctid: u64) -> bool {
            unsafe {
                utils::u64_to_item_pointer(ctid, &mut self.tid);

                if visibilitymap_get_status(
                    (*self.scan).rel,
                    item_pointer_get_block_number(&self.tid),
                    &mut self.vmbuf,
                ) != 0
                {
                    return true;
                }

                let mut call_again = false;
                let mut all_dead = false;
                ExecClearTuple(self.slot);
                table_index_fetch_tuple(
                    self.scan,
                    &mut self.tid,
                    self.snapshot,
                    self.slot,
                    &mut call_again,
                    &mut all_dead,
                )
            }
        }
    }
}
