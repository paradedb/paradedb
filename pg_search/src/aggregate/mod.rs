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

mod agg_builder;
pub mod agg_result;
pub mod agg_spec;

use std::error::Error;
use std::ptr::NonNull;

use crate::aggregate::agg_spec::AggregationSpec;
use crate::aggregate::mvcc_collector::MVCCFilterCollector;
use crate::aggregate::vischeck::TSVisibilityChecker;
use crate::api::OrderByInfo;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::launch_parallel_process;
use crate::parallel_worker::mqueue::MessageQueueSender;
use crate::parallel_worker::ParallelStateManager;
use crate::parallel_worker::{chunk_range, QueryWorkerStyle, WorkerStyle};
use crate::parallel_worker::{ParallelProcess, ParallelState, ParallelStateType, ParallelWorker};
use crate::postgres::customscan::aggregatescan::privdat::{AggregateType, GroupingColumn};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::spinlock::Spinlock;
use crate::postgres::storage::metadata::MetaPage;
use crate::postgres::utils::ExprContextGuard;
use crate::query::QueryContext;
use crate::query::SearchQueryInput;
// Re-export AggQueryBuilder as the main public interface
pub use agg_builder::AggQueryBuilder;

use pgrx::{check_for_interrupts, pg_sys};
use rustc_hash::FxHashSet;
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

type NumDeletedDocs = u32;

/// Type alias for parallel aggregation result type
type ParallelAggregationResult = Result<
    Option<Vec<Result<IntermediateAggregationResults, tantivy::TantivyError>>>,
    Box<dyn Error>,
>;

/// Aggregation mode for parallel execution
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum AggregationMode {
    /// SQL aggregations with FILTER support
    Sql {
        aggregate_types: Vec<AggregateType>,
        grouping_columns: Vec<GroupingColumn>,
        orderby_info: Vec<OrderByInfo>,
        limit: Option<u32>,
        offset: Option<u32>,
    },
    /// JSON aggregations (direct Tantivy aggregation API)
    Json { aggregations: Aggregations },
}

/// Parallel process for all aggregations (unified)
struct ParallelAggregation {
    state: State,
    config: Config,
    base_query_bytes: Vec<u8>,
    mode_bytes: Vec<u8>, // Serialized AggregationMode
    segment_ids: Vec<(SegmentId, NumDeletedDocs)>,
    ambulkdelete_epoch: u32,
}

impl ParallelStateType for State {}
impl ParallelStateType for Config {}
impl ParallelStateType for (SegmentId, NumDeletedDocs) {}

impl ParallelProcess for ParallelAggregation {
    fn state_values(&self) -> Vec<&dyn ParallelState> {
        vec![
            &self.state,
            &self.config,
            &self.base_query_bytes,
            &self.mode_bytes,
            &self.segment_ids,
            &self.ambulkdelete_epoch,
        ]
    }
}

impl ParallelAggregation {
    /// Create for SQL aggregations
    pub fn new(
        indexrelid: pg_sys::Oid,
        builder: &AggQueryBuilder,
        solve_mvcc: bool,
        memory_limit: u64,
        bucket_limit: u32,
        segment_ids: Vec<(SegmentId, NumDeletedDocs)>,
        ambulkdelete_epoch: u32,
    ) -> anyhow::Result<Self> {
        let mode = AggregationMode::Sql {
            aggregate_types: builder.agg_spec.aggs.clone(),
            grouping_columns: builder.agg_spec.groupby.clone(),
            orderby_info: builder.orderby_info.to_vec(),
            limit: *builder.limit,
            offset: *builder.offset,
        };

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
            base_query_bytes: serde_json::to_vec(builder.base_query)?,
            mode_bytes: serde_json::to_vec(&mode)?,
            segment_ids,
            ambulkdelete_epoch,
        })
    }

    /// Create for JSON aggregations
    #[allow(clippy::too_many_arguments)]
    pub fn new_json(
        indexrelid: pg_sys::Oid,
        base_query: &SearchQueryInput,
        aggregations: &Aggregations,
        solve_mvcc: bool,
        memory_limit: u64,
        bucket_limit: u32,
        segment_ids: Vec<(SegmentId, NumDeletedDocs)>,
        ambulkdelete_epoch: u32,
    ) -> anyhow::Result<Self> {
        let mode = AggregationMode::Json {
            aggregations: aggregations.clone(),
        };

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
            base_query_bytes: serde_json::to_vec(base_query)?,
            mode_bytes: serde_json::to_vec(&mode)?,
            segment_ids,
            ambulkdelete_epoch,
        })
    }
}

struct ParallelAggregationWorker<'a> {
    state: &'a mut State,
    config: Config,
    base_query: SearchQueryInput,
    mode: AggregationMode,
    segment_ids: Vec<(SegmentId, NumDeletedDocs)>,
    #[allow(dead_code)]
    ambulkdelete_epoch: u32,
}

impl<'a> ParallelAggregationWorker<'a> {
    /// Create for non-parallel local execution
    #[allow(clippy::too_many_arguments)]
    fn new_local(
        aggregation: Aggregations,
        query: SearchQueryInput,
        segment_ids: Vec<(SegmentId, NumDeletedDocs)>,
        ambulkdelete_epoch: u32,
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
            base_query: query,
            mode: AggregationMode::Json {
                aggregations: aggregation,
            },
            segment_ids,
            ambulkdelete_epoch,
        }
    }

    fn checkout_segments(&mut self, worker_number: i32) -> FxHashSet<SegmentId> {
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
        self.segment_ids
            .get(self.state.remaining_segments)
            .cloned()
            .map(|(segment_id, _)| segment_id)
    }

    fn execute_aggregate(
        &mut self,
        worker_style: QueryWorkerStyle,
    ) -> anyhow::Result<Option<IntermediateAggregationResults>> {
        let segment_ids = self.checkout_segments(worker_style.worker_number());
        if segment_ids.is_empty() {
            return Ok(None);
        }
        let indexrel =
            PgSearchRelation::with_lock(self.config.indexrelid, pg_sys::AccessShareLock as _);
        let standalone_context = ExprContextGuard::new();
        let reader = SearchIndexReader::open_with_context(
            &indexrel,
            self.base_query.clone(),
            false,
            MvccSatisfies::ParallelWorker(segment_ids.clone()),
            NonNull::new(standalone_context.as_ptr()),
            None,
        )?;

        // Build or use pre-built aggregations
        let aggregations = match &self.mode {
            AggregationMode::Json { aggregations } => {
                // JSON aggregations: use pre-built Aggregations
                aggregations.clone()
            }
            AggregationMode::Sql {
                aggregate_types,
                grouping_columns,
                orderby_info,
                limit,
                offset,
            } => {
                // SQL aggregations: rebuild with Query objects in this worker
                let schema = indexrel
                    .schema()
                    .map_err(|e| anyhow::anyhow!("Failed to get schema: {}", e))?;
                let qctx = QueryContext::new(&schema, &reader, &indexrel, standalone_context);

                // Reconstruct AggregationSpec from deserialized fields
                let agg_spec = AggregationSpec {
                    aggs: aggregate_types.clone(),
                    groupby: grouping_columns.clone(),
                };

                let builder =
                    AggQueryBuilder::new(&self.base_query, &agg_spec, orderby_info, limit, offset);
                builder
                    .build_aggregation_query_from_search_input(&qctx)
                    .map_err(|e| anyhow::anyhow!("Failed to build filter aggregations: {}", e))?
            }
        };

        let base_collector = DistributedAggregationCollector::from_aggs(
            aggregations,
            AggregationLimitsGuard::new(
                Some(self.config.memory_limit),
                Some(self.config.bucket_limit),
            ),
        );

        let start = std::time::Instant::now();
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
            reader.collect(mvcc_collector)
        } else {
            reader.collect(base_collector)
        };
        pgrx::debug1!(
            "Worker #{}: collected {segment_ids:?} in {:?}",
            unsafe { pg_sys::ParallelWorkerNumber },
            start.elapsed()
        );
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
        let base_query_bytes = state_manager
            .slice::<u8>(2)
            .expect("wrong type for base_query_bytes")
            .expect("missing base_query_bytes value");
        let mode_bytes = state_manager
            .slice::<u8>(3)
            .expect("wrong type for mode_bytes")
            .expect("missing mode_bytes value");
        let segment_ids = state_manager
            .slice::<(SegmentId, NumDeletedDocs)>(4)
            .expect("wrong type for segment_ids")
            .expect("missing segment_ids value");
        let ambulkdelete_epoch = state_manager
            .object::<u32>(5)
            .expect("wrong type for ambulkdelete_epoch")
            .expect("missing ambulkdelete_epoch value");

        let base_query = serde_json::from_slice::<SearchQueryInput>(base_query_bytes)
            .expect("base_query_bytes should deserialize into SearchQueryInput");
        let mode = serde_json::from_slice::<AggregationMode>(mode_bytes)
            .expect("mode_bytes should deserialize into AggregationMode");

        Self {
            state,
            config: *config,
            base_query,
            mode,
            segment_ids: segment_ids.to_vec(),
            ambulkdelete_epoch: *ambulkdelete_epoch,
        }
    }

    fn run(mut self, mq_sender: &MessageQueueSender, worker_number: i32) -> anyhow::Result<()> {
        // wait for all workers to launch
        while self.state.launched_workers() == 0 {
            check_for_interrupts!();
            std::thread::yield_now();
        }

        if let Some(intermediate_results) =
            self.execute_aggregate(QueryWorkerStyle::ParallelWorker(worker_number))?
        {
            let bytes = postcard::to_allocvec(&intermediate_results)?;
            Ok(mq_sender.send(bytes)?)
        } else {
            Ok(())
        }
    }
}

/// Execute aggregations (with parallelization)
///
/// Main entry point for SQL aggregations. Handles both simple aggregations and GROUP BY.
/// Supports parallel execution.
///
/// # Arguments
/// * `index` - Postgres relation to aggregate over
/// * `builder` - Aggregation builder with query parameters
/// * `solve_mvcc` - Apply MVCC visibility filtering
/// * `memory_limit` - Max memory for aggregation (typically work_mem)
/// * `bucket_limit` - Max buckets for GROUP BY (default: 65000)
///
/// # Returns
/// JSON with structure: `{"filter_0": {"doc_count": N, "filtered_agg": {...}}, ...}`
pub fn execute_aggregation(
    index: &PgSearchRelation,
    builder: &AggQueryBuilder,
    solve_mvcc: bool,
    memory_limit: u64,
    bucket_limit: u32,
) -> Result<serde_json::Value, Box<dyn Error>> {
    let (reader, standalone_context, ambulkdelete_epoch, segment_ids) =
        open_index_for_aggregation(index, builder.base_query, MvccSatisfies::Snapshot)?;

    let schema = index
        .schema()
        .map_err(|e| -> Box<dyn Error> { Box::new(e) })?;

    let qctx = QueryContext::new(&schema, &reader, index, standalone_context);

    let aggregations = builder.build_aggregation_query_from_search_input(&qctx)?;

    // Determine if we can use parallel execution
    let (can_use_parallel, nworkers) = can_parallelize(&segment_ids);

    // Execute aggregation (parallel or sequential)
    if can_use_parallel {
        // Parallel execution
        let process = ParallelAggregation::new(
            index.oid(),
            builder,
            solve_mvcc,
            memory_limit,
            bucket_limit,
            segment_ids.clone(),
            ambulkdelete_epoch,
        )?;

        match execute_parallel_helper(process, nworkers)? {
            Some(agg_results) => {
                return merge_parallel_results(
                    aggregations,
                    agg_results,
                    memory_limit,
                    bucket_limit,
                );
            }
            None => {
                // Parallel execution not available (not enough workers launched)
                // Fall through to sequential execution
            }
        }
    }

    // Sequential execution
    execute_sequential(
        aggregations,
        builder.base_query,
        ambulkdelete_epoch,
        segment_ids,
        index.oid(),
        solve_mvcc,
        memory_limit,
        bucket_limit,
    )
}

/// Execute parallel aggregation and return intermediate results
fn execute_parallel_helper(
    process: ParallelAggregation,
    nworkers: usize,
) -> ParallelAggregationResult {
    pgrx::debug1!(
        "requesting {nworkers} parallel workers, with parallel_leader_participation={}",
        unsafe { *std::ptr::addr_of!(pg_sys::parallel_leader_participation) }
    );

    if let Some(mut process) = launch_parallel_process!(
        ParallelAggregation<ParallelAggregationWorker>,
        process,
        WorkerStyle::Query,
        nworkers,
        16384
    ) {
        // Signal workers with the actual number launched
        let mut nlaunched = process.launched_workers();
        pgrx::debug1!("launched {nlaunched} parallel workers");

        if unsafe { pg_sys::parallel_leader_participation } {
            nlaunched += 1;
            pgrx::debug1!(
                "with parallel_leader_participation=true, actual worker count={nlaunched}"
            );
        }

        process
            .state_manager_mut()
            .object::<State>(0)?
            .unwrap()
            .set_launched_workers(nlaunched);

        // Leader participation
        let mut agg_results = Vec::with_capacity(nlaunched);
        if unsafe { pg_sys::parallel_leader_participation } {
            let mut worker =
                ParallelAggregationWorker::new_parallel_worker(*process.state_manager());
            match worker.execute_aggregate(QueryWorkerStyle::ParallelLeader) {
                Ok(Some(result)) => agg_results.push(Ok(result)),
                Ok(None) => {}
                Err(e) => return Err(e.into()),
            }
        }

        // Wait for workers to finish, collecting their intermediate aggregate results
        for (_worker_number, message) in process {
            match postcard::from_bytes::<IntermediateAggregationResults>(&message) {
                Ok(worker_results) => agg_results.push(Ok(worker_results)),
                Err(e) => return Err(e.into()),
            }
        }

        Ok(Some(agg_results))
    } else {
        Ok(None)
    }
}

/// Execute sequential aggregation using pre-built aggregations
#[allow(clippy::too_many_arguments)]
fn execute_sequential(
    aggregations: Aggregations,
    base_query: &SearchQueryInput,
    ambulkdelete_epoch: u32,
    segment_ids: Vec<(SegmentId, NumDeletedDocs)>,
    indexrelid: pg_sys::Oid,
    solve_mvcc: bool,
    memory_limit: u64,
    bucket_limit: u32,
) -> Result<serde_json::Value, Box<dyn Error>> {
    pgrx::debug1!("executing aggregation sequentially");

    let mut state = State {
        mutex: Spinlock::default(),
        nlaunched: 1,
        remaining_segments: segment_ids.len(),
    };

    let mut worker = ParallelAggregationWorker::new_local(
        aggregations.clone(),
        base_query.clone(),
        segment_ids,
        ambulkdelete_epoch,
        indexrelid,
        solve_mvcc,
        memory_limit,
        bucket_limit,
        &mut state,
    );

    if let Some(agg_results) = worker.execute_aggregate(QueryWorkerStyle::NonParallel)? {
        let result = agg_results.into_final_result(
            aggregations,
            AggregationLimitsGuard::new(Some(memory_limit), Some(bucket_limit)),
        )?;
        Ok(serde_json::to_value(result)?)
    } else {
        Ok(serde_json::Value::Null)
    }
}

/// Execute aggregations from JSON (legacy API used by aggregate() SQL function)
///
/// This is for raw Tantivy aggregation JSON without SQL FILTER clause support.
/// Uses ParallelAggregation infrastructure which can serialize Aggregations directly
/// (works because legacy JSON API doesn't contain Query objects).
pub fn execute_aggregate_json(
    index: &PgSearchRelation,
    query: &SearchQueryInput,
    agg_json: serde_json::Value,
    solve_mvcc: bool,
    memory_limit: u64,
    bucket_limit: u32,
) -> Result<serde_json::Value, Box<dyn Error>> {
    let (_reader, _standalone_context, ambulkdelete_epoch, segment_ids) =
        open_index_for_aggregation(index, query, MvccSatisfies::Snapshot)?;

    let agg_req: Aggregations = serde_json::from_value(agg_json)?;

    // Determine if we can use parallel execution
    let (can_use_parallel, nworkers) = can_parallelize(&segment_ids);

    // Execute aggregation (parallel or sequential)
    if can_use_parallel {
        let process = ParallelAggregation::new_json(
            index.oid(),
            query,
            &agg_req,
            solve_mvcc,
            memory_limit,
            bucket_limit,
            segment_ids.clone(),
            ambulkdelete_epoch,
        )?;

        if let Some(agg_results) = execute_parallel_helper(process, nworkers)? {
            return merge_parallel_results(agg_req, agg_results, memory_limit, bucket_limit);
        }
        // If parallel worker launch failed (launch_parallel_process! returned None),
        // fall back to sequential execution
    }

    // Sequential execution (or fallback if parallel workers couldn't be launched)
    execute_sequential(
        agg_req,
        query,
        ambulkdelete_epoch,
        segment_ids,
        index.oid(),
        solve_mvcc,
        memory_limit,
        bucket_limit,
    )
}

/// Type alias for the return value of open_index_for_aggregation
type IndexAggregationContext = (
    SearchIndexReader,
    ExprContextGuard,
    u32,
    Vec<(SegmentId, NumDeletedDocs)>,
);

/// Helper to open index reader with standalone context
/// Returns (reader, standalone_context, ambulkdelete_epoch, segment_ids)
fn open_index_for_aggregation(
    index: &PgSearchRelation,
    query: &SearchQueryInput,
    mvcc_satisfies: MvccSatisfies,
) -> Result<IndexAggregationContext, Box<dyn Error>> {
    let standalone_context = ExprContextGuard::new();
    let reader = SearchIndexReader::open_with_context(
        index,
        query.clone(),
        false,
        mvcc_satisfies,
        NonNull::new(standalone_context.as_ptr()),
        None,
    )?;

    let ambulkdelete_epoch = MetaPage::open(index).ambulkdelete_epoch();
    let segment_ids = reader
        .segment_readers()
        .iter()
        .map(|r| (r.segment_id(), r.num_deleted_docs()))
        .collect::<Vec<_>>();

    Ok((reader, standalone_context, ambulkdelete_epoch, segment_ids))
}

/// Merge parallel aggregation results into final JSON
/// Common helper for both SQL and JSON aggregations
fn merge_parallel_results(
    aggregations: Aggregations,
    agg_results: Vec<tantivy::Result<IntermediateAggregationResults>>,
    memory_limit: u64,
    bucket_limit: u32,
) -> Result<serde_json::Value, Box<dyn Error>> {
    let collector = DistributedAggregationCollector::from_aggs(
        aggregations.clone(),
        AggregationLimitsGuard::new(Some(memory_limit), Some(bucket_limit)),
    );
    let merged = collector.merge_fruits(agg_results)?.into_final_result(
        aggregations,
        AggregationLimitsGuard::new(Some(memory_limit), Some(bucket_limit)),
    )?;
    Ok(serde_json::to_value(merged)?)
}

/// Determine if parallel execution is possible based on available workers and segments
fn can_parallelize(segment_ids: &[(SegmentId, NumDeletedDocs)]) -> (bool, usize) {
    let mut nworkers =
        unsafe { (pg_sys::max_parallel_workers_per_gather as usize).min(segment_ids.len()) };

    if nworkers > 0 && unsafe { pg_sys::parallel_leader_participation } {
        nworkers -= 1;
    }

    let can_use_parallel = nworkers > 0 && segment_ids.len() > 1;
    (can_use_parallel, nworkers)
}

pub mod mvcc_collector {
    use parking_lot::Mutex;
    use std::sync::Arc;
    use tantivy::collector::{Collector, SegmentCollector};

    use crate::aggregate::vischeck::TSVisibilityChecker;
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
                ctids_buffer: Vec::new(),
                filtered_buffer: Vec::new(),
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
        ctids_buffer: Vec<Option<u64>>,
        filtered_buffer: Vec<u32>,
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
            // Get the ctids for these docs.
            if self.ctids_buffer.len() < docs.len() {
                self.ctids_buffer.resize(docs.len(), None);
            }
            self.ctid_ff
                .as_u64s(docs, &mut self.ctids_buffer[..docs.len()]);

            // Determine which ctids are visible.
            self.filtered_buffer.clear();
            let mut vischeck = self.lock.lock();
            for (doc, ctid) in docs.iter().zip(self.ctids_buffer.iter()) {
                let ctid = ctid.expect("ctid should be present");
                if vischeck.is_visible(ctid) {
                    self.filtered_buffer.push(*doc);
                }
            }
            drop(vischeck);

            self.inner.collect_block(&self.filtered_buffer);
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
