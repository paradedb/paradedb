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

use std::error::Error;
use std::ptr::NonNull;

use crate::aggregate::mvcc_collector::MVCCFilterCollector;
use crate::aggregate::vischeck::TSVisibilityChecker;
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
use crate::query::SearchQueryInput;
use crate::schema::SearchIndexSchema;

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

/// Parallel process for all aggregations (unified)
struct ParallelAggregation {
    state: State,
    config: Config,
    base_query_bytes: Vec<u8>,
    // For SQL aggregations (with heap filter support)
    aggregate_types_bytes: Vec<u8>,
    grouping_columns_bytes: Vec<u8>,
    // For JSON aggregations (legacy API) - mutually exclusive with above
    agg_json_bytes: Vec<u8>,
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
            &self.aggregate_types_bytes,
            &self.grouping_columns_bytes,
            &self.agg_json_bytes,
            &self.segment_ids,
            &self.ambulkdelete_epoch,
        ]
    }
}

impl ParallelAggregation {
    /// Create for SQL aggregations
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        indexrelid: pg_sys::Oid,
        base_query: &SearchQueryInput,
        aggregate_types: &[AggregateType],
        grouping_columns: &[GroupingColumn],
        solve_mvcc: bool,
        memory_limit: u64,
        bucket_limit: u32,
        segment_ids: Vec<(SegmentId, NumDeletedDocs)>,
        ambulkdelete_epoch: u32,
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
            base_query_bytes: serde_json::to_vec(base_query)?,
            aggregate_types_bytes: serde_json::to_vec(aggregate_types)?,
            grouping_columns_bytes: serde_json::to_vec(grouping_columns)?,
            agg_json_bytes: Vec::new(), // Empty for filter aggregations
            segment_ids,
            ambulkdelete_epoch,
        })
    }

    /// Create for JSON aggregations (legacy API)
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
            aggregate_types_bytes: Vec::new(), // Empty for JSON aggregations
            grouping_columns_bytes: Vec::new(), // Empty for JSON aggregations
            agg_json_bytes: serde_json::to_vec(aggregations)?,
            segment_ids,
            ambulkdelete_epoch,
        })
    }
}

struct ParallelAggregationWorker<'a> {
    state: &'a mut State,
    config: Config,
    base_query: SearchQueryInput,
    // For filter aggregations
    aggregate_types: Vec<AggregateType>,
    grouping_columns: Vec<GroupingColumn>,
    // For JSON aggregations - mutually exclusive with above (aggregate_types and grouping_columns)
    agg_json: Option<Aggregations>,
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
            aggregate_types: Vec::new(),
            grouping_columns: Vec::new(),
            agg_json: Some(aggregation),
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
        let standalone_context = unsafe { pg_sys::CreateStandaloneExprContext() };
        let reader = SearchIndexReader::open_with_context(
            &indexrel,
            self.base_query.clone(),
            false,
            MvccSatisfies::ParallelWorker(segment_ids.clone()),
            NonNull::new(standalone_context),
            None,
        )?;

        // Build or use pre-built aggregations
        let aggregations = if let Some(ref agg_json) = self.agg_json {
            // JSON aggregations: use pre-built Aggregations
            agg_json.clone()
        } else {
            // Filter aggregations: rebuild with Query objects in this worker
            let schema = SearchIndexSchema::open(&indexrel)?;
            build_filter_aggregations(
                &self.base_query,
                &self.aggregate_types,
                &self.grouping_columns,
                &schema,
                &reader,
                &indexrel,
                standalone_context,
            )
            .map_err(|e| anyhow::anyhow!("Failed to build filter aggregations: {}", e))?
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
        unsafe { pg_sys::FreeExprContext(standalone_context, true) };
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
        let aggregate_types_bytes = state_manager
            .slice::<u8>(3)
            .expect("wrong type for aggregate_types_bytes")
            .expect("missing aggregate_types_bytes value");
        let grouping_columns_bytes = state_manager
            .slice::<u8>(4)
            .expect("wrong type for grouping_columns_bytes")
            .expect("missing grouping_columns_bytes value");
        let agg_json_bytes = state_manager
            .slice::<u8>(5)
            .expect("wrong type for agg_json_bytes")
            .expect("missing agg_json_bytes value");
        let segment_ids = state_manager
            .slice::<(SegmentId, NumDeletedDocs)>(6)
            .expect("wrong type for segment_ids")
            .expect("missing segment_ids value");
        let ambulkdelete_epoch = state_manager
            .object::<u32>(7)
            .expect("wrong type for ambulkdelete_epoch")
            .expect("missing ambulkdelete_epoch value");

        let base_query = serde_json::from_slice::<SearchQueryInput>(base_query_bytes)
            .expect("base_query_bytes should deserialize into SearchQueryInput");

        // Check if this is a JSON aggregation or filter aggregation
        let (aggregate_types, grouping_columns, agg_json) = if !agg_json_bytes.is_empty() {
            // JSON aggregation
            let agg = serde_json::from_slice::<Aggregations>(agg_json_bytes)
                .expect("agg_json_bytes should deserialize into Aggregations");
            (Vec::new(), Vec::new(), Some(agg))
        } else {
            // Filter aggregation
            let agg_types = serde_json::from_slice::<Vec<AggregateType>>(aggregate_types_bytes)
                .expect("aggregate_types_bytes should deserialize into Vec<AggregateType>");
            let group_cols = serde_json::from_slice::<Vec<GroupingColumn>>(grouping_columns_bytes)
                .expect("grouping_columns_bytes should deserialize into Vec<GroupingColumn>");
            (agg_types, group_cols, None)
        };

        Self {
            state,
            config: *config,
            base_query,
            aggregate_types,
            grouping_columns,
            agg_json,
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

/// Execute aggregations with SQL FILTER clause support (with parallelization)
///
/// Main entry point for aggregations with filter support. Handles both simple aggregations
/// and GROUP BY using Tantivy's FilterAggregation feature. Supports parallel execution.
///
/// # Arguments
/// * `base_query` - WHERE clause (defines document set to aggregate)
/// * `aggregate_types` - Aggregates with optional FILTER clauses  
/// * `grouping_columns` - GROUP BY columns (empty for simple aggregations)
/// * `solve_mvcc` - Apply MVCC visibility filtering
/// * `memory_limit` - Max memory for aggregation (typically work_mem)
/// * `bucket_limit` - Max buckets for GROUP BY (default: 65000)
///
/// # Returns
/// JSON with structure: `{"filter_0": {"doc_count": N, "filtered_agg": {...}}, ...}`
pub fn execute_aggregation(
    index: &PgSearchRelation,
    base_query: &SearchQueryInput,
    aggregate_types: &[AggregateType],
    grouping_columns: &[GroupingColumn],
    solve_mvcc: bool,
    memory_limit: u64,
    bucket_limit: u32,
) -> Result<serde_json::Value, Box<dyn Error>> {
    unsafe {
        let standalone_context = pg_sys::CreateStandaloneExprContext();
        let reader = SearchIndexReader::open_with_context(
            index,
            base_query.clone(),
            false,
            MvccSatisfies::Snapshot,
            NonNull::new(standalone_context),
            None,
        )?;

        let ambulkdelete_epoch = MetaPage::open(index).ambulkdelete_epoch();
        let segment_ids = reader
            .segment_readers()
            .iter()
            .map(|r| (r.segment_id(), r.num_deleted_docs()))
            .collect::<Vec<_>>();

        // limit number of workers to the number of segments
        let mut nworkers =
            (pg_sys::max_parallel_workers_per_gather as usize).min(segment_ids.len());

        if nworkers > 0 && pg_sys::parallel_leader_participation {
            // make sure to account for the leader being a worker too
            nworkers -= 1;
        }

        // Use parallel execution if we have multiple segments and parallel workers are enabled
        if nworkers > 0 && segment_ids.len() > 1 {
            execute_aggregation_parallel(
                index,
                base_query,
                aggregate_types,
                grouping_columns,
                solve_mvcc,
                memory_limit,
                bucket_limit,
                segment_ids,
                ambulkdelete_epoch,
                nworkers,
            )
        } else {
            execute_aggregation_sequential(
                index,
                base_query,
                aggregate_types,
                grouping_columns,
                solve_mvcc,
                memory_limit,
                bucket_limit,
            )
        }
    }
}

/// Sequential execution path for filter aggregations (single worker)
fn execute_aggregation_sequential(
    index: &PgSearchRelation,
    base_query: &SearchQueryInput,
    aggregate_types: &[AggregateType],
    grouping_columns: &[GroupingColumn],
    solve_mvcc: bool,
    memory_limit: u64,
    bucket_limit: u32,
) -> Result<serde_json::Value, Box<dyn Error>> {
    unsafe {
        let standalone_context = pg_sys::CreateStandaloneExprContext();
        let reader = SearchIndexReader::open_with_context(
            index,
            base_query.clone(),
            false,
            MvccSatisfies::Snapshot,
            NonNull::new(standalone_context),
            None,
        )?;

        let schema = SearchIndexSchema::open(index)?;
        let ambulkdelete_epoch = MetaPage::open(index).ambulkdelete_epoch();
        let segment_ids = reader
            .segment_readers()
            .iter()
            .map(|r| (r.segment_id(), r.num_deleted_docs()))
            .collect::<Vec<_>>();

        // Build filter aggregations
        let aggregations = build_filter_aggregations(
            base_query,
            aggregate_types,
            grouping_columns,
            &schema,
            &reader,
            index,
            standalone_context,
        )?;

        execute_aggregation_sequential_helper(
            index,
            base_query,
            aggregations,
            ambulkdelete_epoch,
            segment_ids,
            solve_mvcc,
            memory_limit,
            bucket_limit,
        )
    }
}

/// Parallel execution path for filter aggregations (multiple workers)
#[allow(clippy::too_many_arguments)]
fn execute_aggregation_parallel(
    index: &PgSearchRelation,
    base_query: &SearchQueryInput,
    aggregate_types: &[AggregateType],
    grouping_columns: &[GroupingColumn],
    solve_mvcc: bool,
    memory_limit: u64,
    bucket_limit: u32,
    segment_ids: Vec<(SegmentId, NumDeletedDocs)>,
    ambulkdelete_epoch: u32,
    nworkers: usize,
) -> Result<serde_json::Value, Box<dyn Error>> {
    let process = ParallelAggregation::new(
        index.oid(),
        base_query,
        aggregate_types,
        grouping_columns,
        solve_mvcc,
        memory_limit,
        bucket_limit,
        segment_ids,
        ambulkdelete_epoch,
    )?;

    if let Some(agg_results) = execute_aggregation_parallel_helper(process, nworkers)? {
        // Need to build aggregations to merge results (can't serialize Query objects)
        let standalone_context = unsafe { pg_sys::CreateStandaloneExprContext() };
        let reader = SearchIndexReader::open_with_context(
            index,
            base_query.clone(),
            false,
            MvccSatisfies::Snapshot,
            NonNull::new(standalone_context),
            None,
        )?;
        let schema = SearchIndexSchema::open(index)?;

        let aggregations = build_filter_aggregations(
            base_query,
            aggregate_types,
            grouping_columns,
            &schema,
            &reader,
            index,
            standalone_context,
        )
        .map_err(|e| -> Box<dyn Error> { Box::new(std::io::Error::other(e.to_string())) })?;

        return merge_parallel_results(aggregations, agg_results, memory_limit, bucket_limit);
    }

    // Parallel execution not available, fall back to sequential
    execute_aggregation_sequential(
        index,
        base_query,
        aggregate_types,
        grouping_columns,
        solve_mvcc,
        memory_limit,
        bucket_limit,
    )
}

/// Common parallel execution helper for all aggregations
fn execute_aggregation_parallel_helper(
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

/// Helper for converting SearchQueryInput to Tantivy Query
/// Eliminates duplication of query conversion logic
struct QueryConverter<'a> {
    schema: &'a crate::schema::SearchIndexSchema,
    reader: &'a SearchIndexReader,
    index: &'a PgSearchRelation,
    context: *mut pg_sys::ExprContext,
}

impl<'a> QueryConverter<'a> {
    /// Convert filter expression or use AllQuery for non-filtered aggregates
    fn convert_filter(
        &self,
        filter: Option<&SearchQueryInput>,
    ) -> Result<Box<dyn tantivy::query::Query>, Box<dyn Error>> {
        Ok(match filter {
            Some(query) => query.clone().into_tantivy_query(
                self.schema,
                &|| {
                    tantivy::query::QueryParser::for_index(
                        self.reader.searcher().index(),
                        self.schema.fields().map(|(f, _)| f).collect(),
                    )
                },
                self.reader.searcher(),
                self.index.oid(),
                self.index.heap_relation().map(|r| r.oid()),
                NonNull::new(self.context),
                None,
            )?,
            None => Box::new(tantivy::query::AllQuery),
        })
    }
}

/// Helper for building nested TermsAggregations
/// Eliminates duplication of nested aggregation building logic
struct TermsAggregationBuilder;

impl TermsAggregationBuilder {
    /// Build nested TermsAggregations from innermost to outermost
    /// Returns a map with "grouped" as the key for the outermost terms aggregation
    /// This is used for GROUP BY queries where FilterAggregation is at the top level
    fn build_nested(
        grouping_columns: &[GroupingColumn],
        leaf_aggs: std::collections::HashMap<String, tantivy::aggregation::agg_req::Aggregation>,
    ) -> Result<
        std::collections::HashMap<String, tantivy::aggregation::agg_req::Aggregation>,
        Box<dyn Error>,
    > {
        let mut current = leaf_aggs;

        // Build from innermost to outermost, reversing the order
        for column in grouping_columns.iter().rev() {
            let terms_agg = tantivy::aggregation::agg_req::Aggregation {
                agg: serde_json::from_value(serde_json::json!({
                    "terms": {
                        "field": column.field_name,
                        "size": 65000,
                        "segment_size": 65000
                    }
                }))?,
                sub_aggregation: tantivy::aggregation::agg_req::Aggregations::from(current),
            };

            let mut next_level = std::collections::HashMap::new();
            // Use "grouped" as the key name for all levels
            // The transformation function will convert this to group_0, group_1, etc.
            next_level.insert("grouped".to_string(), terms_agg);
            current = next_level;
        }

        Ok(current)
    }
}

/// Create base aggregation without filter wrapper
/// Centralizes the logic for creating unfiltered aggregations
fn create_base_aggregation(
    agg_type: &AggregateType,
) -> Result<tantivy::aggregation::agg_req::Aggregation, Box<dyn Error>> {
    let unfiltered = if agg_type.filter_expr().is_some() {
        agg_type.convert_filtered_aggregate_to_unfiltered()
    } else {
        agg_type.clone()
    };
    Ok(serde_json::from_value(unfiltered.to_json())?)
}

/// Unified function to build filter aggregations - works for both simple and grouped cases
/// This replaces both build_simple_filter_aggregations and build_grouped_filter_aggregations
/// Reduces complexity by having a single code path for all aggregation scenarios
fn build_filter_aggregations(
    base_query: &crate::query::SearchQueryInput,
    aggregate_types: &[AggregateType],
    grouping_columns: &[GroupingColumn],
    schema: &crate::schema::SearchIndexSchema,
    reader: &crate::index::reader::index::SearchIndexReader,
    index: &crate::postgres::rel::PgSearchRelation,
    context: *mut pg_sys::ExprContext,
) -> Result<tantivy::aggregation::agg_req::Aggregations, Box<dyn Error>> {
    use tantivy::aggregation::agg_req::{Aggregation, AggregationVariants, Aggregations};
    use tantivy::aggregation::bucket::FilterAggregation;

    let converter = QueryConverter {
        schema,
        reader,
        index,
        context,
    };

    // Special case: GROUP BY without aggregates
    if aggregate_types.is_empty() && !grouping_columns.is_empty() {
        // Build nested terms aggregations with group_X naming at each level
        let mut current_aggs = std::collections::HashMap::new();
        for (i, grouping_column) in grouping_columns.iter().enumerate().rev() {
            let terms_json = serde_json::json!({
                "terms": {
                    "field": grouping_column.field_name,
                    "size": 65000,
                    "segment_size": 65000
                }
            });

            let terms_aggregation = Aggregation {
                agg: serde_json::from_value(terms_json)?,
                sub_aggregation: Aggregations::from(current_aggs.clone()),
            };

            let mut new_level = std::collections::HashMap::new();
            new_level.insert(format!("group_{i}"), terms_aggregation);
            current_aggs = new_level;
        }

        return Ok(Aggregations::from(current_aggs));
    }

    // If no grouping, build simple filter aggregations
    if grouping_columns.is_empty() {
        let mut leaf_aggs = std::collections::HashMap::new();
        for (idx, agg_type) in aggregate_types.iter().enumerate() {
            let filter_query = converter.convert_filter(agg_type.filter_expr().as_ref())?;
            let base_agg = create_base_aggregation(agg_type)?;

            // Wrap in FilterAggregation
            let mut filter_sub_aggs = std::collections::HashMap::new();
            filter_sub_aggs.insert("filtered_agg".to_string(), base_agg);

            let filter_agg = Aggregation {
                agg: AggregationVariants::Filter(FilterAggregation::new_with_query(filter_query)),
                sub_aggregation: Aggregations::from(filter_sub_aggs),
            };

            leaf_aggs.insert(format!("filter_{idx}"), filter_agg);
        }
        return Ok(Aggregations::from(leaf_aggs));
    }

    // For GROUP BY: Put FilterAggregation at the TOP level with TermsAggregations inside
    // This is required because FilterAggregation needs direct SegmentReader access
    // Structure: filter_0 -> grouped -> buckets -> [leaf metrics]
    let mut root_aggs = std::collections::HashMap::new();

    // Always add a sentinel aggregation to ensure all groups are present
    // This is necessary because filtered aggregates only generate groups that match their filters
    // The sentinel uses the base query (WHERE clause) to generate ALL groups matching the query
    // This ensures we get ALL groups from the base result set, not just those matching aggregate filters
    let base_query_tantivy = converter.convert_filter(Some(base_query))?;
    let sentinel_terms =
        TermsAggregationBuilder::build_nested(grouping_columns, std::collections::HashMap::new())?;
    let sentinel_filter = Aggregation {
        agg: AggregationVariants::Filter(FilterAggregation::new_with_query(base_query_tantivy)),
        sub_aggregation: Aggregations::from(sentinel_terms),
    };
    root_aggs.insert("filter_sentinel".to_string(), sentinel_filter);

    for (idx, agg_type) in aggregate_types.iter().enumerate() {
        let filter_query = converter.convert_filter(agg_type.filter_expr().as_ref())?;
        let base_agg = create_base_aggregation(agg_type)?;

        // Create leaf with the metric aggregation
        let mut metric_aggs = std::collections::HashMap::new();
        metric_aggs.insert(idx.to_string(), base_agg);

        // Build nested TermsAggregations with metric at the leaf
        let terms_structure = TermsAggregationBuilder::build_nested(grouping_columns, metric_aggs)?;

        // Wrap in FilterAggregation
        let filter_agg = Aggregation {
            agg: AggregationVariants::Filter(FilterAggregation::new_with_query(filter_query)),
            sub_aggregation: Aggregations::from(terms_structure),
        };

        root_aggs.insert(format!("filter_{idx}"), filter_agg);
    }

    Ok(Aggregations::from(root_aggs))
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
    unsafe {
        let standalone_context = pg_sys::CreateStandaloneExprContext();
        let reader = SearchIndexReader::open_with_context(
            index,
            query.clone(),
            false,
            MvccSatisfies::Snapshot,
            NonNull::new(standalone_context),
            None,
        )?;

        // Parse JSON into Tantivy Aggregations (no Query objects, so serializable)
        let agg_req: Aggregations = serde_json::from_value(agg_json)?;

        let ambulkdelete_epoch = MetaPage::open(index).ambulkdelete_epoch();
        let segment_ids = reader
            .segment_readers()
            .iter()
            .map(|r| (r.segment_id(), r.num_deleted_docs()))
            .collect::<Vec<_>>();

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

        // Limit number of workers to the number of segments
        let mut nworkers =
            (pg_sys::max_parallel_workers_per_gather as usize).min(reader.segment_readers().len());

        if nworkers > 0 && pg_sys::parallel_leader_participation {
            nworkers -= 1;
        }

        if let Some(agg_results) = execute_aggregation_parallel_helper(process, nworkers)? {
            merge_parallel_results(agg_req, agg_results, memory_limit, bucket_limit)
        } else {
            // No parallel execution available, fall back to sequential
            execute_aggregation_sequential_helper(
                index,
                query,
                agg_req,
                ambulkdelete_epoch,
                segment_ids,
                solve_mvcc,
                memory_limit,
                bucket_limit,
            )
        }
    }
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

/// Sequential execution helper using ParallelAggregationWorker::new_local
/// This provides a common execution path for both filter and JSON aggregations
#[allow(clippy::too_many_arguments)]
fn execute_aggregation_sequential_helper(
    index: &PgSearchRelation,
    query: &SearchQueryInput,
    aggregations: Aggregations,
    ambulkdelete_epoch: u32,
    segment_ids: Vec<(SegmentId, NumDeletedDocs)>,
    solve_mvcc: bool,
    memory_limit: u64,
    bucket_limit: u32,
) -> Result<serde_json::Value, Box<dyn Error>> {
    pgrx::debug1!("executing aggregation sequentially with ParallelAggregationWorker::new_local");

    let mut state = State {
        mutex: Spinlock::default(),
        nlaunched: 1,
        remaining_segments: segment_ids.len(),
    };

    let mut worker = ParallelAggregationWorker::new_local(
        aggregations.clone(),
        query.clone(),
        segment_ids,
        ambulkdelete_epoch,
        index.oid(),
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
