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

//! Serial-vs-parallel path policy and path cost for BaseScan paths (#4664).
//!
//! `create_custom_path` (mod.rs) computes each method's path cost ([`estimate_path_cost`]) and its
//! [`WorkerPathPolicy`] ([`decide_scan_parallelism`]).
//!
//! A costable scan is costed by its work, emits both a serial and a partial path, and clears
//! PostgreSQL's native paths -- otherwise the honest scan-work cost lets PostgreSQL's own BM25 index
//! scan (correct, but without fast-field / Block-WAND execution) undercut us. PostgreSQL then costs
//! the Gather and picks serial vs parallel, which it can only do once it sees the upper plan (a bulk
//! `SELECT` ships every row; a `COUNT(*)` collapses rows in a Partial Aggregate first). pg_search
//! overrides that choice only where PostgreSQL gets it wrong: an effective LIMIT (it costs the Gather
//! on `rel->rows`, not the `k` rows that cross), prunable top-K (Block-WAND is invisible to it), a
//! join (`parallel_setup_cost` dwarfs the per-scan work), or grouping (it under-costs the serial
//! HashAggregate) -- plus uncostable or un-ANALYZEd scans it can't cost at all.
//!
//! [`decide_scan_parallelism`] decision tree, in order:
//!   1. Prunable score-DESC term -> serial only (Block-WAND keeps it sublinear).
//!   2. Un-ANALYZEd -> row heuristic (`compute_nworkers`): no row count, so parallelize by segment.
//!   3. Otherwise by costability/shape:
//!        - costable + effective LIMIT -> cost model forces, on `k` ([`cost_test_limited`]);
//!        - costable + no LIMIT, join  -> force parallel (a Parallel Hash Join needs the partial path);
//!        - costable + no LIMIT, group -> row heuristic (PostgreSQL under-costs the serial HashAggregate);
//!        - costable + no LIMIT, else  -> emit both, PostgreSQL picks;
//!        - uncostable sorted          -> parallel when workers exist (must visit every segment);
//!        - uncostable unsorted        -> row heuristic (caps fit -- can stop early at LIMIT).
//!
//! Blind spots: `fraction` is exact for sorted scans, approximate for unsorted; a write-heavy
//! index's mutable-segment open cost and phrase `size_hint` under-counts are not modeled.

use super::*;
use serde::{Deserialize, Serialize};

fn cpu_index_tuple_cost() -> f64 {
    unsafe { pg_sys::cpu_index_tuple_cost }
}
fn parallel_setup_cost() -> f64 {
    unsafe { pg_sys::parallel_setup_cost }
}
fn parallel_tuple_cost() -> f64 {
    unsafe { pg_sys::parallel_tuple_cost }
}

/// Which branch of [`decide_scan_parallelism`] produced the policy; the EXPLAIN VERBOSE label. Names
/// the branch, not the serial-vs-parallel outcome (the plan shape shows that).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(super) enum WorkerDecisionReason {
    /// Prunable score-DESC single term: Block-WAND keeps serial scoring sublinear (#4664), so
    /// workers would only add overhead.
    BlockWandPrunable,
    /// Costable scan, no effective LIMIT: pg_search offered both paths and let PostgreSQL choose. (A
    /// costable scan with no workers to split across also lands here -- it just emits serial.)
    CostModel,
    /// Costable scan with an effective LIMIT (top-K / unsorted LIMIT): pg_search costed the Gather on
    /// `k` and forced the winner, because PostgreSQL over-costs a bounded Gather (see module docs).
    CostModelLimited,
    /// Uncostable sorted scan: it must k-way-merge across segments, so it parallelizes one worker
    /// per segment (the structural ceiling), or runs serial when no workers are available.
    SortedPerSegment,
    /// The row-count heuristic (`compute_nworkers`): no ANALYZE stats, or an unsorted scan with no
    /// usable cost estimate. Caps workers so each gets at least `min_rows_per_worker` rows.
    RowHeuristic,
}

impl WorkerDecisionReason {
    /// Reader-facing label for EXPLAIN VERBOSE: each names the decision branch the paths came from.
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::BlockWandPrunable => "Prunable top-K",
            Self::CostModel => "Cost model",
            Self::CostModelLimited => "Cost model (LIMIT)",
            Self::SortedPerSegment => "Per-segment",
            Self::RowHeuristic => "Row-capped",
        }
    }
}

/// How `create_custom_path` should shape the paths it emits for one exec method: one serial path,
/// one partial path, or both (and let PostgreSQL pick).
#[derive(Clone, Copy)]
pub(super) enum WorkerPathPolicy {
    /// Emit exactly one serial path. pg_search determined parallelism is wrong or impossible here:
    /// prunable top-K, no workers to split across, or a serial verdict from the cost-model-limited or
    /// row-heuristic test.
    SerialOnly { reason: WorkerDecisionReason },
    /// Emit exactly one partial (parallel-aware) path, binding PostgreSQL to parallel. Used where
    /// pg_search must force parallel: an uncostable sorted scan, a parallel row-heuristic verdict, a
    /// cost-model-limited winner ([`cost_test_limited`]), or a `debug_parallel_query`-forced costable
    /// scan.
    ParallelOnly {
        nworkers: NonZeroUsize,
        reason: WorkerDecisionReason,
    },
    /// Emit BOTH a serial and a partial path and let PostgreSQL cost the Gather and choose. Used for
    /// costable scans with no effective LIMIT, where PostgreSQL can compare serial-vs-parallel fairly.
    CostedBoth {
        nworkers: NonZeroUsize,
        reason: WorkerDecisionReason,
    },
}

impl WorkerPathPolicy {
    pub(super) fn reason(self) -> WorkerDecisionReason {
        match self {
            Self::SerialOnly { reason }
            | Self::ParallelOnly { reason, .. }
            | Self::CostedBoth { reason, .. } => reason,
        }
    }
}

/// How a parallel scan's work divides: `nworkers`, plus the leader (a full share) when it participates.
pub(super) fn parallel_divisor(nworkers: NonZeroUsize, leader_participates: bool) -> f64 {
    let nworkers = nworkers.get();
    if leader_participates {
        (nworkers + 1) as f64
    } else {
        nworkers as f64
    }
}

pub(super) struct PathCostBasis {
    pub(super) parallelizable_cost: f64,
}

/// Tantivy drive cost and the match count it scales against (see [`drive_fraction`]).
pub(super) struct DriveCost {
    pub(super) cost: u64,
    pub(super) matches: f64,
}

/// The scan's drive cost in PostgreSQL units: the Tantivy drive cost scaled to PG cost units and by
/// the early-termination `fraction`.
fn drive_work(drive_cost: u64, fraction: f64) -> f64 {
    drive_cost as f64 * cpu_index_tuple_cost() * fraction
}

/// Share of the docset the scan must drive before it can stop: a sorted scan scores every match
/// (1.0); an unsorted LIMIT scan stops at ~LIMIT/matches; 1.0 otherwise. No log2(LIMIT) term -- a
/// non-prunable scan examines each matching doc once, so LIMIT's only effect is this share.
fn drive_fraction(is_sorted: bool, limit: Option<f64>, matches: f64) -> f64 {
    if is_sorted {
        1.0
    } else {
        limit.map_or(1.0, |limit| (limit / matches).min(1.0))
    }
}

/// `Query::cost`, memoized so the index opens at most once per query.
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

/// The query's Tantivy drive cost when the scan is costable, else `None` -- `None` for a
/// runtime-bound/correlated/external predicate (no resolved value to open a scorer for) or an open
/// failure. Memoized so the index opens at most once per query.
///
/// # Safety
/// `root` must point to a valid `PlannerInfo` for the duration of this call.
pub(super) unsafe fn costable_drive_cost(
    query: &SearchQueryInput,
    bm25_index: &PgSearchRelation,
    quals: &Qual,
    root: *mut pg_sys::PlannerInfo,
    cost_memo: &mut CostMemo,
) -> Option<u64> {
    if quals.contains_exprs()
        || quals.contains_external_var()
        || quals.contains_correlated_param(root)
    {
        return None;
    }
    cost_memo.get_or_compute(|| estimate_query_cost(bm25_index, query.clone()))
}

/// Cost-model leaf for a costable effective-LIMIT scan (see module docs for why pg_search forces
/// this instead of offering both paths). Parallelize when splitting the scan `work` (drive cost
/// scaled by the early-termination `fraction`) across workers saves more than the Gather overhead,
/// which is costed on `base_result_rows` (`= k`, the rows that actually cross).
fn cost_test_limited(
    drive_cost: u64,
    nworkers: NonZeroUsize,
    is_sorted: bool,
    limit: Option<f64>,
    matches: f64,
    base_result_rows: f64,
    parallel_leader_participates: bool,
) -> WorkerPathPolicy {
    // A zero `drive_cost` (sampled largest segment matched nothing) yields work = 0 -> serial: a
    // genuinely tiny match set reads zero too, indistinguishable from a skewed sample miss.
    let work = drive_work(drive_cost, drive_fraction(is_sorted, limit, matches));

    // Gather overhead: `parallel_setup_cost` plus per-row transport of the `k` crossing rows (1.05
    // covers Gather-Merge IPC).
    let divisor = parallel_divisor(nworkers, parallel_leader_participates);
    const GATHER_MERGE_IPC_FACTOR: f64 = 1.05;
    let gather_overhead =
        parallel_setup_cost() + parallel_tuple_cost() * base_result_rows * GATHER_MERGE_IPC_FACTOR;
    if work / divisor + gather_overhead < work {
        WorkerPathPolicy::ParallelOnly {
            nworkers,
            reason: WorkerDecisionReason::CostModelLimited,
        }
    } else {
        WorkerPathPolicy::SerialOnly {
            reason: WorkerDecisionReason::CostModelLimited,
        }
    }
}

/// Inputs to [`decide_scan_parallelism`].
pub(super) struct ScanParallelismInputs<'a> {
    pub(super) prunability: TopKPrunability,
    pub(super) query: &'a SearchQueryInput,
    /// The costable drive cost (see [`costable_drive_cost`]); `Some` marks the scan as costable and
    /// feeds both [`cost_test_limited`] and [`estimate_path_cost`].
    pub(super) drive_cost: Option<u64>,
    pub(super) row_estimate: RowEstimate,
    pub(super) is_sorted: bool,
    pub(super) limit: Option<f64>,
    /// The LIMIT-capped output cardinality (`min(matches, limit)`). `< matches` exactly when an
    /// effective LIMIT is in play, which routes a costable scan to [`cost_test_limited`].
    pub(super) base_result_rows: f64,
    pub(super) segment_count: usize,
    pub(super) consider_parallel: bool,
    pub(super) quals: &'a Qual,
    pub(super) root: *mut pg_sys::PlannerInfo,
    pub(super) parallel_leader_participates: bool,
    pub(super) is_join_context: bool,
    /// True when a `GROUP BY` / `SELECT DISTINCT` sits above this scan; routes it through the row
    /// heuristic because PG under-costs the serial HashAggregate for non-collapsing grouping.
    pub(super) has_grouping: bool,
}

/// Decide how to shape a scan's paths. The numbered steps mirror the module-level decision tree;
/// the inline comments carry the per-branch rationale.
///
/// # Safety
/// `inputs.root` must point to a valid `PlannerInfo` for the duration of this call.
pub(super) unsafe fn decide_scan_parallelism(inputs: ScanParallelismInputs) -> WorkerPathPolicy {
    let ScanParallelismInputs {
        prunability,
        query,
        drive_cost,
        row_estimate,
        is_sorted,
        limit,
        base_result_rows,
        segment_count,
        consider_parallel,
        quals,
        root,
        parallel_leader_participates,
        is_join_context,
        has_grouping,
    } = inputs;

    // 1. Prunable short-circuit -> serial only. Block-WAND keeps a score-DESC single term sublinear
    //    (#4664), so workers would only add overhead. A hard gate: the cost model can't see
    //    Block-WAND, and the predicate must actually prune, not just look like a term.
    if matches!(prunability, TopKPrunability::PrunableCandidate) && query.is_topk_prunable() {
        return WorkerPathPolicy::SerialOnly {
            reason: WorkerDecisionReason::BlockWandPrunable,
        };
    }

    let external_var = quals.contains_external_var();
    let correlated = quals.contains_correlated_param(root);

    // The row heuristic (`compute_nworkers`): used by the two fallback branches below, so computed
    // lazily here. It commits to serial or parallel itself (no cost to hand PostgreSQL).
    let heuristic_policy = || {
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
        match NonZeroUsize::new(nworkers) {
            Some(nworkers) => WorkerPathPolicy::ParallelOnly {
                nworkers,
                reason: WorkerDecisionReason::RowHeuristic,
            },
            None => WorkerPathPolicy::SerialOnly {
                reason: WorkerDecisionReason::RowHeuristic,
            },
        }
    };

    // 2. No row stats (table never ANALYZEd) -> the row heuristic. Routing every unanalyzed scan
    //    here (not just the uncostable ones) keeps the decision consistent regardless of shape.
    let Some(matches) = row_estimate.known_rows() else {
        return heuristic_policy();
    };

    // Structural ceiling from `max_useful_workers`: 0 (serial) for a single segment, parallelism
    // off, or correlated/external -- though `debug_parallel_query` bumps a single segment to one.
    let structural_workers = if consider_parallel {
        max_useful_workers(segment_count, external_var, correlated)
    } else {
        0
    };

    #[cfg(not(feature = "pg15"))]
    let debug_parallel_query = unsafe { pg_sys::debug_parallel_query != 0 };
    #[cfg(feature = "pg15")]
    let debug_parallel_query = false;

    // 3. Route on costability + shape:
    match (drive_cost, is_sorted) {
        // Costable -- the cost-model branch. A zero structural ceiling leaves nothing to split.
        (Some(drive_cost), _) => match NonZeroUsize::new(structural_workers) {
            None => WorkerPathPolicy::SerialOnly {
                reason: WorkerDecisionReason::CostModel,
            },
            // `debug_parallel_query` forces parallel for testing; emit the partial path the cost
            // model would otherwise serialize. Reason stays `CostModel` (the Gather shows the forced
            // outcome). Only the costable branch needs this -- uncostable scans honor the GUC via
            // `compute_nworkers` (row-capped), so handling it earlier would bypass that cap.
            Some(nworkers) if debug_parallel_query => WorkerPathPolicy::ParallelOnly {
                nworkers,
                reason: WorkerDecisionReason::CostModel,
            },
            // Effective LIMIT (`k < matches`): PostgreSQL over-costs the bounded Gather, so pg_search
            // costs it on `k` and forces the winner.
            Some(nworkers) if base_result_rows < matches => cost_test_limited(
                drive_cost,
                nworkers,
                is_sorted,
                limit,
                matches,
                base_result_rows,
                parallel_leader_participates,
            ),
            // No effective LIMIT, join input: a Parallel Hash Join needs a partial path, but
            // `parallel_setup_cost` dwarfs the per-scan work so PG cost-chooses serial. Force
            // parallel so the partial path exists and wins (mirrors main's force-for-joins, #4101).
            Some(nworkers) if is_join_context => WorkerPathPolicy::ParallelOnly {
                nworkers,
                reason: WorkerDecisionReason::CostModel,
            },
            // No effective LIMIT, grouping input (GROUP BY / SELECT DISTINCT): PG under-costs the
            // serial HashAggregate (`cost_agg` has no cache/bandwidth term) and over-charges the
            // Gather, so it picks serial even when parallel is ~3x faster on a high-cardinality group
            // set. Force via the row heuristic like main; scalar aggregates collapse and fall through.
            Some(_) if has_grouping => heuristic_policy(),
            // No effective LIMIT, non-grouping non-join: rows either all cross the Gather or collapse
            // (scalar aggregates), so PostgreSQL costs it correctly -- offer both and let it choose.
            Some(nworkers) => WorkerPathPolicy::CostedBoth {
                nworkers,
                reason: WorkerDecisionReason::CostModel,
            },
        },
        // Uncostable sorted: must visit every segment, so workers always help. Parallel when workers
        // exist, else serial.
        (None, true) => match NonZeroUsize::new(structural_workers) {
            Some(nworkers) => WorkerPathPolicy::ParallelOnly {
                nworkers,
                reason: WorkerDecisionReason::SortedPerSegment,
            },
            None => WorkerPathPolicy::SerialOnly {
                reason: WorkerDecisionReason::SortedPerSegment,
            },
        },
        // Uncostable unsorted: it can stop early at LIMIT, so the row heuristic's caps fit;
        // full structural parallelism would over-do it.
        (None, false) => heuristic_policy(),
    }
}

/// A scan path's intrinsic cost, independent of the parallel divisor: the scan work PG cannot infer
/// from output rows, plus the local output/materialization cost PG already understands. `rows` stays
/// `base_result_rows` at the call site so Gather costing still charges only the tuples that cross.
pub(super) fn estimate_path_cost(
    is_sorted: bool,
    per_tuple_cost: f64,
    base_result_rows: f64,
    drive: Option<DriveCost>,
    limit: Option<f64>,
) -> PathCostBasis {
    let output_cost = base_result_rows * per_tuple_cost;
    let scan_work = drive
        .map(|drive| drive_work(drive.cost, drive_fraction(is_sorted, limit, drive.matches)))
        .unwrap_or(0.0);

    PathCostBasis {
        parallelizable_cost: scan_work + output_cost,
    }
}
