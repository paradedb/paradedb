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

use std::cmp::Ordering;
use std::ops::Bound;
use std::sync::Arc;

use arrow_schema::{SchemaRef, SortOptions};
use datafusion::common::SplitPoint;
use datafusion::physical_expr::{
    LexOrdering, PhysicalSortExpr, RangePartitioning as DataFusionRangePartitioning,
};
use datafusion::physical_plan::Partitioning;
use datafusion::physical_plan::expressions::Column;
use serde::{Deserialize, Serialize};

use crate::api::FieldName;
use crate::index::stats::Segment1DBounds;
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::query::SearchQueryInput;
use crate::query::pdb_query::pdb::Query;

/// Defines logical boundaries for scanning the index. A boundary need not be an actual indexed
/// value, unlike an empirical minimum or maximum. The DataFusion execution plan turns these
/// boundaries into exhaustive query ranges and maps the current execution segments to them,
/// rather than relying on dynamic segment checkout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangePartitioning {
    /// The index field used to define the boundaries.
    pub partition_by: FieldName,
    /// The values that split the data space into separate partitions. A length of N
    /// produces N+1 partitions.
    pub split_points: Vec<PdbOwnedValue>,
}

/// The rows one partition holds: the value range and whether the NULLs are among them.
#[derive(Debug, Clone, PartialEq)]
pub struct PartitionRange {
    values: Option<(Bound<PdbOwnedValue>, Bound<PdbOwnedValue>)>,
    includes_nulls: bool,
}

impl PartitionRange {
    /// The half-open value range, or `None` when a NULL split point closes the partition so no
    /// value belongs to it.
    pub fn values(&self) -> Option<(&Bound<PdbOwnedValue>, &Bound<PdbOwnedValue>)> {
        self.values.as_ref().map(|(lower, upper)| (lower, upper))
    }

    pub fn includes_nulls(&self) -> bool {
        self.includes_nulls
    }
}

impl RangePartitioning {
    /// The partition a row with a NULL partition field lands in. The kd-tree of a partitioned
    /// build sends NULLs below every split, so its lowest partition is this one, and so is the
    /// first partition of DataFusion's `NULLS FIRST` range ordering. Every consumer of the NULL
    /// rule reads it from here.
    pub const NULL_PARTITION: usize = 0;

    /// Returns the logical bounds as a range query, including the NULL clause where needed.
    ///
    /// **Consumer Caveats**:
    /// - A row whose partition field is NULL will be deterministically routed to
    ///   [`Self::NULL_PARTITION`].
    /// - A multi-valued field can fall into multiple partition ranges and duplicate the row. This is statically prevented during index configuration for `partition_by` columns.
    pub fn partition_bounds(&self, partition: usize) -> SearchQueryInput {
        let Some(range) = self.partition_range(partition) else {
            return SearchQueryInput::All;
        };
        let range_query = range
            .values()
            .map(|(lower, upper)| SearchQueryInput::FieldedQuery {
                field: self.partition_by.clone(),
                query: if matches!((lower, upper), (Bound::Unbounded, Bound::Unbounded)) {
                    // After a NULL split, this partition contains every non-NULL value.
                    // The range compiler requires at least one finite bound.
                    Query::Exists
                } else {
                    Query::Range {
                        lower_bound: lower.clone(),
                        upper_bound: upper.clone(),
                    }
                },
            });
        let null_query = range.includes_nulls().then(|| SearchQueryInput::Boolean {
            // A pure-negative Boolean matches nothing. All supplies the positive clause.
            must: vec![SearchQueryInput::All],
            should: vec![],
            must_not: vec![SearchQueryInput::FieldedQuery {
                field: self.partition_by.clone(),
                query: Query::Exists,
            }],
            minimum_should_match: None,
        });
        match (range_query, null_query) {
            (Some(range_query), Some(null_query)) => SearchQueryInput::Boolean {
                must: vec![],
                should: vec![range_query, null_query],
                must_not: vec![],
                minimum_should_match: None,
            },
            (Some(query), None) | (None, Some(query)) => query,
            (None, None) => SearchQueryInput::Empty,
        }
    }

    /// The rows of `partition`, or `None` without split points, when every row belongs to the
    /// single partition. [`Self::partition_bounds`] queries exactly these rows.
    pub fn partition_range(&self, partition: usize) -> Option<PartitionRange> {
        if self.split_points.is_empty() {
            return None;
        }
        let lower = match partition
            .checked_sub(1)
            .and_then(|i| self.split_points.get(i))
        {
            None | Some(PdbOwnedValue::Null) => Bound::Unbounded,
            Some(val) => Bound::Included(val.clone()),
        };
        let values = match self.split_points.get(partition) {
            Some(PdbOwnedValue::Null) => None,
            Some(val) => Some((lower, Bound::Excluded(val.clone()))),
            None => Some((lower, Bound::Unbounded)),
        };
        Some(PartitionRange {
            values,
            includes_nulls: partition == Self::NULL_PARTITION,
        })
    }

    /// Translates these boundaries into a DataFusion [`Partitioning::Range`] declaration
    /// over `schema`, so the planner can co-partition operators (e.g. joins) without a
    /// repartition or broadcast.
    ///
    /// Returns `None` when the declaration would not be faithful to the execution
    /// semantics of [`Self::partition_bounds`]:
    /// - the `partition_by` column is missing from the schema, or
    /// - a split point is NULL (`partition_bounds` gives NULL split points bespoke
    ///   empty-range semantics that DataFusion's model does not express), or
    /// - a split point cannot be represented as a `ScalarValue` of the column's type.
    #[cfg(any(test, feature = "pg_test"))]
    pub fn to_datafusion(&self, schema: &SchemaRef) -> Option<Partitioning> {
        let (col_idx, field) = schema.column_with_name(self.partition_by.as_ref())?;

        let split_points = self
            .split_points
            .iter()
            .map(|value| {
                value
                    .to_scalar(field.data_type())
                    .map(|sv| SplitPoint::new(vec![sv]))
            })
            .collect::<Option<Vec<_>>>()?;

        // `partition_bounds` routes NULLs to `NULL_PARTITION`, the first one, and uses
        // lower-inclusive, upper-exclusive interior ranges, which is exactly DataFusion's
        // split-point convention under an ascending NULLS FIRST ordering.
        let sort_expr = PhysicalSortExpr {
            expr: Arc::new(Column::new(self.partition_by.as_ref(), col_idx)),
            options: SortOptions {
                descending: false,
                nulls_first: true,
            },
        };
        let ordering = LexOrdering::new([sort_expr])?;

        // `new` rather than `try_new`: a repeated split point produces an empty partition in
        // both our execution model and DataFusion's, but fails `try_new`'s strict-ordering
        // validation. The build never repeats a split point; the tolerance costs nothing.
        Some(Partitioning::Range(DataFusionRangePartitioning::new(
            ordering,
            split_points,
        )))
    }
}

/// The split points a partitioned build stamped on an index's segments, kept as raw points
/// rather than a static `RangePartitioning`, so that `TaskEstimator`s and distributed
/// execution engines can ask for their preferred number of partitions at runtime.
///
/// **Precondition**: `points` must be sorted ascending, otherwise the generated
/// boundaries will produce overlapping or gapped ranges that silently drop or duplicate rows.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RangeSplitPoints {
    /// The index field used to define the boundaries.
    pub partition_by: FieldName,
    /// The points to cut on, sorted ascending: the edges of every box the build stamped, so a
    /// partition cut on them lines up with the segments. Typically there are more than the
    /// target number of partitions, so `build` can down-sample them evenly.
    pub points: Vec<PdbOwnedValue>,
}

impl RangeSplitPoints {
    /// The partitions these points cut for a request of `target_partitions`: never more than
    /// the points seat, so no partition is empty.
    pub fn partitions_for(&self, target_partitions: usize) -> usize {
        target_partitions.min(self.points.len() + 1).max(1)
    }

    /// Generates a concrete `RangePartitioning` bounding exactly
    /// `self.partitions_for(target_partitions)` partitions.
    ///
    /// If `target_partitions` is smaller than the points seat, the points are evenly
    /// down-sampled (effectively merging contiguous partitions).
    pub fn build(&self, target_partitions: usize) -> RangePartitioning {
        let points = &self.points;
        debug_assert!(
            points
                .windows(2)
                .all(|w| w[0].total_cmp(&w[1]) != std::cmp::Ordering::Greater),
            "RangeSplitPoints requires its points to be sorted ascending"
        );

        let num_points = points.len();
        let actual_partitions = self.partitions_for(target_partitions);

        if actual_partitions <= 1 {
            return RangePartitioning {
                partition_by: self.partition_by.clone(),
                split_points: vec![],
            };
        }

        // When split points were pre-optimized for exactly `actual_partitions`, preserve
        // them as-is without lossy downsampling.
        if num_points == actual_partitions - 1 {
            return RangePartitioning {
                partition_by: self.partition_by.clone(),
                split_points: points.clone(),
            };
        }

        let mut new_split_points = Vec::with_capacity(actual_partitions - 1);

        // Down-sample evenly. We want `actual_partitions - 1` split points.
        for i in 1..actual_partitions {
            let split_idx = (i * num_points) / actual_partitions;
            new_split_points.push(points[split_idx].clone());
        }

        RangePartitioning {
            partition_by: self.partition_by.clone(),
            split_points: new_split_points,
        }
    }

    /// Converts these points into a DataFusion [`Partitioning::Range`] bounding `partition_count`
    /// partitions, using all `self.points` as samples so DataFusion stage scaling (`scale`)
    /// and anchor execution down-sample from the identical base.
    ///
    /// Returns `None` if:
    /// - `partition_count <= 1` or `partition_count > self.points.len() + 1`,
    /// - `self.partition_by` is not present in `schema`,
    /// - any split point cannot be represented as a `ScalarValue` of the column's type, or
    /// - DataFusion validation fails (e.g. non-strictly ordered points).
    pub fn to_datafusion(
        &self,
        schema: &SchemaRef,
        partition_count: usize,
    ) -> Option<Partitioning> {
        if partition_count <= 1 || partition_count > self.points.len() + 1 {
            return None;
        }

        let (col_idx, field) = schema.column_with_name(self.partition_by.as_ref())?;

        let samples = self
            .points
            .iter()
            .map(|value| {
                value
                    .to_scalar(field.data_type())
                    .map(|sv| SplitPoint::new(vec![sv]))
            })
            .collect::<Option<Vec<_>>>()?;

        let sort_expr = PhysicalSortExpr {
            expr: Arc::new(Column::new(self.partition_by.as_ref(), col_idx)),
            options: SortOptions {
                descending: false,
                nulls_first: true,
            },
        };
        let ordering = LexOrdering::new([sort_expr])?;

        // Fast path: if the points were pre-optimized for exactly `partition_count`, use
        // `DataFusionRangePartitioning::new` directly to preserve the exact split points
        // without passing through DataFusion's linear downsampling.
        if self.points.len() == partition_count - 1 {
            return Some(Partitioning::Range(DataFusionRangePartitioning::new(
                ordering, samples,
            )));
        }

        DataFusionRangePartitioning::try_new_with_samples(ordering, samples, partition_count)
            .ok()
            .map(Partitioning::Range)
    }
}

/// Segment layout and document volume metadata for one table participating in
/// joint range partition optimization.
#[derive(Debug, Clone, Copy)]
pub struct JointPartitionInput<'a> {
    /// 1D bounding intervals, empirical min/max stats, and doc counts for all visible segments.
    pub segments: &'a [Segment1DBounds],
    /// Total row count estimate for the table (used to scale balance penalties and fallback volumes).
    pub total_rows: u64,
}

impl<'a> JointPartitionInput<'a> {
    pub fn new(segments: &'a [Segment1DBounds], total_rows: u64) -> Self {
        Self {
            segments,
            total_rows,
        }
    }

    pub fn empty() -> Self {
        Self {
            segments: &[],
            total_rows: 0,
        }
    }
}

/// Optimizes `target_partitions - 1` split points across two joining tables (or a single table)
/// to minimize the volume of data in partially included segments, subject to worker load balance.
///
/// # Problem Formulation
///
/// In distributed MPP joins, range partitioning maps rows from both inputs into `target_partitions`
/// worker streams. When a worker's assigned range interval `[c_{k-1}, c_k)` bisects an index segment,
/// that segment cannot be cleanly pruned or fully included; it becomes a "partial" segment requiring
/// row-level evaluation via a Tantivy `RangeQuery`.
///
/// Naïvely picking split points from only one table, or treating all partial segments equally (+1 cost),
/// leads to severe performance degradation:
/// 1. Slicing through a segment of a 20M-row table (e.g. 625K docs) costs orders of magnitude more
///    range filtering than slicing through a segment of a 2.2M-row table (e.g. 70K docs).
/// 2. Imposing the split points of a small table onto a large table can produce extreme data volume
///    imbalance (e.g. 100x skew across workers), making the slowest worker dominate query latency.
///
/// # Optimization Algorithm
///
/// This function frames split point selection as a shortest-path dynamic programming problem on an
/// interval DAG:
///
/// 1. **Candidate Pooling**: Collects and deduplicates all non-unbounded segment boundaries from both
///    tables: `C = {c_1, c_2, ..., c_M}`.
///
/// 2. **Stabbing Volume Cost**: For each candidate boundary `c`, computes the total document volume of
///    all segments from both tables sliced by `c`:
///    `CutCost(c) = sum(s.num_docs for s cut by c)`
///
/// 3. **Interval Balance Penalty**: For each partition interval `[c_i, c_j)`, calculates cumulative
///    documents `Delta_L, Delta_R` and applies a quadratic penalty relative to ideal partition targets
///    `T_L = TotalDocs_L / K` and `T_R = TotalDocs_R / K`:
///    `BalancePenalty = lambda * (Delta_L - T_L)^2 / T_L + lambda * min(1, T_R / T_L) * (Delta_R - T_R)^2 / T_R`
///    where `lambda = 4.0`. The quadratic cost penalizes severe stragglers much more heavily than mild skew.
///
/// 4. **Dynamic Programming**:
///    `dp[k][j]` represents the minimum cost of partitioning the prefix up to candidate `c_j` into `k`
///    partitions:
///    `dp[k][j] = min_{i < j} (dp[k-1][i] + Cost(i, j))`
///    where `Cost(i, j) = CutCost(c_j) + BalancePenalty(i, j)` (with `CutCost = 0` for the final partition ending at +infinity).
///
/// 5. **Backtracking**: Recovers the optimal `K-1` candidate split points.
///
/// # Complexity
///
/// `O(K * M^2)` time and `O(K * M)` space, where `K` is `target_partitions` (e.g. 8) and `M` is
/// the number of unique segment boundaries (`<= 2 * (left_segs + right_segs)`, typically `<= 128`).
/// Total planning time is on the order of 10–30 microseconds.
#[allow(clippy::needless_range_loop)]
pub fn optimize_joint_split_points(
    left: JointPartitionInput<'_>,
    right: JointPartitionInput<'_>,
    target_partitions: usize,
) -> Vec<PdbOwnedValue> {
    if target_partitions <= 1 {
        return Vec::new();
    }
    let k_partitions = target_partitions;

    // 1. Gather all unique candidate cut points from non-unbounded segment boundaries.
    let mut candidates = Vec::new();
    for s in left.segments.iter().chain(right.segments.iter()) {
        for bound in [&s.lower, &s.upper] {
            if let Bound::Included(v) | Bound::Excluded(v) = bound {
                candidates.push(v.clone());
            }
        }
    }
    candidates.sort_unstable_by(PdbOwnedValue::total_cmp);
    candidates.dedup_by(|a, b| a.total_cmp(b) == Ordering::Equal);

    let m_candidates = candidates.len();
    if m_candidates == 0 {
        return Vec::new();
    }
    if m_candidates < k_partitions - 1 {
        return candidates;
    }

    // 2. Precompute cut costs (stabbing document volume) for each candidate point.
    let mut cut_cost = Vec::with_capacity(m_candidates);
    for cand in &candidates {
        let stab_l: u64 = left
            .segments
            .iter()
            .filter(|s| segment_is_cut_by(s, cand))
            .map(|s| s.num_docs)
            .sum();
        let stab_r: u64 = right
            .segments
            .iter()
            .filter(|s| segment_is_cut_by(s, cand))
            .map(|s| s.num_docs)
            .sum();
        cut_cost.push(stab_l + stab_r);
    }

    // 3. Precompute cumulative document volumes up to each candidate point.
    let cum_docs_l: Vec<f64> = candidates
        .iter()
        .map(|c| left.segments.iter().map(|s| segment_docs_below(s, c)).sum())
        .collect();
    let cum_docs_r: Vec<f64> = candidates
        .iter()
        .map(|c| {
            right
                .segments
                .iter()
                .map(|s| segment_docs_below(s, c))
                .sum()
        })
        .collect();

    let total_docs_l: f64 = left
        .segments
        .iter()
        .map(|s| s.num_docs as f64)
        .sum::<f64>()
        .max(left.total_rows as f64);
    let total_docs_r: f64 = right
        .segments
        .iter()
        .map(|s| s.num_docs as f64)
        .sum::<f64>()
        .max(right.total_rows as f64);

    let target_l = if total_docs_l > 0.0 {
        total_docs_l / k_partitions as f64
    } else {
        0.0
    };
    let target_r = if total_docs_r > 0.0 {
        total_docs_r / k_partitions as f64
    } else {
        0.0
    };

    let lambda = 4.0;

    // Helper to compute cost of an interval (start_idx, end_idx)
    // start_idx: None means -infinity; Some(i) means candidate i
    // end_idx: candidate index j in 0..m_candidates, or m_candidates for +infinity
    let interval_cost = |start_idx: Option<usize>, end_idx: usize| -> f64 {
        let (v_l_start, v_r_start) = match start_idx {
            None => (0.0, 0.0),
            Some(i) => (cum_docs_l[i], cum_docs_r[i]),
        };
        let (v_l_end, v_r_end, cut_c) = if end_idx < m_candidates {
            (
                cum_docs_l[end_idx],
                cum_docs_r[end_idx],
                cut_cost[end_idx] as f64,
            )
        } else {
            (total_docs_l, total_docs_r, 0.0)
        };

        let delta_l = (v_l_end - v_l_start).max(0.0);
        let delta_r = (v_r_end - v_r_start).max(0.0);

        let mut balance_penalty = 0.0;
        if total_docs_l > 0.0 && target_l > 0.0 {
            let diff_l = delta_l - target_l;
            balance_penalty += lambda * (diff_l * diff_l) / target_l;
        }
        if total_docs_r > 0.0 && target_r > 0.0 {
            let diff_r = delta_r - target_r;
            let r_scale = if total_docs_l > 0.0 {
                (total_docs_r / total_docs_l).min(1.0)
            } else {
                1.0
            };
            balance_penalty += lambda * r_scale * (diff_r * diff_r) / target_r;
        }

        cut_c + balance_penalty
    };

    // 4. Dynamic programming:
    // dp[k][j]: min cost of partitioning up to candidate j into k partitions.
    let mut dp = vec![vec![f64::INFINITY; m_candidates + 1]; k_partitions + 1];
    let mut parent = vec![vec![0usize; m_candidates + 1]; k_partitions + 1];

    for j in 0..=m_candidates {
        dp[1][j] = interval_cost(None, j);
    }

    for k in 2..=k_partitions {
        for j in (k - 1)..=m_candidates {
            let mut best_cost = f64::INFINITY;
            let mut best_i = 0;
            for i in (k - 2)..j {
                let cost = dp[k - 1][i] + interval_cost(Some(i), j);
                if cost < best_cost {
                    best_cost = cost;
                    best_i = i;
                }
            }
            dp[k][j] = best_cost;
            parent[k][j] = best_i;
        }
    }

    if !dp[k_partitions][m_candidates].is_finite() {
        // Fallback: even downsampling if DP found no path
        let mut fallback = Vec::with_capacity(k_partitions - 1);
        for i in 1..k_partitions {
            let split_idx = (i * m_candidates) / k_partitions;
            fallback.push(candidates[split_idx].clone());
        }
        return fallback;
    }

    // 5. Backtrack from parent[k_partitions][m_candidates] to find split points.
    let mut split_indices = Vec::with_capacity(k_partitions - 1);
    let mut curr_j = m_candidates;
    for k in (2..=k_partitions).rev() {
        let prev_i = parent[k][curr_j];
        split_indices.push(prev_i);
        curr_j = prev_i;
    }
    split_indices.reverse();

    split_indices
        .into_iter()
        .map(|idx| candidates[idx].clone())
        .collect()
}

/// Determines whether candidate split point `point` bisects segment `s`.
///
/// A split point `P` divides the value space into left partition `(-inf, P)` and right
/// partition `[P, inf)`.
///
/// 1. **Logical Box Bounds**: A segment with bounds `[lower, upper)` is only bisected if `P` falls
///    strictly in the interior: `lower < P < upper`. If `P <= lower` or `P >= upper`, the entire
///    segment falls cleanly into the right or left partition.
///
/// 2. **Empirical Value Bounds**: If empirical min/max stats `[min, max]` exist for the segment:
///    - If `P <= min`, all documents are `>= P`, so every document lands in `[P, inf)` (right partition).
///      The segment is fully included on the right and excluded on the left; it is NOT cut.
///    - If `P > max`, all documents are `< P`, so every document lands in `(-inf, P)` (left partition).
///      The segment is fully included on the left and excluded on the right; it is NOT cut.
///    - Only when `min < P <= max` does the boundary slice through the document population, causing
///      the segment to be partially included on both sides and requiring a Tantivy `RangeQuery`.
fn segment_is_cut_by(s: &Segment1DBounds, point: &PdbOwnedValue) -> bool {
    let logically_cuts = match (&s.lower, &s.upper) {
        (Bound::Unbounded, Bound::Unbounded) => true,
        (Bound::Unbounded, Bound::Included(u) | Bound::Excluded(u)) => {
            point.total_cmp(u) == Ordering::Less
        }
        (Bound::Included(l) | Bound::Excluded(l), Bound::Unbounded) => {
            l.total_cmp(point) == Ordering::Less
        }
        (Bound::Included(l) | Bound::Excluded(l), Bound::Included(u) | Bound::Excluded(u)) => {
            l.total_cmp(point) == Ordering::Less && point.total_cmp(u) == Ordering::Less
        }
    };
    if !logically_cuts {
        return false;
    }
    match (&s.empirical_min, &s.empirical_max) {
        (Some(min), Some(max)) => {
            point.total_cmp(min) == Ordering::Greater && point.total_cmp(max) != Ordering::Greater
        }
        (Some(min), None) => point.total_cmp(min) == Ordering::Greater,
        (None, Some(max)) => point.total_cmp(max) != Ordering::Greater,
        (None, None) => true,
    }
}

/// Estimates the cumulative number of documents in segment `s` with values strictly below `point`.
///
/// - If `point <= min` (or `point <= lower`), 0 documents lie below `point`.
/// - If `point > max` (or `point >= upper`), all `s.num_docs` lie below `point`.
/// - If `point` falls within `[min, max]`, linear density interpolation estimates the fraction:
///   `docs = s.num_docs * (P - min) / (max - min)`
///   For non-numeric types or segments with zero width, falls back to an even 50% midpoint split.
fn segment_docs_below(s: &Segment1DBounds, point: &PdbOwnedValue) -> f64 {
    let at_or_before_start = match &s.empirical_min {
        Some(min) => point.total_cmp(min) != Ordering::Greater,
        None => match &s.lower {
            Bound::Included(l) | Bound::Excluded(l) => point.total_cmp(l) != Ordering::Greater,
            Bound::Unbounded => false,
        },
    };
    if at_or_before_start {
        return 0.0;
    }

    let at_or_after_end = match &s.empirical_max {
        Some(max) => point.total_cmp(max) != Ordering::Less,
        None => match &s.upper {
            Bound::Included(u) | Bound::Excluded(u) => point.total_cmp(u) != Ordering::Less,
            Bound::Unbounded => false,
        },
    };
    if at_or_after_end {
        return s.num_docs as f64;
    }

    let (min_val, max_val) = match (&s.empirical_min, &s.empirical_max) {
        (Some(min), Some(max)) => (min, max),
        _ => match (&s.lower, &s.upper) {
            (Bound::Included(l) | Bound::Excluded(l), Bound::Included(u) | Bound::Excluded(u)) => {
                (l, u)
            }
            _ => return (s.num_docs as f64) * 0.5,
        },
    };

    match (
        value_to_f64(min_val),
        value_to_f64(max_val),
        value_to_f64(point),
    ) {
        (Some(min_f), Some(max_f), Some(pt_f)) if max_f > min_f => {
            let frac = ((pt_f - min_f) / (max_f - min_f)).clamp(0.0, 1.0);
            (s.num_docs as f64) * frac
        }
        _ => (s.num_docs as f64) * 0.5,
    }
}

/// Extracts a numeric `f64` representation from `PdbOwnedValue` for density interpolation.
/// Supports unsigned and signed integers, floats, and date/timestamps. Returns `None` for
/// non-numeric types (e.g. strings or JSON objects).
fn value_to_f64(val: &PdbOwnedValue) -> Option<f64> {
    match val {
        PdbOwnedValue::U64(v) => Some(*v as f64),
        PdbOwnedValue::I64(v) => Some(*v as f64),
        PdbOwnedValue::F64(v) => Some(*v),
        PdbOwnedValue::Date(v) => Some(i64::from(*v) as f64),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ops::Bound;

    fn make_seg(
        lower: i64,
        upper: i64,
        emp_min: i64,
        emp_max: i64,
        num_docs: u64,
    ) -> Segment1DBounds {
        Segment1DBounds {
            segment_id: tantivy::index::SegmentId::generate_random(),
            lower: Bound::Included(PdbOwnedValue::I64(lower)),
            upper: Bound::Excluded(PdbOwnedValue::I64(upper)),
            empirical_min: Some(PdbOwnedValue::I64(emp_min)),
            empirical_max: Some(PdbOwnedValue::I64(emp_max)),
            num_docs,
        }
    }

    #[test]
    fn test_optimize_joint_split_points_clean_1d() {
        // Two tables with 4 aligned segments each
        let left_segs = vec![
            make_seg(0, 10, 0, 9, 100),
            make_seg(10, 20, 10, 19, 100),
            make_seg(20, 30, 20, 29, 100),
            make_seg(30, 40, 30, 39, 100),
        ];
        let right_segs = vec![
            make_seg(0, 10, 0, 9, 50),
            make_seg(10, 20, 10, 19, 50),
            make_seg(20, 30, 20, 29, 50),
            make_seg(30, 40, 30, 39, 50),
        ];

        let left = JointPartitionInput::new(&left_segs, 400);
        let right = JointPartitionInput::new(&right_segs, 200);

        let points = optimize_joint_split_points(left, right, 4);
        assert_eq!(
            points,
            vec![
                PdbOwnedValue::I64(10),
                PdbOwnedValue::I64(20),
                PdbOwnedValue::I64(30),
            ]
        );
    }

    #[test]
    fn test_optimize_joint_split_points_single_table_even_split() {
        // Single table with 8 segments, asking for 4 partitions
        let segs = vec![
            make_seg(0, 10, 0, 9, 100),
            make_seg(10, 20, 10, 19, 100),
            make_seg(20, 30, 20, 29, 100),
            make_seg(30, 40, 30, 39, 100),
            make_seg(40, 50, 40, 49, 100),
            make_seg(50, 60, 50, 59, 100),
            make_seg(60, 70, 60, 69, 100),
            make_seg(70, 80, 70, 79, 100),
        ];
        let left = JointPartitionInput::new(&segs, 800);
        let right = JointPartitionInput::empty();

        let points = optimize_joint_split_points(left, right, 4);
        assert_eq!(
            points,
            vec![
                PdbOwnedValue::I64(20),
                PdbOwnedValue::I64(40),
                PdbOwnedValue::I64(60),
            ]
        );
    }

    #[test]
    fn test_optimize_joint_split_points_avoids_slicing_heavy_table() {
        // Table L: 4 heavy segments (1M rows each) with overlapping 1D projections (like KD-tree slices)
        // Seg 0: [0, 100)
        // Seg 1: [50, 150)
        // Seg 2: [100, 200)
        // Seg 3: [150, 250)
        let left_segs = vec![
            make_seg(0, 100, 0, 99, 1_000_000),
            make_seg(50, 150, 50, 149, 1_000_000),
            make_seg(100, 200, 100, 199, 1_000_000),
            make_seg(150, 250, 150, 249, 1_000_000),
        ];

        // Table R: tiny 1D table with split point at 125 (cuts through both Seg 1 and Seg 2 of L)
        let right_segs = vec![
            make_seg(0, 125, 0, 124, 10_000),
            make_seg(125, 250, 125, 249, 10_000),
        ];

        let left = JointPartitionInput::new(&left_segs, 4_000_000);
        let right = JointPartitionInput::new(&right_segs, 20_000);

        // For 2 partitions: cutting at 100 cuts only Seg 1 (1M docs stabbed in L).
        // Cutting at 125 cuts Seg 1 AND Seg 2 (2M docs stabbed in L).
        let points = optimize_joint_split_points(left, right, 2);
        assert_eq!(points.len(), 1);
        // The algorithm should prefer 100 over 125 because stabbing 1M fewer rows in L
        // far outweighs cutting R at 100 (which only stabs 10k rows in R).
        assert_eq!(points[0], PdbOwnedValue::I64(100));
    }

    #[test]
    fn test_optimize_joint_split_points_edge_cases() {
        let segs = vec![make_seg(0, 10, 0, 9, 100), make_seg(10, 20, 10, 19, 100)];
        let input = JointPartitionInput::new(&segs, 200);

        // K <= 1 returns empty
        assert!(optimize_joint_split_points(input, JointPartitionInput::empty(), 1).is_empty());
        assert!(optimize_joint_split_points(input, JointPartitionInput::empty(), 0).is_empty());

        // Empty inputs return empty
        assert!(
            optimize_joint_split_points(
                JointPartitionInput::empty(),
                JointPartitionInput::empty(),
                4
            )
            .is_empty()
        );

        // Fewer candidates than K - 1 returns all candidates
        let points = optimize_joint_split_points(input, JointPartitionInput::empty(), 10);
        assert_eq!(points.len(), 3); // 0, 10, 20
    }
}
