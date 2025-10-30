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

use std::cell::RefCell;

use crate::aggregate::mvcc_collector::MVCCFilterCollector;
use crate::aggregate::vischeck::TSVisibilityChecker;
use crate::api::{HashMap, OrderByInfo};
use crate::gucs;
use crate::index::reader::index::{SearchIndexReader, TopNSearchResults, MAX_TOPN_FEATURES};
use crate::postgres::customscan::aggregatescan::exec::AggregationResults;
use crate::postgres::customscan::aggregatescan::AggregateType;
use crate::postgres::customscan::builders::custom_path::ExecMethodType;
use crate::postgres::customscan::pdbscan::exec_methods::{ExecMethod, ExecState};
use crate::postgres::customscan::pdbscan::parallel::checkout_segment;
use crate::postgres::customscan::pdbscan::projections::window_agg::WindowAggregateInfo;
use crate::postgres::customscan::pdbscan::scan_state::PdbScanState;
use crate::postgres::ParallelScanState;
use crate::query::SearchQueryInput;

use pgrx::{check_for_interrupts, direct_function_call, pg_sys, IntoDatum};
use tantivy::aggregation::agg_req::Aggregations;
use tantivy::aggregation::intermediate_agg_result::IntermediateAggregationResults;
use tantivy::aggregation::{AggregationLimitsGuard, DistributedAggregationCollector};
use tantivy::index::SegmentId;

struct PreparedAggregations {
    aggregations: Aggregations,
    combined_agg_types: Vec<AggregateType>,
    agg_index_to_te_index: Vec<usize>,
}

pub struct TopNScanExecState {
    // required
    limit: usize,
    orderby_info: Option<Vec<OrderByInfo>>,

    // set during init
    search_query_input: Option<SearchQueryInput>,
    search_reader: Option<SearchIndexReader>,

    // state tracking
    search_results: TopNSearchResults,
    nresults: usize,
    did_query: bool,
    exhausted: bool,
    found: usize,
    offset: usize,
    chunk_size: usize,
    // If parallel, the segments which have been claimed by this worker.
    claimed_segments: RefCell<Option<Vec<SegmentId>>>,
    scale_factor: f64,
    // Window aggregates to compute
    window_aggregates: Vec<WindowAggregateInfo>,
}

impl TopNScanExecState {
    pub fn new(
        heaprelid: pg_sys::Oid,
        limit: usize,
        orderby_info: Option<Vec<OrderByInfo>>,
    ) -> Self {
        if matches!(&orderby_info, Some(orderby_info) if orderby_info.len() > MAX_TOPN_FEATURES) {
            panic!("Cannot sort by more than {MAX_TOPN_FEATURES} features.");
        }

        // the scale_factor to multiply the limit that the query asked for by
        // calculated as (1 + (1 + n_dead) / (1 + n_live)) * limit_fetch_multiplier
        // where n_dead and n_live are the number of dead and live tuples in the heaprel
        // and limit_fetch_multiplier is a GUC
        let scale_factor = unsafe {
            let n_dead = direct_function_call::<i64>(
                pg_sys::pg_stat_get_dead_tuples,
                &[heaprelid.into_datum()],
            )
            .unwrap();
            let n_live = direct_function_call::<i64>(
                pg_sys::pg_stat_get_live_tuples,
                &[heaprelid.into_datum()],
            )
            .unwrap();

            1.0 + ((1.0 + n_dead as f64) / (1.0 + n_live as f64))
        } * crate::gucs::limit_fetch_multiplier();

        Self {
            limit,
            orderby_info,
            search_query_input: None,
            search_reader: None,
            search_results: TopNSearchResults::empty(),
            nresults: 0,
            did_query: false,
            exhausted: false,
            found: 0,
            offset: 0,
            chunk_size: 0,
            claimed_segments: RefCell::default(),
            scale_factor,
            window_aggregates: Vec::new(),
        }
    }

    /// Produces an iterator of Segments to query.
    ///
    /// This method produces segments in three different modes:
    /// 1. For non-parallel execution: emits all segments eagerly.
    /// 2. For parallel execution, depending on whether this exec has been executed before (since
    ///    being `reset`):
    ///    a. 0th execution: lazily emits segments, so that this worker will finish querying
    ///    one segment and adding it to the Collector before attempting to claim another. This
    ///    allows all of the workers to load balance the work of searching the segments.
    ///    b. Nth execution: eagerly emits all segments which were previously collected. This is
    ///    necessary to allow for re-scans (when a Top-N result later proves not to be visible)
    ///    to consistently revisit the same segments.
    fn segments_to_query<'s>(
        &'s self,
        search_reader: &SearchIndexReader,
        parallel_state: Option<*mut ParallelScanState>,
    ) -> Box<dyn Iterator<Item = SegmentId> + 's> {
        match (parallel_state, self.claimed_segments.borrow().clone()) {
            (None, _) => {
                // Not parallel: will search all segments.
                let all_segments = search_reader.segment_ids();
                Box::new(all_segments.into_iter().inspect(|segment_id| {
                    check_for_interrupts!();
                }))
            }
            (Some(_), Some(claimed_segments)) => {
                // Parallel, but we have already claimed our segments. Emit them again.
                Box::new(claimed_segments.into_iter().inspect(|segment_id| {
                    check_for_interrupts!();
                }))
            }
            (Some(parallel_state), None) => {
                // Parallel, and we have not claimed our segments. Claim them, lazily, and then
                // record that we have done so.
                let mut claimed_segments = Vec::new();
                Box::new(std::iter::from_fn(move || {
                    check_for_interrupts!();
                    let maybe_segment_id = unsafe { checkout_segment(parallel_state) };
                    let Some(segment_id) = maybe_segment_id else {
                        // No more segments: record that we successfully claimed all segments, and
                        // then conclude iteration.
                        *self.claimed_segments.borrow_mut() =
                            Some(std::mem::take(&mut claimed_segments));
                        return None;
                    };

                    // We claimed a segment. record it, and then return it.
                    claimed_segments.push(segment_id);
                    Some(segment_id)
                }))
            }
        }
    }

    fn prepare_aggregations(&self, state: &mut PdbScanState) -> Option<PreparedAggregations> {
        if self.window_aggregates.is_empty() {
            return None;
        }

        // Combine all window aggregates from all TargetLists
        // This allows us to execute all aggregations in a single Tantivy pass
        // TODO: This could also be done via a multi-collector.
        let mut combined_agg_types = Vec::new();
        let mut agg_index_to_te_index = Vec::new();
        for agg_info in &self.window_aggregates {
            for agg_type in agg_info.targetlist.aggregates() {
                combined_agg_types.push(agg_type.clone());
                agg_index_to_te_index.push(agg_info.target_entry_index);
            }
        }

        // Convert aggregates to Tantivy Aggregations
        let mut aggregations = tantivy::aggregation::agg_req::Aggregations::new();
        for (idx, agg_type) in combined_agg_types.iter().enumerate() {
            let agg_variant = agg_type.clone().into();
            aggregations.insert(
                idx.to_string(),
                tantivy::aggregation::agg_req::Aggregation {
                    agg: agg_variant,
                    sub_aggregation: tantivy::aggregation::agg_req::Aggregations::new(),
                },
            );
        }

        Some(PreparedAggregations {
            aggregations,
            combined_agg_types,
            agg_index_to_te_index,
        })
    }

    fn finalize_aggregates(
        &self,
        aggregations: PreparedAggregations,
        agg_limits: AggregationLimitsGuard,
        intermediate_results: IntermediateAggregationResults,
    ) -> HashMap<usize, pg_sys::Datum> {
        let final_result = intermediate_results
            .into_final_result(aggregations.aggregations, agg_limits)
            .expect("failed to finalize aggregate");

        // For window functions (no GROUP BY), we expect a single ungrouped result
        // Convert to AggregationResults and extract Datums
        let agg_results_wrapper: AggregationResults = final_result.into();
        let datum_vec =
            agg_results_wrapper.flatten_ungrouped_to_datums(&aggregations.combined_agg_types);

        // Map aggregate results to target entry indices
        aggregations
            .agg_index_to_te_index
            .into_iter()
            .enumerate()
            .map(|(agg_idx, te_idx)| {
                let datum = datum_vec
                    .get(agg_idx)
                    .and_then(|d| *d)
                    .unwrap_or(pg_sys::Datum::null());
                (te_idx, datum)
            })
            .collect()
    }
}

impl ExecMethod for TopNScanExecState {
    /// Initialize the exec method with data from the scan state
    fn init(&mut self, state: &mut PdbScanState, cstate: *mut pg_sys::CustomScanState) {
        // Call the default init behavior first
        self.reset(state);

        // Transfer window aggregates from scan state to exec state
        self.window_aggregates = state.window_aggregates.clone();
    }

    ///
    /// Query more results.
    ///
    /// Called either because:
    /// * We've never run a query before (did_query=False).
    /// * Some of the results that we returned were not visible, and so the `chunk_size`, or
    ///   `offset` values have changed.
    ///
    fn query(&mut self, state: &mut PdbScanState) -> bool {
        self.did_query = true;

        if self.found >= self.limit || self.exhausted {
            return false;
        }

        // We track the total number of queries executed by Top-N (for any of the above reasons).
        state.increment_query_count();

        // Calculate the limit for this query, and what the offset will be for the next query.
        let local_limit =
            (self.limit as f64 * self.scale_factor).max(self.chunk_size as f64) as usize;
        let next_offset = self.offset + local_limit;

        // TODO: only execute aggregations on our first query.
        let aggregations = self.prepare_aggregations(state);
        let agg_limits = AggregationLimitsGuard::new(
            Some(gucs::adjust_work_mem().get().try_into().unwrap()),
            // TODO: configure?
            Some(tantivy::aggregation::DEFAULT_BUCKET_LIMIT),
        );

        // Run the TopN (and optional aggregate) query.
        self.search_results = if let Some(orderby_info) = self.orderby_info.as_ref() {
            let maybe_aggregation_collector = aggregations.as_ref().map(|aggregations| {
                // Wrap the aggregation collector with MVCC filtering to respect transaction visibility.
                // This ensures that aggregations only include documents visible to the current transaction,
                // matching the behavior of the TopN results.
                let base_collector = DistributedAggregationCollector::from_aggs(
                    aggregations.aggregations.clone(),
                    agg_limits.clone(),
                );

                let heaprel = state.heaprel();
                MVCCFilterCollector::new(
                    base_collector,
                    TSVisibilityChecker::with_rel_and_snap(heaprel.as_ptr(), unsafe {
                        pg_sys::GetActiveSnapshot()
                    }),
                )
            });
            self.search_reader
                .as_ref()
                .unwrap()
                .search_top_n_in_segments(
                    self.segments_to_query(
                        state.search_reader.as_ref().unwrap(),
                        state.parallel_state,
                    ),
                    orderby_info,
                    local_limit,
                    self.offset,
                    maybe_aggregation_collector,
                )
        } else {
            self.search_reader
                .as_ref()
                .unwrap()
                .search_top_n_unordered_in_segments(
                    self.segments_to_query(
                        state.search_reader.as_ref().unwrap(),
                        state.parallel_state,
                    ),
                    local_limit,
                    self.offset,
                )
        };

        // If aggregates were executed, publish their results in our state for projection during
        // the scan.
        if let Some(aggregations) = aggregations {
            let agg_result = self
                .search_results
                .take_aggregation_results()
                .expect("an aggregation request should produce a result");
            let intermediate_results = if let Some(parallel_state) = state.parallel_state {
                let segment_count = self
                    .claimed_segments
                    .borrow()
                    .as_ref()
                    .expect("Should have claimed segments while running.")
                    .len();
                unsafe {
                    (*parallel_state)
                        .aggregation_append(agg_result, segment_count)
                        .expect("Failed to append aggregation result");

                    (*parallel_state).aggregation_wait()
                }
            } else {
                // Not parallel: finalize without propagating through the parallel state.
                agg_result
            };

            let window_aggregate_results =
                self.finalize_aggregates(aggregations, agg_limits, intermediate_results);

            pgrx::log!(">>> final aggregation result: {window_aggregate_results:?}");
            state.window_aggregate_results = Some(window_aggregate_results);
        }

        // Record the offset to start from for the next query.
        self.offset = next_offset;

        // If we got fewer results than we requested, then the query is exhausted: there is no
        // point executing further queries.
        self.exhausted = self.search_results.original_len() < local_limit;

        // But if we got any results at all, then the query was a success.
        self.search_results.original_len() > 0
    }

    fn increment_visible(&mut self) {
        self.found += 1;
    }

    fn internal_next(&mut self, state: &mut PdbScanState) -> ExecState {
        loop {
            check_for_interrupts!();

            match self.search_results.next() {
                None if !self.did_query => {
                    // we haven't even done a query yet, so this is our very first time in
                    return ExecState::Eof;
                }
                None | Some(_) if self.found >= self.limit => {
                    // we found all the matching rows
                    return ExecState::Eof;
                }
                Some((scored, doc_address)) => {
                    self.nresults += 1;
                    return ExecState::RequiresVisibilityCheck {
                        ctid: scored.ctid,
                        score: scored.bm25,
                        doc_address,
                    };
                }
                None => {
                    // Fall through to query more results.
                }
            }

            // set the chunk size to the scaling factor times the limit
            // on subsequent retries, multiply the chunk size by the scale factor
            // but do not exceed the max chunk size
            self.chunk_size = (self.chunk_size * crate::gucs::topn_retry_scale_factor() as usize)
                .max(
                    (self.limit as f64
                        * self.scale_factor
                        * crate::gucs::topn_retry_scale_factor() as f64)
                        as usize,
                )
                .min(crate::gucs::max_topn_chunk_size() as usize);

            // Then try querying again, and continue looping if we got more results.
            if !self.query(state) {
                return ExecState::Eof;
            }
        }
    }

    fn reset(&mut self, state: &mut PdbScanState) {
        // Reset state
        self.claimed_segments.take();
        self.did_query = false;
        self.exhausted = false;
        self.search_query_input = Some(state.search_query_input().clone());
        self.search_reader = state.search_reader.clone();
        self.search_results = TopNSearchResults::empty();

        // Get window aggregates from state if available
        if let ExecMethodType::TopN {
            window_aggregates, ..
        } = &state.exec_method_type
        {
            self.window_aggregates = window_aggregates.clone();
        }

        // Reset counters - excluding nresults which tracks processed results
        self.chunk_size = 0;
        self.found = 0;
        self.offset = 0;
    }
}
