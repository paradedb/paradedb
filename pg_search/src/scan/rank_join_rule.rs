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
//! Supports two-sided conjunctive (`ScanMode::Standard`), disjunctive (`ScanMode::Tagged`),
//! and single-sided (`NRJN`) top-K joins. See `iterative-joins/TOPK_JOIN_OPTIMIZATION_DESIGN.md`
//! for the three plan-time gates (Gate 1: Tantivy Top-K capability, Gate 2: score contribution
//! balance, Gate 3: minimum input cardinality `rank_join_min_rows`) and the monotonicity
//! verification rules.

use std::sync::Arc;

use arrow_array::{RecordBatch, RecordBatchOptions};
use arrow_schema::{DataType, Schema, SchemaRef, SortOptions};
use datafusion::common::config::ConfigOptions;
use datafusion::common::tree_node::{Transformed, TreeNode, TreeNodeRecursion};
use datafusion::common::{Result, ScalarValue};
use datafusion::logical_expr::{ColumnarValue, JoinType, Operator};
use datafusion::physical_expr::expressions::{BinaryExpr, CastExpr, Column, Literal, TryCastExpr};
use datafusion::physical_expr::{LexOrdering, PhysicalExpr, PhysicalSortExpr, ScalarFunctionExpr};
use datafusion::physical_optimizer::PhysicalOptimizerRule;
use datafusion::physical_plan::coalesce_partitions::CoalescePartitionsExec;
use datafusion::physical_plan::joins::HashJoinExec;
use datafusion::physical_plan::projection::ProjectionExec;
use datafusion::physical_plan::sorts::sort::SortExec;
use datafusion::physical_plan::{
    ExecutionPlan, ExecutionPlanProperties, replace_children_if_necessary,
};

use crate::api::{FieldName, OrderByFeature, OrderByInfo, SortDirection};
use crate::postgres::customscan::joinscan::visibility_filter::VisibilityFilterExec;
use crate::query::SearchQueryInput;
use crate::query::pdb_query::pdb::{self, ScoreAdjustStyle};
use crate::scan::ScanMode;
use crate::scan::execution_plan::{LazySortConfig, PgSearchScanPlan, SharedThresholdMode};
use crate::scan::rank_join_exec::{
    InputScorePair, RankJoinExec, RankJoinInputs, RankJoinSortExprs, ScoreCombiner,
};
use crate::scan::tantivy_decode_exec::TantivyDecodeExec;
use crate::scan::tantivy_fetch_exec::TantivyFetchExec;

/// Maximum `LIMIT + OFFSET` ($K$) eligible for `RankJoinExec`.
pub(crate) const MAX_RANK_JOIN_K: usize = 1_000;

/// Minimum ratio $\min(\Delta_L^{\max}, \Delta_R^{\max}) / \max(\Delta_L^{\max}, \Delta_R^{\max})$
/// required by Gate 2 so heavily lopsided two-sided score queries do not force `RankJoinExec`
/// to scan the weaker-scored input to completion.
pub(crate) const SCORE_BALANCE_THRESHOLD: f64 = 0.20;

/// Physical optimizer rule that replaces top-K scored `HashJoinExec`s with `RankJoinExec`.
#[derive(Debug, Default)]
pub struct RankJoinRule {
    /// Optional override for `paradedb.rank_join_min_rows` in unit tests.
    min_rows_override: Option<usize>,
}

impl RankJoinRule {
    pub fn new() -> Self {
        Self {
            min_rows_override: None,
        }
    }

    #[cfg(any(test, feature = "pg_test"))]
    pub fn new_with_min_rows(min_rows: usize) -> Self {
        Self {
            min_rows_override: Some(min_rows),
        }
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
        // RankJoin relies on dynamic filters (`FilterPushdown`) to prune the random-access
        // probe side on the fly. Because cross-process dynamic filters are disabled under MPP
        // (`with_distributed_remote_dynamic_filters(false)`), do not plan `RankJoinExec` when
        // MPP is active.
        if self.min_rows_override.is_none()
            && (!crate::gucs::enable_rank_join()
                || crate::postgres::customscan::mpp::glue::mpp_is_active())
        {
            return Ok(plan);
        }
        let min_rows = self
            .min_rows_override
            .unwrap_or_else(crate::gucs::rank_join_min_rows);

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
            !has_unconfigured_tagless_disjunctive_scan(&rewritten),
            "JoinSearchPredicateRule elided predicate tagging for a scan that RankJoinRule did not rewrite into lazy_sort"
        );

        Ok(rewritten)
    }
}

/// Statistics and score range for a join input subtree used by Gates 2 and 3.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SubtreeScoreStats {
    pub estimated_rows: u64,
    pub score_min: f64,
    pub score_max: f64,
}

impl SubtreeScoreStats {
    /// Verifies Gate 1 (non-constant BM25 score when `is_score_sort` is true) and computes
    /// `[0.0, score_max]` for a leaf scan.
    pub(crate) fn from_leaf_queries(
        is_score_sort: bool,
        base_query: &SearchQueryInput,
        disjunct_queries: &[SearchQueryInput],
        estimated_rows: u64,
    ) -> Option<Self> {
        let base_has_score = !is_constant_score_query(base_query);
        let disjuncts_have_score = disjunct_queries.iter().any(|q| !is_constant_score_query(q));
        if is_score_sort && !base_has_score && !disjuncts_have_score {
            return None;
        }
        let score_max = if is_score_sort {
            let mut total = if base_has_score {
                estimate_query_max_score(base_query)
            } else {
                0.0
            };
            for q in disjunct_queries {
                if !is_constant_score_query(q) {
                    total += estimate_query_max_score(q);
                }
            }
            total.max(1.0)
        } else {
            1.0
        };
        Some(Self {
            estimated_rows,
            score_min: 0.0,
            score_max,
        })
    }

    /// Verifies Gate 2 (score balance when both sides are scored) and Gate 3 (`min_rows`),
    /// and computes the combined `[score_min, score_max]` across `combiner`.
    pub(crate) fn combine(
        combiner: &ScoreCombiner,
        left: Self,
        right: Self,
        min_rows: usize,
        skip_min_rows_gate: bool,
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

        // Gate 3: Both inputs must meet the minimum row threshold (`rank_join_min_rows`).
        let min_matched = left.estimated_rows.min(right.estimated_rows);
        if min_rows > 0 && min_matched < min_rows as u64 && !skip_min_rows_gate {
            return None;
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

    fn to_sort_options(self, root_descending: bool) -> SortOptions {
        let descending = match self {
            MonotonicDirection::Increasing => root_descending,
            MonotonicDirection::Decreasing => !root_descending,
        };
        SortOptions {
            descending,
            nulls_first: false,
        }
    }
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
fn try_rewrite_under_sort(
    node: &Arc<dyn ExecutionPlan>,
    required_ordering: &LexOrdering,
    k: usize,
    min_rows: usize,
) -> Result<Option<Arc<dyn ExecutionPlan>>> {
    let initial_window = compute_initial_window(k);
    Ok(
        try_rewrite_join_subtree(node, required_ordering, min_rows, initial_window)?
            .map(|(plan, _)| plan),
    )
}

fn compute_initial_window(k: usize) -> usize {
    let scale_factor = if cfg!(test) {
        2
    } else {
        crate::gucs::topk_retry_scale_factor().max(2) as usize
    };
    let max_window = if cfg!(test) {
        100_000
    } else {
        crate::gucs::max_topk_chunk_size().max(1) as usize
    };
    k.saturating_mul(scale_factor).max(1).min(max_window)
}

fn try_rewrite_join_subtree(
    node: &Arc<dyn ExecutionPlan>,
    required_ordering: &LexOrdering,
    min_rows: usize,
    initial_window: usize,
) -> Result<Option<(Arc<dyn ExecutionPlan>, SubtreeScoreStats)>> {
    if let Some(child_ordering) = map_ordering_through_passthrough(node, required_ordering)? {
        let Some((new_child, stats)) = try_rewrite_join_subtree(
            node.children()[0],
            &child_ordering,
            min_rows,
            initial_window,
        )?
        else {
            return Ok(None);
        };
        let rebuilt = replace_children_if_necessary(Arc::clone(node), vec![new_child])?;
        return Ok(Some((rebuilt, stats)));
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

    // Single-sided (`NRJN`) joins disable SA on the unscored child (`initial_window = 0.0`),
    // which is only sound when all leaf scans in the subtree use `ScanMode::Standard`
    // (conjunctive join where every valid row matches the scored child's SA query).
    if uses_left != uses_right && !subtree_all_standard_scans(node) {
        return Ok(None);
    }

    let Some((new_left, left_stats)) = (if uses_left {
        try_rewrite_child_branch(
            join.left(),
            &decomposed.left_sort_expr,
            min_rows,
            initial_window,
        )?
    } else {
        try_rewrite_unscored_branch(join.left())?
    }) else {
        return Ok(None);
    };

    let Some((new_right, right_stats)) = (if uses_right {
        try_rewrite_child_branch(
            join.right(),
            &decomposed.right_sort_expr,
            min_rows,
            initial_window,
        )?
    } else {
        try_rewrite_unscored_branch(join.right())?
    }) else {
        return Ok(None);
    };

    // If `JoinSearchPredicateRule` already elided predicate tagging on this subtree
    // (based on logical `scan_info.estimate`), do not reject if segment pruning at reader
    // open time lowered `planner_estimated_rows`.
    let skip_min_rows_gate = subtree_has_tagless_disjunctive_scans(node);
    let Some(combined_stats) = SubtreeScoreStats::combine(
        &decomposed.combiner,
        left_stats,
        right_stats,
        min_rows,
        skip_min_rows_gate,
    ) else {
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
        *join.partition_mode(),
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

fn subtree_all_standard_scans(node: &Arc<dyn ExecutionPlan>) -> bool {
    if let Some(scan) = node.downcast_ref::<PgSearchScanPlan>() {
        return matches!(scan.scan_mode(), ScanMode::Standard { .. });
    }
    node.children().into_iter().all(subtree_all_standard_scans)
}

fn subtree_has_tagless_disjunctive_scans(node: &Arc<dyn ExecutionPlan>) -> bool {
    if let Some(scan) = node.downcast_ref::<PgSearchScanPlan>() {
        return scan.is_tagless_disjunctive();
    }
    node.children()
        .into_iter()
        .any(subtree_has_tagless_disjunctive_scans)
}

fn has_unconfigured_tagless_disjunctive_scan(node: &Arc<dyn ExecutionPlan>) -> bool {
    if let Some(scan) = node.downcast_ref::<PgSearchScanPlan>() {
        return scan.is_tagless_disjunctive() && scan.lazy_sort().is_none();
    }
    node.children()
        .into_iter()
        .any(has_unconfigured_tagless_disjunctive_scan)
}

fn is_passthrough_node(node: &Arc<dyn ExecutionPlan>) -> bool {
    node.is::<ProjectionExec>()
        || node.is::<VisibilityFilterExec>()
        || node.is::<CoalescePartitionsExec>()
        || node.is::<TantivyDecodeExec>()
        || node.is::<TantivyFetchExec>()
}

/// Inspects and rewrites an unscored child branch of a single-sided (`NRJN`) join: must be a
/// leaf `PgSearchScanPlan` (behind optional pass-through nodes) in `ScanMode::Standard`.
fn try_rewrite_unscored_branch(
    node: &Arc<dyn ExecutionPlan>,
) -> Result<Option<(Arc<dyn ExecutionPlan>, SubtreeScoreStats)>> {
    if is_passthrough_node(node) {
        let Some((new_child, stats)) = try_rewrite_unscored_branch(node.children()[0])? else {
            return Ok(None);
        };
        let rebuilt = replace_children_if_necessary(Arc::clone(node), vec![new_child])?;
        return Ok(Some((rebuilt, stats)));
    }
    let Some(scan) = node.downcast_ref::<PgSearchScanPlan>() else {
        return Ok(None);
    };
    if !matches!(scan.scan_mode(), ScanMode::Standard { .. }) {
        return Ok(None);
    }
    let stats = SubtreeScoreStats {
        estimated_rows: scan.planner_estimated_rows(),
        score_min: 0.0,
        score_max: 0.0,
    };
    let configured_scan = scan.clone().with_lazy_sort(LazySortConfig {
        orderby_info: None,
        ordering: None,
        initial_window: 0.0,
        shared_threshold: SharedThresholdMode::Disabled,
    });
    Ok(Some((Arc::new(configured_scan), stats)))
}

/// Inspects and rewrites one scored child branch of a `HashJoinExec`: either a leaf
/// `PgSearchScanPlan` (behind optional pass-through nodes) or a nested `HashJoinExec`.
fn try_rewrite_child_branch(
    node: &Arc<dyn ExecutionPlan>,
    required_sort: &PhysicalSortExpr,
    min_rows: usize,
    initial_window: usize,
) -> Result<Option<(Arc<dyn ExecutionPlan>, SubtreeScoreStats)>> {
    let Some(ordering) = LexOrdering::new(vec![required_sort.clone()]) else {
        return Ok(None);
    };
    if let Some(child_ordering) = map_ordering_through_passthrough(node, &ordering)? {
        let Some((new_child, stats)) = try_rewrite_child_branch(
            node.children()[0],
            &child_ordering[0],
            min_rows,
            initial_window,
        )?
        else {
            return Ok(None);
        };
        let rebuilt = replace_children_if_necessary(Arc::clone(node), vec![new_child])?;
        return Ok(Some((rebuilt, stats)));
    }
    if node.is::<HashJoinExec>() {
        return try_rewrite_join_subtree(node, &ordering, min_rows, initial_window);
    }
    let Some(scan) = node.downcast_ref::<PgSearchScanPlan>() else {
        return Ok(None);
    };
    let Some(order_by) = resolve_scan_order_by(scan, required_sort) else {
        return Ok(None);
    };
    let stats = match scan.scan_mode() {
        ScanMode::Standard { .. } => SubtreeScoreStats::from_leaf_queries(
            order_by.is_score(),
            scan.resolved_query(),
            &[],
            scan.planner_estimated_rows(),
        ),
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
    };
    let Some(stats) = stats else {
        return Ok(None);
    };
    let configured_scan = scan.clone().with_lazy_sort(LazySortConfig {
        orderby_info: Some(vec![order_by]),
        ordering: LexOrdering::new(vec![required_sort.clone()]),
        initial_window: initial_window as f64,
        shared_threshold: SharedThresholdMode::Disabled,
    });
    Ok(Some((Arc::new(configured_scan), stats)))
}

fn resolve_scan_order_by(
    scan: &PgSearchScanPlan,
    required_sort: &PhysicalSortExpr,
) -> Option<OrderByInfo> {
    let col = required_sort.expr.downcast_ref::<Column>()?;
    let schema = scan.schema();
    if col.index() >= schema.fields().len() {
        return None;
    }
    let field_name = schema.field(col.index()).name();
    // Deferred columns cannot be sorted inside `LazySortMode`.
    if scan.deferred_fields().iter().any(|d| d.name == *field_name) {
        return None;
    }
    let feature = if is_score_column_name(field_name) {
        OrderByFeature::Score { rti: 0 }
    } else {
        OrderByFeature::Field {
            name: FieldName::from(field_name.clone()),
            rti: 0,
        }
    };
    let direction = match (
        required_sort.options.descending,
        required_sort.options.nulls_first,
    ) {
        (true, true) => SortDirection::DescNullsFirst,
        (true, false) => SortDirection::DescNullsLast,
        (false, true) => SortDirection::AscNullsFirst,
        (false, false) => SortDirection::AscNullsLast,
    };
    Some(OrderByInfo { feature, direction })
}

fn is_hash_join_eligible(join: &HashJoinExec) -> bool {
    *join.join_type() == JoinType::Inner
        && !join.null_aware
        && !join.on().is_empty()
        && join.left().output_partitioning().partition_count() == 1
        && join.right().output_partitioning().partition_count() == 1
}

/// Maps `ordering` across a pass-through single-child operator (`ProjectionExec`,
/// `VisibilityFilterExec`, `TantivyDecodeExec`, `TantivyFetchExec`, `CoalescePartitionsExec`).
fn map_ordering_through_passthrough(
    node: &Arc<dyn ExecutionPlan>,
    ordering: &LexOrdering,
) -> Result<Option<LexOrdering>> {
    if let Some(proj) = node.downcast_ref::<ProjectionExec>() {
        let proj_exprs = proj.expr();
        let mut mapped = Vec::with_capacity(ordering.len());
        for sort_expr in ordering.iter() {
            let mut valid = true;
            let rewritten = Arc::clone(&sort_expr.expr)
                .transform_down(|expr| {
                    if let Some(col) = expr.downcast_ref::<Column>() {
                        if col.index() >= proj_exprs.len() {
                            valid = false;
                            return Ok(Transformed::no(expr));
                        }
                        return Ok(Transformed::new(
                            Arc::clone(&proj_exprs[col.index()].expr),
                            true,
                            TreeNodeRecursion::Jump,
                        ));
                    }
                    Ok(Transformed::no(expr))
                })?
                .data;
            if !valid {
                return Ok(None);
            }
            mapped.push(PhysicalSortExpr {
                expr: rewritten,
                options: sort_expr.options,
            });
        }
        return Ok(LexOrdering::new(mapped));
    }

    if node.is::<VisibilityFilterExec>() || node.is::<CoalescePartitionsExec>() {
        return Ok(Some(ordering.clone()));
    }

    if node.is::<TantivyDecodeExec>() || node.is::<TantivyFetchExec>() {
        let out_schema = node.schema();
        let in_schema = node.children()[0].schema();
        let mut valid = true;
        for sort_expr in ordering.iter() {
            let _ = sort_expr.expr.apply(|e| {
                if let Some(col) = e.downcast_ref::<Column>()
                    && (col.index() >= out_schema.fields().len()
                        || col.index() >= in_schema.fields().len()
                        || out_schema.field(col.index()).data_type()
                            != in_schema.field(col.index()).data_type())
                {
                    valid = false;
                    return Ok(TreeNodeRecursion::Stop);
                }
                Ok(TreeNodeRecursion::Continue)
            });
            if !valid {
                return Ok(None);
            }
        }
        return Ok(Some(ordering.clone()));
    }

    Ok(None)
}

/// Maps `ordering` from `join.schema()` (post-projection) to `join.join_schema()` (unprojected).
fn unproject_join_ordering(join: &HashJoinExec, ordering: &LexOrdering) -> Result<LexOrdering> {
    let Some(proj) = &join.projection else {
        return Ok(ordering.clone());
    };
    let join_schema = join.join_schema();
    let mut mapped = Vec::with_capacity(ordering.len());
    for sort_expr in ordering.iter() {
        let rewritten = Arc::clone(&sort_expr.expr)
            .transform_down(|expr| {
                if let Some(col) = expr.downcast_ref::<Column>() {
                    let unproj_idx = proj[col.index()];
                    let field_name = join_schema.field(unproj_idx).name();
                    return Ok(Transformed::new(
                        Arc::new(Column::new(field_name, unproj_idx)) as Arc<dyn PhysicalExpr>,
                        true,
                        TreeNodeRecursion::Jump,
                    ));
                }
                Ok(Transformed::no(expr))
            })?
            .data;
        mapped.push(PhysicalSortExpr {
            expr: rewritten,
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
    let mut side = ExprSide::Constant;
    let _ = expr.apply(|node| {
        if let Some(col) = node.downcast_ref::<Column>() {
            let col_side = if col.index() < left_len {
                ExprSide::LeftOnly
            } else {
                ExprSide::RightOnly
            };
            side = side.merge(col_side);
        }
        Ok(TreeNodeRecursion::Continue)
    });
    side
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
    let v = match scalar {
        ScalarValue::Float64(Some(v)) => *v,
        ScalarValue::Float32(Some(v)) => *v as f64,
        ScalarValue::Int64(Some(v)) => *v as f64,
        ScalarValue::Int32(Some(v)) => *v as f64,
        ScalarValue::Int16(Some(v)) => *v as f64,
        ScalarValue::Int8(Some(v)) => *v as f64,
        ScalarValue::UInt64(Some(v)) => *v as f64,
        ScalarValue::UInt32(Some(v)) => *v as f64,
        ScalarValue::UInt16(Some(v)) => *v as f64,
        ScalarValue::UInt8(Some(v)) => *v as f64,
        _ => return None,
    };
    v.is_finite().then_some(v)
}

pub(crate) fn is_score_column_name(name: &str) -> bool {
    name.starts_with("pdb.score(")
        || name.starts_with("__pdb_score")
        || name.starts_with("_pdb_score")
}

/// Checks whether `expr` (over `join_schema`) is guaranteed to evaluate to a non-negative
/// number ($\ge 0$), as required for monotonic multiplication `a * b`.
fn is_expr_non_negative(expr: &Arc<dyn PhysicalExpr>) -> bool {
    if let Some(col) = expr.downcast_ref::<Column>() {
        // BM25 scores (`pdb.score(...)` / `__pdb_score`) are always >= 0.
        return is_score_column_name(col.name());
    }
    if let Some(cast_expr) = expr.downcast_ref::<CastExpr>() {
        return is_expr_non_negative(cast_expr.expr());
    }
    if let Some(try_cast) = expr.downcast_ref::<TryCastExpr>() {
        return is_expr_non_negative(try_cast.expr());
    }
    if let Some(c) = eval_constant_expr(expr) {
        return c >= 0.0;
    }
    if let Some(bin) = expr.downcast_ref::<BinaryExpr>() {
        return match bin.op() {
            Operator::Plus | Operator::Multiply | Operator::Divide => {
                is_expr_non_negative(bin.left()) && is_expr_non_negative(bin.right())
            }
            _ => false,
        };
    }
    if let Some(func) = expr.downcast_ref::<ScalarFunctionExpr>() {
        let name = func.name().to_ascii_lowercase();
        return match name.as_str() {
            "greatest" => func.args().iter().any(is_expr_non_negative),
            "least" => !func.args().is_empty() && func.args().iter().all(is_expr_non_negative),
            "sqrt" | "exp" => true,
            "ln" => {
                // `ln(x)` is >= 0 when `x >= 1`, e.g., `ln(score + 1.0)`.
                if let Some(arg) = func.args().first()
                    && let Some(bin) = arg.downcast_ref::<BinaryExpr>()
                    && *bin.op() == Operator::Plus
                {
                    let l_const = eval_constant_expr(bin.left()).unwrap_or(0.0);
                    let r_const = eval_constant_expr(bin.right()).unwrap_or(0.0);
                    return (l_const >= 1.0 && is_expr_non_negative(bin.right()))
                        || (r_const >= 1.0 && is_expr_non_negative(bin.left()));
                }
                false
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
    Ok(Arc::clone(expr)
        .transform_down(|node| {
            if let Some(col) = node.downcast_ref::<Column>() {
                return Ok(Transformed::new(
                    Arc::new(Column::new(col.name(), col.index() - left_len))
                        as Arc<dyn PhysicalExpr>,
                    true,
                    TreeNodeRecursion::Jump,
                ));
            }
            Ok(Transformed::no(node))
        })?
        .data)
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
    let root_descending = sort_expr.options.descending;

    struct BuilderState {
        left_len: usize,
        root_descending: bool,
        left_sort: Option<PhysicalSortExpr>,
        right_sort: Option<PhysicalSortExpr>,
    }

    impl BuilderState {
        fn record_left_leaf(
            &mut self,
            expr: &Arc<dyn PhysicalExpr>,
            dir: MonotonicDirection,
        ) -> Option<ScoreCombiner> {
            let candidate = PhysicalSortExpr {
                expr: Arc::clone(expr),
                options: dir.to_sort_options(self.root_descending),
            };
            if let Some(existing) = &self.left_sort {
                if existing != &candidate {
                    return None;
                }
            } else {
                self.left_sort = Some(candidate);
            }
            Some(ScoreCombiner::LeftScore)
        }

        fn record_right_leaf(
            &mut self,
            expr: &Arc<dyn PhysicalExpr>,
            dir: MonotonicDirection,
        ) -> Option<ScoreCombiner> {
            let shifted = shift_right_expr(expr, self.left_len).ok()?;
            let candidate = PhysicalSortExpr {
                expr: shifted,
                options: dir.to_sort_options(self.root_descending),
            };
            if let Some(existing) = &self.right_sort {
                if existing != &candidate {
                    return None;
                }
            } else {
                self.right_sort = Some(candidate);
            }
            Some(ScoreCombiner::RightScore)
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
            // match `PgSearchScanPlan`'s native `__pdb_score` column without a cast wrapper.
            if let Some(cast_expr) = expr.downcast_ref::<CastExpr>() {
                if matches!(cast_expr.cast_type(), DataType::Float32 | DataType::Float64) {
                    return self.analyze(cast_expr.expr(), dir);
                }
                return None;
            }
            if let Some(try_cast) = expr.downcast_ref::<TryCastExpr>() {
                if matches!(try_cast.cast_type(), DataType::Float32 | DataType::Float64) {
                    return self.analyze(try_cast.expr(), dir);
                }
                return None;
            }

            // A bare `Column` on one side is always a leaf score input.
            if expr.is::<Column>() {
                return match side {
                    ExprSide::LeftOnly => self.record_left_leaf(expr, dir),
                    ExprSide::RightOnly => self.record_right_leaf(expr, dir),
                    _ => None,
                };
            }

            // If `expr` is a binary expression where BOTH operands belong to the same side
            // (e.g., `score_a + score_b` coming from a child 2-way join inside a 3-way join),
            // treat the whole subexpression as that child join's composite score leaf!
            if let Some(bin) = expr.downcast_ref::<BinaryExpr>() {
                let l_side = classify_expr_side(bin.left(), self.left_len);
                let r_side = classify_expr_side(bin.right(), self.left_len);
                if l_side == ExprSide::LeftOnly && r_side == ExprSide::LeftOnly {
                    return self.record_left_leaf(expr, dir);
                }
                if l_side == ExprSide::RightOnly && r_side == ExprSide::RightOnly {
                    return self.record_right_leaf(expr, dir);
                }

                return match bin.op() {
                    Operator::Plus => {
                        let l = self.analyze(bin.left(), dir)?;
                        let r = self.analyze(bin.right(), dir)?;
                        Some(ScoreCombiner::Add(Box::new(l), Box::new(r)))
                    }
                    Operator::Minus => {
                        let l = self.analyze(bin.left(), dir)?;
                        let r = self.analyze(bin.right(), dir.flip())?;
                        Some(ScoreCombiner::Sub(Box::new(l), Box::new(r)))
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
                    Operator::Divide => {
                        // Division by a non-zero constant `a / c`.
                        let c = eval_constant_expr(bin.right())?;
                        if c == 0.0 {
                            return None;
                        }
                        let child_dir = if c > 0.0 { dir } else { dir.flip() };
                        let l = self.analyze(bin.left(), child_dir)?;
                        Some(ScoreCombiner::Div(
                            Box::new(l),
                            Box::new(ScoreCombiner::Constant(c)),
                        ))
                    }
                    _ => None,
                };
            }

            if let Some(func) = expr.downcast_ref::<ScalarFunctionExpr>() {
                let name = func.name().to_ascii_lowercase();
                return match name.as_str() {
                    "greatest" => {
                        if func.args().is_empty() {
                            return None;
                        }
                        let args: Option<Vec<ScoreCombiner>> =
                            func.args().iter().map(|a| self.analyze(a, dir)).collect();
                        Some(ScoreCombiner::Greatest(args?))
                    }
                    "least" => {
                        if func.args().is_empty() {
                            return None;
                        }
                        let args: Option<Vec<ScoreCombiner>> =
                            func.args().iter().map(|a| self.analyze(a, dir)).collect();
                        Some(ScoreCombiner::Least(args?))
                    }
                    "ln" => {
                        if func.args().len() != 1 {
                            return None;
                        }
                        let inner = self.analyze(&func.args()[0], dir)?;
                        Some(ScoreCombiner::Ln(Box::new(inner)))
                    }
                    "sqrt" => {
                        if func.args().len() != 1 {
                            return None;
                        }
                        let inner = self.analyze(&func.args()[0], dir)?;
                        Some(ScoreCombiner::Sqrt(Box::new(inner)))
                    }
                    "exp" => {
                        if func.args().len() != 1 {
                            return None;
                        }
                        let inner = self.analyze(&func.args()[0], dir)?;
                        Some(ScoreCombiner::Exp(Box::new(inner)))
                    }
                    _ => None,
                };
            }

            None
        }
    }

    let mut state = BuilderState {
        left_len,
        root_descending,
        left_sort: None,
        right_sort: None,
    };
    let combiner = state.analyze(&sort_expr.expr, MonotonicDirection::Increasing)?;
    if !combiner.uses_left() && !combiner.uses_right() {
        return None;
    }
    let dummy_zero = || PhysicalSortExpr {
        expr: Arc::new(Literal::new(ScalarValue::Float64(Some(0.0)))),
        options: SortOptions {
            descending: root_descending,
            nulls_first: false,
        },
    };
    let left_sort_expr = state.left_sort.unwrap_or_else(dummy_zero);
    let right_sort_expr = state.right_sort.unwrap_or_else(dummy_zero);

    Some(DecomposedJoinScore {
        left_sort_expr,
        right_sort_expr,
        combiner,
    })
}

/// Returns `true` if `query` produces a flat/constant BM25 score across all matching documents
/// (e.g. `All`, `ConstScore`, `Range`, `TermSet`), making it unsuitable for BM25 Top-K pruning.
pub(crate) fn is_constant_score_query(query: &SearchQueryInput) -> bool {
    match query {
        SearchQueryInput::Uninitialized
        | SearchQueryInput::All
        | SearchQueryInput::Empty
        | SearchQueryInput::ConstScore { .. }
        | SearchQueryInput::TermSet { .. } => true,
        SearchQueryInput::WithIndex { query, .. }
        | SearchQueryInput::Boost { query, .. }
        | SearchQueryInput::HeapFilter {
            indexed_query: query,
            ..
        } => is_constant_score_query(query),
        SearchQueryInput::ScoreFilter { query, .. } => {
            query.as_deref().is_none_or(is_constant_score_query)
        }
        SearchQueryInput::Boolean { must, should, .. } => {
            if must.is_empty() && should.is_empty() {
                true
            } else {
                must.iter()
                    .chain(should.iter())
                    .all(is_constant_score_query)
            }
        }
        SearchQueryInput::DisjunctionMax { disjuncts, .. } => {
            disjuncts.is_empty() || disjuncts.iter().all(is_constant_score_query)
        }
        SearchQueryInput::MoreLikeThis { .. }
        | SearchQueryInput::Parse { .. }
        | SearchQueryInput::PostgresExpression { .. } => false,
        SearchQueryInput::FieldedQuery { query, .. } => is_constant_score_pdb_query(query),
    }
}

fn is_constant_score_pdb_query(query: &pdb::Query) -> bool {
    match query {
        pdb::Query::All
        | pdb::Query::Empty
        | pdb::Query::Exists
        | pdb::Query::FastFieldRangeWeight { .. }
        | pdb::Query::Range { .. }
        | pdb::Query::RangeContains { .. }
        | pdb::Query::RangeIntersects { .. }
        | pdb::Query::RangeTerm { .. }
        | pdb::Query::RangeWithin { .. }
        | pdb::Query::Regex { .. }
        | pdb::Query::StoredRange { .. }
        | pdb::Query::TermSet { .. } => true,
        pdb::Query::ScoreAdjusted { query, score } => match score {
            Some(ScoreAdjustStyle::Const(_)) => true,
            _ => is_constant_score_pdb_query(query),
        },
        pdb::Query::MoreLikeThis { .. }
        | pdb::Query::UnclassifiedString { .. }
        | pdb::Query::UnclassifiedArray { .. }
        | pdb::Query::FuzzyTerm { .. }
        | pdb::Query::Match { .. }
        | pdb::Query::MatchArray { .. }
        | pdb::Query::Parse { .. }
        | pdb::Query::ParseWithField { .. }
        | pdb::Query::Phrase { .. }
        | pdb::Query::PhraseArray { .. }
        | pdb::Query::PhrasePrefix { .. }
        | pdb::Query::Proximity { .. }
        | pdb::Query::TokenizedPhrase { .. }
        | pdb::Query::RegexPhrase { .. }
        | pdb::Query::Term { .. } => false,
    }
}

/// Estimates a relative upper bound on the BM25 score of `query` for Gate 2's balance check,
/// accounting for query-level `Boost`, `ConstScore`, `Boolean` sums, and `DisjunctionMax`.
pub(crate) fn estimate_query_max_score(query: &SearchQueryInput) -> f64 {
    match query {
        SearchQueryInput::Uninitialized | SearchQueryInput::Empty => 0.0,
        SearchQueryInput::All | SearchQueryInput::TermSet { .. } => 1.0,
        SearchQueryInput::ConstScore { score, .. } => (*score as f64).max(0.0),
        SearchQueryInput::Boost { query, factor } => {
            (*factor as f64).max(0.0) * estimate_query_max_score(query)
        }
        SearchQueryInput::WithIndex { query, .. }
        | SearchQueryInput::HeapFilter {
            indexed_query: query,
            ..
        } => estimate_query_max_score(query),
        SearchQueryInput::ScoreFilter { query, .. } => {
            query.as_deref().map_or(1.0, estimate_query_max_score)
        }
        SearchQueryInput::Boolean { must, should, .. } => must
            .iter()
            .chain(should.iter())
            .map(estimate_query_max_score)
            .sum::<f64>()
            .max(1.0),
        SearchQueryInput::DisjunctionMax { disjuncts, .. } => disjuncts
            .iter()
            .map(estimate_query_max_score)
            .fold(0.0_f64, f64::max)
            .max(1.0),
        SearchQueryInput::MoreLikeThis { .. }
        | SearchQueryInput::Parse { .. }
        | SearchQueryInput::PostgresExpression { .. } => 1.0,
        SearchQueryInput::FieldedQuery { query, .. } => estimate_pdb_query_max_score(query),
    }
}

fn estimate_pdb_query_max_score(query: &pdb::Query) -> f64 {
    match query {
        pdb::Query::ScoreAdjusted { query, score } => match score {
            Some(ScoreAdjustStyle::Boost(b)) => {
                (*b as f64).max(0.0) * estimate_pdb_query_max_score(query)
            }
            Some(ScoreAdjustStyle::Const(c)) => (*c as f64).max(0.0),
            None => estimate_pdb_query_max_score(query),
        },
        _ => 1.0,
    }
}

#[cfg(any(test, feature = "pg_test"))]
mod tests {
    use super::*;
    use arrow_schema::Field;
    use datafusion::physical_plan::joins::PartitionMode;

    fn test_schemas() -> (SchemaRef, SchemaRef) {
        let left = Arc::new(Schema::new(vec![
            Field::new("l_id", DataType::Int32, false),
            Field::new("__pdb_score_l", DataType::Float32, false),
        ]));
        let right = Arc::new(Schema::new(vec![
            Field::new("r_id", DataType::Int32, false),
            Field::new("__pdb_score_r", DataType::Float32, false),
        ]));
        (left, right)
    }

    #[test]
    fn test_analyze_additive_and_weighted_scores() {
        let (left_schema, right_schema) = test_schemas();
        // `2.0 * CAST(__pdb_score_l AS FLOAT8) + 3.0 * CAST(__pdb_score_r AS FLOAT8) DESC`
        let l_score: Arc<dyn PhysicalExpr> = Arc::new(CastExpr::new(
            Arc::new(Column::new("__pdb_score_l", 1)),
            DataType::Float64,
            None,
        ));
        let r_score: Arc<dyn PhysicalExpr> = Arc::new(CastExpr::new(
            Arc::new(Column::new("__pdb_score_r", 3)),
            DataType::Float64,
            None,
        ));
        let expr: Arc<dyn PhysicalExpr> = Arc::new(BinaryExpr::new(
            Arc::new(BinaryExpr::new(
                Arc::new(Literal::new(ScalarValue::Float64(Some(2.0)))),
                Operator::Multiply,
                l_score,
            )),
            Operator::Plus,
            Arc::new(BinaryExpr::new(
                Arc::new(Literal::new(ScalarValue::Float64(Some(3.0)))),
                Operator::Multiply,
                r_score,
            )),
        ));
        let sort_expr = PhysicalSortExpr {
            expr,
            options: SortOptions {
                descending: true,
                nulls_first: false,
            },
        };

        let decomposed = analyze_join_score_expr(&sort_expr, &left_schema, &right_schema)
            .expect("should decompose weighted additive score");
        // Verify `CastExpr` was unwrapped on both leaves so `PgSearchScanPlan` columns match directly.
        let l_col = decomposed
            .left_sort_expr
            .expr
            .downcast_ref::<Column>()
            .unwrap();
        let r_col = decomposed
            .right_sort_expr
            .expr
            .downcast_ref::<Column>()
            .unwrap();
        assert_eq!(l_col.index(), 1);
        assert_eq!(r_col.index(), 1);
        assert!(decomposed.left_sort_expr.options.descending);
        assert!(decomposed.right_sort_expr.options.descending);
        assert_eq!(
            decomposed.combiner.eval(InputScorePair {
                left: 4.0,
                right: 5.0
            }),
            23.0
        );
    }

    #[test]
    fn test_analyze_multiplicative_non_negative_and_reject_signed() {
        let (left_schema, right_schema) = test_schemas();
        // `__pdb_score_l * __pdb_score_r` -> allowed because both are `__pdb_score*` (>= 0)
        let mult_scores = PhysicalSortExpr {
            expr: Arc::new(BinaryExpr::new(
                Arc::new(Column::new("__pdb_score_l", 1)),
                Operator::Multiply,
                Arc::new(Column::new("__pdb_score_r", 3)),
            )),
            options: SortOptions {
                descending: true,
                nulls_first: false,
            },
        };
        assert!(analyze_join_score_expr(&mult_scores, &left_schema, &right_schema).is_some());

        // `l_id * r_id` -> rejected because `l_id` and `r_id` are not proven non-negative
        let mult_signed = PhysicalSortExpr {
            expr: Arc::new(BinaryExpr::new(
                Arc::new(Column::new("l_id", 0)),
                Operator::Multiply,
                Arc::new(Column::new("r_id", 2)),
            )),
            options: SortOptions {
                descending: true,
                nulls_first: false,
            },
        };
        assert!(analyze_join_score_expr(&mult_signed, &left_schema, &right_schema).is_none());
    }

    fn make_test_scan(alias: &str, query: SearchQueryInput, rows: u64) -> Arc<dyn ExecutionPlan> {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("__pdb_score", DataType::Float32, false),
        ]));
        Arc::new(
            PgSearchScanPlan::new(
                None,
                schema,
                query,
                None,
                vec![],
                None,
                1,
                None,
                1,
                None,
                None,
            )
            .with_table_alias(alias)
            .with_estimated_rows_for_test(rows),
        )
    }

    fn bm25_query() -> SearchQueryInput {
        SearchQueryInput::Parse {
            query_string: "shoes".into(),
            lenient: None,
            conjunction_mode: None,
        }
    }

    #[test]
    fn test_rank_join_rule_and_ensure_requirements_removes_top_sort() {
        use datafusion::common::NullEquality;
        use datafusion::physical_expr::projection::ProjectionExpr;
        use datafusion::physical_optimizer::optimizer::PhysicalOptimizer;
        use datafusion::physical_plan::limit::GlobalLimitExec;

        let left_scan = make_test_scan("l", bm25_query(), 50_000);
        let right_scan = make_test_scan("r", bm25_query(), 50_000);

        let join: Arc<dyn ExecutionPlan> = Arc::new(
            HashJoinExec::try_new(
                left_scan,
                right_scan,
                vec![(
                    Arc::new(Column::new("id", 0)),
                    Arc::new(Column::new("id", 0)),
                )],
                None,
                &JoinType::Inner,
                None,
                PartitionMode::CollectLeft,
                NullEquality::NullEqualsNothing,
                false,
            )
            .unwrap(),
        );

        // Project `l_id`, `l_score`, `r_id`, `r_score`
        let proj: Arc<dyn ExecutionPlan> = Arc::new(
            ProjectionExec::try_new(
                vec![
                    ProjectionExpr::new(Arc::new(Column::new("id", 0)), "l_id"),
                    ProjectionExpr::new(Arc::new(Column::new("__pdb_score", 1)), "l_score"),
                    ProjectionExpr::new(Arc::new(Column::new("id", 2)), "r_id"),
                    ProjectionExpr::new(Arc::new(Column::new("__pdb_score", 3)), "r_score"),
                ],
                join,
            )
            .unwrap(),
        );

        // `ORDER BY l_score + r_score DESC, l_id ASC LIMIT 10`
        let sum_expr: Arc<dyn PhysicalExpr> = Arc::new(BinaryExpr::new(
            Arc::new(Column::new("l_score", 1)),
            Operator::Plus,
            Arc::new(Column::new("r_score", 3)),
        ));
        let sort_order = LexOrdering::new(vec![
            PhysicalSortExpr {
                expr: sum_expr,
                options: SortOptions {
                    descending: true,
                    nulls_first: false,
                },
            },
            PhysicalSortExpr {
                expr: Arc::new(Column::new("l_id", 0)),
                options: SortOptions {
                    descending: false,
                    nulls_first: false,
                },
            },
        ])
        .unwrap();
        let top_sort: Arc<dyn ExecutionPlan> =
            Arc::new(SortExec::new(sort_order, proj).with_fetch(Some(10)));

        let config = ConfigOptions::default();
        let rule = RankJoinRule::new_with_min_rows(10_000);
        let after_rank_join = rule.optimize(top_sort, &config).unwrap();

        // Now run `EnsureRequirements` and verify `SortExec(fetch=10)`
        // is replaced by `GlobalLimitExec(fetch=10)` with no child `SortExec`s inserted!
        let ensure_requirements = PhysicalOptimizer::default()
            .rules
            .into_iter()
            .find(|r| r.name() == "EnsureRequirements")
            .expect("EnsureRequirements rule should exist");
        let after_enforce = ensure_requirements
            .optimize(after_rank_join, &config)
            .unwrap();

        assert!(
            after_enforce.is::<GlobalLimitExec>(),
            "expected GlobalLimitExec at root after EnsureRequirements, got: {}",
            after_enforce.name()
        );
        let proj_node = &after_enforce.children()[0];
        assert!(proj_node.is::<ProjectionExec>());
        let rank_join_node = proj_node.children()[0]
            .downcast_ref::<RankJoinExec>()
            .expect("expected RankJoinExec below ProjectionExec");
        let left_leaf = rank_join_node
            .left()
            .downcast_ref::<PgSearchScanPlan>()
            .expect("expected PgSearchScanPlan on left without SortExec");
        let right_leaf = rank_join_node
            .right()
            .downcast_ref::<PgSearchScanPlan>()
            .expect("expected PgSearchScanPlan on right without SortExec");
        assert!(left_leaf.lazy_sort().is_some());
        assert!(right_leaf.lazy_sort().is_some());
    }

    #[test]
    fn test_rank_join_rule_gates_reject_ineligible_plans() {
        use datafusion::common::NullEquality;

        let make_plan = |l_query: SearchQueryInput,
                         r_query: SearchQueryInput,
                         rows: u64|
         -> Arc<dyn ExecutionPlan> {
            let left_scan = make_test_scan("l", l_query, rows);
            let right_scan = make_test_scan("r", r_query, rows);
            let join: Arc<dyn ExecutionPlan> = Arc::new(
                HashJoinExec::try_new(
                    left_scan,
                    right_scan,
                    vec![(
                        Arc::new(Column::new("id", 0)),
                        Arc::new(Column::new("id", 0)),
                    )],
                    None,
                    &JoinType::Inner,
                    None,
                    PartitionMode::CollectLeft,
                    NullEquality::NullEqualsNothing,
                    false,
                )
                .unwrap(),
            );
            let sum_expr: Arc<dyn PhysicalExpr> = Arc::new(BinaryExpr::new(
                Arc::new(Column::new("__pdb_score", 1)),
                Operator::Plus,
                Arc::new(Column::new("__pdb_score", 3)),
            ));
            let sort_order = LexOrdering::new(vec![PhysicalSortExpr {
                expr: sum_expr,
                options: SortOptions {
                    descending: true,
                    nulls_first: false,
                },
            }])
            .unwrap();
            Arc::new(SortExec::new(sort_order, join).with_fetch(Some(10)))
        };

        let config = ConfigOptions::default();
        let rule = RankJoinRule::new_with_min_rows(10_000);

        // Gate 1 rejection: `SearchQueryInput::All` is a constant-score query.
        let plan_const = make_plan(SearchQueryInput::All, bm25_query(), 50_000);
        let out_const = rule.optimize(plan_const, &config).unwrap();
        assert!(out_const.children()[0].is::<HashJoinExec>());

        // Gate 2 rejection: 100x boost imbalance (`ratio = 0.01 < 0.20`).
        let boosted = SearchQueryInput::Boost {
            query: Box::new(bm25_query()),
            factor: 100.0,
        };
        let plan_imbalanced = make_plan(boosted, bm25_query(), 50_000);
        let out_imbalanced = rule.optimize(plan_imbalanced, &config).unwrap();
        assert!(out_imbalanced.children()[0].is::<HashJoinExec>());

        // Gate 3 rejection: row count below `min_rows` (1,000 < 10,000).
        let plan_small = make_plan(bm25_query(), bm25_query(), 1_000);
        let out_small = rule.optimize(plan_small, &config).unwrap();
        assert!(out_small.children()[0].is::<HashJoinExec>());
    }
}
