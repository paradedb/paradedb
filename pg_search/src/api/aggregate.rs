use crate::api::aggregate::mvcc_collector::MVCCFilterCollector;
use crate::api::aggregate::vischeck::TSVisibilityChecker;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::parallel_worker::mqueue::MessageQueueSender;
use crate::postgres::parallel_worker::{begin_parallel_process, ParallelProcess, ParallelWorker};
use crate::query::SearchQueryInput;
use pgrx::{default, pg_extern, pg_sys, Json, JsonB, PgRelation};
use std::error::Error;
use std::ffi::c_void;
use tantivy::aggregation::{AggregationCollector, AggregationLimitsGuard};

pub struct ParallelAggregationProcess;

impl ParallelProcess for ParallelAggregationProcess {
    fn empty() -> Self
    where
        Self: Sized,
    {
        Self
    }

    fn state(&self) -> Vec<u8> {
        vec![]
    }

    fn create_worker(
        &self,
        state: *mut c_void,
        mq_sender: MessageQueueSender,
    ) -> Box<dyn ParallelWorker> {
        Box::new(ParallelAggregationWorker { sender: mq_sender })
    }
}

pub struct ParallelAggregationWorker {
    sender: MessageQueueSender,
}

impl ParallelWorker for ParallelAggregationWorker {
    unsafe fn run(&mut self) {
        for i in 1..100 {
            self.sender
                .send(format!(
                    "hello, world from ParallelWorkerNumber {}, msg #={i}",
                    unsafe { pg_sys::ParallelWorkerNumber },
                ))
                .expect("should be able to send message");
        }
        pgrx::warning!("sent 100 messages from {}", unsafe {
            pg_sys::ParallelWorkerNumber
        });
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
    let process = begin_parallel_process(ParallelAggregationProcess, bucket_limit as _)
        .expect("failed to start parallel process");

    for (worker_number, message) in process {
        pgrx::warning!(
            "worker #{worker_number} says: {}",
            std::str::from_utf8(&message).unwrap()
        );
    }

    unsafe {
        let heaprel = indexrel.heap_relation().unwrap();
        let reader = SearchIndexReader::open(&indexrel, MvccSatisfies::Snapshot)?;
        let agg_req = serde_json::from_value(agg.0)?;

        // Create the base aggregation collector
        let base_collector = AggregationCollector::from_aggs(
            agg_req,
            AggregationLimitsGuard::new(
                Some(memory_limit.try_into()?),
                Some(bucket_limit.try_into()?),
            ),
        );

        // Wrap in MVCC collector if needed
        let search_results = if solve_mvcc {
            let mvcc_collector = MVCCFilterCollector::new(
                base_collector,
                TSVisibilityChecker::with_rel_and_snap(
                    heaprel.as_ptr(),
                    pg_sys::GetActiveSnapshot(),
                ),
            );
            reader.collect(&query, mvcc_collector, false)
        } else {
            reader.collect(&query, base_collector, false)
        };

        let results = serde_json::to_value(search_results)?;
        Ok(JsonB(results))
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
