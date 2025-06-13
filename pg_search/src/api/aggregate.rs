use crate::api::aggregate::mvcc_collector::MVCCFilterCollector;
use crate::api::aggregate::vischeck::TSVisibilityChecker;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::parallel_worker::mqueue::MessageQueueSender;
use crate::parallel_worker::ParallelStateManager;
use crate::parallel_worker::WorkerStyle;
use crate::parallel_worker::{ParallelProcess, ParallelState, ParallelStateType, ParallelWorker};
use crate::postgres::spinlock::Spinlock;
use crate::query::SearchQueryInput;
use crate::{chunk_range, launch_parallel_process};
use pgrx::{check_for_interrupts, default, pg_extern, pg_sys, Json, JsonB, PgRelation};
use rustc_hash::FxHashSet;
use std::error::Error;
use tantivy::aggregation::agg_req::Aggregations;
use tantivy::aggregation::intermediate_agg_result::IntermediateAggregationResults;
use tantivy::aggregation::{AggregationLimitsGuard, DistributedAggregationCollector};
use tantivy::collector::Collector;
use tantivy::index::SegmentId;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct State {
    // these require the Spinlock mutex for atomic access (read and write)
    mutex: Spinlock,
    nlaunched: usize,
    remaining_segments: usize,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct Config {
    indexrelid: pg_sys::Oid,
    total_segments: usize,
    solve_mvcc: bool,

    memory_limit: u64,
    bucket_limit: u32,
}

impl State {
    fn set_launched_workers(&mut self, nlaunched: usize) {
        let _lock = self.mutex.acquire();
        self.nlaunched = nlaunched;
    }

    fn launched_workers(&mut self) -> usize {
        let _lock = self.mutex.acquire();
        self.nlaunched
    }
}

struct ParallelAggregation {
    state: State,
    config: Config,
    query_bytes: Vec<u8>,
    agg_req_bytes: Vec<u8>,
    segment_ids: Vec<SegmentId>,
}

impl ParallelStateType for State {}
impl ParallelStateType for Config {}
impl ParallelStateType for SegmentId {}

impl ParallelProcess for ParallelAggregation {
    fn state_values(&self) -> Vec<&dyn ParallelState> {
        vec![
            &self.state,
            &self.config,
            &self.agg_req_bytes,
            &self.query_bytes,
            &self.segment_ids,
        ]
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
            state: State {
                mutex: Spinlock::new(),
                nlaunched: 0,
                remaining_segments: segment_ids.len(),
            },
            config: Config {
                indexrelid,
                total_segments: segment_ids.len(),
                solve_mvcc,
                memory_limit,
                bucket_limit,
            },
            agg_req_bytes: serde_json::to_vec(aggregations)?,
            query_bytes: serde_json::to_vec(query)?,
            segment_ids,
        })
    }
}

struct ParallelAggregationWorker<'a> {
    state: &'a mut State,
    config: Config,
    aggregation: Aggregations,
    query: SearchQueryInput,
    segment_ids: Vec<SegmentId>,
}

impl<'a> ParallelAggregationWorker<'a> {
    #[allow(clippy::too_many_arguments)]
    fn new_local(
        aggregation: Aggregations,
        query: SearchQueryInput,
        segment_ids: Vec<SegmentId>,
        indexrelid: pg_sys::Oid,
        solve_mvcc: bool,
        memory_limit: u64,
        bucket_limit: u32,
        state: &'a mut State,
    ) -> Self {
        Self {
            state,
            config: Config {
                indexrelid,
                total_segments: segment_ids.len(),
                solve_mvcc,
                memory_limit,
                bucket_limit,
            },
            aggregation,
            query,
            segment_ids,
        }
    }

    fn checkout_segments(&mut self, worker_number: i32) -> FxHashSet<SegmentId> {
        let worker_number = worker_number
            + if unsafe { pg_sys::parallel_leader_participation } {
                1
            } else {
                0
            };

        let nworkers = self.state.launched_workers();
        let nsegments = self.config.total_segments;

        let mut segment_ids = FxHashSet::default();
        let (_, many_segments) = chunk_range(nsegments, nworkers, worker_number as usize);
        while let Some(segment_id) = self.checkout_segment() {
            segment_ids.insert(segment_id);

            if segment_ids.len() == many_segments {
                // we have all the segments we need
                break;
            }
        }
        segment_ids
    }

    fn checkout_segment(&mut self) -> Option<SegmentId> {
        let _lock = self.state.mutex.acquire();
        if self.state.remaining_segments == 0 {
            return None;
        }
        self.state.remaining_segments -= 1;
        self.segment_ids.get(self.state.remaining_segments).cloned()
    }

    fn execute_aggregate(
        &mut self,
        worker_number: i32,
    ) -> anyhow::Result<Option<IntermediateAggregationResults>> {
        let segment_ids = self.checkout_segments(worker_number);
        if segment_ids.is_empty() {
            return Ok(None);
        }
        let indexrel =
            unsafe { PgRelation::with_lock(self.config.indexrelid, pg_sys::AccessShareLock as _) };
        let reader =
            SearchIndexReader::open(&indexrel, MvccSatisfies::ParallelWorker(segment_ids))?;

        let base_collector = DistributedAggregationCollector::from_aggs(
            self.aggregation.clone(),
            AggregationLimitsGuard::new(
                Some(self.config.memory_limit),
                Some(self.config.bucket_limit),
            ),
        );

        let intermediate_results = if self.config.solve_mvcc {
            let heaprel = indexrel
                .heap_relation()
                .expect("index should belong to a heap relation");
            let mvcc_collector = MVCCFilterCollector::new(
                base_collector,
                TSVisibilityChecker::with_rel_and_snap(heaprel.as_ptr(), unsafe {
                    pg_sys::GetActiveSnapshot()
                }),
            );
            reader.collect(&self.query, mvcc_collector, false)
        } else {
            reader.collect(&self.query, base_collector, false)
        };
        Ok(Some(intermediate_results))
    }
}

impl ParallelWorker for ParallelAggregationWorker<'_> {
    fn new_parallel_worker(state_manager: ParallelStateManager) -> Self {
        let state = state_manager
            .object::<State>(0)
            .expect("wrong type for state")
            .expect("missing state value");
        let config = state_manager
            .object::<Config>(1)
            .expect("wrong type for config")
            .expect("missing config value");
        let agg_req_bytes = state_manager
            .slice::<u8>(2)
            .expect("wrong type for agg_req_bytes")
            .expect("missing agg_req_bytes value");
        let query_bytes = state_manager
            .slice::<u8>(3)
            .expect("wrong type for query_bytes")
            .expect("missing query_bytes value");
        let segment_ids = state_manager
            .slice::<SegmentId>(4)
            .expect("wrong type for segment_ids")
            .expect("missing segment_ids value");

        let aggregation = serde_json::from_slice::<Aggregations>(agg_req_bytes)
            .expect("agg_req_bytes should deserialize into an Aggregations");
        let query = serde_json::from_slice::<SearchQueryInput>(query_bytes)
            .expect("query_bytes should deserialize into an SearchQueryInput");
        Self {
            state,
            config: *config,
            aggregation,
            query,
            segment_ids: segment_ids.to_vec(),
        }
    }

    fn run(mut self, mq_sender: &MessageQueueSender, worker_number: i32) -> anyhow::Result<()> {
        // wait for all workers to launch
        while self.state.launched_workers() == 0 {
            check_for_interrupts!();
            std::thread::yield_now();
        }

        if let Some(intermediate_results) = self.execute_aggregate(worker_number)? {
            let bytes = postcard::to_allocvec(&intermediate_results)?;
            Ok(mq_sender.send(bytes)?)
        } else {
            Ok(())
        }
    }
}

#[pg_extern]
pub fn aggregate(
    index: PgRelation,
    query: SearchQueryInput,
    agg: Json,
    solve_mvcc: default!(bool, true),
    memory_limit: default!(i64, 500000000),
    bucket_limit: default!(i64, 65000),
) -> Result<JsonB, Box<dyn Error>> {
    unsafe {
        let reader = SearchIndexReader::open(&index, MvccSatisfies::Snapshot)?;
        let agg_req = serde_json::from_value(agg.0)?;
        let process = ParallelAggregation::new(
            index.oid(),
            &query,
            &agg_req,
            solve_mvcc,
            memory_limit.try_into()?,
            bucket_limit.try_into()?,
            reader.segment_ids(),
        )?;

        // limit number of workers to the number of segments
        let nworkers =
            (pg_sys::max_parallel_workers_per_gather as usize).min(reader.segment_readers().len());
        if let Some(mut process) = launch_parallel_process!(
            ParallelAggregation<ParallelAggregationWorker>,
            process,
            WorkerStyle::Query,
            nworkers,
            16384
        ) {
            // signal our workers with the number of workers actually launched
            // they need this before they can begin checking out the correct segment counts
            let mut nlaunched = process.launched_workers();
            if pg_sys::parallel_leader_participation {
                nlaunched += 1;
            }

            process
                .state_manager_mut()
                .object::<State>(0)?
                .unwrap()
                .set_launched_workers(nlaunched);

            // leader participation
            let mut agg_results = Vec::with_capacity(nlaunched);
            if pg_sys::parallel_leader_participation {
                let mut worker =
                    ParallelAggregationWorker::new_parallel_worker(*process.state_manager());
                if let Some(result) = worker.execute_aggregate(-1)? {
                    agg_results.push(Ok(result));
                }
            }

            // wait for workers to finish, collecting their intermediate aggregate results
            for (_worker_number, message) in process {
                let worker_results =
                    postcard::from_bytes::<IntermediateAggregationResults>(&message)?;

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
        } else {
            // couldn't launch any workers, so we just execute the aggregate right here in this backend
            let segment_ids = reader.segment_ids();
            let mut state = State {
                mutex: Spinlock::default(),
                nlaunched: 1,
                remaining_segments: segment_ids.len(),
            };
            let mut worker = ParallelAggregationWorker::new_local(
                agg_req.clone(),
                query,
                segment_ids,
                index.oid(),
                solve_mvcc,
                memory_limit as _,
                bucket_limit as _,
                &mut state,
            );
            if let Some(agg_results) = worker.execute_aggregate(-1)? {
                let result = agg_results.into_final_result(
                    agg_req,
                    AggregationLimitsGuard::new(
                        Some(memory_limit.try_into()?),
                        Some(bucket_limit.try_into()?),
                    ),
                )?;
                Ok(JsonB(serde_json::to_value(result)?))
            } else {
                Ok(JsonB(serde_json::Value::Null))
            }
        }
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
                pg_sys::ExecClearTuple(self.slot);
                if self.vmbuf != pg_sys::InvalidBuffer as pg_sys::Buffer {
                    pg_sys::ReleaseBuffer(self.vmbuf);
                }
            }
        }
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

                if pg_sys::visibilitymap_get_status(
                    (*self.scan).rel,
                    item_pointer_get_block_number(&self.tid),
                    &mut self.vmbuf,
                ) != 0
                {
                    return true;
                }

                let mut call_again = false;
                let mut all_dead = false;
                pg_sys::ExecClearTuple(self.slot);
                pg_sys::table_index_fetch_tuple(
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
