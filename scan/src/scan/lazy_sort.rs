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
//! [`RankJoinExec`](crate::scan::rank_join_exec::RankJoinExec) (materializing each `SA` window in
//! bounded chunks so early-terminating downstream consumers avoid upfront fast-field and
//! dictionary reads across the remainder of the window), as well as on-demand Random Access (`RA`)
//! join-key point lookups and tagless `ScanMode::Disjunctive` scoring.
//!
//! [`PgSearchScanPlan`]: crate::scan::execution_plan::PgSearchScanPlan

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use arrow_array::RecordBatch;
use arrow_schema::SchemaRef;
use datafusion::common::{DataFusionError, Result};
use datafusion::physical_expr::expressions::DynamicFilterPhysicalExpr;
use datafusion::physical_expr::{DynamicFilterTracking, LexOrdering, PhysicalExpr};
use datafusion::physical_plan::metrics::{
    BaselineMetrics, Count, ExecutionPlanMetricsSet, MetricBuilder, RecordOutput,
};
use futures::Stream;
use tantivy::aggregation::intermediate_agg_result::IntermediateAggregationResults;
use tantivy::index::SegmentId;

use crate::api::OrderByInfo;
use crate::index::fast_fields_helper::FFHelper;
use crate::index::reader::index::{
    MultiSegmentSearchResults, RaLookupWeights, SearchIndexReader, SearchIndexScore,
    TopKAuxiliaryCollector, TopKLimit, TopKOffset, TopKSearch, TopKWindow, VectorRouting,
};
use crate::index::stats::{PartitionSegments, segments_for_partition};
use crate::postgres::ParallelScanState;
use crate::postgres::customscan::mpp::interrupt::{
    check_for_cancel, check_for_interrupts_or_cancel,
};
use crate::postgres::customscan::parallel::checkout_segment_for_source;
use crate::postgres::heap::VisibilityChecker;
use crate::query::SearchQueryInput;
use crate::scan::Scanner;
use crate::scan::execution_plan::{
    FilterStepMetrics, ScanState, ScanStreamContext, ScannerConfig, build_filters,
};
use crate::scan::pre_filter::{
    InListLinearFallback, PreFilter, PreFilters, RankLookupPhysicalExpr,
};
use crate::scan::range_partitioning::RangePartitioning;

/// Controls how a non-range-partitioned (`ExecutionState::Shared`) lazy-sorted
/// `PgSearchScanPlan` interacts with `ParallelScanState`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LazyParallelMode {
    /// Claims segments from the shared `ParallelScanState` worker pool (when present)
    /// and, for single-table `BaseScan` (`source_idx.is_none()`), shares the cross-worker
    /// Top-K threshold on the initial window pull.
    WorkerPool,
    /// Ignores `ParallelScanState` so a single-task `RankJoinExec` leaf scan searches all
    /// segments in the index without cross-worker segment claiming or threshold sharing.
    Unpartitioned,
}

/// Estimated probability `[0.0, 1.0]` that a candidate document from a Top-K scan
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

    /// Conditional survival probability `P(self | already_satisfied) = self / already_satisfied`
    /// when `self` is a conjunction (`and`) that includes `already_satisfied` and a scan has
    /// already applied `already_satisfied` internally.
    pub fn given(self, already_satisfied: Self) -> Self {
        if already_satisfied.0 > 0.0 {
            Self((self.0 / already_satisfied.0).clamp(0.0, 1.0))
        } else {
            Self::ALL
        }
    }

    /// Computes the initial Top-K window size for a target `k` and base `fetch_multiplier`,
    /// scaled inversely by `self` and clamped to `max_topk_chunk_size`.
    pub fn compute_initial_window(self, k: usize, fetch_multiplier: f64) -> f64 {
        let max_window = crate::gucs::max_topk_chunk_size().max(1) as f64;
        let base_window = ((k as f64) * fetch_multiplier).max(1.0);
        let min_window = base_window.min(max_window);
        let s = self.0.clamp(0.0, 1.0);
        (base_window / s).clamp(min_window, max_window)
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

pub type LazySortTelemetry =
    Arc<Mutex<crate::postgres::customscan::mpp::MainThread<LazySortTelemetryState>>>;

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
///   join key but none of `disjunct_queries`). A non-`All` `base_query` is a `0.0`-score filter on
///   both streams.
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
    /// Controls how a non-range-partitioned (`ExecutionState::Shared`) scan interacts with
    /// `ParallelScanState` for segment claiming and first-pull cross-worker threshold sharing.
    pub parallel_mode: LazyParallelMode,
    /// Optional process-local telemetry and first-pull auxiliary collector hook for `BaseScan`.
    pub telemetry: Option<LazySortTelemetry>,
}

impl std::fmt::Debug for LazySortConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LazySortConfig")
            .field("orderby_info", &self.orderby_info)
            .field("ordering", &self.ordering)
            .field("initial_window", &self.initial_window)
            .field("parallel_mode", &self.parallel_mode)
            .field("has_telemetry", &self.telemetry.is_some())
            .finish()
    }
}

/// Tracks the expanding window `[offset .. offset + limit)` across iterative Top-K pulls.
///
/// TODO: Make Sorted Access (`SA`) truly lazy/incremental across pulls instead of re-executing
/// `search_top_k_in_segments` (or `search_top_k_segments_with_range_filter`) from scratch with an
/// increasing `offset` on each retry: <https://github.com/paradedb/paradedb/issues/7243>.
/// Re-running `TopDocs` with `offset + limit` re-traverses and re-scores all `offset` previously
/// emitted documents on every pull; maintaining stateful per-segment cursors or passing the last
/// emitted `(sort_key, DocAddress)` bound into the collector would allow each pull to collect only
/// `limit` new documents.
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
                let (retry_scale, max_chunk_size) =
                    crate::postgres::customscan::mpp::run_on_main_thread(|_| {
                        (
                            crate::gucs::topk_retry_scale_factor(),
                            crate::gucs::max_topk_chunk_size() as usize,
                        )
                    });
                let next_chunk_size = (chunk_size * retry_scale as usize)
                    .max((self.initial_window * retry_scale as f64) as usize)
                    .min(max_chunk_size);
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
        source_idx: Option<usize>,
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
                source_idx,
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
        routing: &VectorRouting,
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
                        routing,
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
                source_idx,
                claimed_segments,
                all_segments_claimed,
            } => {
                let aux_collector = if pull.is_first_pull {
                    lazy_sort.telemetry.as_ref().and_then(|t| {
                        crate::postgres::customscan::mpp::run_on_main_thread(|token| {
                            t.lock()
                                .unwrap()
                                .get_mut(token)
                                .first_pull_aux_collector
                                .take()
                        })
                    })
                } else {
                    None
                };
                // Cross-worker DSM threshold sharing (`ParallelScanState::shared_threshold`) is
                // only valid for single-table `BaseScan` (`source_idx.is_none()`); in `JoinScan`
                // (`source_idx.is_some()`), passing `None` uses Tantivy's single-process shared
                // threshold within each worker's scan without pruning against another worker's
                // pre-join candidates.
                let shared_threshold_state = match lazy_sort.parallel_mode {
                    LazyParallelMode::WorkerPool if pull.is_first_pull && source_idx.is_none() => {
                        *parallel_state
                    }
                    LazyParallelMode::WorkerPool | LazyParallelMode::Unpartitioned => None,
                };
                let seg_iter = Self::dynamic_segment_iter(
                    reader,
                    *parallel_state,
                    source_idx.unwrap_or(0),
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
                        routing,
                    )
                } else {
                    let mut res = reader.search_top_k_unordered_in_segments(
                        seg_iter,
                        pull.window.limit.0,
                        pull.window.offset.0,
                    );
                    if !check_for_interrupts_or_cancel()
                        && let Some(aux) = aux_collector
                    {
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
            None => Box::new(
                reader
                    .segment_ids()
                    .into_iter()
                    .take_while(|_| !check_for_interrupts_or_cancel()),
            ),
            Some(ps) => {
                let already_claimed = claimed_segments.borrow().clone();
                let claimed_for_iter = Rc::clone(claimed_segments);
                let done_for_iter = Rc::clone(all_segments_claimed);
                Box::new(
                    already_claimed
                        .into_iter()
                        .take_while(|_| !check_for_interrupts_or_cancel())
                        .chain(std::iter::from_fn(move || {
                            if done_for_iter.get() || check_for_interrupts_or_cancel() {
                                return None;
                            }
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
            None => Box::new(
                reader
                    .segment_ids()
                    .into_iter()
                    .take_while(|_| !check_for_interrupts_or_cancel()),
            ),
            Some(_) => Box::new(
                claimed_segments
                    .borrow()
                    .clone()
                    .into_iter()
                    .take_while(|_| !check_for_interrupts_or_cancel()),
            ),
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
            let replace_query = matches!(base_query.as_ref(), SearchQueryInput::All) && !pushed;
            // The reader was opened with the unwrapped base, so the `0.0` score is applied here.
            // `RA` never scores the base when disjuncts exist (`prepare_ra_lookup_weights`), and
            // `RankJoinExec` bounds `RA`-only partners at `0.0`, so `SA` must not score it either.
            let sa = if has_disjuncts && replace_query {
                reader.with_disjunctive_sa_queries(true, disjunct_queries)
            } else if has_disjuncts {
                reader
                    .with_constant_score(0.0)
                    .with_disjunctive_sa_queries(false, disjunct_queries)
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
    scanner: &Scanner,
    dynamic_filters: &[Arc<dyn PhysicalExpr>],
    schema: &SchemaRef,
    score_column_schema_idx: Option<usize>,
) -> Result<(Vec<PreFilter>, Option<f32>)> {
    let (pre_filters, score_threshold) =
        build_filters(dynamic_filters, schema, score_column_schema_idx)?;
    let pushable_score_threshold = scanner
        .can_pushdown_score_threshold()
        .then_some(score_threshold)
        .flatten();
    Ok((pre_filters, pushable_score_threshold))
}

fn record_pull_telemetry(
    lazy_sort: &LazySortConfig,
    segment_target: &LazySegmentTarget,
    top_k_search: &mut TopKSearch,
    queries: &Count,
) {
    queries.add(1);
    if let Some(ref telemetry) = lazy_sort.telemetry {
        crate::postgres::customscan::mpp::run_on_main_thread(|token| {
            let mut guard = telemetry.lock().unwrap();
            let telem = guard.get_mut(token);
            telem.queries_executed += 1;
            if !top_k_search.segment_info.is_empty() {
                telem
                    .segment_info
                    .extend(std::mem::take(&mut top_k_search.segment_info));
            }
            if let Some(agg_result) = top_k_search.aggregation_results.take() {
                telem.first_pull_aggregation =
                    Some((agg_result, segment_target.claimed_segment_count()));
            }
        });
    }
}

/// Maximum number of `(SearchIndexScore, DocAddress)` hits materialized into a single
/// `RecordBatch` per stream step from an `SA` Top-K window, avoiding upfront fast-field and
/// dictionary materialization of multi-thousand-row windows when downstream `LIMIT` or
/// [`RankJoinExec`](crate::scan::rank_join_exec::RankJoinExec) terminates early.
const SA_MATERIALIZE_CHUNK_SIZE: usize = 128;

/// Buffers the rank-ordered `(score, DocAddress)` window returned by a single `SA` Top-K pull
/// so [`LazySortStreamState`] can materialize and emit Arrow `RecordBatch`es incrementally in
/// [`SA_MATERIALIZE_CHUNK_SIZE`] slices instead of decoding the entire window up front.
struct PendingSaWindow {
    /// Rank-ordered hits from the current Top-K window.
    results: Vec<(SearchIndexScore, tantivy::DocAddress)>,
    /// Index of the next uninspected hit in `results`.
    cursor: usize,
}

impl PendingSaWindow {
    fn new(results: Vec<(SearchIndexScore, tantivy::DocAddress)>) -> Self {
        Self { results, cursor: 0 }
    }

    /// Drains up to [`SA_MATERIALIZE_CHUNK_SIZE`] un-emitted hits in rank order, recording them
    /// in `emitted_docs` and advancing `cursor` past any immediately trailing already-emitted
    /// documents.
    fn next_chunk(
        &mut self,
        emitted_docs: &mut crate::api::HashSet<tantivy::DocAddress>,
    ) -> Vec<(SearchIndexScore, tantivy::DocAddress)> {
        let mut chunk = Vec::with_capacity(
            self.results
                .len()
                .saturating_sub(self.cursor)
                .min(SA_MATERIALIZE_CHUNK_SIZE),
        );
        while self.cursor < self.results.len() && chunk.len() < SA_MATERIALIZE_CHUNK_SIZE {
            let hit = self.results[self.cursor];
            self.cursor += 1;
            if emitted_docs.insert(hit.1) {
                chunk.push(hit);
            }
        }
        while self.cursor < self.results.len()
            && emitted_docs.contains(&self.results[self.cursor].1)
        {
            self.cursor += 1;
        }
        chunk
    }

    fn is_exhausted(&self) -> bool {
        self.cursor >= self.results.len()
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
    vector_routing: VectorRouting,
    window_cursor: TopKWindowCursor,
    pending_sa_window: Option<PendingSaWindow>,
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
    fn init(
        ctx: ScanStreamContext,
        lazy_sort: LazySortConfig,
        token: crate::postgres::customscan::mpp::MainThreadToken,
    ) -> Result<Self> {
        let ScanStreamContext {
            vector_routing,
            mut scan_state,
            parallel_state,
            range_boundaries,
            assigned_partition_segments,
            target_partition,
            schema,
            mut dynamic_filters,
            scan_fetched_fields,
            plan_metrics,
            baseline_metrics,
            rows_scanned,
            rows_pruned,
        } = ctx;

        let ScanState {
            source_idx,
            planner_estimated_rows: _,
            scanner_config,
            ffhelper,
            visibility,
            mut reader,
        } = scan_state.take(token).expect("ScanState initialized once");
        let parallel_state = parallel_state.map(|p| *p.get(token));
        let score_column_schema_idx = scanner_config
            .which_fast_fields
            .iter()
            .position(|wff| wff.is_score());

        let queries = MetricBuilder::new(&plan_metrics).counter("queries", target_partition);
        let filter_metrics = FilterStepMetrics::init(
            &mut reader,
            &mut dynamic_filters,
            InListLinearFallback::PushToIndex,
        );
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

        // `RankJoinRule` sets `LazyParallelMode::Unpartitioned`, and `RankJoinExec` pushes its
        // incomplete (`Watching`) lookup `DynamicFilterPhysicalExpr` to every child that can be
        // `RA`-probed (whereas a `HashJoinExec` build-side filter is already `Static` or
        // `AllComplete` before the probe stream is polled, and the scored child of a root
        // single-sided `NRJN` join has no lookup filter).
        let is_rank_join_leaf = matches!(lazy_sort.parallel_mode, LazyParallelMode::Unpartitioned)
            && dynamic_filters.iter().any(|df| {
                matches!(
                    DynamicFilterTracking::classify(df),
                    DynamicFilterTracking::Watching(_)
                )
            });
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
            vector_routing,
            window_cursor,
            pending_sa_window: None,
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
                let local_cols = df.children().into_iter().cloned().collect();
                let cloned = lookup.clone().with_cols(local_cols);
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
            &self.scanner,
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
                &self.schema,
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
            check_for_cancel()?;
            self.queries.add(1);
            MetricBuilder::new(&self.plan_metrics)
                .counter("ra_queries", self.target_partition)
                .add(1);
            if !ra_results.is_empty() {
                let batch = self.scanner.materialize_topk_window(
                    &ra_results,
                    &self.ffhelper,
                    &mut self.visibility,
                    pre_filters_wrapper.as_ref(),
                    &self.schema,
                )?;
                check_for_cancel()?;
                batch
            } else {
                None
            }
        } else if lookup
            .keys()
            .iter()
            .any(|tuple| !tuple.is_empty() && tuple.iter().all(|s| !s.is_null()))
        {
            return Err(DataFusionError::Internal(format!(
                "Failed to convert non-null RankLookup keys to Tantivy query: {lookup}"
            )));
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

    /// Executes one expanding `SA` Top-K index search and buffers any matched `(score, DocAddress)`
    /// hits in `pending_sa_window` for chunked materialization.
    fn start_sa_window(&mut self, pull: &WindowPull) -> Result<()> {
        let timer = self.baseline_metrics.elapsed_compute().timer();
        let mut top_k_search = self.segment_target.search_window(
            &self.sa_reader,
            &self.lazy_sort,
            pull,
            &self.vector_routing,
        );
        check_for_cancel()?;
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
        timer.done();

        if top_k_search.results.is_empty() {
            self.filter_metrics.flush(
                &self.scanner,
                &self.plan_metrics,
                self.target_partition,
                &self.rows_scanned,
                &self.rows_pruned,
                self.has_exhausted_all_sa(),
            );
        } else {
            self.pending_sa_window = Some(PendingSaWindow::new(top_k_search.results));
        }
        Ok(())
    }

    /// Materializes the next [`SA_MATERIALIZE_CHUNK_SIZE`] slice from `pending_sa_window`,
    /// deduplicating against `emitted_docs` lazily so documents emitted by interleaved `RA`
    /// lookups are skipped.
    fn materialize_next_sa_chunk(&mut self) -> Result<Option<RecordBatch>> {
        let Some(mut pending) = self.pending_sa_window.take() else {
            return Ok(None);
        };
        let chunk = pending.next_chunk(&mut self.emitted_docs);
        if !pending.is_exhausted() {
            self.pending_sa_window = Some(pending);
        }

        let timer = self.baseline_metrics.elapsed_compute().timer();
        let next_batch = if !chunk.is_empty() {
            let (pre_filters, _) = update_scanner_filters(
                &self.scanner,
                &self.dynamic_filters,
                &self.schema,
                self.score_column_schema_idx,
            )?;
            let pre_filters_wrapper = (!pre_filters.is_empty()).then_some(PreFilters {
                filters: &pre_filters,
                schema: &self.schema,
            });
            let batch = self.scanner.materialize_topk_window(
                &chunk,
                &self.ffhelper,
                &mut self.visibility,
                pre_filters_wrapper.as_ref(),
                &self.schema,
            )?;
            check_for_cancel()?;
            batch
        } else {
            None
        };
        timer.done();

        let all_sa_exhausted = self.has_exhausted_all_sa();
        self.filter_metrics.flush(
            &self.scanner,
            &self.plan_metrics,
            self.target_partition,
            &self.rows_scanned,
            &self.rows_pruned,
            all_sa_exhausted,
        );

        if next_batch.is_some() && all_sa_exhausted {
            self.signal_parent_sa_exhausted();
        }
        Ok(next_batch)
    }

    fn has_exhausted_all_sa(&self) -> bool {
        self.window_cursor.is_exhausted() && self.pending_sa_window.is_none()
    }

    /// Returns `true` when an `SA` pull produced no rows, the index is exhausted, and the scan
    /// does not need to stay open for future `RA` point lookups.
    fn should_terminate_after_empty_sa(&self) -> bool {
        self.has_exhausted_all_sa()
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
        // Find the `Watching` lookup filter pushed down by our parent `RankJoinExec` and
        // update it with the exhaustion sentinel.
        if let Some(dyn_filter) = self.dynamic_filters.iter().find_map(|df| {
            matches!(
                DynamicFilterTracking::classify(df),
                DynamicFilterTracking::Watching(_)
            )
            .then(|| df.downcast_ref::<DynamicFilterPhysicalExpr>())
            .flatten()
        }) {
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
    enum LazySortStep {
        Yield(RecordBatch),
        Continue,
        Done,
    }

    let mut ctx_opt = Some(ctx);
    async_stream::try_stream! {
        let mut state = crate::postgres::customscan::mpp::try_run_on_main_thread(|token| {
            let ctx = ctx_opt.take().expect("ScanStreamContext initialized once");
            LazySortStreamState::init(ctx, lazy_sort, token)
                .map(crate::postgres::customscan::mpp::MainThread::new)
        })?;

        loop {
            // `block_on_next` holds interrupts while this stream runs, so the collector's
            // own check can't fire. Bail out instead and let the caller act on the interrupt
            // once `block_on` has returned.
            check_for_cancel()?;

            let step = crate::postgres::customscan::mpp::try_run_on_main_thread(
                |token| -> Result<LazySortStep> {
                    let state = state.get_mut(token);

                    // 1. Service on-demand Random Access (`RA`) join-key point lookups from a parent
                    //    `RankJoinExec` without advancing the Sorted Access (`SA`) window cursor.
                    if let Some(lookup) = state.take_pending_ra_lookup() {
                        let batch = state.execute_ra_lookup(&lookup)?;
                        check_for_cancel()?;
                        return Ok(LazySortStep::Yield(state.record_output(batch)));
                    }

                    // 2. When no unmaterialized `SA` hits remain from the current window, pull the
                    //    next expanding Sorted Access (`SA`) Top-K window `[offset .. offset + limit)`.
                    //    When `SA` is exhausted, either stay alive to service future `RA` probes
                    //    (signaling `lit(false)` to the parent `RankJoinExec`) or terminate the stream.
                    if state.pending_sa_window.is_none() {
                        let Some(pull) = state.next_sa_window() else {
                            return Ok(match state.handle_exhausted_sa() {
                                Some(empty_batch) => {
                                    LazySortStep::Yield(state.record_output(empty_batch))
                                }
                                None => LazySortStep::Done,
                            });
                        };
                        state.start_sa_window(&pull)?;
                        check_for_cancel()?;
                        if state.pending_sa_window.is_none() {
                            return Ok(if state.should_terminate_after_empty_sa() {
                                LazySortStep::Done
                            } else {
                                LazySortStep::Continue
                            });
                        }
                    }

                    // 3. Materialize the next chunk from `pending_sa_window`, deduplicating against
                    //    previously emitted `SA`/`RA` documents.
                    let sa_batch = state.materialize_next_sa_chunk()?;
                    check_for_cancel()?;
                    if let Some(batch) = sa_batch {
                        Ok(LazySortStep::Yield(state.record_output(batch)))
                    } else if state.should_terminate_after_empty_sa() {
                        Ok(LazySortStep::Done)
                    } else {
                        Ok(LazySortStep::Continue)
                    }
                },
            )?;

            match step {
                LazySortStep::Yield(batch) => yield batch,
                LazySortStep::Continue => continue,
                LazySortStep::Done => break,
            }
        }
        crate::postgres::customscan::mpp::run_on_main_thread(|token| {
            state.get(token).finish();
        });
    }
}
