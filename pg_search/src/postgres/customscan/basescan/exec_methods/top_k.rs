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

use std::sync::{Arc, Mutex};

use arrow_array::{Float32Array, RecordBatch, UInt64Array};
use datafusion::execution::{SendableRecordBatchStream, TaskContext};
use datafusion::physical_plan::ExecutionPlan;
use tokio::runtime::Runtime;

use crate::aggregate::exec::AggregationExec;
use crate::api::version::VersionInfo;
use crate::api::{HashMap, OrderByInfo};
use crate::gucs;
use crate::gucs::WorkMem;
use crate::index::fast_fields_helper::{FFHelper, WhichFastField, build_arrow_schema};
use crate::index::reader::index::{MAX_TOPK_FEATURES, TopKAuxiliaryCollector};
use crate::postgres::customscan::aggregatescan::exec::AggregationResults;
use crate::postgres::customscan::aggregatescan::{AggIndexInfo, AggregateType};
use crate::postgres::customscan::basescan::exec_methods::{ExecMethod, ExecState};
use crate::postgres::customscan::basescan::projections::window_agg::WindowAggregateInfo;
use crate::postgres::customscan::basescan::scan_state::BaseScanState;
use crate::postgres::customscan::builders::custom_path::ExecMethodType;
use crate::postgres::customscan::limit_offset::LimitOffset;
use crate::postgres::customscan::mpp::interrupt::block_on_next;
use crate::postgres::heap::VisibilityChecker;
use crate::scan::execution_plan::{
    LazySortConfig, LazySortTelemetry, LazySortTelemetryState, PgSearchScanPlan, ScanState,
    ScannerConfig, SharedThresholdMode, TopKSelectivity, UnsafeSendSync,
};

use pgrx::{check_for_interrupts, pg_sys};
use tantivy::aggregation::AggregationLimitsGuard;
use tantivy::aggregation::agg_req::Aggregations;
use tantivy::aggregation::intermediate_agg_result::IntermediateAggregationResults;

struct PreparedAggregations {
    aggregations: Aggregations,
    combined_agg_types: Vec<AggregateType>,
    agg_index_to_te_index: Vec<usize>,
    /// Whether MVCC filtering should be enabled for these aggregations.
    /// Determined by the mvcc_visibility setting of any pdb.agg() calls.
    /// If any aggregate has MVCC disabled, this will be false.
    mvcc_enabled: bool,
}

pub struct TopKScanExecState {
    /// Resolved value of `LIMIT + OFFSET` (the K our scan must produce so PG's
    /// outer Limit node can apply OFFSET and return LIMIT). `Some` when both
    /// sides are static at construction; otherwise resolved in `init` from
    /// the `LimitOffset` carried on `ExecMethodType::TopK`.
    limit: Option<usize>,
    orderby_info: Option<Vec<OrderByInfo>>,

    visibility_selectivity: TopKSelectivity,
    // Window aggregates to compute
    window_aggregates: Vec<WindowAggregateInfo>,

    runtime: Option<Runtime>,
    stream: Option<SendableRecordBatchStream>,
    telemetry: Option<LazySortTelemetry>,
    pending_aggregations: Option<(PreparedAggregations, AggregationLimitsGuard)>,
    current_record_batch: Option<RecordBatch>,
    current_batch_row_idx: usize,
    did_query: bool,
    found: usize,
}

impl TopKScanExecState {
    pub fn new(
        heaprelid: pg_sys::Oid,
        limit_offset: &LimitOffset,
        orderby_info: Option<Vec<OrderByInfo>>,
    ) -> Self {
        if matches!(&orderby_info, Some(orderby_info) if orderby_info.len() > MAX_TOPK_FEATURES) {
            panic!("Cannot sort by more than {MAX_TOPK_FEATURES} features.");
        }

        let visibility_selectivity = TopKSelectivity::from_heap_visibility(heaprelid);

        // Resolve K eagerly when both sides are static; otherwise leave it as
        // None and resolve in `init` from the LimitOffset carried on
        // ExecMethodType::TopK.
        Self {
            limit: limit_offset.static_fetch(),
            orderby_info,
            visibility_selectivity,
            window_aggregates: Vec::new(),
            runtime: None,
            stream: None,
            telemetry: None,
            pending_aggregations: None,
            current_record_batch: None,
            current_batch_row_idx: 0,
            did_query: false,
            found: 0,
        }
    }

    fn limit(&self) -> usize {
        self.limit
            .expect("TopK limit must be resolved before query execution")
    }

<<<<<<< HEAD
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
    ///    necessary to allow for re-scans (when a Top K result later proves not to be visible)
    fn segments_to_query<'s>(
        &'s self,
        search_reader: &SearchIndexReader,
        parallel_state: Option<*mut ParallelScanState>,
    ) -> Box<dyn Iterator<Item = SegmentId> + 's> {
        match (parallel_state, self.claimed_segments.borrow().clone()) {
            (None, _) => {
                // Not parallel: will search all segments.
                let all_segments = search_reader.segment_ids();
                Box::new(all_segments.into_iter().inspect(|_segment_id| {
                    check_for_interrupts!();
                }))
            }
            (Some(_), Some(claimed_segments)) => {
                // Parallel, but we have already claimed our segments. Emit them again.
                Box::new(claimed_segments.into_iter().inspect(|_segment_id| {
                    check_for_interrupts!();
                }))
            }
            (Some(parallel_state), None) => {
                // Parallel, and we have not claimed our segments. Claim them, lazily, and then
                // record that we have done so.
                let mut claimed_segments = Vec::new();
                Box::new(std::iter::from_fn(move || {
                    check_for_interrupts!();
                    let maybe_segment_id =
                        unsafe { checkout_segment_for_source(parallel_state, 0) };
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

=======
>>>>>>> cb78a94 (chore: Break out a lazy sorted scan. (#7003))
    fn prepare_aggregations(&self, state: &mut BaseScanState) -> Option<PreparedAggregations> {
        if self.window_aggregates.is_empty() || state.window_aggregate_results.is_some() {
            // There are no aggregates, or we already executed them and stashed their results.
            return None;
        }

        // Combine all window aggregates from all TargetLists
        // This allows us to execute all aggregations in a single Tantivy pass
        // NOTE: This could also be done via a multi-collector.
        let mut combined_agg_types = Vec::new();
        let mut agg_index_to_te_index = Vec::new();
        for agg_info in &self.window_aggregates {
            for agg_type in agg_info.targetlist.aggregates() {
                combined_agg_types.push(agg_type.clone());
                agg_index_to_te_index.push(agg_info.target_entry_index);
            }
        }

        // Determine the query-level visibility setting, erroring on contradicting
        // settings, then resolve it against this execution. Only Custom aggregates
        // (pdb.agg) carry a setting; standard SQL aggregates always use the default.
        let mvcc_enabled = AggregateType::resolve_visibility(combined_agg_types.iter())
            .resolve_filtering(state.indexrel(), state.search_query_input(), None);

        // Convert aggregates to Tantivy Aggregations
        let mut aggregations: tantivy::aggregation::agg_req::Aggregations = Default::default();
        for (idx, agg_type) in combined_agg_types.iter().enumerate() {
            let mut agg = if let AggregateType::Custom { agg_json, .. } = agg_type {
                // For Custom aggregates, Tantivy's deserializer handles nested "aggs" automatically
                serde_json::from_value(agg_json.clone())
                    .unwrap_or_else(|e| panic!("Failed to deserialize custom aggregate: {}", e))
            } else {
                // For standard aggregates, convert to variant and wrap with empty sub_aggregation
                let agg_variant = agg_type.clone().into();
                tantivy::aggregation::agg_req::Aggregation {
                    agg: agg_variant,
                    sub_aggregation: Default::default(),
                }
            };
            if state
                .indexrel()
                .created_by_version()
                .stores_datetimes_in_i64()
            {
                // We need to rewrite date_histogram requests to regular histogram requests because
                // we are no longer storing dates in tantivy's DateTime.
                // We rewrite these here instead of at AggregateRequest construction time because we
                // need the unmodified json later to decide how to rewrite the results.
                crate::postgres::customscan::aggregatescan::json_rewrite::rewrite_date_histogram_to_histogram(&mut agg)
                    .expect("a valid date_histogram should always be a valid histogram");
            }
            aggregations.insert(idx.to_string(), agg);
        }

        Some(PreparedAggregations {
            aggregations,
            combined_agg_types,
            agg_index_to_te_index,
            mvcc_enabled,
        })
    }

    fn finalize_aggregates(
        &self,
        prepared: PreparedAggregations,
        agg_limits: AggregationLimitsGuard,
        intermediate_results: IntermediateAggregationResults,
        index_info: &AggIndexInfo,
    ) -> HashMap<usize, pg_sys::Datum> {
        let final_result = intermediate_results
            .into_final_result(prepared.aggregations, agg_limits)
            .expect("failed to finalize aggregate");

        // For window functions (no GROUP BY), we expect a single ungrouped result
        // Convert to AggregationResults and extract Datums
        let agg_results_wrapper: AggregationResults = final_result.into();
        let datum_vec = agg_results_wrapper
            .flatten_ungrouped_to_datums(&prepared.combined_agg_types, index_info);

        // Map aggregate results to target entry indices
        prepared
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

    fn create_lazy_sorted_stream(
        &mut self,
        state: &mut BaseScanState,
    ) -> Option<SendableRecordBatchStream> {
        if self.did_query {
            return None;
        }
        self.did_query = true;

        let prepared = self.prepare_aggregations(state);
        let bucket_limit: u32 = gucs::max_term_agg_buckets() as u32;
        let agg_limits = AggregationLimitsGuard::new(
            Some(WorkMem::Tantivy.bytes().try_into().unwrap()),
            Some(bucket_limit),
        );

        let first_pull_aux_collector = prepared.as_ref().map(|prepared| {
            let search_reader = state.search_reader.as_ref().unwrap();
            let (aggregation_collector, vischeck) = prepared.aggregations.plan(
                search_reader,
                state.heaprel(),
                prepared.mvcc_enabled,
                agg_limits.clone(),
                None,
            );
            TopKAuxiliaryCollector {
                aggregation_collector,
                vischeck,
            }
        });

        self.pending_aggregations = prepared.map(|p| (p, agg_limits));

        let telemetry: LazySortTelemetry =
            Arc::new(Mutex::new(UnsafeSendSync(LazySortTelemetryState {
                first_pull_aux_collector,
                queries_executed: 0,
                segment_info: Default::default(),
                first_pull_aggregation: None,
            })));
        self.telemetry = Some(Arc::clone(&telemetry));

        let search_reader = state.search_reader.as_ref().unwrap();
        let index_rel = state.indexrel.as_ref().unwrap();
        let heap_rel = state.heaprel.as_ref().unwrap();

        let which_fast_fields = vec![WhichFastField::Ctid, WhichFastField::Score(None)];
        let ffhelper = Arc::new(FFHelper::for_ctid(search_reader));
        let visibility = state
            .visibility_checker
            .as_ref()
            .expect("TopKScanExecState: visibility_checker should be initialized")
            .clone()
            .with_ffhelper(Arc::clone(&ffhelper))
            .with_check_visibility(false);

        let scanner_config = ScannerConfig {
            which_fast_fields: which_fast_fields.clone(),
            heap_relid: heap_rel.oid().to_u32(),
            batch_size_hint: None,
            score_needed: search_reader.need_scores(),
            scan_mode: crate::scan::ScanMode::all(),
        };

        let state_partition = ScanState {
            source_idx: None,
            planner_estimated_rows: 0,
            scanner_config,
            ffhelper: Arc::clone(&ffhelper),
            visibility: Box::new(visibility) as Box<VisibilityChecker>,
            reader: search_reader.clone(),
        };

        let lazy_sort = LazySortConfig {
            orderby_info: self.orderby_info.clone(),
            ordering: None,
            initial_window: self
                .visibility_selectivity
                .compute_initial_window(self.limit(), crate::gucs::limit_fetch_multiplier()),
            shared_threshold: SharedThresholdMode::EnabledOnFirstPull,
            telemetry: Some(telemetry),
        };

        let plan = PgSearchScanPlan::new(
            Some(state_partition),
            build_arrow_schema(&which_fast_fields),
            state.search_query_input().clone(),
            None,
            Vec::new(),
            None,
            index_rel.oid().to_u32(),
            None,
            1,
            state.parallel_state(),
            None,
        )
        .with_table_alias(index_rel.name())
        .with_lazy_sort(lazy_sort);

        let task_ctx = Arc::new(TaskContext::default());
        let stream = match plan.execute(0, task_ctx) {
            Ok(stream) => stream,
            Err(e) => {
                pgrx::error!("Failed to execute TopK scan plan: {e}");
            }
        };
        Some(stream)
    }

    fn drain_telemetry(&mut self, state: &mut BaseScanState) {
        let Some(ref telemetry) = self.telemetry else {
            return;
        };
        let (queries_delta, segment_info, first_pull_aggregation) = {
            let mut guard = telemetry.lock().unwrap();
            (
                std::mem::take(&mut guard.0.queries_executed),
                std::mem::take(&mut guard.0.segment_info),
                guard.0.first_pull_aggregation.take(),
            )
        };

        for _ in 0..queries_delta {
            state.increment_query_count();
        }
        if !segment_info.is_empty() {
            state.accumulate_segment_info(segment_info);
        }

        if let Some((prepared, agg_limits)) = self.pending_aggregations.take() {
            let (agg_result, claimed_segment_count) =
                first_pull_aggregation.expect("an aggregation request should produce a result");
            let intermediate_results = if let Some(parallel_state) = state.parallel_state() {
                let segment_count =
                    claimed_segment_count.expect("Should have claimed segments while running.");
                let parallel_state = unsafe { &mut *parallel_state };
                parallel_state
                    .aggregation_append(agg_result, segment_count)
                    .expect("Failed to append aggregation result");

                parallel_state.aggregation_wait()
            } else {
                agg_result
            };

            let search_reader = state.search_reader.as_ref().unwrap();
            let index_info = AggIndexInfo {
                created_by_version: search_reader.index_created_by_version(),
                schema: search_reader.schema().clone(),
            };
            let window_aggregate_results =
                self.finalize_aggregates(prepared, agg_limits, intermediate_results, &index_info);

            state.window_aggregate_results = Some(window_aggregate_results);
        }
    }
}

impl ExecMethod for TopKScanExecState {
    /// Initialize the exec method with data from the scan state
    fn init(&mut self, state: &mut BaseScanState, cstate: *mut pg_sys::CustomScanState) {
        // Resolve K = LIMIT + OFFSET from EState when not already known.
        // `fetch_mut` converts each `Param` → `Static` in place on the
        // `LimitOffset` carried by `ExecMethodType::TopK`, so EXPLAIN ANALYZE
        // sees resolved values without us holding a separate copy.
        if self.limit.is_none() {
            let estate = unsafe { (*cstate).ss.ps.state };
            if let ExecMethodType::TopK {
                ref mut limit_offset,
                ..
            } = state.exec_method_type
            {
                self.limit = limit_offset
                    .resolve_mut(estate)
                    .expect("LIMIT must be resolvable from EState (param missing or NULL)")
                    .static_fetch();
            }
        }

        // handle parameterized vectors
        let estate = unsafe { (*cstate).ss.ps.state };
        let planstate = unsafe { std::ptr::addr_of_mut!((*cstate).ss.ps) };
        if !estate.is_null() {
            let resolve = |infos: &mut [OrderByInfo]| {
                for info in infos.iter_mut() {
                    unsafe { info.resolve_query_vector(estate, planstate) };
                }
            };
            if let ExecMethodType::TopK {
                orderby_info: Some(infos),
                ..
            } = &mut state.exec_method_type
            {
                resolve(infos);
            }
            if let Some(infos) = self.orderby_info.as_mut() {
                resolve(infos);
            }
        }

        // Call the default init behavior first
        self.reset(state);

        // Transfer window aggregates from scan state to exec state
        self.window_aggregates = state.window_aggregates.clone();
    }

    fn query(&mut self, state: &mut BaseScanState) -> bool {
        if self.found >= self.limit() {
            return false;
        }

<<<<<<< HEAD
        // We track the total number of queries executed by Top K (for any of the above reasons).
        state.increment_query_count();

        // Calculate the limit for this query, and what the offset will be for the next query.
        let local_limit =
            (self.limit() as f64 * self.scale_factor).max(self.chunk_size as f64) as usize;
        let next_offset = self.offset + local_limit;

        let prepared = self.prepare_aggregations(state);

        // Use the GUC for term aggregation bucket limits (single source of truth).
        let bucket_limit: u32 = gucs::max_term_agg_buckets() as u32;

        let agg_limits = AggregationLimitsGuard::new(
            Some(WorkMem::Tantivy.bytes().try_into().unwrap()),
            Some(bucket_limit),
        );

        // Run the Top K (and optional aggregate) query.
        let mut search_results = if let Some(orderby_info) = self.orderby_info.as_ref() {
            let maybe_aux_collector = prepared.as_ref().map(|prepared| {
                let search_reader = self.search_reader.as_ref().unwrap();
                let (aggregation_collector, vischeck) = prepared.aggregations.plan(
                    search_reader,
                    state.heaprel(),
                    prepared.mvcc_enabled,
                    agg_limits.clone(),
                    None,
                );

                TopKAuxiliaryCollector {
                    aggregation_collector,
                    vischeck,
                }
            });
            // We are passing parallel_state because it contains the shared threshold. We only want
            // to use the shared threshold on the first query, as additional queries will
            // necessarily be below it.
            let maybe_parallel_state = if state.query_count() == 1 {
                state.parallel_state()
            } else {
                None
            };
            let mut top_k_search = self
                .search_reader
                .as_ref()
                .unwrap()
                .search_top_k_in_segments(
                    self.segments_to_query(
                        state.search_reader.as_ref().unwrap(),
                        state.parallel_state(),
                    ),
                    orderby_info,
                    local_limit,
                    self.offset,
                    maybe_aux_collector,
                    maybe_parallel_state,
                );
            // Per-segment Fruit JSON → local ScanTelemetry. Workers publish into
            // DSM once at EndCustomScan; the leader merges at Shutdown.
            if !top_k_search.segment_info.is_empty() {
                state.accumulate_segment_info(std::mem::take(&mut top_k_search.segment_info));
            }
            top_k_search
        } else {
            self.search_reader
                .as_ref()
                .unwrap()
                .search_top_k_unordered_in_segments(
                    self.segments_to_query(
                        state.search_reader.as_ref().unwrap(),
                        state.parallel_state(),
                    ),
                    local_limit,
                    self.offset,
                )
        };

        // If aggregates were executed, publish their results in our state for projection during
        // the scan.
        if let Some(prepared) = prepared {
            let intermediate_results = if self.orderby_info.is_some() {
                // Ordered TopK: aggregation was piggybacked on the search via aux collector
                let agg_result = search_results
                    .aggregation_results
                    .take()
                    .expect("an aggregation request should produce a result");
                if let Some(parallel_state) = state.parallel_state() {
                    let segment_count = self
                        .claimed_segments
                        .borrow()
                        .as_ref()
                        .expect("Should have claimed segments while running.")
                        .len();
                    let parallel_state = unsafe { &mut *parallel_state };
                    parallel_state
                        .aggregation_append(agg_result, segment_count)
                        .expect("Failed to append aggregation result");

                    parallel_state.aggregation_wait()
                } else {
                    agg_result
                }
            } else {
                // Unordered TopK: run a standalone aggregation query since the
                // unordered path does not support aggregation collectors
                let search_reader = self.search_reader.as_ref().unwrap();
                let (aggregation_collector, vischeck) = prepared.aggregations.plan(
                    search_reader,
                    state.heaprel(),
                    prepared.mvcc_enabled,
                    agg_limits.clone(),
                    None,
                );
                if let Some(vischeck) = vischeck {
                    search_reader.collect(MVCCFilterCollector::new(aggregation_collector, vischeck))
                } else {
                    search_reader.collect(aggregation_collector)
                }
            };

            let search_reader = state.search_reader.as_ref().unwrap();
            let index_info = AggIndexInfo {
                created_by_version: search_reader.index_created_by_version(),
                schema: search_reader.schema().clone(),
            };
            let window_aggregate_results =
                self.finalize_aggregates(prepared, agg_limits, intermediate_results, &index_info);

            state.window_aggregate_results = Some(window_aggregate_results);
=======
        if self.runtime.is_none() {
            self.runtime = Some(
                tokio::runtime::Builder::new_current_thread()
                    .build()
                    .expect("Failed to create tokio runtime for DataFusion stream execution"),
            );
>>>>>>> cb78a94 (chore: Break out a lazy sorted scan. (#7003))
        }

        loop {
            if let Some(stream) = &mut self.stream {
                let runtime = self.runtime.as_ref().unwrap();
                let polled = block_on_next(runtime, stream);
                self.drain_telemetry(state);
                match polled {
                    Some(Ok(batch)) => {
                        self.current_record_batch = Some(batch);
                        self.current_batch_row_idx = 0;
                        return true;
                    }
                    Some(Err(e)) => {
                        pgrx::error!("Error polling TopK DataFusion stream: {e}");
                    }
                    None => {
                        self.stream = None;
                        return false;
                    }
                }
            }

            match self.create_lazy_sorted_stream(state) {
                Some(stream) => {
                    self.stream = Some(stream);
                }
                None => {
                    return false;
                }
            }
        }
    }

    fn increment_visible(&mut self) {
        self.found += 1;
    }

    fn internal_next(&mut self, _state: &mut BaseScanState) -> ExecState {
        check_for_interrupts!();

        if self.found >= self.limit() {
            return ExecState::Eof;
        }

        let record_batch = match self.current_record_batch.as_ref() {
            Some(batch) => batch,
            None => return ExecState::Eof,
        };

        let row_idx = self.current_batch_row_idx;
        if row_idx >= record_batch.num_rows() {
            self.current_record_batch = None;
            return ExecState::Eof;
        }

        self.current_batch_row_idx += 1;

        let ctid_array = record_batch
            .column(0)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .expect("ctid column should be UInt64Array");
        let score_array = record_batch
            .column(1)
            .as_any()
            .downcast_ref::<Float32Array>()
            .expect("score column should be Float32Array");

        ExecState::FromHeap {
            ctid: ctid_array.value(row_idx),
            score: score_array.value(row_idx),
        }
    }

    fn reset(&mut self, state: &mut BaseScanState) {
        self.did_query = false;
        self.stream = None;
        self.telemetry = None;
        self.pending_aggregations = None;
        self.current_record_batch = None;
        self.current_batch_row_idx = 0;
        self.found = 0;

        // Get window aggregates from state if available
        if let ExecMethodType::TopK {
            window_aggregates, ..
        } = &state.exec_method_type
        {
            self.window_aggregates = window_aggregates.clone();
        }
    }
}
