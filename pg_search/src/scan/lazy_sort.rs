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

//! Expanding-window Top-K ("lazy sort") configuration and stream execution for
//! [`PgSearchScanPlan`](crate::scan::execution_plan::PgSearchScanPlan).
//!
//! When a `PgSearchScanPlan` is configured with [`LazySortConfig`], `execute()` delegates to
//! [`build_lazy_sort_stream`] instead of `build_standard_stream`:
//! rather than streaming every matching document segment-by-segment, the scan issues iterative,
//! expanding-window Top-K queries (`SearchIndexReader::search_top_k_in_segments`) and materializes
//! each window in sort order until the upstream consumer (e.g. `BaseScan`'s `LIMIT` or a Top-K join)
//! is satisfied or the index is exhausted.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use arrow_array::RecordBatch;
use arrow_schema::SchemaRef;
use datafusion::common::{DataFusionError, Result};
use datafusion::physical_expr::{LexOrdering, PhysicalExpr};
use datafusion::physical_plan::metrics::{Count, MetricBuilder, RecordOutput};
use futures::Stream;
use tantivy::aggregation::intermediate_agg_result::IntermediateAggregationResults;
use tantivy::index::SegmentId;

use crate::api::OrderByInfo;
use crate::index::reader::index::{
    MultiSegmentSearchResults, SearchIndexReader, TopKAuxiliaryCollector, TopKLimit, TopKOffset,
    TopKSearch, TopKWindow,
};
use crate::index::stats::{PartitionSegments, segments_for_partition};
use crate::postgres::ParallelScanState;
use crate::postgres::customscan::mpp::interrupt::{cancel_pending, interrupted};
use crate::postgres::customscan::parallel::checkout_segment_for_source;
use crate::query::SearchQueryInput;
use crate::scan::Scanner;
use crate::scan::execution_plan::{
    FilterStepMetrics, ScanStreamContext, ScannerConfig, UnsafeSendSync, build_filters,
};
use crate::scan::pre_filter::{PreFilter, PreFilters};
use crate::scan::range_partitioning::RangePartitioning;

/// Controls whether the cross-worker shared threshold in `ParallelScanState` is passed
/// to Tantivy's Top-K collector on the initial window pull.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SharedThresholdMode {
    EnabledOnFirstPull,
    Disabled,
}

/// Estimated probability `(0.0, 1.0]` that a candidate document from a Top-K scan
/// survives downstream filtering (e.g., MVCC heap visibility in `BaseScan`).
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct TopKSelectivity(pub f64);

impl TopKSelectivity {
    /// Estimates the MVCC visibility selectivity of `heaprelid` from PostgreSQL's
    /// live and dead tuple statistics using Laplace smoothing:
    /// `p_live = (1 + n_live) / ((1 + n_live) + (1 + n_dead))`.
    pub fn from_heap_visibility(heaprelid: pgrx::pg_sys::Oid) -> Self {
        let (n_dead, n_live) = unsafe {
            let n_dead = pgrx::direct_function_call::<i64>(
                pgrx::pg_sys::pg_stat_get_dead_tuples,
                &[pgrx::IntoDatum::into_datum(heaprelid)],
            )
            .unwrap();
            let n_live = pgrx::direct_function_call::<i64>(
                pgrx::pg_sys::pg_stat_get_live_tuples,
                &[pgrx::IntoDatum::into_datum(heaprelid)],
            )
            .unwrap();
            (n_dead.max(0) as f64, n_live.max(0) as f64)
        };
        let live = 1.0 + n_live;
        let dead = 1.0 + n_dead;
        Self(live / (live + dead))
    }

    /// Computes the initial Top-K window size for a target `k` and base `fetch_multiplier`,
    /// scaled inversely by `self` and clamped to `max_topk_chunk_size`.
    pub fn compute_initial_window(self, k: usize, fetch_multiplier: f64) -> f64 {
        let max_window = if cfg!(test) {
            100_000.0
        } else {
            crate::gucs::max_topk_chunk_size().max(1) as f64
        };
        let base_window = ((k as f64) * fetch_multiplier).max(1.0);
        let min_window = base_window.min(max_window);
        let s = self.0.clamp(0.0, 1.0);
        if s > 0.0 {
            (base_window / s).clamp(min_window, max_window)
        } else {
            max_window
        }
    }
}

/// Process-local telemetry and first-pull auxiliary collector hook for `BaseScan`'s `TopKScanExecState`.
#[derive(Default)]
pub struct LazySortTelemetryState {
    pub first_pull_aux_collector: Option<TopKAuxiliaryCollector>,
    pub queries_executed: usize,
    pub segment_info: BTreeMap<SegmentId, serde_json::Value>,
    pub first_pull_aggregation: Option<(IntermediateAggregationResults, Option<usize>)>,
}

pub type LazySortTelemetry = Arc<Mutex<UnsafeSendSync<LazySortTelemetryState>>>;

/// Configures `PgSearchScanPlan` to execute iterative, expanding-window Top-K queries
/// via `SearchIndexReader::search_top_k_in_segments` instead of streaming segment-by-segment.
#[derive(Clone)]
pub struct LazySortConfig {
    /// Tantivy sort keys passed to `search_top_k_in_segments`.
    /// When `None`, the scan uses `search_top_k_unordered_in_segments`.
    pub orderby_info: Option<Vec<OrderByInfo>>,
    /// DataFusion physical ordering declared in `EquivalenceProperties` when `orderby_info` is `Some`.
    pub ordering: Option<LexOrdering>,
    /// Initial Top-K window size (`TopKSelectivity::compute_initial_window` for `BaseScan` and joins).
    pub initial_window: f64,
    /// Controls cross-worker threshold sharing on the first window pull.
    pub shared_threshold: SharedThresholdMode,
    /// Optional process-local telemetry and first-pull auxiliary collector hook for `BaseScan`.
    /// Not serialized across MPP dispatch (workers reconstruct with `None`).
    pub telemetry: Option<LazySortTelemetry>,
}

impl std::fmt::Debug for LazySortConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LazySortConfig")
            .field("orderby_info", &self.orderby_info)
            .field("ordering", &self.ordering)
            .field("initial_window", &self.initial_window)
            .field("shared_threshold", &self.shared_threshold)
            .field("has_telemetry", &self.telemetry.is_some())
            .finish()
    }
}

/// Tracks the expanding window `[offset .. offset + limit)` across iterative Top-K pulls.
enum WindowStep {
    FirstPull,
    Retry { chunk_size: usize },
}

struct WindowPull {
    window: TopKWindow,
    is_first_pull: bool,
}

struct TopKWindowCursor {
    initial_window: f64,
    next_offset: TopKOffset,
    step: WindowStep,
}

impl TopKWindowCursor {
    fn new(initial_window: f64) -> Self {
        Self {
            initial_window,
            next_offset: TopKOffset(0),
            step: WindowStep::FirstPull,
        }
    }

    fn next_window(&mut self) -> Option<WindowPull> {
        let (chunk_size, is_first_pull) = match self.step {
            WindowStep::FirstPull => {
                self.step = WindowStep::Retry { chunk_size: 0 };
                (0, true)
            }
            WindowStep::Retry { chunk_size } => {
                let retry_scale = crate::gucs::topk_retry_scale_factor();
                let next_chunk_size = (chunk_size * retry_scale as usize)
                    .max((self.initial_window * retry_scale as f64) as usize)
                    .min(crate::gucs::max_topk_chunk_size() as usize);
                self.step = WindowStep::Retry {
                    chunk_size: next_chunk_size,
                };
                (next_chunk_size, false)
            }
        };
        let local_limit = self.initial_window.max(chunk_size as f64) as usize;
        if local_limit == 0 {
            return None;
        }
        let offset = self.next_offset;
        self.next_offset = TopKOffset(offset.0 + local_limit);
        Some(WindowPull {
            window: TopKWindow {
                limit: TopKLimit(local_limit),
                offset,
            },
            is_first_pull,
        })
    }
}

/// Encapsulates segment selection and Top-K window execution across iterative pulls.
enum LazySegmentTarget {
    /// Range-partitioned segment classification and partition bounds.
    ///
    /// TODO(#6898): `PgSearchScanPlan::with_lazy_sort` currently forces single-partition
    /// `Shared` execution; this variant is wired up for range-partitioned rank joins.
    RangePartitioned {
        segments: PartitionSegments,
        bounds: Box<SearchQueryInput>,
    },
    Dynamic {
        parallel_state: Option<*mut ParallelScanState>,
        claim_source_idx: usize,
        claimed_segments: Rc<RefCell<Vec<SegmentId>>>,
        all_segments_claimed: Rc<Cell<bool>>,
    },
}

impl LazySegmentTarget {
    fn new(
        reader: &SearchIndexReader,
        range_boundaries: Option<&RangePartitioning>,
        assigned_partition_segments: Option<PartitionSegments>,
        target_partition: usize,
        parallel_state: Option<*mut ParallelScanState>,
        source_idx: Option<usize>,
    ) -> Self {
        if let Some(rb) = range_boundaries {
            let segments = assigned_partition_segments
                .unwrap_or_else(|| segments_for_partition(reader, rb, target_partition));
            let bounds = Box::new(rb.partition_bounds(target_partition));
            Self::RangePartitioned { segments, bounds }
        } else {
            Self::Dynamic {
                parallel_state,
                claim_source_idx: source_idx.unwrap_or(0),
                claimed_segments: Rc::new(RefCell::new(Vec::new())),
                all_segments_claimed: Rc::new(Cell::new(false)),
            }
        }
    }

    fn claimed_segment_count(&self) -> Option<usize> {
        match self {
            Self::Dynamic {
                parallel_state: Some(_),
                claimed_segments,
                ..
            } => Some(claimed_segments.borrow().len()),
            _ => None,
        }
    }

    fn search_window(
        &self,
        reader: &SearchIndexReader,
        lazy_sort: &LazySortConfig,
        pull: &WindowPull,
    ) -> TopKSearch {
        match self {
            Self::RangePartitioned { segments, bounds } => {
                if let Some(ref orderby_info) = lazy_sort.orderby_info {
                    reader.search_top_k_segments_with_range_filter(
                        segments,
                        bounds,
                        orderby_info,
                        pull.window,
                    )
                } else {
                    reader.search_top_k_unordered_segments_with_range_filter(
                        segments,
                        bounds,
                        pull.window,
                    )
                }
            }
            Self::Dynamic {
                parallel_state,
                claim_source_idx,
                claimed_segments,
                all_segments_claimed,
            } => {
                let aux_collector = if pull.is_first_pull {
                    lazy_sort
                        .telemetry
                        .as_ref()
                        .and_then(|t| t.lock().unwrap().0.first_pull_aux_collector.take())
                } else {
                    None
                };
                let shared_threshold_state = match lazy_sort.shared_threshold {
                    SharedThresholdMode::EnabledOnFirstPull if pull.is_first_pull => {
                        *parallel_state
                    }
                    SharedThresholdMode::EnabledOnFirstPull | SharedThresholdMode::Disabled => None,
                };
                let seg_iter = Self::dynamic_segment_iter(
                    reader,
                    *parallel_state,
                    *claim_source_idx,
                    claimed_segments,
                    all_segments_claimed,
                );
                if let Some(ref orderby_info) = lazy_sort.orderby_info {
                    reader.search_top_k_in_segments(
                        seg_iter,
                        orderby_info,
                        pull.window.limit.0,
                        pull.window.offset.0,
                        aux_collector,
                        shared_threshold_state,
                    )
                } else {
                    let mut res = reader.search_top_k_unordered_in_segments(
                        seg_iter,
                        pull.window.limit.0,
                        pull.window.offset.0,
                    );
                    if let Some(aux) = aux_collector {
                        res.aggregation_results = Some(Self::collect_unordered_aux_aggregation(
                            reader,
                            *parallel_state,
                            claimed_segments,
                            aux,
                        ));
                    }
                    res
                }
            }
        }
    }

    fn dynamic_segment_iter(
        reader: &SearchIndexReader,
        parallel_state: Option<*mut ParallelScanState>,
        claim_source_idx: usize,
        claimed_segments: &Rc<RefCell<Vec<SegmentId>>>,
        all_segments_claimed: &Rc<Cell<bool>>,
    ) -> Box<dyn Iterator<Item = SegmentId>> {
        match parallel_state {
            None => Box::new(reader.segment_ids().into_iter().inspect(|_| {
                pgrx::check_for_interrupts!();
            })),
            Some(ps) => {
                let already_claimed = claimed_segments.borrow().clone();
                let claimed_for_iter = Rc::clone(claimed_segments);
                let done_for_iter = Rc::clone(all_segments_claimed);
                Box::new(
                    already_claimed
                        .into_iter()
                        .inspect(|_| {
                            pgrx::check_for_interrupts!();
                        })
                        .chain(std::iter::from_fn(move || {
                            if done_for_iter.get() {
                                return None;
                            }
                            pgrx::check_for_interrupts!();
                            let maybe_seg =
                                unsafe { checkout_segment_for_source(ps, claim_source_idx) };
                            match maybe_seg {
                                Some(seg_id) => {
                                    claimed_for_iter.borrow_mut().push(seg_id);
                                    Some(seg_id)
                                }
                                None => {
                                    done_for_iter.set(true);
                                    None
                                }
                            }
                        })),
                )
            }
        }
    }

    fn collect_unordered_aux_aggregation(
        reader: &SearchIndexReader,
        parallel_state: Option<*mut ParallelScanState>,
        claimed_segments: &Rc<RefCell<Vec<SegmentId>>>,
        aux: TopKAuxiliaryCollector,
    ) -> IntermediateAggregationResults {
        let segs_for_agg: Box<dyn Iterator<Item = SegmentId>> = match parallel_state {
            None => Box::new(reader.segment_ids().into_iter()),
            Some(_) => Box::new(claimed_segments.borrow().clone().into_iter()),
        };
        if let Some(vischeck) = aux.vischeck {
            reader.collect_segments(
                segs_for_agg,
                crate::aggregate::mvcc_collector::MVCCFilterCollector::new(
                    aux.aggregation_collector,
                    vischeck,
                ),
            )
        } else {
            reader.collect_segments(segs_for_agg, aux.aggregation_collector)
        }
    }
}

fn init_lazy_sort_scanner(
    reader: &SearchIndexReader,
    scanner_config: ScannerConfig,
    scan_fetched_fields: &[String],
) -> Result<Scanner> {
    let need_scores = scanner_config
        .which_fast_fields
        .iter()
        .any(|wff| wff.is_score());
    let mut scanner = Scanner::new(
        MultiSegmentSearchResults::empty(reader.searcher().clone()),
        scanner_config.batch_size_hint,
        scanner_config.which_fast_fields,
        scanner_config.heap_relid,
    );
    scanner.register_tagged_queries(&scanner_config.scan_mode, reader, need_scores)?;
    if !scan_fetched_fields.is_empty() {
        scanner.fetch_ordinals_in_scan(scan_fetched_fields);
    }
    Ok(scanner)
}

fn update_scanner_filters(
    scanner: &mut Scanner,
    dynamic_filters: &[Arc<dyn PhysicalExpr>],
    schema: &SchemaRef,
    score_column_schema_idx: Option<usize>,
) -> Result<Vec<PreFilter>> {
    let (pre_filters, score_threshold) =
        build_filters(dynamic_filters, schema, score_column_schema_idx)?;
    if scanner.can_pushdown_score_threshold() {
        scanner.set_score_threshold(score_threshold);
    }
    Ok(pre_filters)
}

fn record_pull_telemetry(
    lazy_sort: &LazySortConfig,
    segment_target: &LazySegmentTarget,
    top_k_search: &mut TopKSearch,
    queries: &Count,
) {
    queries.add(1);
    if let Some(ref telemetry) = lazy_sort.telemetry {
        let mut guard = telemetry.lock().unwrap();
        guard.0.queries_executed += 1;
        if !top_k_search.segment_info.is_empty() {
            guard
                .0
                .segment_info
                .extend(std::mem::take(&mut top_k_search.segment_info));
        }
        if let Some(agg_result) = top_k_search.aggregation_results.take() {
            guard.0.first_pull_aggregation =
                Some((agg_result, segment_target.claimed_segment_count()));
        }
    }
}

/// Builds the iterative expanding-window Top-K stream for a `PgSearchScanPlan` configured
/// with [`LazySortConfig`].
pub(super) fn build_lazy_sort_stream(
    ctx: ScanStreamContext,
    lazy_sort: LazySortConfig,
) -> impl Stream<Item = Result<RecordBatch>> {
    let ScanStreamContext {
        source_idx,
        planner_estimated_rows: _,
        scanner_config,
        ffhelper,
        mut visibility,
        mut reader,
        parallel_state,
        range_boundaries,
        assigned_partition_segments,
        target_partition,
        schema,
        score_column_schema_idx,
        mut dynamic_filters,
        scan_fetched_fields,
        plan_metrics,
        baseline_metrics,
        rows_scanned,
        rows_pruned,
    } = ctx;

    let queries = MetricBuilder::new(&plan_metrics).counter("queries", target_partition);

    async_stream::try_stream! {
        // Push static/completed dynamic filters into the index reader before the first pull,
        // then initialize the batch materializer, segment target, and expanding-window cursor.
        let mut filter_metrics = FilterStepMetrics::init(&mut reader, &mut dynamic_filters);
        let mut scanner = init_lazy_sort_scanner(
            &reader,
            scanner_config,
            &scan_fetched_fields,
        )?;
        let segment_target = LazySegmentTarget::new(
            &reader,
            range_boundaries.as_ref(),
            assigned_partition_segments,
            target_partition,
            parallel_state,
            source_idx,
        );
        let mut window_cursor = TopKWindowCursor::new(lazy_sort.initial_window);

        // 1. Advance through expanding Top-K windows `[offset .. offset + limit)`.
        while let Some(pull) = window_cursor.next_window() {
            // 2. Refresh dynamic pre-filters and score threshold for this window pull.
            let timer = baseline_metrics.elapsed_compute().timer();
            let pre_filters = update_scanner_filters(
                &mut scanner,
                &dynamic_filters,
                &schema,
                score_column_schema_idx,
            )?;
            let pre_filters_wrapper = (!pre_filters.is_empty()).then_some(PreFilters {
                filters: &pre_filters,
                schema: &schema,
            });

            // 3. Execute the Top-K search across segments (and optional first-pull aggregation)
            //    and forward query/segment telemetry.
            // `block_on_next` holds interrupts while this stream runs, so the collector's
            // own check can't fire. Bail out instead and let the caller act on the interrupt
            // once `block_on` has returned.
            if cancel_pending() {
                Err::<(), DataFusionError>(interrupted())?;
            }
            let mut top_k_search = segment_target.search_window(&reader, &lazy_sort, &pull);
            record_pull_telemetry(&lazy_sort, &segment_target, &mut top_k_search, &queries);

            // 4. Materialize surviving rows in Top-K order and flush filter metrics.
            //    Fewer raw hits than requested indicates that the index is exhausted.
            let exhausted = top_k_search.results.len() < pull.window.limit.0;
            let next_batch = scanner.materialize_topk_window(
                &top_k_search.results,
                &ffhelper,
                &mut visibility,
                pre_filters_wrapper.as_ref(),
                &schema,
            );
            timer.done();

            filter_metrics.flush(
                &scanner,
                &plan_metrics,
                target_partition,
                &rows_scanned,
                &rows_pruned,
                exhausted,
            );

            if let Some(record_batch) = next_batch {
                yield record_batch.record_output(&baseline_metrics);
            }

            if exhausted {
                break;
            }
        }
        baseline_metrics.done();
    }
}
