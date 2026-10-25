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

//! Physical optimizer rule (`RankJoinRule`) that rewrites eligible top-K scored
//! `HashJoinExec` nodes into [`RankJoinExec`] and configures leaf [`PgSearchScanPlan`]s
//! with [`LazySortConfig`].
//!
//! # Plan-Time Heuristics & Query Shapes
//!
//! `RankJoinRule` chooses between one-pass `HashJoinExec` (which scans in `DocId` order and
//! pushes a build-side dynamic filter) and pull-based [`RankJoinExec`] (`SA` expanding Top-K
//! windows + lazy `RA` `TermSetQuery` point lookups) using three gates:
//!
//! 1. **Gate 1 (Leaf `SA` / `RA` Eligibility)**:
//!    - Scored leaves (`OrderByFeature::Score`) must have at least one non-constant-score BM25
//!      query (`!SearchQueryInput::is_constant_score`) so Tantivy's `TopDocs` / Block-Max WAND
//!      collector can prune by score.
//!    - Non-score fast-field sorts (`OrderByFeature::Field`) are allowed only when every join on
//!      the path from the root `RankJoinExec` to the leaf is single-sided (`LeftScore` or
//!      `RightScore`), and the column is a non-deferred sortable fast field.
//!    - When the root `HashJoinExec` has a pure-`OR` cross-table search tag filter
//!      (`ScanMode::Tagged`), `try_elide_disjunctive_tags` attempts to strip the boolean
//!      `__<alias>_tag_<idx>` columns and convert the leaves to `ScanMode::Disjunctive` for
//!      two-sided score joins (falling back to the original `ScanMode::Tagged` subtree if the
//!      join is single-sided or fails any gate).
//!
//! 2. **Gate 2 (Score Contribution Balance & Disjunctive Restriction)**:
//!    - When both sides contribute to the monotonic `ORDER BY` expression (`uses_left &&
//!      uses_right`), their maximum score swings `delta_l`, `delta_r` must satisfy
//!      `min(delta_l, delta_r) >= 0.20 * max(delta_l, delta_r)`
//!      ([`SCORE_BALANCE_THRESHOLD`]) so heavily lopsided expressions do not scan the weaker
//!      side to completion.
//!    - When `paradedb.rank_join_min_rows > 0`, two-sided `RankJoinExec` is restricted to
//!      tagless disjunctive (`ScanMode::Disjunctive`) subtrees (where `HashJoinExec` over
//!      `ScanMode::Tagged` would otherwise have to scan 100% of every table with
//!      `base_query = All`), keeping conjunctive (`ScanMode::Standard`) two-sided joins on
//!      parallel `HashJoinExec` with dynamic filter pushdown.
//!
//! 3. **Gate 3 (Cardinality, Unscored Selectivity & Initial Window Scaling)**:
//!    - Requires `K <= 1,000` ([`MAX_RANK_JOIN_K`]), estimated rows `>=`
//!      `paradedb.rank_join_min_rows` on every joined input, and combined unscored (`RA`)
//!      selectivity `>= 0.08` ([`UNSCORED_SELECTIVITY_THRESHOLD`]) in single-sided (`NRJN`)
//!      joins.
//!    - Scales each `SA` leaf's `initial_window` inversely by the subtree survival probability
//!      `p_survive = p_unscored * p_tagged` via
//!      [`TopKSelectivity::compute_initial_window`].
//!
//! # Why Physical-Time Optimization & Tag Elision (`ScanMode::Tagged` -> `ScanMode::Disjunctive`)
//!
//! During initial logical plan construction (`JoinScan::validate_and_build_clause` and
//! `build_source_df`), cross-table search predicates are always lowered to `ScanMode::Tagged`
//! with boolean match-tag columns (`__<alias>_tag_<idx>`) on each leaf `PgSearchTableProvider`.
//! Keeping search predicates as ordinary boolean tag columns throughout logical optimization
//! allows DataFusion's standard logical passes (`PushDownFilter`, `OuterJoinToInner`,
//! `SimplifyExpressions`, `OptimizeProjections`) to push, simplify, and project them uniformly
//! without committing to a join strategy before physical plan shape is known.
//!
//! Deciding whether a join subtree can use [`RankJoinExec`] (and whether a disjunctive tag
//! filter can be absorbed into `ScanMode::Disjunctive` leaf scans) depends on physical-plan
//! properties that only exist after physical planning:
//! - whether `TopKPushdown` produced a top-level `SortExec(fetch=K)` with `K <= 1,000`,
//! - the concrete `HashJoinExec` join tree, join keys, and `TantivyDecodeExec` placement, and
//! - the opened `SearchIndexReader` segment and selectivity estimates required by Gates 1-3.
//!
//! When a root `HashJoinExec` has a pure-`OR` cross-table tag `JoinFilter`, `RankJoinRule`
//! tentatively "undoes" the logical tag-column rewrite via [`try_elide_disjunctive_tags`]:
//! stripping the `__<alias>_tag_<idx>` columns from leaf scans and intermediate projections/joins,
//! removing the `JoinFilter`, and converting the leaves from `ScanMode::Tagged` to
//! `ScanMode::Disjunctive`. If the resulting candidate passes all rank-join eligibility gates,
//! it is rewritten to [`RankJoinExec`]; otherwise `RankJoinRule` falls back to the original
//! `ScanMode::Tagged` subtree so `HashJoinExec` (or a single-sided `NRJN` join) still evaluates
//! the boolean tag filter.
//!
//! # Optimizer Rule Ordering & MPP Co-Partitioning
//!
//! In `build_base_session` (`pg_search/src/postgres/customscan/joinscan/scan_state.rs`),
//! `RankJoinRule` is registered in the physical optimizer pipeline:
//! - *after* `LateMaterializationRule` and `RangeCoPartitionedJoinRule`, so deferred column
//!   decode nodes (`TantivyDecodeExec`) and range-partitioned join modes are already placed.
//!   Because `RankJoinExec` requires in-memory `DynamicFilterPhysicalExpr` RPC channels within
//!   each task and cross-stage dynamic filters are not yet supported (TODO:
//!   <https://github.com/paradedb/paradedb/issues/5559>), `subtree_join_partition_mode`
//!   selects `PartitionMode::Partitioned` (preserving `RangePartitioned` leaf scans for single-stage
//!   MPP execution) iff every `HashJoinExec` in the subtree was range co-partitioned on the same
//!   partition count (`> 1`); otherwise it falls back to `PartitionMode::CollectLeft` and collapses
//!   leaf scans to a single `Shared` partition;
//! - *before* `EnsureRequirements`, so `EnforceSorting` sees that `RankJoinExec` satisfies the
//!   `SortExec(fetch=K)` ordering and replaces `SortExec(fetch=K)` with `GlobalLimitExec(fetch=K)`
//!   (or `SortPreservingMergeExec(fetch=K)` over `LocalLimitExec(fetch=K)` when `Partitioned`);
//! - *before* `FilterPushdown` (post-optimization phase), so
//!   `RankJoinExec::gather_filters_for_pushdown` can push its `DynamicFilterPhysicalExpr`
//!   (`RankLookupPhysicalExpr`) down through `ProjectionExec` and nested `RankJoinExec`s to leaf
//!   `PgSearchScanPlan`s;
//! - *before* `DeferredPlacementRule`, so `make_fields_eager` can pull any secondary tie-breaker
//!   columns referenced by `RankJoinExec` into the leaf scans before final placement.

use std::collections::BTreeSet;
use std::sync::Arc;

use arrow_array::{RecordBatch, RecordBatchOptions};
use arrow_schema::{DataType, Schema, SchemaRef, SortOptions};
use datafusion::common::config::ConfigOptions;
use datafusion::common::tree_node::{Transformed, TreeNode, TreeNodeRecursion};
use datafusion::common::{Result, ScalarValue};
use datafusion::logical_expr::{ColumnarValue, JoinType, Operator};
use datafusion::physical_expr::expressions::{BinaryExpr, CastExpr, Column, Literal, TryCastExpr};
use datafusion::physical_expr::projection::ProjectionExpr;
use datafusion::physical_expr::utils::collect_columns;
use datafusion::physical_expr::{LexOrdering, PhysicalExpr, PhysicalSortExpr};
use datafusion::physical_optimizer::PhysicalOptimizerRule;
use datafusion::physical_plan::coalesce_partitions::CoalescePartitionsExec;
use datafusion::physical_plan::joins::utils::JoinFilter;
use datafusion::physical_plan::joins::{HashJoinExec, PartitionMode};
use datafusion::physical_plan::projection::ProjectionExec;
use datafusion::physical_plan::sorts::sort::SortExec;
use datafusion::physical_plan::{
    ExecutionPlan, ExecutionPlanProperties, Partitioning, replace_children_if_necessary,
};

use crate::api::{FieldName, OrderByFeature, OrderByInfo, SortDirection};
use crate::postgres::customscan::joinscan::ScoreColumn;
use crate::postgres::customscan::joinscan::range_partitioning_rule::is_hash_join_range_co_partitioned;
use crate::postgres::customscan::joinscan::visibility_filter::VisibilityFilterExec;
use crate::query::SearchQueryInput;
use crate::scan::ScanMode;
use crate::scan::deferred_lookup::PhysicalDeferredField;
use crate::scan::execution_plan::{
    LazySortConfig, PgSearchScanPlan, SharedThresholdMode, TopKSelectivity,
};
use crate::scan::rank_join_exec::{
    InputScorePair, RankJoinExec, RankJoinInputs, RankJoinSortExprs, ScoreCombiner,
};
use crate::scan::tantivy_decode_exec::TantivyDecodeExec;
use crate::scan::tantivy_fetch_exec::TantivyFetchExec;

/// Maximum `LIMIT + OFFSET` (`K`) eligible for `RankJoinExec`.
pub(crate) const MAX_RANK_JOIN_K: usize = 1_000;

/// Minimum ratio `min(delta_l_max, delta_r_max) / max(delta_l_max, delta_r_max)`
/// required by Gate 2 so heavily lopsided two-sided score queries do not force `RankJoinExec`
/// to scan the weaker-scored input to completion.
pub(crate) const SCORE_BALANCE_THRESHOLD: f64 = 0.20;

/// Minimum estimated selectivity (`planner_estimated_rows / total_docs`) required across
/// unscored (`RA`) inputs of a single-sided (`NRJN`) join when `min_rows > 0`.
///
/// When the unscored build side is selective, most rows pulled from the `SA` stream miss during
/// `RA` point lookups, requiring repeated `SA` window expansions on a single thread while
/// parallel `HashJoinExec` can build a small hash table on the selective input and push a join
/// dynamic filter into the `SA` scan.
///
/// Note: The `0.08` (`8%`) cutoff is somewhat arbitrary and is calibrated around Tantivy's
/// `DocSet::size_hint()` behavior when `QueryEstimate::estimate_docs` falls back to
/// `scorer_estimate`:
/// - `FastFieldRangeQuery` (`RangeDocSet::size_hint()`) has no histogram and unconditionally
///   reports `column.num_docs() / 10` (`~0.09999` after integer division on the largest segment),
///   so a threshold `<= 0.099` is required to admit fast-field range filters (e.g.
///   `u.reputation > 100`) on the unscored side.
/// - `TermSetQuery` reports `~1.0` (`max_doc - first_match`) under `Gallop` and `LinearScan`,
///   and reports the exact segment match count `bitset.len()` under `BitsetFromPostings`
///   (admitting multi-row term sets that match `>= 8%` of the table while keeping small/unique
///   `BitsetFromPostings` sets on `HashJoinExec`).
/// - Inverted-index text queries (`TermQuery`, `BooleanQuery`) use actual `doc_freq` statistics,
///   so selective text filters on the unscored side (e.g. `~6%` in
///   `join_top_k_score_desc_low_selectivity`) fall below `0.08` and stay on parallel `HashJoinExec`.
pub(crate) const UNSCORED_SELECTIVITY_THRESHOLD: f64 = 0.08;

/// Physical optimizer rule that replaces top-K scored `HashJoinExec`s with `RankJoinExec`.
#[derive(Debug, Default)]
pub struct RankJoinRule;

impl RankJoinRule {
    pub fn new() -> Self {
        Self
    }
}

impl PhysicalOptimizerRule for RankJoinRule {
    fn name(&self) -> &str {
        "RankJoinRule"
    }

    fn schema_check(&self) -> bool {
        true
    }

    fn optimize(
        &self,
        plan: Arc<dyn ExecutionPlan>,
        _config: &ConfigOptions,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        if !crate::gucs::enable_rank_join() {
            return Ok(plan);
        }
        let min_rows = crate::gucs::rank_join_min_rows();

        let rewritten = plan
            .transform_down(|node| {
                let Some(sort_exec) = node.downcast_ref::<SortExec>() else {
                    return Ok(Transformed::no(node));
                };
                let Some(k) = sort_exec.fetch() else {
                    return Ok(Transformed::no(node));
                };
                if k == 0 || k > MAX_RANK_JOIN_K {
                    return Ok(Transformed::no(node));
                };

                let Some(rewritten_input) =
                    try_rewrite_under_sort(sort_exec.input(), sort_exec.expr(), k, min_rows)?
                else {
                    return Ok(Transformed::no(node));
                };

                let new_sort =
                    replace_children_if_necessary(Arc::clone(&node), vec![rewritten_input])?;
                Ok(Transformed::new(new_sort, true, TreeNodeRecursion::Jump))
            })
            .map(|t| t.data)?;

        debug_assert!(
            !any_scan(&rewritten, |s| s.is_tagless_disjunctive()
                && s.lazy_sort().is_none()),
            "RankJoinRule produced a ScanMode::Disjunctive scan without lazy_sort"
        );

        Ok(rewritten)
    }
}

/// Statistics and score range for a join input subtree used by Gates 2 and 3
/// and for sizing the initial `SA` window via [`TopKSelectivity`].
#[derive(Debug, Clone, Copy)]
pub(crate) struct SubtreeScoreStats {
    pub estimated_rows: u64,
    pub unscored_selectivity: TopKSelectivity,
    pub tagged_selectivity: Option<TopKSelectivity>,
    pub score_min: f64,
    pub score_max: f64,
}

impl SubtreeScoreStats {
    /// Combined survival probability of a candidate row pulled from the `SA` scan across
    /// all unscored (`RA`) inputs and any cross-table `ScanMode::Tagged` disjunction.
    pub(crate) fn survival_selectivity(&self) -> TopKSelectivity {
        match self.tagged_selectivity {
            Some(tagged) => self.unscored_selectivity.and(tagged),
            None => self.unscored_selectivity,
        }
    }

    /// Verifies Gate 1 (non-constant BM25 score when `is_score_sort` is true) and computes
    /// `[0.0, score_max]` for a leaf scan.
    pub(crate) fn from_leaf_queries(
        is_score_sort: bool,
        base_query: &SearchQueryInput,
        disjunct_queries: &[SearchQueryInput],
        estimated_rows: u64,
    ) -> Option<Self> {
        let base_has_score = !base_query.is_constant_score();
        let disjuncts_have_score = disjunct_queries.iter().any(|q| !q.is_constant_score());
        if is_score_sort && !base_has_score && !disjuncts_have_score {
            return None;
        }
        let score_max = if is_score_sort {
            let mut total = if base_has_score {
                base_query.estimate_max_score()
            } else {
                0.0
            };
            for q in disjunct_queries {
                if !q.is_constant_score() {
                    total += q.estimate_max_score();
                }
            }
            total.max(1.0)
        } else {
            1.0
        };
        Some(Self {
            estimated_rows,
            unscored_selectivity: TopKSelectivity::ALL,
            tagged_selectivity: None,
            score_min: 0.0,
            score_max,
        })
    }

    pub(crate) fn combine_tagged(
        left: Option<TopKSelectivity>,
        right: Option<TopKSelectivity>,
    ) -> Option<TopKSelectivity> {
        match (left, right) {
            (Some(l), Some(r)) => Some(l.or(r)),
            (Some(l), None) => Some(l),
            (None, Some(r)) => Some(r),
            (None, None) => None,
        }
    }

    /// Verifies Gate 2 (score balance when both sides are scored) and Gate 3 (`min_rows` and
    /// `UNSCORED_SELECTIVITY_THRESHOLD`), and computes the combined `[score_min, score_max]`
    /// across `combiner`.
    pub(crate) fn combine(
        combiner: &ScoreCombiner,
        left: Self,
        right: Self,
        min_rows: usize,
    ) -> Option<Self> {
        let f_max_max = combiner.eval(InputScorePair {
            left: left.score_max,
            right: right.score_max,
        });
        let f_min_max = combiner.eval(InputScorePair {
            left: left.score_min,
            right: right.score_max,
        });
        let f_max_min = combiner.eval(InputScorePair {
            left: left.score_max,
            right: right.score_min,
        });

        // Gate 2: When both sides are scored, both must contribute comparably to the combined score.
        if combiner.uses_left() && combiner.uses_right() {
            let delta_l = (f_max_max - f_min_max).abs();
            let delta_r = (f_max_max - f_max_min).abs();
            if delta_l <= 0.0 || delta_r <= 0.0 || !delta_l.is_finite() || !delta_r.is_finite() {
                return None;
            }
            if delta_l.min(delta_r) < SCORE_BALANCE_THRESHOLD * delta_l.max(delta_r) {
                return None;
            }
        }

        let min_matched = left.estimated_rows.min(right.estimated_rows);
        let unscored_selectivity = left.unscored_selectivity.and(right.unscored_selectivity);
        let tagged_selectivity =
            Self::combine_tagged(left.tagged_selectivity, right.tagged_selectivity);

        // Gate 3: Both inputs must meet the minimum row threshold (`rank_join_min_rows`),
        // and any unscored (`RA`) inputs in a single-sided (`NRJN`) join must meet
        // `UNSCORED_SELECTIVITY_THRESHOLD`.
        if min_rows > 0 {
            if min_matched < min_rows as u64 {
                return None;
            }
            if unscored_selectivity.0 < UNSCORED_SELECTIVITY_THRESHOLD {
                return None;
            }
        }

        let corners = [
            combiner.eval(InputScorePair {
                left: left.score_min,
                right: right.score_min,
            }),
            f_min_max,
            f_max_min,
            f_max_max,
        ];
        Some(Self {
            estimated_rows: min_matched,
            unscored_selectivity,
            tagged_selectivity,
            score_min: corners.into_iter().fold(f64::INFINITY, f64::min),
            score_max: corners.into_iter().fold(f64::NEG_INFINITY, f64::max),
        })
    }
}

/// Monotonicity direction of a subexpression with respect to the top-level sort expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MonotonicDirection {
    /// Larger values of the subexpression increase the parent expression's value.
    Increasing,
    /// Larger values of the subexpression decrease the parent expression's value.
    Decreasing,
}

impl MonotonicDirection {
    fn flip(self) -> Self {
        match self {
            MonotonicDirection::Increasing => MonotonicDirection::Decreasing,
            MonotonicDirection::Decreasing => MonotonicDirection::Increasing,
        }
    }

    fn to_sort_options(self, root_options: SortOptions) -> SortOptions {
        match self {
            MonotonicDirection::Increasing => root_options,
            MonotonicDirection::Decreasing => SortOptions {
                descending: !root_options.descending,
                nulls_first: !root_options.nulls_first,
            },
        }
    }
}

/// Controls whether leaf scans in a join branch may sort by a non-score fast-field column
/// (`OrderByFeature::Field`). Non-score column sorting is only valid when every join from the
/// root `RankJoinExec` down to the leaf scan is single-sided (`ScoreCombiner::LeftScore` or
/// `ScoreCombiner::RightScore`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FieldSortPolicy {
    Allowed,
    Disallowed,
}

/// Whether `try_rewrite_scored_branch` is descending from the root `SortExec` (which must
/// terminate at a `HashJoinExec`) or from a parent `HashJoinExec` into one of its scored
/// children (which may terminate at either a nested `HashJoinExec` or a leaf `PgSearchScanPlan`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BranchLevel {
    RootJoin,
    ChildBranch,
}

/// Result of decomposing a join's primary score sort expression across its left and right children.
#[derive(Debug, Clone)]
pub(crate) struct DecomposedJoinScore {
    pub left_sort_expr: PhysicalSortExpr,
    pub right_sort_expr: PhysicalSortExpr,
    pub combiner: ScoreCombiner,
}

/// Walks down from `SortExec` through pass-through nodes to find, verify (Gates 1, 2, and 3),
/// and rewrite an eligible `HashJoinExec` subtree into `RankJoinExec` in a single pass.
///
/// When the root `HashJoinExec` has a pure-`OR` cross-table search tag filter (`ScanMode::Tagged`),
/// attempts to elide the tag columns and convert leaf scans to `ScanMode::Disjunctive` first
/// (for two-sided score joins where `SA` unions the base and disjunct queries and `RA` scores
/// both unconditionally). If the tagless candidate is rejected by `try_rewrite_scored_branch`
/// (e.g. single-sided `NRJN` join or failed gate), falls back to the original `ScanMode::Tagged`
/// subtree so the `JoinFilter` is never lost.
///
/// TODO: Reorder joins based on rank / score correlation rather than relying solely on the
/// selectivity-driven join tree topology chosen by PostgreSQL (`collect_join_sources_join_rel`).
/// In multi-way joins:
/// - For single-sided (`NRJN`) sorts (`A JOIN B JOIN C` sorted by `C`), if PostgreSQL groups the
///   two unscored relations first as `(A JOIN B) JOIN C`, `try_rewrite_unscored_branch` rejects the
///   rewrite because the unscored side is a `HashJoinExec` rather than a single leaf scan, whereas
///   reordering to `((C JOIN B) JOIN A)` places the sorted leaf `C` at the base of a left-deep
///   `RankJoinExec` chain that probes `B` and `A` via `RA`.
/// - For multi-sided scored joins, ordering joins by score contribution / rank correlation and
///   fanout (rather than pure filter selectivity) would tighten the top-K threshold earlier and
///   reduce wasted `SA` window expansions and `RA` point lookups.
fn try_rewrite_under_sort(
    node: &Arc<dyn ExecutionPlan>,
    required_ordering: &LexOrdering,
    k: usize,
    min_rows: usize,
) -> Result<Option<Arc<dyn ExecutionPlan>>> {
    let mut eager_fields = Vec::new();
    let mut rewritten_with_stats = None;
    if let Some(tagless_node) = try_elide_disjunctive_tags(node)? {
        let join_mode = subtree_join_partition_mode(&tagless_node)?;
        rewritten_with_stats = try_rewrite_scored_branch(
            &tagless_node,
            required_ordering,
            min_rows,
            join_mode,
            FieldSortPolicy::Allowed,
            BranchLevel::RootJoin,
            &mut eager_fields,
        )?;
    }
    if rewritten_with_stats.is_none() {
        eager_fields.clear();
        let join_mode = subtree_join_partition_mode(node)?;
        rewritten_with_stats = try_rewrite_scored_branch(
            node,
            required_ordering,
            min_rows,
            join_mode,
            FieldSortPolicy::Allowed,
            BranchLevel::RootJoin,
            &mut eager_fields,
        )?;
    }
    let Some((rewritten, root_stats)) = rewritten_with_stats else {
        return Ok(None);
    };
    let initial_window = compute_initial_window(k, root_stats.survival_selectivity());
    let rewritten = apply_sa_initial_window(rewritten, initial_window)?;
    let rewritten =
        crate::scan::deferred_placement_rule::make_fields_eager(rewritten, &eager_fields)?;
    Ok(Some(rewritten))
}

/// Returns `PartitionMode::Partitioned` iff every `HashJoinExec` in `node` is range
/// co-partitioned on the same partition count (`> 1`), with no `CoalescePartitionsExec`
/// inside `node` and all leaf `PgSearchScanPlan`s matching that range partition count.
/// When satisfied, the entire rank-join subtree executes within a single MPP stage without
/// cross-stage dynamic filters. Otherwise returns `PartitionMode::CollectLeft`.
///
/// TODO: Support cross-stage dynamic filters for full MPP `RankJoinExec` execution across
/// non-co-partitioned stages: <https://github.com/paradedb/paradedb/issues/5559>.
fn subtree_join_partition_mode(node: &Arc<dyn ExecutionPlan>) -> Result<PartitionMode> {
    let mut expected_partitions: Option<usize> = None;
    let mut all_co_partitioned = true;
    node.apply(|plan| {
        if plan.is::<CoalescePartitionsExec>() {
            all_co_partitioned = false;
            return Ok(TreeNodeRecursion::Stop);
        }
        if let Some(join) = plan.downcast_ref::<HashJoinExec>() {
            if !is_hash_join_range_co_partitioned(join)? {
                all_co_partitioned = false;
                return Ok(TreeNodeRecursion::Stop);
            }
            let part_count = join.left().output_partitioning().partition_count();
            if expected_partitions
                .replace(part_count)
                .is_some_and(|prev| prev != part_count)
            {
                all_co_partitioned = false;
                return Ok(TreeNodeRecursion::Stop);
            }
        } else if let Some(scan) = plan.downcast_ref::<PgSearchScanPlan>() {
            let partitioning = scan.properties().output_partitioning();
            let part_count = partitioning.partition_count();
            if !matches!(partitioning, Partitioning::Range(_))
                || part_count <= 1
                || expected_partitions
                    .replace(part_count)
                    .is_some_and(|prev| prev != part_count)
            {
                all_co_partitioned = false;
                return Ok(TreeNodeRecursion::Stop);
            }
        }
        Ok(TreeNodeRecursion::Continue)
    })?;
    if all_co_partitioned && expected_partitions.is_some_and(|n| n > 1) {
        Ok(PartitionMode::Partitioned)
    } else {
        Ok(PartitionMode::CollectLeft)
    }
}

/// Checks whether the join subtree rooted under `node` has a root `HashJoinExec` whose `filter`
/// is a pure `OR` chain covering every `ScanMode::Tagged` predicate across every leaf scan in
/// the join subtree, with no intermediate `HashJoinExec` filters. If so, returns a candidate
/// plan with the root `HashJoinExec.filter` removed, the tag columns stripped from leaf scans
/// and intermediate projections/joins, and leaf scans converted to `ScanMode::Disjunctive`.
fn try_elide_disjunctive_tags(
    node: &Arc<dyn ExecutionPlan>,
) -> Result<Option<Arc<dyn ExecutionPlan>>> {
    let mut elided_tags = None;
    let mut matched_tags = BTreeSet::new();
    let Some((stripped_plan, root_old_to_new)) =
        strip_disjunctive_tags_from_plan(node, &mut elided_tags, &mut matched_tags)?
    else {
        return Ok(None);
    };

    // Every filter column must be produced by a leaf `ScanMode::Tagged` scan, and the output
    // schema at `node` (immediately below `SortExec`) must not expose any elided tag columns.
    let all_tags_matched = elided_tags.is_some_and(|tags| matched_tags.len() == tags.len());
    let schema_unchanged = root_old_to_new
        .iter()
        .copied()
        .eq((0..root_old_to_new.len()).map(Some));
    Ok((all_tags_matched && schema_unchanged).then_some(stripped_plan))
}

fn extract_pure_or_filter_columns(filter: &JoinFilter) -> Option<BTreeSet<String>> {
    let bin = filter.expression().downcast_ref::<BinaryExpr>()?;
    if *bin.op() != Operator::Or {
        return None;
    }
    let mut cols = BTreeSet::new();
    if !collect_or_filter_columns(filter.expression(), filter.schema(), &mut cols) {
        return None;
    }
    (!cols.is_empty()).then_some(cols)
}

fn collect_or_filter_columns(
    expr: &Arc<dyn PhysicalExpr>,
    filter_schema: &SchemaRef,
    out: &mut BTreeSet<String>,
) -> bool {
    if let Some(bin) = expr.downcast_ref::<BinaryExpr>()
        && *bin.op() == Operator::Or
    {
        return collect_or_filter_columns(bin.left(), filter_schema, out)
            && collect_or_filter_columns(bin.right(), filter_schema, out);
    }
    if let Some(col) = expr.downcast_ref::<Column>()
        && col.index() < filter_schema.fields().len()
    {
        out.insert(filter_schema.field(col.index()).name().clone());
        return true;
    }
    false
}

/// Positional `old_col_idx -> Option<new_col_idx>` mapping produced when stripping elided tag columns.
type ColumnIndexMap = Vec<Option<usize>>;

/// Verifies and strips `elided_tags` from leaf `PgSearchScanPlan`s through intermediate
/// `HashJoinExec`s and `ProjectionExec`s, returning the updated plan and the positional
/// `old_col_idx -> Option<new_col_idx>` mapping for `node`'s output schema.
fn strip_disjunctive_tags_from_plan(
    node: &Arc<dyn ExecutionPlan>,
    elided_tags: &mut Option<BTreeSet<String>>,
    matched_tags: &mut BTreeSet<String>,
) -> Result<Option<(Arc<dyn ExecutionPlan>, ColumnIndexMap)>> {
    if let Some(scan) = node.downcast_ref::<PgSearchScanPlan>() {
        let Some(tags) = elided_tags.as_ref() else {
            return Ok(None);
        };
        let Some((new_scan, old_to_new)) = scan.with_elided_disjunctive_tags(tags, matched_tags)
        else {
            return Ok(None);
        };
        return Ok(Some((Arc::new(new_scan), old_to_new)));
    }

    if let Some(proj) = node.downcast_ref::<ProjectionExec>() {
        let Some((new_input, child_old_to_new)) =
            strip_disjunctive_tags_from_plan(proj.input(), elided_tags, matched_tags)?
        else {
            return Ok(None);
        };
        let new_input_schema = new_input.schema();
        let mut new_exprs = Vec::with_capacity(proj.expr().len());
        let mut my_old_to_new = Vec::with_capacity(proj.expr().len());
        for proj_expr in proj.expr() {
            match remap_physical_expr_columns(
                &proj_expr.expr,
                &child_old_to_new,
                &new_input_schema,
            )? {
                Some(remapped_expr) => {
                    my_old_to_new.push(Some(new_exprs.len()));
                    new_exprs.push(ProjectionExpr::new(remapped_expr, proj_expr.alias.clone()));
                }
                None if proj_expr.expr.is::<Column>() => my_old_to_new.push(None),
                None => return Ok(None),
            }
        }
        if new_exprs.is_empty() {
            return Ok(None);
        }
        let new_proj = ProjectionExec::try_new(new_exprs, new_input)?;
        return Ok(Some((Arc::new(new_proj), my_old_to_new)));
    }

    if let Some(join) = node.downcast_ref::<HashJoinExec>() {
        if !is_hash_join_eligible(join) {
            return Ok(None);
        }
        if elided_tags.is_none() {
            let Some(tags) = join.filter().and_then(extract_pure_or_filter_columns) else {
                return Ok(None);
            };
            *elided_tags = Some(tags);
        } else if join.filter().is_some() {
            return Ok(None);
        }
        let Some((new_left, left_map)) =
            strip_disjunctive_tags_from_plan(join.left(), elided_tags, matched_tags)?
        else {
            return Ok(None);
        };
        let Some((new_right, right_map)) =
            strip_disjunctive_tags_from_plan(join.right(), elided_tags, matched_tags)?
        else {
            return Ok(None);
        };
        let new_left_schema = new_left.schema();
        let new_right_schema = new_right.schema();
        let mut new_on = Vec::with_capacity(join.on().len());
        for (l_key, r_key) in join.on() {
            let Some(new_l) = remap_physical_expr_columns(l_key, &left_map, &new_left_schema)?
            else {
                return Ok(None);
            };
            let Some(new_r) = remap_physical_expr_columns(r_key, &right_map, &new_right_schema)?
            else {
                return Ok(None);
            };
            new_on.push((new_l, new_r));
        }

        let new_left_len = new_left_schema.fields().len();
        let mut unproj_map = Vec::with_capacity(left_map.len() + right_map.len());
        unproj_map.extend(left_map.iter().copied());
        unproj_map.extend(
            right_map
                .iter()
                .map(|opt| opt.map(|new_r| new_left_len + new_r)),
        );

        let (new_projection, my_old_to_new) = match &join.projection {
            Some(proj_indices) => {
                let mut new_proj = Vec::with_capacity(proj_indices.len());
                let mut out_map = Vec::with_capacity(proj_indices.len());
                for &old_unproj_idx in proj_indices.iter() {
                    let mapped = unproj_map.get(old_unproj_idx).copied().flatten();
                    out_map.push(mapped.map(|_| new_proj.len()));
                    if let Some(new_unproj_idx) = mapped {
                        new_proj.push(new_unproj_idx);
                    }
                }
                if new_proj.is_empty() {
                    return Ok(None);
                }
                (Some(new_proj), out_map)
            }
            None => (None, unproj_map),
        };

        let new_join = HashJoinExec::try_new(
            new_left,
            new_right,
            new_on,
            None,
            join.join_type(),
            new_projection,
            *join.partition_mode(),
            join.null_equality(),
            join.null_aware,
        )?;
        return Ok(Some((Arc::new(new_join), my_old_to_new)));
    }

    if is_passthrough_node(node) {
        let Some((new_child, child_old_to_new)) =
            strip_disjunctive_tags_from_plan(node.children()[0], elided_tags, matched_tags)?
        else {
            return Ok(None);
        };
        if !node.is::<CoalescePartitionsExec>()
            && !child_old_to_new
                .iter()
                .copied()
                .eq((0..child_old_to_new.len()).map(Some))
        {
            return Ok(None);
        }
        let rebuilt = replace_children_if_necessary(Arc::clone(node), vec![new_child])?;
        return Ok(Some((rebuilt, child_old_to_new)));
    }

    Ok(None)
}

fn remap_physical_expr_columns(
    expr: &Arc<dyn PhysicalExpr>,
    old_to_new: &[Option<usize>],
    new_schema: &SchemaRef,
) -> Result<Option<Arc<dyn PhysicalExpr>>> {
    map_expr_columns(expr, |col| {
        let new_idx = old_to_new.get(col.index()).copied().flatten()?;
        let field = new_schema.fields().get(new_idx)?;
        Some(Arc::new(Column::new(field.name(), new_idx)))
    })
}

fn map_expr_columns(
    expr: &Arc<dyn PhysicalExpr>,
    mut map_col: impl FnMut(&Column) -> Option<Arc<dyn PhysicalExpr>>,
) -> Result<Option<Arc<dyn PhysicalExpr>>> {
    let mut valid = true;
    let rewritten = Arc::clone(expr)
        .transform_down(|node| {
            if let Some(col) = node.downcast_ref::<Column>() {
                let Some(replacement) = map_col(col) else {
                    valid = false;
                    return Ok(Transformed::no(node));
                };
                return Ok(Transformed::new(replacement, true, TreeNodeRecursion::Jump));
            }
            Ok(Transformed::no(node))
        })?
        .data;
    Ok(valid.then_some(rewritten))
}

fn map_ordering_columns(
    ordering: &LexOrdering,
    mut map_col: impl FnMut(&Column) -> Option<Arc<dyn PhysicalExpr>>,
) -> Result<Option<LexOrdering>> {
    let mut mapped = Vec::with_capacity(ordering.len());
    for sort_expr in ordering.iter() {
        let Some(expr) = map_expr_columns(&sort_expr.expr, &mut map_col)? else {
            return Ok(None);
        };
        mapped.push(PhysicalSortExpr {
            expr,
            options: sort_expr.options,
        });
    }
    Ok(LexOrdering::new(mapped))
}

fn compute_initial_window(k: usize, selectivity: TopKSelectivity) -> f64 {
    let scale_factor = crate::gucs::topk_retry_scale_factor().max(2) as f64
        * crate::gucs::limit_fetch_multiplier();
    selectivity.compute_initial_window(k, scale_factor)
}

fn apply_sa_initial_window(
    plan: Arc<dyn ExecutionPlan>,
    initial_window: f64,
) -> Result<Arc<dyn ExecutionPlan>> {
    plan.transform_down(|node| {
        if let Some(scan) = node.downcast_ref::<PgSearchScanPlan>()
            && let Some(ls) = scan.lazy_sort()
            && ls.initial_window > 0.0
        {
            let mut updated_ls = ls.clone();
            updated_ls.initial_window = initial_window;
            return Ok(Transformed::yes(Arc::new(
                scan.clone().with_lazy_sort(updated_ls),
            )));
        }
        Ok(Transformed::no(node))
    })
    .map(|t| t.data)
}

fn collect_decode_tiebreaker_fields(
    decode: &TantivyDecodeExec,
    ordering: &LexOrdering,
    out: &mut Vec<PhysicalDeferredField>,
) {
    for sort_expr in ordering.iter().skip(1) {
        for col in collect_columns(&sort_expr.expr) {
            if let Some(field) = decode
                .deferred_fields()
                .iter()
                .find(|f| f.col_idx == col.index())
                && !out.contains(field)
            {
                out.push(field.clone());
            }
        }
    }
}

fn try_rewrite_scored_branch(
    node: &Arc<dyn ExecutionPlan>,
    required_ordering: &LexOrdering,
    min_rows: usize,
    join_mode: PartitionMode,
    field_sort_policy: FieldSortPolicy,
    level: BranchLevel,
    eager_fields: &mut Vec<PhysicalDeferredField>,
) -> Result<Option<(Arc<dyn ExecutionPlan>, SubtreeScoreStats)>> {
    if let Some(child_ordering) = map_ordering_through_passthrough(node, required_ordering)? {
        let Some((new_child, stats)) = try_rewrite_scored_branch(
            node.children()[0],
            &child_ordering,
            min_rows,
            join_mode,
            field_sort_policy,
            level,
            eager_fields,
        )?
        else {
            return Ok(None);
        };
        if let Some(decode) = node.downcast_ref::<TantivyDecodeExec>() {
            collect_decode_tiebreaker_fields(decode, required_ordering, eager_fields);
        }
        let rebuilt = replace_children_if_necessary(Arc::clone(node), vec![new_child])?;
        return Ok(Some((rebuilt, stats)));
    }

    if level == BranchLevel::ChildBranch
        && let Some(scan) = node.downcast_ref::<PgSearchScanPlan>()
    {
        return Ok(try_rewrite_scored_leaf(
            scan,
            &required_ordering[0],
            join_mode,
            field_sort_policy,
        ));
    }

    let Some(join) = node.downcast_ref::<HashJoinExec>() else {
        return Ok(None);
    };
    if !is_hash_join_eligible(join) {
        return Ok(None);
    }

    let unprojected_ordering = unproject_join_ordering(join, required_ordering)?;
    let left_schema = join.left().schema();
    let right_schema = join.right().schema();
    let Some(decomposed) =
        analyze_join_score_expr(&unprojected_ordering[0], &left_schema, &right_schema)
    else {
        return Ok(None);
    };

    let uses_left = decomposed.combiner.uses_left();
    let uses_right = decomposed.combiner.uses_right();
    let has_disjunctive_scans = any_scan(node, PgSearchScanPlan::is_tagless_disjunctive);

    // Single-sided (`NRJN`) joins disable SA on the unscored child (`initial_window = 0.0`),
    // which is sound for `ScanMode::Standard` and `ScanMode::Tagged` (where the scored child
    // scans `base_query` and the join's `filter` evaluates any cross-table tag disjunction),
    // but not for tagless `ScanMode::Disjunctive`.
    if uses_left != uses_right && has_disjunctive_scans {
        return Ok(None);
    }

    // Two-sided score joins (`uses_left && uses_right`) over conjunctive (`ScanMode::Standard`)
    // inputs suffer from multiplicative RA miss rates across independent predicates and slow
    // additive upper-bound decay compared to parallel MPP `HashJoinExec` with dynamic filter
    // pushdown. Only plan two-sided `RankJoinExec` when the subtree contains tagless disjunctive
    // (`ScanMode::Disjunctive`) scans (where the alternative `ScanMode::Tagged` must scan 100%
    // of every table), or when `min_rows == 0` (used in tests to force `RankJoinExec`).
    if uses_left && uses_right && min_rows > 0 && !has_disjunctive_scans {
        return Ok(None);
    }

    let child_field_sort_policy = if field_sort_policy == FieldSortPolicy::Allowed
        && matches!(
            decomposed.combiner,
            ScoreCombiner::LeftScore | ScoreCombiner::RightScore
        ) {
        FieldSortPolicy::Allowed
    } else {
        FieldSortPolicy::Disallowed
    };

    let Some((new_left, left_stats)) = (if uses_left {
        let Some(left_ordering) = LexOrdering::new(vec![decomposed.left_sort_expr.clone()]) else {
            return Ok(None);
        };
        try_rewrite_scored_branch(
            join.left(),
            &left_ordering,
            min_rows,
            join_mode,
            child_field_sort_policy,
            BranchLevel::ChildBranch,
            eager_fields,
        )?
    } else {
        let left_keys: Vec<Arc<dyn PhysicalExpr>> =
            join.on().iter().map(|(l, _)| Arc::clone(l)).collect();
        try_rewrite_unscored_branch(join.left(), &left_keys, join_mode)?
    }) else {
        return Ok(None);
    };

    let Some((new_right, right_stats)) = (if uses_right {
        let Some(right_ordering) = LexOrdering::new(vec![decomposed.right_sort_expr.clone()])
        else {
            return Ok(None);
        };
        try_rewrite_scored_branch(
            join.right(),
            &right_ordering,
            min_rows,
            join_mode,
            child_field_sort_policy,
            BranchLevel::ChildBranch,
            eager_fields,
        )?
    } else {
        let right_keys: Vec<Arc<dyn PhysicalExpr>> =
            join.on().iter().map(|(_, r)| Arc::clone(r)).collect();
        try_rewrite_unscored_branch(join.right(), &right_keys, join_mode)?
    }) else {
        return Ok(None);
    };

    let Some(combined_stats) =
        SubtreeScoreStats::combine(&decomposed.combiner, left_stats, right_stats, min_rows)
    else {
        return Ok(None);
    };

    let rank_join = RankJoinExec::try_new(
        RankJoinInputs {
            left: new_left,
            right: new_right,
        },
        join.on().to_vec(),
        join.filter().cloned(),
        join.projection.as_deref().map(<[usize]>::to_vec),
        join_mode,
        join.null_equality(),
        RankJoinSortExprs {
            left: decomposed.left_sort_expr,
            right: decomposed.right_sort_expr,
            output: required_ordering.clone(),
        },
        decomposed.combiner,
    )?;

    Ok(Some((Arc::new(rank_join), combined_stats)))
}

fn any_scan(
    node: &Arc<dyn ExecutionPlan>,
    mut pred: impl FnMut(&PgSearchScanPlan) -> bool,
) -> bool {
    node.exists(|n| Ok(n.downcast_ref::<PgSearchScanPlan>().is_some_and(&mut pred)))
        .unwrap_or(false)
}

fn is_passthrough_node(node: &Arc<dyn ExecutionPlan>) -> bool {
    node.is::<ProjectionExec>()
        || node.is::<VisibilityFilterExec>()
        || node.is::<CoalescePartitionsExec>()
        || node.is::<TantivyDecodeExec>()
        || node.is::<TantivyFetchExec>()
}

/// Inspects and rewrites an unscored child branch of a single-sided (`NRJN`) join: either a
/// leaf `PgSearchScanPlan` in `ScanMode::Standard` or `ScanMode::Tagged`, or a nested
/// `HashJoinExec` whose parent join keys map cleanly to one child side for cascading `RA` lookups.
fn try_rewrite_unscored_branch(
    node: &Arc<dyn ExecutionPlan>,
    parent_keys: &[Arc<dyn PhysicalExpr>],
    join_mode: PartitionMode,
) -> Result<Option<(Arc<dyn ExecutionPlan>, SubtreeScoreStats)>> {
    if let Some(proj) = node.downcast_ref::<ProjectionExec>() {
        let child_keys: Option<Vec<_>> = parent_keys
            .iter()
            .map(|key| {
                let col = key.downcast_ref::<Column>()?;
                let expr = &proj.expr().get(col.index())?.expr;
                expr.is::<Column>().then(|| Arc::clone(expr))
            })
            .collect();
        let Some(child_keys) = child_keys else {
            return Ok(None);
        };
        let Some((new_child, stats)) =
            try_rewrite_unscored_branch(proj.input(), &child_keys, join_mode)?
        else {
            return Ok(None);
        };
        let rebuilt = replace_children_if_necessary(Arc::clone(node), vec![new_child])?;
        return Ok(Some((rebuilt, stats)));
    }
    if is_passthrough_node(node) {
        let Some((new_child, stats)) =
            try_rewrite_unscored_branch(node.children()[0], parent_keys, join_mode)?
        else {
            return Ok(None);
        };
        let rebuilt = replace_children_if_necessary(Arc::clone(node), vec![new_child])?;
        return Ok(Some((rebuilt, stats)));
    }
    if let Some(join) = node.downcast_ref::<HashJoinExec>() {
        if !is_hash_join_eligible(join) {
            return Ok(None);
        }
        let left_len = join.left().schema().fields().len();
        let mut target_side: Option<ExprSide> = None;
        let mut shifted_parent_keys = Vec::with_capacity(parent_keys.len());
        for key in parent_keys {
            if !key.is::<Column>() {
                return Ok(None);
            }
            let Some(unproj_key) = unproject_join_expr(join, key)? else {
                return Ok(None);
            };
            let side = classify_expr_side(&unproj_key, left_len);
            if !matches!(side, ExprSide::LeftOnly | ExprSide::RightOnly) {
                return Ok(None);
            }
            if target_side.replace(side).is_some_and(|prev| prev != side) {
                return Ok(None);
            }
            let child_expr = if side == ExprSide::LeftOnly {
                unproj_key
            } else {
                shift_right_expr(&unproj_key, left_len)?
            };
            shifted_parent_keys.push(child_expr);
        }
        if target_side.is_none() {
            return Ok(None);
        }

        let mut left_keys: Vec<Arc<dyn PhysicalExpr>> =
            join.on().iter().map(|(l, _)| Arc::clone(l)).collect();
        let mut right_keys: Vec<Arc<dyn PhysicalExpr>> =
            join.on().iter().map(|(_, r)| Arc::clone(r)).collect();
        if target_side == Some(ExprSide::LeftOnly) {
            left_keys.extend(shifted_parent_keys);
        } else {
            right_keys.extend(shifted_parent_keys);
        }

        let Some((new_left, left_stats)) =
            try_rewrite_unscored_branch(join.left(), &left_keys, join_mode)?
        else {
            return Ok(None);
        };
        let Some((new_right, right_stats)) =
            try_rewrite_unscored_branch(join.right(), &right_keys, join_mode)?
        else {
            return Ok(None);
        };
        let stats = SubtreeScoreStats {
            estimated_rows: left_stats.estimated_rows.min(right_stats.estimated_rows),
            unscored_selectivity: left_stats
                .unscored_selectivity
                .and(right_stats.unscored_selectivity),
            tagged_selectivity: SubtreeScoreStats::combine_tagged(
                left_stats.tagged_selectivity,
                right_stats.tagged_selectivity,
            ),
            score_min: 0.0,
            score_max: 0.0,
        };
        let dummy_zero = zero_sort_expr(SortOptions {
            descending: true,
            nulls_first: false,
        });
        let rank_join = RankJoinExec::try_new(
            RankJoinInputs {
                left: new_left,
                right: new_right,
            },
            join.on().to_vec(),
            join.filter().cloned(),
            join.projection.as_deref().map(<[usize]>::to_vec),
            join_mode,
            join.null_equality(),
            RankJoinSortExprs {
                left: dummy_zero.clone(),
                right: dummy_zero.clone(),
                output: LexOrdering::new(vec![dummy_zero]).unwrap(),
            },
            ScoreCombiner::Constant(0.0),
        )?;
        return Ok(Some((Arc::new(rank_join), stats)));
    }
    let Some(scan) = node.downcast_ref::<PgSearchScanPlan>() else {
        return Ok(None);
    };
    let (unscored_selectivity, tagged_selectivity) = match scan.scan_mode() {
        ScanMode::Standard { .. } => (scan.estimated_selectivity(), None),
        ScanMode::Tagged { base_query, .. } => {
            let base_sel = if matches!(base_query.as_ref(), SearchQueryInput::All) {
                TopKSelectivity::ALL
            } else {
                scan.estimated_selectivity()
            };
            (base_sel, scan.estimated_tagged_selectivity())
        }
        _ => return Ok(None),
    };
    let stats = SubtreeScoreStats {
        estimated_rows: scan.planner_estimated_rows(),
        unscored_selectivity,
        tagged_selectivity,
        score_min: 0.0,
        score_max: 0.0,
    };
    let configured_scan = configure_leaf_lazy_sort(
        scan,
        LazySortConfig {
            orderby_info: None,
            ordering: None,
            initial_window: 0.0,
            shared_threshold: SharedThresholdMode::Disabled,
            telemetry: None,
        },
        join_mode,
    );
    Ok(Some((Arc::new(configured_scan), stats)))
}

fn try_rewrite_scored_leaf(
    scan: &PgSearchScanPlan,
    required_sort: &PhysicalSortExpr,
    join_mode: PartitionMode,
    field_sort_policy: FieldSortPolicy,
) -> Option<(Arc<dyn ExecutionPlan>, SubtreeScoreStats)> {
    let order_by = resolve_scan_order_by(scan, required_sort, field_sort_policy)?;
    let stats = match scan.scan_mode() {
        ScanMode::Standard { .. } => SubtreeScoreStats::from_leaf_queries(
            order_by.is_score(),
            scan.resolved_query(),
            &[],
            scan.planner_estimated_rows(),
        ),
        ScanMode::Tagged { base_query, .. } if !order_by.is_score() => {
            SubtreeScoreStats::from_leaf_queries(
                false,
                base_query,
                &[],
                scan.planner_estimated_rows(),
            )
            .map(|mut s| {
                s.tagged_selectivity = scan.estimated_tagged_selectivity();
                s
            })
        }
        ScanMode::Disjunctive {
            base_query,
            disjunct_queries,
        } => SubtreeScoreStats::from_leaf_queries(
            order_by.is_score(),
            base_query,
            disjunct_queries,
            scan.planner_estimated_rows(),
        ),
        ScanMode::Tagged { .. } => None,
    }?;
    // Placeholder `initial_window: 1.0` marks this leaf as `SA`; `try_rewrite_under_sort`
    // replaces it with the final selectivity-scaled window once the full join tree's
    // `SubtreeScoreStats` has been computed.
    let configured_scan = configure_leaf_lazy_sort(
        scan,
        LazySortConfig {
            orderby_info: Some(vec![order_by]),
            ordering: LexOrdering::new(vec![required_sort.clone()]),
            initial_window: 1.0,
            shared_threshold: SharedThresholdMode::Disabled,
            telemetry: None,
        },
        join_mode,
    );
    Some((Arc::new(configured_scan), stats))
}

fn configure_leaf_lazy_sort(
    scan: &PgSearchScanPlan,
    lazy_sort: LazySortConfig,
    join_mode: PartitionMode,
) -> PgSearchScanPlan {
    let mut cloned = scan.clone();
    if join_mode != PartitionMode::Partitioned {
        cloned = cloned.into_single_shared_partition();
    }
    cloned.with_lazy_sort(lazy_sort)
}

fn resolve_scan_order_by(
    scan: &PgSearchScanPlan,
    required_sort: &PhysicalSortExpr,
    field_sort_policy: FieldSortPolicy,
) -> Option<OrderByInfo> {
    let col = required_sort.expr.downcast_ref::<Column>()?;
    let schema = scan.schema();
    let field_name = schema.fields().get(col.index())?.name();
    let feature = if ScoreColumn::is_score_column(field_name) {
        OrderByFeature::Score { rti: 0 }
    } else if field_sort_policy == FieldSortPolicy::Allowed
        && scan.is_sortable_fast_field(col.index())
    {
        OrderByFeature::Field {
            name: FieldName::from(field_name.clone()),
            rti: 0,
        }
    } else {
        return None;
    };
    Some(OrderByInfo {
        feature,
        direction: SortDirection::from(required_sort.options),
    })
}

fn is_hash_join_eligible(join: &HashJoinExec) -> bool {
    *join.join_type() == JoinType::Inner && !join.null_aware && !join.on().is_empty()
}

/// Maps `ordering` across a pass-through single-child operator (`ProjectionExec`,
/// `VisibilityFilterExec`, `TantivyDecodeExec`, `TantivyFetchExec`, `CoalescePartitionsExec`).
fn map_ordering_through_passthrough(
    node: &Arc<dyn ExecutionPlan>,
    ordering: &LexOrdering,
) -> Result<Option<LexOrdering>> {
    if let Some(proj) = node.downcast_ref::<ProjectionExec>() {
        let proj_exprs = proj.expr();
        return map_ordering_columns(ordering, |col| {
            proj_exprs.get(col.index()).map(|pe| Arc::clone(&pe.expr))
        });
    }

    if let Some(vis) = node.downcast_ref::<VisibilityFilterExec>() {
        let Some(proj) = vis.projection() else {
            return Ok(Some(ordering.clone()));
        };
        let in_schema = vis.children()[0].schema();
        return map_ordering_columns(ordering, |col| {
            let &in_idx = proj.get(col.index())?;
            let field = in_schema.fields().get(in_idx)?;
            Some(Arc::new(Column::new(field.name(), in_idx)))
        });
    }

    if node.is::<CoalescePartitionsExec>() {
        return Ok(Some(ordering.clone()));
    }

    if node.is::<TantivyDecodeExec>() || node.is::<TantivyFetchExec>() {
        let out_schema = node.schema();
        let in_schema = node.children()[0].schema();
        let valid = ordering.iter().enumerate().all(|(idx, sort_expr)| {
            collect_columns(&sort_expr.expr).iter().all(|col| {
                let in_bounds = col.index() < out_schema.fields().len()
                    && col.index() < in_schema.fields().len();
                // Primary score expression (`idx == 0`) cannot reference a deferred
                // column whose DataType changes across `TantivyDecodeExec`. Secondary
                // tiebreakers (`idx > 0`) may reference deferred columns, which will
                // be converted to eager delivery if the join rewrite succeeds.
                in_bounds
                    && (idx > 0
                        || out_schema.field(col.index()).data_type()
                            == in_schema.field(col.index()).data_type())
            })
        });
        return Ok(valid.then(|| ordering.clone()));
    }

    Ok(None)
}

/// Maps `expr` from `join.schema()` (post-projection) to `join.join_schema()` (unprojected).
fn unproject_join_expr(
    join: &HashJoinExec,
    expr: &Arc<dyn PhysicalExpr>,
) -> Result<Option<Arc<dyn PhysicalExpr>>> {
    let Some(proj) = &join.projection else {
        return Ok(Some(Arc::clone(expr)));
    };
    let join_schema = join.join_schema();
    map_expr_columns(expr, |col| {
        let &unproj_idx = proj.get(col.index())?;
        let field = join_schema.fields().get(unproj_idx)?;
        Some(Arc::new(Column::new(field.name(), unproj_idx)))
    })
}

/// Maps `ordering` from `join.schema()` (post-projection) to `join.join_schema()` (unprojected).
fn unproject_join_ordering(join: &HashJoinExec, ordering: &LexOrdering) -> Result<LexOrdering> {
    let mut mapped = Vec::with_capacity(ordering.len());
    for sort_expr in ordering.iter() {
        let expr = unproject_join_expr(join, &sort_expr.expr)?.expect("valid join projection");
        mapped.push(PhysicalSortExpr {
            expr,
            options: sort_expr.options,
        });
    }
    Ok(LexOrdering::new(mapped).expect("non-empty ordering"))
}

/// Side reference classification for a `PhysicalExpr` over `join_schema`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExprSide {
    Constant,
    LeftOnly,
    RightOnly,
    Both,
}

impl ExprSide {
    fn merge(self, other: Self) -> Self {
        match (self, other) {
            (ExprSide::Constant, x) | (x, ExprSide::Constant) => x,
            (ExprSide::LeftOnly, ExprSide::LeftOnly) => ExprSide::LeftOnly,
            (ExprSide::RightOnly, ExprSide::RightOnly) => ExprSide::RightOnly,
            _ => ExprSide::Both,
        }
    }
}

fn classify_expr_side(expr: &Arc<dyn PhysicalExpr>, left_len: usize) -> ExprSide {
    collect_columns(expr)
        .into_iter()
        .fold(ExprSide::Constant, |side, col| {
            let col_side = if col.index() < left_len {
                ExprSide::LeftOnly
            } else {
                ExprSide::RightOnly
            };
            side.merge(col_side)
        })
}

/// Evaluates a constant `PhysicalExpr` (referencing no columns) into an `f64`.
fn eval_constant_expr(expr: &Arc<dyn PhysicalExpr>) -> Option<f64> {
    if let Some(lit) = expr.downcast_ref::<Literal>() {
        return scalar_to_f64(lit.value());
    }
    let dummy_schema = Arc::new(Schema::empty());
    let dummy_batch = RecordBatch::try_new_with_options(
        dummy_schema,
        vec![],
        &RecordBatchOptions::new().with_row_count(Some(1)),
    )
    .ok()?;
    let val = expr.evaluate(&dummy_batch).ok()?;
    match val {
        ColumnarValue::Scalar(s) => scalar_to_f64(&s),
        ColumnarValue::Array(arr) => {
            let s = ScalarValue::try_from_array(&arr, 0).ok()?;
            scalar_to_f64(&s)
        }
    }
}

fn scalar_to_f64(scalar: &ScalarValue) -> Option<f64> {
    let ScalarValue::Float64(Some(v)) = scalar.cast_to(&DataType::Float64).ok()? else {
        return None;
    };
    v.is_finite().then_some(v)
}

fn unwrap_float_cast(expr: &Arc<dyn PhysicalExpr>) -> Option<(&Arc<dyn PhysicalExpr>, &DataType)> {
    if let Some(cast_expr) = expr.downcast_ref::<CastExpr>() {
        return Some((cast_expr.expr(), cast_expr.cast_type()));
    }
    if let Some(try_cast) = expr.downcast_ref::<TryCastExpr>() {
        return Some((try_cast.expr(), try_cast.cast_type()));
    }
    None
}

/// Checks whether `expr` (over `join_schema`) is guaranteed to evaluate to a non-negative
/// number (`>= 0`), as required for monotonic multiplication `a * b`.
fn is_expr_non_negative(expr: &Arc<dyn PhysicalExpr>) -> bool {
    if let Some(col) = expr.downcast_ref::<Column>() {
        // BM25 scores (`pdb.score(...)`) are always >= 0.
        return ScoreColumn::is_score_column(col.name());
    }
    if let Some((inner, _)) = unwrap_float_cast(expr) {
        return is_expr_non_negative(inner);
    }
    if let Some(c) = eval_constant_expr(expr) {
        return c >= 0.0;
    }
    if let Some(bin) = expr.downcast_ref::<BinaryExpr>() {
        return match bin.op() {
            Operator::Plus | Operator::Multiply => {
                is_expr_non_negative(bin.left()) && is_expr_non_negative(bin.right())
            }
            _ => false,
        };
    }
    false
}

/// Shifts column indices in a `RightOnly` expression down by `left_len` so the resulting
/// `PhysicalExpr` is valid against `right_schema`.
fn shift_right_expr(
    expr: &Arc<dyn PhysicalExpr>,
    left_len: usize,
) -> Result<Arc<dyn PhysicalExpr>> {
    Ok(map_expr_columns(expr, |col| {
        Some(Arc::new(Column::new(col.name(), col.index() - left_len)))
    })?
    .expect("infallible shift"))
}

fn zero_sort_expr(options: SortOptions) -> PhysicalSortExpr {
    PhysicalSortExpr {
        expr: Arc::new(Literal::new(ScalarValue::Float64(Some(0.0)))),
        options,
    }
}

/// Analyzes `sort_expr` over `join_schema` (`[left_fields..., right_fields...]`) and
/// decomposes it into `(left_sort_expr, right_sort_expr, ScoreCombiner)` if it is a
/// valid monotonic combination of both sides.
pub(crate) fn analyze_join_score_expr(
    sort_expr: &PhysicalSortExpr,
    left_schema: &SchemaRef,
    _right_schema: &SchemaRef,
) -> Option<DecomposedJoinScore> {
    let left_len = left_schema.fields().len();
    let root_options = sort_expr.options;

    struct BuilderState {
        left_len: usize,
        root_options: SortOptions,
        left_sort: Option<PhysicalSortExpr>,
        right_sort: Option<PhysicalSortExpr>,
    }

    impl BuilderState {
        fn record_leaf(
            &mut self,
            side: ExprSide,
            expr: &Arc<dyn PhysicalExpr>,
            dir: MonotonicDirection,
        ) -> Option<ScoreCombiner> {
            let options = dir.to_sort_options(self.root_options);
            let (slot, candidate_expr, combiner) = match side {
                ExprSide::LeftOnly => (
                    &mut self.left_sort,
                    Arc::clone(expr),
                    ScoreCombiner::LeftScore,
                ),
                ExprSide::RightOnly => (
                    &mut self.right_sort,
                    shift_right_expr(expr, self.left_len).ok()?,
                    ScoreCombiner::RightScore,
                ),
                _ => return None,
            };
            let candidate = PhysicalSortExpr {
                expr: candidate_expr,
                options,
            };
            if slot.as_ref().is_some_and(|existing| existing != &candidate) {
                return None;
            }
            *slot = Some(candidate);
            Some(combiner)
        }

        fn analyze(
            &mut self,
            expr: &Arc<dyn PhysicalExpr>,
            dir: MonotonicDirection,
        ) -> Option<ScoreCombiner> {
            let side = classify_expr_side(expr, self.left_len);
            if side == ExprSide::Constant {
                return eval_constant_expr(expr).map(ScoreCombiner::Constant);
            }

            // Unwrap numeric casts (`CAST(score AS FLOAT8)` / `FLOAT4`) so leaf columns
            // match `PgSearchScanPlan`'s native `pdb.score(...)` column without a cast wrapper.
            if let Some((inner, cast_type)) = unwrap_float_cast(expr) {
                if matches!(cast_type, DataType::Float32 | DataType::Float64) {
                    return self.analyze(inner, dir);
                }
                return None;
            }

            // A bare `Column` on one side is always a leaf score input.
            if expr.is::<Column>() {
                return self.record_leaf(side, expr, dir);
            }

            // If `expr` is a binary expression where BOTH operands belong to the same side
            // (e.g., `score_a + score_b` coming from a child 2-way join inside a 3-way join),
            // treat the whole subexpression as that child join's composite score leaf!
            if let Some(bin) = expr.downcast_ref::<BinaryExpr>() {
                let l_side = classify_expr_side(bin.left(), self.left_len);
                let r_side = classify_expr_side(bin.right(), self.left_len);
                if l_side == r_side && matches!(l_side, ExprSide::LeftOnly | ExprSide::RightOnly) {
                    return self.record_leaf(l_side, expr, dir);
                }

                return match bin.op() {
                    Operator::Plus => {
                        let l = self.analyze(bin.left(), dir)?;
                        let r = self.analyze(bin.right(), dir)?;
                        Some(ScoreCombiner::Add(Box::new(l), Box::new(r)))
                    }
                    Operator::Multiply => {
                        if let Some(c) = eval_constant_expr(bin.left()) {
                            if c == 0.0 {
                                return None;
                            }
                            let child_dir = if c > 0.0 { dir } else { dir.flip() };
                            let r = self.analyze(bin.right(), child_dir)?;
                            return Some(ScoreCombiner::Mul(
                                Box::new(ScoreCombiner::Constant(c)),
                                Box::new(r),
                            ));
                        }
                        if let Some(c) = eval_constant_expr(bin.right()) {
                            if c == 0.0 {
                                return None;
                            }
                            let child_dir = if c > 0.0 { dir } else { dir.flip() };
                            let l = self.analyze(bin.left(), child_dir)?;
                            return Some(ScoreCombiner::Mul(
                                Box::new(l),
                                Box::new(ScoreCombiner::Constant(c)),
                            ));
                        }
                        // Non-constant multiplication `a * b` is monotonic when both operands are >= 0.
                        if is_expr_non_negative(bin.left()) && is_expr_non_negative(bin.right()) {
                            let l = self.analyze(bin.left(), dir)?;
                            let r = self.analyze(bin.right(), dir)?;
                            return Some(ScoreCombiner::Mul(Box::new(l), Box::new(r)));
                        }
                        None
                    }
                    _ => None,
                };
            }

            None
        }
    }

    let mut state = BuilderState {
        left_len,
        root_options,
        left_sort: None,
        right_sort: None,
    };
    let combiner = state.analyze(&sort_expr.expr, MonotonicDirection::Increasing)?;
    if !combiner.uses_left() && !combiner.uses_right() {
        return None;
    }
    let left_sort_expr = state
        .left_sort
        .unwrap_or_else(|| zero_sort_expr(root_options));
    let right_sort_expr = state
        .right_sort
        .unwrap_or_else(|| zero_sort_expr(root_options));

    Some(DecomposedJoinScore {
        left_sort_expr,
        right_sort_expr,
        combiner,
    })
}
