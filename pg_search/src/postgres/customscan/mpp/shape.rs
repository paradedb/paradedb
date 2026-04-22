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

//! MPP plan-shape classifier.
//!
//! Classifies a query (at planning time) into one of a handful of MPP
//! topology shapes so `aggregatescan` / `joinscan` can decide whether the
//! plan is MPP-eligible and, if so, pick the right DSM mesh allocation size.
//! The actual plan-rewriting lives in [`super::walker::distribute_plan`],
//! which derives the same shape from plan structure and uses the
//! classifier's output only as a sanity cross-check.
//!
//! # Supported shapes
//!
//! [`MppPlanShape::ScalarAggOnBinaryJoin`] — e.g., `SELECT COUNT(*) FROM f
//! JOIN p WHERE ...`. Per worker: pre-join shuffle of each side → HashJoin →
//! AggregateExec(Partial). Partial rows are shipped to seat 0 and finalized
//! by a single `AggregateExec(FinalPartitioned)` on the leader; workers
//! emit zero rows so PG's Gather sees exactly one row per query.
//!
//! [`MppPlanShape::GroupByAggOnBinaryJoin`] — e.g., `SELECT k, COUNT(*) FROM
//! f JOIN p GROUP BY k`. Per worker: pre-join shuffle → HashJoin →
//! AggregateExec(Partial) → post-Partial shuffle on group keys →
//! AggregateExec(FinalPartitioned). Each worker emits final rows for its
//! group-key hash partition; PG's Gather concatenates — no double-count
//! because every group lives on exactly one worker.
//!
//! [`MppPlanShape::GroupByAggSingleTable`] — `SELECT k, COUNT(*) FROM t
//! GROUP BY k`. No join; Partial → post-Partial shuffle → FinalPartitioned.
//! Not yet plumbed through the walker; classifier accepts but the walker
//! returns `DataFusionError::Plan`.
//!
//! [`MppPlanShape::JoinOnly`] — `SELECT ... FROM f JOIN p`. Pre-join shuffle
//! → HashJoin → emit rows (no aggregate).
//!
//! [`MppPlanShape::Ineligible`] — fall back to the non-MPP serial path.

/// Classify a query so the dispatcher picks the right topology.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MppPlanShape {
    ScalarAggOnBinaryJoin,
    GroupByAggOnBinaryJoin,
    GroupByAggSingleTable,
    JoinOnly,
    Ineligible,
}

/// Shape-classification inputs. Kept as plain fields rather than a reference
/// to a larger state so callers can construct it from whatever they have
/// (RelNode walk, AggregateCSClause inspection, test synthetic data).
pub struct ClassifyInputs {
    /// Number of tables in the join. 0 = no join, 1 = single table,
    /// 2 = binary join, >=3 = multi-table join.
    pub n_join_tables: usize,
    /// True if the query has a GROUP BY clause with at least one column.
    pub has_group_by: bool,
    /// True if every aggregate function in the targetlist is one of
    /// COUNT/SUM/MIN/MAX/AVG/BOOL_*/STDDEV_*/VAR_* — the set with a safe
    /// Partial/Final split. `false` for COUNT(DISTINCT), ARRAY_AGG,
    /// STRING_AGG, ordered-set, hypothetical-set aggregates.
    pub all_aggregates_splittable: bool,
    /// True if the query has at least one aggregate (COUNT, SUM, …). When
    /// `false`, we're classifying a join-only shape.
    pub has_aggregate: bool,
}

/// Classify the query into an [`MppPlanShape`].
///
/// Rules (all implicitly `AND`-ed):
///   * Two tables + has_aggregate + splittable: binary-join aggregate.
///     Split further on `has_group_by` to pick scalar vs group-by topology.
///   * One table + has_aggregate + group_by + splittable:
///     single-table group-by aggregate (post-Partial shuffle helps when
///     cardinality is high).
///   * Two tables + no_aggregate: join-only.
///   * Otherwise: Ineligible.
pub fn classify(inputs: &ClassifyInputs) -> MppPlanShape {
    if !inputs.all_aggregates_splittable && inputs.has_aggregate {
        return MppPlanShape::Ineligible;
    }
    match (
        inputs.n_join_tables,
        inputs.has_aggregate,
        inputs.has_group_by,
    ) {
        (2, true, false) => MppPlanShape::ScalarAggOnBinaryJoin,
        (2, true, true) => MppPlanShape::GroupByAggOnBinaryJoin,
        (1, true, true) => MppPlanShape::GroupByAggSingleTable,
        (2, false, _) => MppPlanShape::JoinOnly,
        _ => MppPlanShape::Ineligible,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs(n: usize, agg: bool, gb: bool, splittable: bool) -> ClassifyInputs {
        ClassifyInputs {
            n_join_tables: n,
            has_group_by: gb,
            all_aggregates_splittable: splittable,
            has_aggregate: agg,
        }
    }

    #[test]
    fn binary_scalar_agg() {
        assert_eq!(
            classify(&inputs(2, true, false, true)),
            MppPlanShape::ScalarAggOnBinaryJoin
        );
    }

    #[test]
    fn binary_groupby_agg() {
        assert_eq!(
            classify(&inputs(2, true, true, true)),
            MppPlanShape::GroupByAggOnBinaryJoin
        );
    }

    #[test]
    fn single_groupby() {
        assert_eq!(
            classify(&inputs(1, true, true, true)),
            MppPlanShape::GroupByAggSingleTable
        );
    }

    #[test]
    fn join_only() {
        assert_eq!(
            classify(&inputs(2, false, false, true)),
            MppPlanShape::JoinOnly
        );
    }

    #[test]
    fn non_splittable_aggregate_is_ineligible() {
        assert_eq!(
            classify(&inputs(2, true, false, false)),
            MppPlanShape::Ineligible
        );
    }

    #[test]
    fn three_table_join_is_ineligible() {
        assert_eq!(
            classify(&inputs(3, true, false, true)),
            MppPlanShape::Ineligible
        );
    }
}
