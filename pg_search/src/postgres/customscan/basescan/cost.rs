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

//! Serial-vs-parallel worker selection and path cost for BaseScan paths,
//! including the score-DESC TopK query-cost model (#4664).

use super::*;

#[derive(Clone, Copy)]
pub(super) enum WorkerDecision {
    Serial,
    Parallel { nworkers: NonZeroUsize },
}

pub(super) struct PathCostBasis {
    pub(super) worker_decision: WorkerDecision,
    pub(super) parallelizable_cost: f64,
    pub(super) total_cost_multiplier: f64,
}

impl WorkerDecision {
    fn from_worker_count(nworkers: usize) -> Self {
        NonZeroUsize::new(nworkers)
            .map(|nworkers| Self::Parallel { nworkers })
            .unwrap_or(Self::Serial)
    }

    /// Effective worker count for dividing scan work. The leader is counted as a
    /// full additional worker when `leader_participates` is true. We do not use
    /// PostgreSQL's discounted leader formula (`1 - 0.3 * nworkers`) because the
    /// bounded TopK cost path and the general path share the same divisor, and
    /// uniform full-credit accounting keeps the two cost helpers comparable
    /// across query shapes.
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

/// Whether an ordered TopK can be costed by the query-cost worker model, and if
/// so whether its ORDER BY is the Block-WAND-prunable shape.
pub(super) enum TopKCostability {
    /// Not a costable ordered TopK (window aggregates, parameterized LIMIT,
    /// runtime quals, or not a TopK) -- fall back to the general worker heuristic.
    GeneralPath,
    /// Costable ordered TopK. `score_desc` is true for ORDER BY score DESC, the
    /// one shape Block-WAND prunes to sublinear work.
    Costable { score_desc: bool },
}

/// Classify a method for the query-cost worker model: `Costable` when it is an
/// ordered TopK whose worker count should come from the cost model rather than
/// the general heuristic, otherwise `GeneralPath`. `Costable.score_desc` is true
/// only for ORDER BY `score DESC`, the one case where Block-WAND can make a single
/// posting list sublinear. Derived purely from the method + ORDER BY +
/// `SearchQueryInput`, so no reader is opened at plan time.
///
/// # Safety
///
/// `root` must point to a valid `PlannerInfo` for the duration of this call. This
/// is a planner-only helper and must not be called from execution.
pub(super) unsafe fn topk_can_prune_for_method(
    method: &ExecMethodType,
    root: *mut pg_sys::PlannerInfo,
    quals: &Qual,
) -> TopKCostability {
    let ExecMethodType::TopK {
        orderby_info: Some(orderby_info),
        window_aggregates,
        limit_offset,
        ..
    } = method
    else {
        return TopKCostability::GeneralPath;
    };

    if !window_aggregates.is_empty() || limit_offset.has_any_param() {
        return TopKCostability::GeneralPath;
    }

    // `window_aggregates` is still empty here because placeholders are
    // deserialized later in `plan_custom_path`, so inspect the target list
    // recursively before costing. This relies on the planner hook replacing
    // WindowFunc nodes with window_agg() placeholders before relation paths
    // are created.
    if query_has_window_agg_functions(root) {
        return TopKCostability::GeneralPath;
    }

    // Runtime-dependent quals can't be costed at plan time; use the general path.
    if quals.contains_exprs() || quals.contains_external_var() {
        return TopKCostability::GeneralPath;
    }

    TopKCostability::Costable {
        score_desc: SearchIndexReader::orderby_uses_score_desc_topk_collector(orderby_info),
    }
}

/// Worker decision for a *non-prunable* ordered TopK (the caller short-circuits prunable
/// single terms, which are always serial): parallelize only when splitting the scan across
/// workers beats PostgreSQL's fixed Gather overhead (`work/divisor + gather_overhead < work`),
/// where `work` is Tantivy `DocSet::cost()` -- which tends to rank query shapes by drive
/// expense (range/phrase carry explicit cost multipliers, a union sums its terms, a bare term
/// is just its `doc_freq`; the exact order is data-dependent) -- times a per-doc examine
/// constant. The decision
/// is cost-vs-overhead, so it responds to the `parallel_*_cost` GUCs. Unanalyzed tables stay
/// serial; this is the only shape considered, score and fast-field sorts alike.
///
/// # Safety
/// `root` must point to a valid `PlannerInfo` for the duration of this call.
#[allow(clippy::too_many_arguments)]
unsafe fn decide_nonprunable_topk_workers(
    row_estimate: RowEstimate,
    query_cost_estimate: Option<u64>,
    segment_count: usize,
    base_result_rows: f64,
    consider_parallel: bool,
    quals: &Qual,
    root: *mut pg_sys::PlannerInfo,
    parallel_leader_participates: bool,
) -> WorkerDecision {
    let max_workers = if consider_parallel {
        max_useful_workers(
            segment_count,
            quals.contains_external_var(),
            quals.contains_correlated_param(root),
        )
    } else {
        0
    };
    let candidate = WorkerDecision::from_worker_count(max_workers);
    let WorkerDecision::Parallel { .. } = candidate else {
        return WorkerDecision::Serial;
    };

    // An unanalyzed table has no trustworthy match count, so stay serial rather
    // than guess.
    let matches = match row_estimate {
        RowEstimate::Known(rows) => rows as f64,
        RowEstimate::Unknown => return WorkerDecision::Serial,
    };

    if matches < 1.0 {
        return WorkerDecision::Serial;
    }

    let work_units = match query_cost_estimate {
        Some(cost) => cost as f64,
        // No cost estimate (expensive-to-estimate query or un-openable index):
        // fall back to the raw match estimate.
        None => matches,
    };

    // Work = the query's drive cost (`Query::cost`, `work_units`) times the cost to examine one
    // docset entry. This basis has no top-K heap-depth (LIMIT) factor: a non-prunable TopK
    // examines every matching doc once (the heap only touches the few that beat the running
    // threshold). Folding a `log2(LIMIT)` factor back in is the open question for the
    // large-LIMIT union cases (see the serial-vs-parallel sweep).
    //
    // `cpu_index_tuple_cost` ("process one index entry") is the per-entry unit. With it the
    // cost-vs-overhead test below crosses into parallel once the divisible work clears the Gather
    // setup cost: at default GUCs, roughly `matches * cpu_index_tuple_cost > parallel_setup_cost`.
    let work = work_units * pg_sys::cpu_index_tuple_cost;

    let divisor = candidate.divisor(parallel_leader_participates);
    const GATHER_MERGE_IPC_FACTOR: f64 = 1.05;
    let gather_overhead = pg_sys::parallel_setup_cost
        + pg_sys::parallel_tuple_cost * base_result_rows * GATHER_MERGE_IPC_FACTOR;
    if work / divisor + gather_overhead < work {
        candidate
    } else {
        WorkerDecision::Serial
    }
}

/// Final worker decision for a method: `general` (the established heuristic), except an
/// ordered TopK overrides it via the query-cost threshold. A prunable single-term score-DESC
/// TopK is always serial -- Block-WAND keeps serial scoring cheap, so it needs no cost()
/// estimate and the reader open is skipped. Non-prunable shapes (unions, phrases, ranges)
/// weigh Tantivy's cost() against gather overhead.
///
/// `topk_query_cost_estimate` memoizes the cost across a query's methods: outer
/// `None` = not yet computed; inner `Option<u64>` = the estimate (`None` = couldn't).
///
/// # Safety
/// `root` must point to a valid `PlannerInfo` for the duration of this call.
#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn decide_method_workers(
    general: WorkerDecision,
    topk_can_prune: TopKCostability,
    query: &SearchQueryInput,
    bm25_index: &PgSearchRelation,
    topk_query_cost_estimate: &mut Option<Option<u64>>,
    row_estimate: RowEstimate,
    segment_count: usize,
    base_result_rows: f64,
    consider_parallel: bool,
    quals: &Qual,
    root: *mut pg_sys::PlannerInfo,
    parallel_leader_participates: bool,
) -> WorkerDecision {
    let TopKCostability::Costable { score_desc } = topk_can_prune else {
        return general;
    };

    if score_desc && query.is_topk_prunable() {
        return WorkerDecision::Serial;
    }

    // Lazy on purpose: reached only past the prunable short-circuit above, so a prunable
    // single-term TopK never opens. The memo is pre-seeded by create_custom_path when its
    // selectivity `else` branch already opened and computed the cost; otherwise it is `None`
    // here and we open once, memoized across the query's methods.
    let query_cost_estimate = match *topk_query_cost_estimate {
        Some(estimate) => estimate,
        None => {
            let estimate = estimate_query_cost(bm25_index, query.clone());
            *topk_query_cost_estimate = Some(estimate);
            estimate
        }
    };

    decide_nonprunable_topk_workers(
        row_estimate,
        query_cost_estimate,
        segment_count,
        base_result_rows,
        consider_parallel,
        quals,
        root,
        parallel_leader_participates,
    )
}

pub(super) struct GeneralPathCostParams<'a> {
    pub(super) method: &'a ExecMethodType,
    pub(super) is_sorted: bool,
    pub(super) float_limit: Option<Cardinality>,
    pub(super) row_estimate: RowEstimate,
    pub(super) segment_count: usize,
    pub(super) per_tuple_cost: f64,
    pub(super) base_result_rows: f64,
    pub(super) consider_parallel: bool,
    pub(super) quals: &'a Qual,
    pub(super) root: *mut pg_sys::PlannerInfo,
    pub(super) is_join_context: bool,
}

/// The cost basis for any method: the path cost plus the worker count from the
/// established heuristic (`compute_nworkers`). This is the basis for every method;
/// ordered TopK additionally overrides the worker decision via the query-cost
/// threshold (see the caller).
///
/// # Safety
/// `params.root` must point to a valid `PlannerInfo` for the duration of the call.
pub(super) unsafe fn cost_general_path(params: GeneralPathCostParams<'_>) -> PathCostBasis {
    let GeneralPathCostParams {
        method,
        is_sorted,
        float_limit,
        row_estimate,
        segment_count,
        per_tuple_cost,
        base_result_rows,
        consider_parallel,
        quals,
        root,
        is_join_context,
    } = params;

    let nworkers = if consider_parallel {
        compute_nworkers(
            is_sorted,
            float_limit,
            row_estimate,
            segment_count,
            quals.contains_external_var(),
            quals.contains_correlated_param(root),
            is_join_context,
        )
    } else {
        0
    };

    let total_cost_multiplier = if is_sorted && method.supports_sorted_index_merge() {
        1.01
    } else {
        1.0
    };

    PathCostBasis {
        worker_decision: WorkerDecision::from_worker_count(nworkers),
        parallelizable_cost: base_result_rows * per_tuple_cost,
        total_cost_multiplier,
    }
}
