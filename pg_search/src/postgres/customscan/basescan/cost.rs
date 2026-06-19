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

//! Serial-vs-parallel worker selection and path cost for BaseScan paths (#4664).
//!
//! For each candidate exec method, `create_custom_path` (mod.rs) computes two independent things:
//!   - the path's intrinsic cost, via [`estimate_path_cost`]; and
//!   - the worker decision (serial, or N parallel workers), via [`decide_scan_parallelism`].
//!
//! [`decide_scan_parallelism`] is the decision tree, in order:
//!   1. Prunable short-circuit -> serial: a score-DESC single prunable term is sublinear under
//!      Block-WAND, so parallel workers would only add overhead.
//!   2. No row stats (un-ANALYZEd) -> the row heuristic (`compute_nworkers`); with no row count it
//!      can't cap, so it parallelizes by segment count.
//!   3. Otherwise, route on cost and scan shape:
//!        - have a cost       -> cost model ([`cost_test`]): work vs Gather overhead;
//!        - no cost, sorted   -> parallelize across every segment (it must visit them all);
//!        - no cost, unsorted -> the row heuristic (row caps fit -- it can stop early at LIMIT).
//!
//! Invariants and known limits:
//!   - A zero structural ceiling (single segment / parallelism off / correlated) always means serial.
//!   - The cost model's `fraction` is exact for sorted (it drives every match) and an approximation
//!     for unsorted (it assumes LIMIT/matches of the docset is driven).
//!   - Blind spots: a write-heavy index's mutable-segment open cost is invisible to the model, and
//!     the phrase `size_hint` under-counts matches.

use super::*;

/// Read-only accessor for a PostgreSQL planner cost GUC (a mutable static). Centralizing the read
/// here keeps the cost helpers safe (no `unsafe` for a config read).
fn cpu_index_tuple_cost() -> f64 {
    unsafe { pg_sys::cpu_index_tuple_cost }
}
fn parallel_setup_cost() -> f64 {
    unsafe { pg_sys::parallel_setup_cost }
}
fn parallel_tuple_cost() -> f64 {
    unsafe { pg_sys::parallel_tuple_cost }
}

#[derive(Clone, Copy)]
pub(super) enum WorkerDecision {
    Serial,
    Parallel { nworkers: NonZeroUsize },
}

pub(super) struct PathCostBasis {
    pub(super) parallelizable_cost: f64,
    pub(super) total_cost_multiplier: f64,
}

/// The query's `Query::cost`, memoized across a query's exec methods so we open the index at most
/// once. The two layers of an `Option<Option<u64>>` were easy to transpose; this names the states.
pub(super) enum CostMemo {
    /// Not yet opened/estimated.
    NotComputed,
    /// Estimated: `Some(c)` is the cost (`0` allowed -- a sample miss), `None` means we couldn't
    /// cost it (open failed).
    Computed(Option<u64>),
}

impl CostMemo {
    /// Seed from `create_custom_path`'s selectivity open: `Some(c)` was already costed, `None`
    /// wasn't.
    pub(super) fn from_precomputed(precomputed: Option<u64>) -> Self {
        match precomputed {
            Some(c) => Self::Computed(Some(c)),
            None => Self::NotComputed,
        }
    }

    /// The cost, computing+memoizing it on first use.
    fn get_or_compute(&mut self, compute: impl FnOnce() -> Option<u64>) -> Option<u64> {
        match self {
            Self::Computed(cost) => *cost,
            Self::NotComputed => {
                let cost = compute();
                *self = Self::Computed(cost);
                cost
            }
        }
    }
}

impl WorkerDecision {
    fn from_worker_count(nworkers: usize) -> Self {
        NonZeroUsize::new(nworkers)
            .map(|nworkers| Self::Parallel { nworkers })
            .unwrap_or(Self::Serial)
    }

    /// How many ways the parallel scan work is divided. The leader runs a full share
    /// of the scan when it participates, so it counts as one more worker beyond
    /// `nworkers` -- not a partial contribution. (Serial scans don't divide: 1.0.)
    pub(super) fn divisor(self, leader_participates: bool) -> f64 {
        let Self::Parallel { nworkers } = self else {
            return 1.0;
        };
        let nworkers = nworkers.get();
        if leader_participates {
            (nworkers + 1) as f64
        } else {
            nworkers as f64
        }
    }

    pub(super) fn nworkers(self) -> Option<NonZeroUsize> {
        match self {
            Self::Serial => None,
            Self::Parallel { nworkers } => Some(nworkers),
        }
    }
}

/// Whether a method may take the Block-WAND prunable serial short-circuit (from method shape +
/// ORDER BY, no index open). `NotPrunable` doesn't; `decide_scan_parallelism` decides those.
#[derive(Clone, Copy)]
pub(super) enum TopKPrunability {
    /// Score-DESC ordered TopK -- the one shape Block-WAND prunes to sublinear. Eligible for the
    /// short-circuit, still subject to the per-predicate `is_topk_prunable()` check at the use site.
    PrunableCandidate,
    /// Everything else (score-ASC / field sort, window aggregates, parameterized LIMIT, external
    /// var, non-TopK): not eligible for the short-circuit.
    NotPrunable,
}

/// Classify a method's eligibility for the prunable serial short-circuit (see [`TopKPrunability`]).
/// Returns `PrunableCandidate` only for a clean `score DESC` ordered TopK; everything else is
/// `NotPrunable`. Derived purely from the method + ORDER BY + quals, so no reader is opened at plan
/// time.
///
/// # Safety
///
/// `root` must point to a valid `PlannerInfo` for the duration of this call. This
/// is a planner-only helper and must not be called from execution.
pub(super) unsafe fn topk_can_prune_for_method(
    method: &ExecMethodType,
    root: *mut pg_sys::PlannerInfo,
    quals: &Qual,
) -> TopKPrunability {
    let ExecMethodType::TopK {
        orderby_info: Some(orderby_info),
        window_aggregates,
        limit_offset,
        ..
    } = method
    else {
        return TopKPrunability::NotPrunable;
    };

    if !window_aggregates.is_empty() || limit_offset.has_any_param() {
        return TopKPrunability::NotPrunable;
    }

    // `window_aggregates` is still empty here because placeholders are
    // deserialized later in `plan_custom_path`, so inspect the target list
    // recursively before classifying. This relies on the planner hook replacing
    // WindowFunc nodes with window_agg() placeholders before relation paths
    // are created.
    if query_has_window_agg_functions(root) {
        return TopKPrunability::NotPrunable;
    }

    // An external-var qual (the inner side of a nested loop) is never the clean bare-term shape
    // Block-WAND prunes. A runtime-bound `$1` predicate is deliberately not excluded here -- by
    // shape it can still be a candidate, and the per-predicate `is_topk_prunable()` check rejects
    // it at the use site.
    if quals.contains_external_var() {
        return TopKPrunability::NotPrunable;
    }

    // A clean ordered TopK: a prunable candidate only when ordered by score DESC (the one direction
    // Block-WAND prunes to sublinear). Score ASC / field sort are ordered but not prunable.
    if SearchIndexReader::orderby_uses_score_desc_topk_collector(orderby_info) {
        TopKPrunability::PrunableCandidate
    } else {
        TopKPrunability::NotPrunable
    }
}

/// Cost-model leaf: parallelize when splitting the scan across workers saves more than PostgreSQL's
/// fixed Gather overhead (`work/divisor + gather_overhead < work`). `work` is the query's drive cost
/// (`Query::cost`) in PostgreSQL units, scaled by the early-termination `fraction`. `segment_workers`
/// is the structural ceiling: serial (0 workers) if it can't parallelize, otherwise the workers to
/// split across.
fn cost_test(
    drive_cost: u64,
    matches: f64,
    segment_workers: WorkerDecision,
    is_sorted: bool,
    limit: Option<f64>,
    base_result_rows: f64,
    parallel_leader_participates: bool,
) -> WorkerDecision {
    let WorkerDecision::Parallel { .. } = segment_workers else {
        return WorkerDecision::Serial;
    };

    // `fraction` is the share of the docset the scan must drive before it can stop: a sorted scan
    // scores every match (1.0), but an unsorted LIMIT scan stops once it has LIMIT matches
    // (~LIMIT/matches). There is deliberately no log2(LIMIT) heap-depth term -- a non-prunable scan
    // examines every matching doc once, so the only effect of LIMIT is this early-termination share.
    let fraction = if is_sorted {
        1.0
    } else {
        limit.map_or(1.0, |l| (l / matches).min(1.0))
    };
    // A zero `drive_cost` (the sampled largest segment matched none of the query) yields work = 0
    // and serializes below. We do not special-case it to parallelize: a genuinely tiny match set
    // reads zero too, indistinguishable from a skewed sample miss, so serial is the safe choice.
    // `cpu_index_tuple_cost` converts the Tantivy cost into PG units, comparable to the gather
    // overhead below.
    let work = drive_cost as f64 * cpu_index_tuple_cost() * fraction;

    // Parallelize only when splitting the work across workers saves more than the fixed Gather
    // overhead (`parallel_setup_cost` plus per-row transport; the 1.05 covers Gather-Merge IPC).
    let divisor = segment_workers.divisor(parallel_leader_participates);
    const GATHER_MERGE_IPC_FACTOR: f64 = 1.05;
    let gather_overhead =
        parallel_setup_cost() + parallel_tuple_cost() * base_result_rows * GATHER_MERGE_IPC_FACTOR;
    if work / divisor + gather_overhead < work {
        segment_workers
    } else {
        WorkerDecision::Serial
    }
}

/// Inputs to [`decide_scan_parallelism`], bundled so the call site is self-labeling.
pub(super) struct ScanParallelismInputs<'a> {
    pub(super) prunability: TopKPrunability,
    pub(super) query: &'a SearchQueryInput,
    pub(super) bm25_index: &'a PgSearchRelation,
    pub(super) row_estimate: RowEstimate,
    pub(super) is_sorted: bool,
    pub(super) limit: Option<f64>,
    pub(super) segment_count: usize,
    pub(super) base_result_rows: f64,
    pub(super) consider_parallel: bool,
    pub(super) quals: &'a Qual,
    pub(super) root: *mut pg_sys::PlannerInfo,
    pub(super) parallel_leader_participates: bool,
    pub(super) is_join_context: bool,
}

/// Decide serial-vs-parallel for a scan (see the module docs for the full tree). In order:
/// 1. Prunable score-DESC single term -> serial: Block-WAND keeps serial scoring sublinear, so
///    parallel workers would only add overhead.
/// 2. No row stats (table never ANALYZEd) -> the row heuristic (no caps -> by segment count).
/// 3. Otherwise route on cost and scan shape:
///    - have a cost       -> cost model (weigh drive cost vs Gather overhead);
///    - no cost, sorted   -> parallelize across every segment (it must visit them all);
///    - no cost, unsorted -> the row heuristic, whose caps fit because the scan can stop early.
///
/// # Safety
/// `inputs.root` must point to a valid `PlannerInfo` for the duration of this call.
pub(super) unsafe fn decide_scan_parallelism(
    inputs: ScanParallelismInputs,
    cost_memo: &mut CostMemo,
) -> WorkerDecision {
    let ScanParallelismInputs {
        prunability,
        query,
        bm25_index,
        row_estimate,
        is_sorted,
        limit,
        segment_count,
        base_result_rows,
        consider_parallel,
        quals,
        root,
        parallel_leader_participates,
        is_join_context,
    } = inputs;

    // 1. Prunable short-circuit -> serial. Block-WAND keeps serial scoring of a score-DESC single
    //    term sublinear (#4664), so parallel workers would only add overhead. The predicate must
    //    actually prune, not just look like a term.
    if matches!(prunability, TopKPrunability::PrunableCandidate) && query.is_topk_prunable() {
        return WorkerDecision::Serial;
    }

    let external_var = quals.contains_external_var();
    let correlated = quals.contains_correlated_param(root);

    // The row heuristic (`compute_nworkers`), used only by the two fallback branches below, so it
    // is computed lazily here. With no row count it applies no caps and parallelizes by segment.
    let heuristic_workers = || {
        let nworkers = if consider_parallel {
            compute_nworkers(
                is_sorted,
                limit,
                row_estimate,
                segment_count,
                external_var,
                correlated,
                is_join_context,
            )
        } else {
            0
        };
        WorkerDecision::from_worker_count(nworkers)
    };

    // 2. No row stats (table never ANALYZEd) -> the row heuristic. Routing every unanalyzed scan
    //    here (not just the uncostable ones) keeps the decision consistent regardless of shape.
    let RowEstimate::Known(matches) = row_estimate else {
        return heuristic_workers();
    };
    let matches = matches as f64;

    // Structural ceiling: the most workers the segments can use. Serial (0) when parallelism is
    // disabled, there is a single segment, or the predicate is correlated/external.
    let segment_workers = if consider_parallel {
        WorkerDecision::from_worker_count(max_useful_workers(
            segment_count,
            external_var,
            correlated,
        ))
    } else {
        WorkerDecision::Serial
    };

    // 3. Cost the query if we can. `None` for a runtime-bound `$1`/subquery or correlated/external
    //    predicate (no resolved value to open a scorer for), or if an eligible open fails.
    let cost = if quals.contains_exprs() || external_var || correlated {
        None
    } else {
        cost_memo.get_or_compute(|| estimate_query_cost(bm25_index, query.clone()))
    };

    // 4. Route on cost + shape:
    match (cost, is_sorted) {
        (Some(drive_cost), _) => cost_test(
            drive_cost,
            matches,
            segment_workers,
            is_sorted,
            limit,
            base_result_rows,
            parallel_leader_participates,
        ),
        // Sorted, no usable cost: it must visit every segment, so workers always help.
        (None, true) => segment_workers,
        // Unsorted, no usable cost: it can stop early at LIMIT, so the row heuristic's caps fit;
        // full structural parallelism would over-do it.
        (None, false) => heuristic_workers(),
    }
}

/// A scan path's intrinsic cost, independent of the worker decision: the per-row work to divide
/// across workers (`base_result_rows * per_tuple_cost`) and a multiplier for the sorted-index merge.
/// The worker decision is made separately by [`decide_scan_parallelism`].
pub(super) fn estimate_path_cost(
    method: &ExecMethodType,
    is_sorted: bool,
    per_tuple_cost: f64,
    base_result_rows: f64,
) -> PathCostBasis {
    let total_cost_multiplier = if is_sorted && method.supports_sorted_index_merge() {
        1.01
    } else {
        1.0
    };
    PathCostBasis {
        parallelizable_cost: base_result_rows * per_tuple_cost,
        total_cost_multiplier,
    }
}
