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

//! Lazy sorted scan configuration and stream execution for [`PgSearchScanPlan`].
//!
//! Supports iterative, expanding-window Sorted Access (`SA`) for `BaseScan` Top-K and
//! [`RankJoinExec`](crate::scan::rank_join_exec::RankJoinExec), as well as on-demand Random Access
//! (`RA`) join-key point lookups and tagless `ScanMode::Disjunctive` scoring.
//!
//! [`PgSearchScanPlan`]: crate::scan::execution_plan::PgSearchScanPlan

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use arrow_array::RecordBatch;
use arrow_schema::SchemaRef;
use datafusion::common::Result;
use datafusion::physical_expr::expressions::DynamicFilterPhysicalExpr;
use datafusion::physical_expr::{LexOrdering, PhysicalExpr};
use datafusion::physical_plan::metrics::{
    BaselineMetrics, Count, ExecutionPlanMetricsSet, MetricBuilder, RecordOutput,
};
use futures::Stream;
use tantivy::aggregation::intermediate_agg_result::IntermediateAggregationResults;
use tantivy::index::SegmentId;

use crate::api::OrderByInfo;
use crate::index::fast_fields_helper::FFHelper;
use crate::index::reader::index::{
    MultiSegmentSearchResults, RaLookupWeights, SearchIndexReader, TopKAuxiliaryCollector,
    TopKLimit, TopKOffset, TopKSearch, TopKWindow,
};
use crate::index::stats::{PartitionSegments, segments_for_partition};
use crate::postgres::ParallelScanState;
use crate::postgres::customscan::parallel::checkout_segment_for_source;
use crate::postgres::heap::VisibilityChecker;
use crate::query::SearchQueryInput;
use crate::scan::Scanner;
use crate::scan::execution_plan::{
    FilterStepMetrics, ScanStreamContext, ScannerConfig, UnsafeSendSync, build_filters,
};
use crate::scan::pre_filter::{PreFilter, PreFilters, RankLookupPhysicalExpr};
use crate::scan::range_partitioning::RangePartitioning;

/// Controls whether a lazy-sorted `PgSearchScanPlan` attaches a shared cross-worker threshold
/// to Tantivy's Top-K collector on the initial window pull.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SharedThresholdMode {
    EnabledOnFirstPull,
    Disabled,
}

/// Estimated probability `(0.0, 1.0]` that a candidate document from a Top-K scan
/// survives downstream filtering (e.g., MVCC heap visibility in `BaseScan`, or `RA`
/// join probes and cross-table tag filters in `RankJoinExec`).
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct TopKSelectivity(pub f64);

impl TopKSelectivity {
    pub const ALL: Self = Self(1.0);

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

    /// Combines two independent survival selectivities conjunctively: `P(A and B) = P(A) * P(B)`.
    pub fn and(self, other: Self) -> Self {
        Self((self.0 * other.0).clamp(0.0, 1.0))
    }

    /// Combines two independent match probabilities disjunctively: `P(A or B) = 1 - (1 - P(A))(1 - P(B))`.
    pub fn or(self, other: Self) -> Self {
        let p_a = self.0.clamp(0.0, 1.0);
        let p_b = other.0.clamp(0.0, 1.0);
        Self((1.0 - (1.0 - p_a) * (1.0 - p_b)).clamp(0.0, 1.0))
    }

    /// Computes the initial Top-K window size for a target `k` and base `fetch_multiplier`,
    /// scaled inversely by `self` and clamped to `max_topk_chunk_size`.
    pub fn compute_initial_window(self, k: usize, fetch_multiplier: f64) -> f64 {
        let max_window = crate::gucs::max_topk_chunk_size().max(1) as f64;
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

/// Configures [`PgSearchScanPlan`](crate::scan::execution_plan::PgSearchScanPlan) for iterative,
/// expanding-window Sorted Access (`SA`) and/or on-demand Random Access (`RA`) join-key point
/// lookups within a single stream:
///
/// - **Interleaved `SA + RA` & Deduplication (`emitted_docs`)**:
///   Each stream poll first checks `dynamic_filters` for a
///   [`RankLookupPhysicalExpr`]. If present,
///   it executes a `TermSetQuery` point lookup (`SearchIndexReader::search_ra_lookup`) without
///   advancing the `SA` window offset. Otherwise, it pulls the next expanding `SA` Top-K window
///   `[offset .. offset + local_limit)` via `search_top_k_in_segments`. Both `SA` and `RA` pulls
///   share `emitted_docs: HashSet<DocAddress>` so no document is ever emitted twice.
/// - **`RA`-Only Mode (`initial_window == 0.0`)**:
///   Used for unscored (`NRJN`) inputs of [`RankJoinExec`](crate::scan::rank_join_exec::RankJoinExec).
///   `SA` is immediately marked exhausted (`lit(false)` on the parent `DynamicFilterPhysicalExpr`)
///   and the scan only executes `RA` point lookups requested by the parent join.
/// - **`ScanMode::Disjunctive` Scoring**:
///   `SA` searches the union of positive-score queries (`with_disjunctive_sa_queries`) so
///   `0.0`-score documents are never visited by `SA`, while `RA` point lookups score `ra_weights`
///   directly on matched `DocId`s (returning `0.0` when a document satisfies `base_query` and the
///   join key but none of `disjunct_queries`).
#[derive(Clone)]
pub struct LazySortConfig {
    /// Tantivy sort keys passed to `search_top_k_in_segments`.
    /// When `None`, the scan uses `search_top_k_unordered_in_segments` (or `RA`-only mode when
    /// `initial_window == 0.0`).
    pub orderby_info: Option<Vec<OrderByInfo>>,
    /// DataFusion physical ordering declared in `EquivalenceProperties` when `orderby_info` is `Some`.
    pub ordering: Option<LexOrdering>,
    /// Initial Top-K window size (`TopKSelectivity::compute_initial_window` for `BaseScan` and joins,
    /// or `0.0` for `RA`-only mode).
    pub initial_window: f64,
    /// Controls cross-worker threshold sharing on the first window pull.
    pub shared_threshold: SharedThresholdMode,
    /// Optional process-local telemetry and first-pull auxiliary collector hook for `BaseScan`.
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
    Exhausted,
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
            step: if initial_window <= 0.0 {
                WindowStep::Exhausted
            } else {
                WindowStep::FirstPull
            },
        }
    }

    fn is_exhausted(&self) -> bool {
        matches!(self.step, WindowStep::Exhausted)
    }

    fn mark_exhausted(&mut self) {
        self.step = WindowStep::Exhausted;
    }

    fn next_window(&mut self) -> Option<WindowPull> {
        let (chunk_size, is_first_pull) = match self.step {
            WindowStep::Exhausted => return None,
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
            self.step = WindowStep::Exhausted;
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
    RangePartitioned {
        target: Box<(PartitionSegments, SearchQueryInput)>,
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
            let bounds = rb.partition_bounds(target_partition);
            Self::RangePartitioned {
                target: Box::new((segments, bounds)),
            }
        } else {
            Self::Dynamic {
                parallel_state,
                claim_source_idx: source_idx.unwrap_or(0),
                claimed_segments: Rc::new(RefCell::new(Vec::new())),
                all_segments_claimed: Rc::new(Cell::new(false)),
            }
        }
    }

    fn range_target(&self) -> Option<&(PartitionSegments, SearchQueryInput)> {
        match self {
            Self::RangePartitioned { target } => Some(target.as_ref()),
            Self::Dynamic { .. } => None,
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
            Self::RangePartitioned { target } => {
                let (segments, bounds) = target.as_ref();
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

/// How a lazy-sorted scan interacts with a parent [`RankJoinExec`](crate::scan::rank_join_exec::RankJoinExec)
/// when Sorted Access (`SA`) reaches the end of the index.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ExhaustionBehavior {
    /// Standalone `BaseScan` Top-K (no parent `RankJoinExec` sentinel).
    TerminateStream,
    /// `RankJoinExec` leaf whose `SA` and `RA` match sets are identical (`ScanMode::Standard`
    /// or `ScanMode::Tagged` with `initial_window > 0`): once `SA` is exhausted, signals
    /// `Boolean(None)` so the parent disables both `SA` and `RA`, then terminates.
    SignalBothExhausted,
    /// `RankJoinExec` leaf that must remain open for `RA` point lookups after `SA` is exhausted
    /// (`ScanMode::Disjunctive` or `RA`-only `initial_window <= 0.0`): signals `lit(false)` and
    /// yields empty `SA` batches until the parent drops the stream.
    KeepAliveForRa,
}

fn prepare_sa_and_ra(
    reader: &SearchIndexReader,
    scan_mode: &crate::scan::ScanMode,
    pushed: bool,
    range_target: Option<&(PartitionSegments, SearchQueryInput)>,
) -> (SearchIndexReader, RaLookupWeights, bool) {
    let (sa_reader, is_disjunctive_sa, base_is_match_all, ra_disjunct_queries) = match scan_mode {
        crate::scan::ScanMode::Disjunctive {
            base_query,
            disjunct_queries,
        } => {
            let has_disjuncts = !disjunct_queries.is_empty();
            let replace_query = matches!(**base_query, SearchQueryInput::All) && !pushed;
            let sa = if has_disjuncts {
                reader.with_disjunctive_sa_queries(replace_query, disjunct_queries)
            } else {
                reader.clone()
            };
            (
                sa,
                has_disjuncts,
                replace_query,
                disjunct_queries.as_slice(),
            )
        }
        crate::scan::ScanMode::Standard { query }
        | crate::scan::ScanMode::Tagged {
            base_query: query, ..
        } => (
            reader.clone(),
            false,
            matches!(query.as_ref(), SearchQueryInput::All) && !pushed,
            &[][..],
        ),
    };
    let ra_weights =
        reader.prepare_ra_lookup_weights(range_target, base_is_match_all, ra_disjunct_queries);
    (sa_reader, ra_weights, is_disjunctive_sa)
}

fn update_scanner_filters(
    scanner: &mut Scanner,
    dynamic_filters: &[Arc<dyn PhysicalExpr>],
    schema: &SchemaRef,
    score_column_schema_idx: Option<usize>,
) -> Result<(Vec<PreFilter>, Option<f32>)> {
    let (pre_filters, score_threshold) =
        build_filters(dynamic_filters, schema, score_column_schema_idx)?;
    if scanner.can_pushdown_score_threshold() {
        scanner.set_score_threshold(score_threshold);
    }
    Ok((pre_filters, score_threshold))
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

/// Mutable state machine driving a lazy-sorted [`PgSearchScanPlan`] stream across
/// interleaved Sorted Access (`SA`) Top-K windows and Random Access (`RA`) point lookups.
///
/// [`PgSearchScanPlan`]: crate::scan::execution_plan::PgSearchScanPlan
struct LazySortStreamState {
    lazy_sort: LazySortConfig,
    reader: SearchIndexReader,
    sa_reader: SearchIndexReader,
    ra_weights: RaLookupWeights,
    scanner: Scanner,
    ffhelper: Arc<FFHelper>,
    visibility: Box<VisibilityChecker>,
    segment_target: LazySegmentTarget,
    window_cursor: TopKWindowCursor,
    filter_metrics: FilterStepMetrics,
    dynamic_filters: Vec<Arc<dyn PhysicalExpr>>,
    schema: SchemaRef,
    score_column_schema_idx: Option<usize>,
    target_partition: usize,
    plan_metrics: ExecutionPlanMetricsSet,
    baseline_metrics: BaselineMetrics,
    queries: Count,
    rows_scanned: Option<Count>,
    rows_pruned: Option<Count>,
    emitted_docs: crate::api::HashSet<tantivy::DocAddress>,
    exhaustion_behavior: ExhaustionBehavior,
}

impl LazySortStreamState {
    /// Pushes static/completed dynamic filters into the index reader before the first pull,
    /// then initializes the segment target, `SA`/`RA` readers, batch materializer, and
    /// expanding-window cursor.
    fn init(ctx: ScanStreamContext, lazy_sort: LazySortConfig) -> Result<Self> {
        let ScanStreamContext {
            source_idx,
            planner_estimated_rows: _,
            scanner_config,
            ffhelper,
            visibility,
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
        let filter_metrics = FilterStepMetrics::init(&mut reader, &mut dynamic_filters);
        let segment_target = LazySegmentTarget::new(
            &reader,
            range_boundaries.as_ref(),
            assigned_partition_segments,
            target_partition,
            parallel_state,
            source_idx,
        );
        let (sa_reader, ra_weights, is_disjunctive_sa) = prepare_sa_and_ra(
            &reader,
            &scanner_config.scan_mode,
            filter_metrics.pushed,
            segment_target.range_target(),
        );
        let scanner = init_lazy_sort_scanner(&reader, scanner_config, &scan_fetched_fields)?;
        let window_cursor = TopKWindowCursor::new(lazy_sort.initial_window);

        let is_rank_join_leaf = matches!(lazy_sort.shared_threshold, SharedThresholdMode::Disabled)
            && !dynamic_filters.is_empty();
        let exhaustion_behavior = if !is_rank_join_leaf {
            ExhaustionBehavior::TerminateStream
        } else if is_disjunctive_sa || lazy_sort.initial_window <= 0.0 {
            ExhaustionBehavior::KeepAliveForRa
        } else {
            ExhaustionBehavior::SignalBothExhausted
        };

        Ok(Self {
            lazy_sort,
            reader,
            sa_reader,
            ra_weights,
            scanner,
            ffhelper,
            visibility,
            segment_target,
            window_cursor,
            filter_metrics,
            dynamic_filters,
            schema,
            score_column_schema_idx,
            target_partition,
            plan_metrics,
            baseline_metrics,
            queries,
            rows_scanned,
            rows_pruned,
            emitted_docs: crate::api::HashSet::default(),
            exhaustion_behavior,
        })
    }

    /// Checks whether a parent `RankJoinExec` has published a [`RankLookupPhysicalExpr`] in
    /// `dynamic_filters`, consuming it by resetting the filter to `lit(true)`.
    fn take_pending_ra_lookup(&self) -> Option<RankLookupPhysicalExpr> {
        for df in &self.dynamic_filters {
            if let Some(dyn_filter) = df.downcast_ref::<DynamicFilterPhysicalExpr>()
                && let Ok(current_expr) = dyn_filter.current()
                && let Some(lookup) = current_expr.downcast_ref::<RankLookupPhysicalExpr>()
            {
                let cloned = lookup.clone();
                let _ = dyn_filter.update(datafusion::physical_expr::expressions::lit(true));
                return Some(cloned);
            }
        }
        None
    }

    /// Executes an on-demand Random Access (`RA`) join-key point lookup and materializes
    /// any newly matched documents.
    fn execute_ra_lookup(&mut self, lookup: &RankLookupPhysicalExpr) -> Result<RecordBatch> {
        let timer = self.baseline_metrics.elapsed_compute().timer();
        let (pre_filters, score_threshold) = update_scanner_filters(
            &mut self.scanner,
            &self.dynamic_filters,
            &self.schema,
            self.score_column_schema_idx,
        )?;
        let pre_filters_wrapper = (!pre_filters.is_empty()).then_some(PreFilters {
            filters: &pre_filters,
            schema: &self.schema,
        });

        let effective_ra_threshold = match (lookup.score_threshold(), score_threshold) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        let ra_batch = if let Some(lookup_query) =
            crate::scan::pre_filter::try_convert_rank_lookup_to_query(
                lookup,
                self.reader.schema(),
                self.reader.index_created_by_version(),
                Some(self.filter_metrics.strategy_sink.clone()),
            ) {
            let ra_results = self.reader.search_ra_lookup(
                &self.ra_weights,
                lookup_query,
                effective_ra_threshold,
                &mut self.emitted_docs,
            );
            self.queries.add(1);
            MetricBuilder::new(&self.plan_metrics)
                .counter("ra_queries", self.target_partition)
                .add(1);
            if !ra_results.is_empty() {
                self.scanner.materialize_topk_window(
                    &ra_results,
                    &self.ffhelper,
                    &mut self.visibility,
                    pre_filters_wrapper.as_ref(),
                    &self.schema,
                )
            } else {
                None
            }
        } else {
            None
        };
        timer.done();

        self.filter_metrics.flush_ra(
            &self.scanner,
            &self.plan_metrics,
            self.target_partition,
            &self.rows_scanned,
            &self.rows_pruned,
            effective_ra_threshold.is_some(),
        );

        Ok(ra_batch.unwrap_or_else(|| RecordBatch::new_empty(self.schema.clone())))
    }

    /// Advances the expanding `SA` window cursor, returning `None` once `SA` is exhausted.
    fn next_sa_window(&mut self) -> Option<WindowPull> {
        self.window_cursor.next_window()
    }

    /// Handles an `SA` poll after `SA` has been exhausted: when `KeepAliveForRa`, signals
    /// `lit(false)` to the parent `RankJoinExec` and returns an empty `RecordBatch`; otherwise
    /// returns `None` to terminate the stream.
    fn handle_exhausted_sa(&self) -> Option<RecordBatch> {
        if self.exhaustion_behavior == ExhaustionBehavior::KeepAliveForRa {
            self.signal_parent_sa_exhausted();
            Some(RecordBatch::new_empty(self.schema.clone()))
        } else {
            None
        }
    }

    /// Executes one expanding `SA` Top-K window pull, deduplicating against `emitted_docs`
    /// and materializing surviving rows.
    fn execute_sa_window(&mut self, pull: &WindowPull) -> Result<Option<RecordBatch>> {
        let timer = self.baseline_metrics.elapsed_compute().timer();
        let (pre_filters, _) = update_scanner_filters(
            &mut self.scanner,
            &self.dynamic_filters,
            &self.schema,
            self.score_column_schema_idx,
        )?;
        let pre_filters_wrapper = (!pre_filters.is_empty()).then_some(PreFilters {
            filters: &pre_filters,
            schema: &self.schema,
        });

        let mut top_k_search =
            self.segment_target
                .search_window(&self.sa_reader, &self.lazy_sort, pull);
        record_pull_telemetry(
            &self.lazy_sort,
            &self.segment_target,
            &mut top_k_search,
            &self.queries,
        );

        let exhausted = top_k_search.results.len() < pull.window.limit.0;
        if exhausted {
            self.window_cursor.mark_exhausted();
        }
        top_k_search
            .results
            .retain(|(_, doc_addr)| self.emitted_docs.insert(*doc_addr));
        let next_batch = if !top_k_search.results.is_empty() {
            self.scanner.materialize_topk_window(
                &top_k_search.results,
                &self.ffhelper,
                &mut self.visibility,
                pre_filters_wrapper.as_ref(),
                &self.schema,
            )
        } else {
            None
        };
        timer.done();

        self.filter_metrics.flush(
            &self.scanner,
            &self.plan_metrics,
            self.target_partition,
            &self.rows_scanned,
            &self.rows_pruned,
            exhausted,
        );

        if next_batch.is_some() && self.window_cursor.is_exhausted() {
            self.signal_parent_sa_exhausted();
        }
        Ok(next_batch)
    }

    /// Returns `true` when an `SA` pull produced no rows, the index is exhausted, and the scan
    /// does not need to stay open for future `RA` point lookups.
    fn should_terminate_after_empty_sa(&self) -> bool {
        self.window_cursor.is_exhausted()
            && self.exhaustion_behavior != ExhaustionBehavior::KeepAliveForRa
    }

    /// Updates the first dynamic filter with the appropriate `SA`-exhaustion sentinel for a
    /// parent `RankJoinExec`.
    fn signal_parent_sa_exhausted(&self) {
        let sentinel = match self.exhaustion_behavior {
            ExhaustionBehavior::TerminateStream => return,
            ExhaustionBehavior::KeepAliveForRa => {
                datafusion::physical_expr::expressions::lit(false)
            }
            ExhaustionBehavior::SignalBothExhausted => datafusion::physical_expr::expressions::lit(
                datafusion::common::ScalarValue::Boolean(None),
            ),
        };
        if let Some(df) = self.dynamic_filters.first()
            && let Some(dyn_filter) = df.downcast_ref::<DynamicFilterPhysicalExpr>()
        {
            let _ = dyn_filter.update(sentinel);
        }
    }

    fn record_output(&self, batch: RecordBatch) -> RecordBatch {
        batch.record_output(&self.baseline_metrics)
    }

    fn finish(&self) {
        self.baseline_metrics.done();
    }
}

/// Builds the iterative `SA + RA` execution stream for a lazy-sorted [`PgSearchScanPlan`].
///
/// [`PgSearchScanPlan`]: crate::scan::execution_plan::PgSearchScanPlan
pub(super) fn build_lazy_sort_stream(
    ctx: ScanStreamContext,
    lazy_sort: LazySortConfig,
) -> impl Stream<Item = Result<RecordBatch>> {
    async_stream::try_stream! {
        let mut state = LazySortStreamState::init(ctx, lazy_sort)?;

        loop {
            // 1. Service on-demand Random Access (`RA`) join-key point lookups from a parent
            //    `RankJoinExec` without advancing the Sorted Access (`SA`) window cursor.
            if let Some(lookup) = state.take_pending_ra_lookup() {
                let batch = state.execute_ra_lookup(&lookup)?;
                yield state.record_output(batch);
                continue;
            }

            // 2. Pull the next expanding Sorted Access (`SA`) Top-K window `[offset .. offset + limit)`.
            //    When `SA` is exhausted, either stay alive to service future `RA` probes (signaling
            //    `lit(false)` to the parent `RankJoinExec`) or terminate the stream.
            let Some(pull) = state.next_sa_window() else {
                if let Some(empty_batch) = state.handle_exhausted_sa() {
                    yield state.record_output(empty_batch);
                    continue;
                }
                break;
            };

            // 3. Execute the `SA` Top-K search, deduplicate against previously emitted `SA`/`RA`
            //    documents, and materialize surviving rows.
            if let Some(batch) = state.execute_sa_window(&pull)? {
                yield state.record_output(batch);
            } else if state.should_terminate_after_empty_sa() {
                break;
            }
        }
        state.finish();
    }
}
