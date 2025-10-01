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
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::spinlock::Spinlock;
use crate::postgres::storage::metadata::MetaPage;
use crate::query::SearchQueryInput;

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
struct ParallelAggregation {
    state: State,
    config: Config,
    query_bytes: Vec<u8>,
    agg_req_bytes: Vec<u8>,
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
            &self.agg_req_bytes,
            &self.query_bytes,
            &self.segment_ids,
            &self.ambulkdelete_epoch,
        ]
    }
}

impl ParallelAggregation {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        indexrelid: pg_sys::Oid,
        query: &SearchQueryInput,
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
            agg_req_bytes: serde_json::to_vec(aggregations)?,
            query_bytes: serde_json::to_vec(query)?,
            segment_ids,
            ambulkdelete_epoch,
        })
    }
}

struct ParallelAggregationWorker<'a> {
    state: &'a mut State,
    config: Config,
    aggregation: Aggregations,
    query: SearchQueryInput,
    segment_ids: Vec<(SegmentId, NumDeletedDocs)>,
    #[allow(dead_code)]
    ambulkdelete_epoch: u32,
}

impl<'a> ParallelAggregationWorker<'a> {
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
            aggregation,
            query,
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
            self.query.clone(),
            false,
            MvccSatisfies::ParallelWorker(segment_ids.clone()),
            NonNull::new(standalone_context),
            None,
        )?;

        let base_collector = DistributedAggregationCollector::from_aggs(
            self.aggregation.clone(),
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
        let agg_req_bytes = state_manager
            .slice::<u8>(2)
            .expect("wrong type for agg_req_bytes")
            .expect("missing agg_req_bytes value");
        let query_bytes = state_manager
            .slice::<u8>(3)
            .expect("wrong type for query_bytes")
            .expect("missing query_bytes value");
        let segment_ids = state_manager
            .slice::<(SegmentId, NumDeletedDocs)>(4)
            .expect("wrong type for segment_ids")
            .expect("missing segment_ids value");
        let ambulkdelete_epoch = state_manager
            .object::<u32>(5)
            .expect("wrong type for ambulkdelete_epoch")
            .expect("missing ambulkdelete_epoch value");

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

/// Execute aggregate with SearchQueryInput filters handled natively
/// This function creates FilterAggregation objects programmatically using SearchQueryInput
pub fn execute_aggregate_with_search_input_filters(
    index: &PgSearchRelation,
    base_query: SearchQueryInput,
    aggregate_types: Vec<crate::postgres::customscan::aggregatescan::privdat::AggregateType>,
    grouping_columns: Vec<crate::postgres::customscan::aggregatescan::privdat::GroupingColumn>,
    solve_mvcc: bool,
    memory_limit: u64,
    bucket_limit: u32,
) -> Result<serde_json::Value, Box<dyn Error>> {
    use crate::index::mvcc::MvccSatisfies;
    use crate::index::reader::index::SearchIndexReader;
    use crate::schema::SearchIndexSchema;
    use std::ptr::NonNull;

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

        // Get the schema for query conversion
        let schema = SearchIndexSchema::open(index)?;

        // Check if this is a GROUP BY query
        let final_aggregations = if grouping_columns.is_empty() {
            // Simple aggregations (no GROUP BY) - use existing logic
            build_simple_filter_aggregations(
                &aggregate_types,
                &schema,
                &reader,
                index,
                standalone_context,
            )?
        } else {
            // GROUP BY aggregations - build nested structure with FilterAggregations
            build_grouped_filter_aggregations(
                &aggregate_types,
                &grouping_columns,
                &schema,
                &reader,
                index,
                standalone_context,
            )?
        };

        // Execute directly with Tantivy aggregations (bypass JSON serialization)
        execute_aggregate_with_tantivy_aggregations(
            index,
            base_query,
            final_aggregations,
            solve_mvcc,
            memory_limit,
            bucket_limit,
        )
    }
}

/// Execute aggregate directly with Tantivy Aggregations object (no JSON serialization)
/// This bypasses the JSON serialization step that causes issues with FilterAggregation::new_with_query
pub fn execute_aggregate_with_tantivy_aggregations(
    index: &PgSearchRelation,
    query: SearchQueryInput,
    aggregations: tantivy::aggregation::agg_req::Aggregations,
    solve_mvcc: bool,
    memory_limit: u64,
    bucket_limit: u32,
) -> Result<serde_json::Value, Box<dyn Error>> {
    use crate::aggregate::mvcc_collector::MVCCFilterCollector;
    use crate::aggregate::vischeck::TSVisibilityChecker;
    use crate::index::mvcc::MvccSatisfies;
    use crate::index::reader::index::SearchIndexReader;
    use std::ptr::NonNull;
    use tantivy::aggregation::{AggregationCollector, AggregationLimitsGuard};

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

        // Create aggregation collector with proper limits
        let base_collector = AggregationCollector::from_aggs(
            aggregations,
            AggregationLimitsGuard::new(Some(memory_limit), Some(bucket_limit)),
        );

        // Apply MVCC filtering if requested (same logic as the multi-query approach)
        let agg_result = if solve_mvcc {
            let heaprel = index
                .heap_relation()
                .expect("index should belong to a heap relation");
            let mvcc_collector = MVCCFilterCollector::new(
                base_collector,
                TSVisibilityChecker::with_rel_and_snap(
                    heaprel.as_ptr(),
                    pg_sys::GetActiveSnapshot(),
                ),
            );
            reader.searcher().search(reader.query(), &mvcc_collector)?
        } else {
            reader.searcher().search(reader.query(), &base_collector)?
        };

        // Convert result to JSON
        let mut result_json = serde_json::to_value(agg_result)?;

        // Flatten FilterAggregation results to match expected structure
        flatten_filter_aggregation_results(&mut result_json);

        // Debug: Log the final result JSON
        pgrx::warning!("=== FilterAggregation Result JSON ===");
        pgrx::warning!(
            "Result: {}",
            serde_json::to_string_pretty(&result_json)
                .unwrap_or_else(|e| format!("Failed to serialize: {e}"))
        );

        pg_sys::FreeExprContext(standalone_context, true);
        Ok(result_json)
    }
}

/// Flatten FilterAggregation results to match the expected structure for existing result processing
/// Converts {"doc_count": X, "filtered_agg": {"value": Y}} to {"value": Y, "doc_count": X}
/// Preserves doc_count for empty result set detection
fn flatten_filter_aggregation_results(result_json: &mut serde_json::Value) {
    if let serde_json::Value::Object(ref mut obj) = result_json {
        for (_key, value) in obj.iter_mut() {
            if let serde_json::Value::Object(ref mut agg_obj) = value {
                // Check if this is a FilterAggregation result with nested structure
                if agg_obj.contains_key("doc_count") && agg_obj.contains_key("filtered_agg") {
                    // Extract the nested aggregation result and preserve doc_count
                    if let (Some(filtered_agg), Some(doc_count)) = (
                        agg_obj.get("filtered_agg").cloned(),
                        agg_obj.get("doc_count").cloned(),
                    ) {
                        // Merge the filtered_agg result with the doc_count
                        if let serde_json::Value::Object(mut filtered_obj) = filtered_agg {
                            filtered_obj.insert("doc_count".to_string(), doc_count);
                            *value = serde_json::Value::Object(filtered_obj);
                        } else {
                            // If filtered_agg is not an object, create a new object with both
                            let mut new_obj = serde_json::Map::new();
                            new_obj.insert("value".to_string(), filtered_agg);
                            new_obj.insert("doc_count".to_string(), doc_count);
                            *value = serde_json::Value::Object(new_obj);
                        }
                    }
                }
                // Also recursively process any nested objects (for grouped aggregations)
                else {
                    flatten_filter_aggregation_results(value);
                }
            }
        }
    }
}

/// Build simple filter aggregations (no GROUP BY)
fn build_simple_filter_aggregations(
    aggregate_types: &[crate::postgres::customscan::aggregatescan::privdat::AggregateType],
    schema: &crate::schema::SearchIndexSchema,
    reader: &crate::index::reader::index::SearchIndexReader,
    index: &crate::postgres::rel::PgSearchRelation,
    standalone_context: *mut pg_sys::ExprContext,
) -> Result<tantivy::aggregation::agg_req::Aggregations, Box<dyn std::error::Error>> {
    use std::collections::BTreeMap;
    use std::ptr::NonNull;
    use tantivy::aggregation::agg_req::{Aggregation, AggregationVariants, Aggregations};
    use tantivy::aggregation::bucket::FilterAggregation;

    let mut aggregations_map = BTreeMap::new();

    for (idx, aggregate_type) in aggregate_types.iter().enumerate() {
        let agg_name = idx.to_string(); // Use "0", "1", etc. to match existing result processing

        if let Some(filter_query) = aggregate_type.filter_expr() {
            // Convert SearchQueryInput to Tantivy Query object
            let tantivy_query = filter_query.into_tantivy_query(
                schema,
                &|| {
                    tantivy::query::QueryParser::for_index(
                        reader.searcher().index(),
                        schema.fields().map(|(field, _)| field).collect::<Vec<_>>(),
                    )
                },
                reader.searcher(),
                index.oid(),
                Some(index.heap_relation().ok_or("No heap relation")?.oid()),
                NonNull::new(standalone_context),
                None, // planstate
            )?;

            // Create the base aggregation (without filter) directly as Aggregation object
            let base_agg_type = aggregate_type.convert_filtered_aggregate_to_unfiltered();
            let base_agg_json = base_agg_type.to_json();
            let base_aggregation: Aggregation = serde_json::from_value(base_agg_json)?;

            // Create FilterAggregation using the Tantivy Query object directly
            let filter_agg = FilterAggregation::new_with_query(tantivy_query);

            // Create sub-aggregations map with the base aggregation
            let mut sub_aggs_map = std::collections::HashMap::new();
            sub_aggs_map.insert("filtered_agg".to_string(), base_aggregation);
            let sub_aggregations = Aggregations::from(sub_aggs_map);

            // Add the filter aggregation to the map
            aggregations_map.insert(
                agg_name,
                Aggregation {
                    agg: AggregationVariants::Filter(filter_agg),
                    sub_aggregation: sub_aggregations,
                },
            );
        } else {
            // No filter - wrap in FilterAggregation with match_all query to get doc_count
            let agg_json = aggregate_type.to_json();
            let base_aggregation: Aggregation = serde_json::from_value(agg_json)?;

            // Create a match_all query for non-filtered aggregations
            let match_all_query = Box::new(tantivy::query::AllQuery);

            // Create FilterAggregation with match_all query
            let filter_agg = FilterAggregation::new_with_query(match_all_query);

            // Create sub-aggregations map with the base aggregation
            let mut sub_aggs_map = std::collections::HashMap::new();
            sub_aggs_map.insert("filtered_agg".to_string(), base_aggregation);
            let sub_aggregations = Aggregations::from(sub_aggs_map);

            // Add the filter aggregation to the map
            aggregations_map.insert(
                agg_name,
                Aggregation {
                    agg: AggregationVariants::Filter(filter_agg),
                    sub_aggregation: sub_aggregations,
                },
            );
        }
    }

    // Create the final aggregation request
    Ok(Aggregations::from(
        aggregations_map
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>(),
    ))
}

/// Build grouped filter aggregations (with GROUP BY)
fn build_grouped_filter_aggregations(
    aggregate_types: &[crate::postgres::customscan::aggregatescan::privdat::AggregateType],
    grouping_columns: &[crate::postgres::customscan::aggregatescan::privdat::GroupingColumn],
    schema: &crate::schema::SearchIndexSchema,
    reader: &crate::index::reader::index::SearchIndexReader,
    index: &crate::postgres::rel::PgSearchRelation,
    standalone_context: *mut pg_sys::ExprContext,
) -> Result<tantivy::aggregation::agg_req::Aggregations, Box<dyn std::error::Error>> {
    use std::collections::BTreeMap;
    use std::ptr::NonNull;
    use tantivy::aggregation::agg_req::{Aggregation, AggregationVariants, Aggregations};
    use tantivy::aggregation::bucket::FilterAggregation;

    // APPROACH: Put FilterAggregation at the top level to avoid SegmentReader access issues
    // Each FilterAggregation contains a TermsAggregation as its sub-aggregation
    let mut root_aggregations_map = BTreeMap::new();

    // Build nested TermsAggregations for multiple GROUP BY columns
    // Start from the innermost level (metrics) and work outward
    let mut base_sub_aggs = std::collections::HashMap::new();

    // Collect all non-filtered aggregates for the base terms aggregation
    for (idx, aggregate_type) in aggregate_types.iter().enumerate() {
        if aggregate_type.filter_expr().is_none() {
            let agg_name = idx.to_string();
            let agg_json = aggregate_type.to_json();
            let aggregation: Aggregation = serde_json::from_value(agg_json)?;
            base_sub_aggs.insert(agg_name, aggregation);
        }
    }

    // Build nested structure for multiple grouping columns
    let mut current_aggs = if base_sub_aggs.is_empty() {
        std::collections::HashMap::new()
    } else {
        base_sub_aggs
    };

    // Build nested TermsAggregations from innermost to outermost
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

        // Create new level with this terms aggregation
        let mut new_level = std::collections::HashMap::new();
        new_level.insert(format!("group_{i}"), terms_aggregation);
        current_aggs = new_level;
    }

    // Add the nested structure to root if we have non-filtered aggregates
    if !current_aggs.is_empty() {
        root_aggregations_map.extend(current_aggs);
    }

    // Handle filtered aggregates - create FilterAggregation for each
    for (idx, aggregate_type) in aggregate_types.iter().enumerate() {
        if let Some(filter_query) = aggregate_type.filter_expr() {
            let agg_name = idx.to_string();

            // Convert SearchQueryInput to Tantivy Query object
            let tantivy_query = filter_query.into_tantivy_query(
                schema,
                &|| {
                    tantivy::query::QueryParser::for_index(
                        reader.searcher().index(),
                        schema.fields().map(|(field, _)| field).collect::<Vec<_>>(),
                    )
                },
                reader.searcher(),
                index.oid(),
                Some(index.heap_relation().ok_or("No heap relation")?.oid()),
                NonNull::new(standalone_context),
                None, // planstate
            )?;

            // Create the base aggregation (without filter)
            let base_agg_type = aggregate_type.convert_filtered_aggregate_to_unfiltered();
            let base_agg_json = base_agg_type.to_json();
            let base_aggregation: Aggregation = serde_json::from_value(base_agg_json)?;

            // Build nested TermsAggregations for this filtered aggregate
            let mut filter_current_aggs = std::collections::HashMap::new();
            filter_current_aggs.insert(agg_name.clone(), base_aggregation);

            // Build nested structure for multiple grouping columns
            for (_i, grouping_column) in grouping_columns.iter().enumerate().rev() {
                let terms_json = serde_json::json!({
                    "terms": {
                        "field": grouping_column.field_name,
                        "size": 65000,
                        "segment_size": 65000
                    }
                });

                let terms_aggregation = Aggregation {
                    agg: serde_json::from_value(terms_json)?,
                    sub_aggregation: Aggregations::from(filter_current_aggs.clone()),
                };

                // Create new level with this terms aggregation
                let mut new_level = std::collections::HashMap::new();
                new_level.insert("grouped".to_string(), terms_aggregation);
                filter_current_aggs = new_level;
            }

            // Create FilterAggregation with the nested structure
            let filter_agg = FilterAggregation::new_with_query(tantivy_query);
            let filter_aggregation = Aggregation {
                agg: AggregationVariants::Filter(filter_agg),
                sub_aggregation: Aggregations::from(filter_current_aggs),
            };

            // Use a name that includes the original index for result processing
            root_aggregations_map.insert(format!("filter_{idx}"), filter_aggregation);
        }
    }

    Ok(Aggregations::from(
        root_aggregations_map
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>(),
    ))
}

pub fn execute_aggregate(
    index: &PgSearchRelation,
    query: SearchQueryInput,
    agg: serde_json::Value,
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
        let agg_req = serde_json::from_value(agg)?;
        let ambulkdelete_epoch = MetaPage::open(index).ambulkdelete_epoch();
        let segment_ids = reader
            .segment_readers()
            .iter()
            .map(|r| (r.segment_id(), r.num_deleted_docs()))
            .collect::<Vec<_>>();
        let process = ParallelAggregation::new(
            index.oid(),
            &query,
            &agg_req,
            solve_mvcc,
            memory_limit,
            bucket_limit,
            segment_ids,
            ambulkdelete_epoch,
        )?;

        // limit number of workers to the number of segments
        let mut nworkers =
            (pg_sys::max_parallel_workers_per_gather as usize).min(reader.segment_readers().len());

        if nworkers > 0 && pg_sys::parallel_leader_participation {
            // make sure to account for the leader being a worker too
            nworkers -= 1;
        }
        pgrx::debug1!(
            "requesting {nworkers} parallel workers, with parallel_leader_participation={}",
            *std::ptr::addr_of!(pg_sys::parallel_leader_participation)
        );
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
            pgrx::debug1!("launched {nlaunched} workers");
            if pg_sys::parallel_leader_participation {
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

            // leader participation
            let mut agg_results = Vec::with_capacity(nlaunched);
            if pg_sys::parallel_leader_participation {
                let mut worker =
                    ParallelAggregationWorker::new_parallel_worker(*process.state_manager());
                if let Some(result) = worker.execute_aggregate(QueryWorkerStyle::ParallelLeader)? {
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
                    AggregationLimitsGuard::new(Some(memory_limit), Some(bucket_limit)),
                );
                collector.merge_fruits(agg_results)?.into_final_result(
                    agg_req,
                    AggregationLimitsGuard::new(Some(memory_limit), Some(bucket_limit)),
                )?
            };

            Ok(serde_json::to_value(merged)?)
        } else {
            // couldn't launch any workers, so we just execute the aggregate right here in this backend
            let segment_ids = reader
                .segment_readers()
                .iter()
                .map(|r| (r.segment_id(), r.num_deleted_docs()))
                .collect::<Vec<_>>();
            let mut state = State {
                mutex: Spinlock::default(),
                nlaunched: 1,
                remaining_segments: segment_ids.len(),
            };
            let mut worker = ParallelAggregationWorker::new_local(
                agg_req.clone(),
                query,
                segment_ids,
                ambulkdelete_epoch,
                index.oid(),
                solve_mvcc,
                memory_limit as _,
                bucket_limit as _,
                &mut state,
            );
            if let Some(agg_results) = worker.execute_aggregate(QueryWorkerStyle::NonParallel)? {
                let result = agg_results.into_final_result(
                    agg_req,
                    AggregationLimitsGuard::new(Some(memory_limit), Some(bucket_limit)),
                )?;
                Ok(serde_json::to_value(result)?)
            } else {
                Ok(serde_json::Value::Null)
            }
        }
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
