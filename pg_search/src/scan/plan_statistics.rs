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

//! Shared statistics helpers for physical optimizer rules.
//!
//! These helpers provide a uniform way to query DataFusion's statistics
//! machinery without duplicating join-estimation logic or reusing a stale
//! `StatisticsContext` across optimizer rewrites.

use datafusion::physical_expr::expressions::Column;
use datafusion::physical_plan::ExecutionPlan;
use datafusion::physical_plan::joins::HashJoinExec;
use datafusion::physical_plan::statistics::{StatisticsArgs, StatisticsContext};
use std::sync::Arc;

/// Returns the estimated output row count of `node`, or `None` if
/// statistics are unavailable or zero.
///
/// This uses a fresh `StatisticsContext` per call because its memoization
/// cache is keyed by raw plan-node pointers, which are invalidated by
/// optimizer-tree rewrites.
pub(crate) fn estimated_output_rows(node: &Arc<dyn ExecutionPlan>) -> Option<usize> {
    let stats = StatisticsContext::new()
        .compute(node.as_ref(), &StatisticsArgs::new())
        .ok()?;
    stats.num_rows.get_value().copied().filter(|rows| *rows > 0)
}

/// Returns true if the join's cardinality estimate is backed by publisher-provided
/// NDV (distinct_count) on all equi-join keys.
///
/// A join is considered NDV-backed when:
/// - It is an Inner, Left, or Right join (Full joins are excluded due to
///   ambiguous cardinality semantics).
/// - Every equi-join key on both sides is a physical `Column` expression.
/// - For each such key column, the child statistics have a non-Absent
///   `distinct_count`.
/// - Both sides have non-Absent `num_rows`.
///
/// Full joins, semi/anti joins, cross joins, and joins with non-Column
/// expressions are not considered NDV-backed.
pub(crate) fn is_ndv_backed(
    join: &HashJoinExec,
    left_stats: &datafusion::common::stats::Statistics,
    right_stats: &datafusion::common::stats::Statistics,
) -> bool {
    use datafusion::common::JoinType;

    // Only Inner, Left, and Right joins have well-defined cardinality estimation
    // based on key NDV. Full joins have ambiguous cardinality semantics; semi/anti joins
    // use different estimation logic.
    if !matches!(
        join.join_type(),
        JoinType::Inner | JoinType::Left | JoinType::Right
    ) {
        return false;
    }

    // Both sides must have a valid row count estimate.
    if left_stats.num_rows.get_value().is_none() || right_stats.num_rows.get_value().is_none() {
        return false;
    }

    // All equi-join keys must be simple Column references with valid NDV on both sides.
    // Use Downcast::downcast_ref (not as_any) — same fork TypeId fix as HashJoinExec.
    for (left_col, right_col) in join.on() {
        let (left_idx, right_idx) = match (
            left_col.as_ref().downcast_ref::<Column>(),
            right_col.as_ref().downcast_ref::<Column>(),
        ) {
            (Some(l), Some(r)) => (l.index(), r.index()),
            _ => return false, // Non-Column expression -> not NDV-backed
        };

        // Check left side distinct_count
        if left_idx >= left_stats.column_statistics.len() {
            return false;
        }
        if left_stats.column_statistics[left_idx]
            .distinct_count
            .get_value()
            .is_none()
        {
            return false;
        }

        // Check right side distinct_count
        if right_idx >= right_stats.column_statistics.len() {
            return false;
        }
        if right_stats.column_statistics[right_idx]
            .distinct_count
            .get_value()
            .is_none()
        {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::physical_plan::EmptyExec;
    use datafusion::physical_plan::ExecutionPlan;
    use std::sync::Arc;

    #[test]
    fn empty_plan_returns_none() {
        let empty = Arc::new(EmptyExec::new(Arc::new(arrow_schema::Schema::empty())));
        assert_eq!(estimated_output_rows(&empty), None);
    }
}
