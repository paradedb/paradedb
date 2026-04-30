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
//! Classifies a logical plan into a topology shape so `aggregatescan` /
//! `joinscan` can decide MPP eligibility and DSM mesh size. The walker
//! ([`super::walker::distribute_plan`]) re-derives the same shape from
//! plan structure and uses the classifier's output only as a cross-check.
//!
//! Shapes:
//!
//! * [`MppPlanShape::ScalarAggOnBinaryJoin`] — `COUNT(*) FROM f JOIN p`.
//!   Per worker: shuffle → HashJoin → Partial. Partials gather to the
//!   leader's `FinalPartitioned`; workers emit zero rows.
//! * [`MppPlanShape::GroupByAggOnBinaryJoin`] — `SELECT k, COUNT(*) … GROUP BY k`.
//!   Per worker: shuffle → HashJoin → Partial → group-key shuffle →
//!   `FinalPartitioned`. Each worker emits its hash partition's groups.
//! * [`MppPlanShape::GroupByAggSingleTable`] — single-table GROUP BY.
//!   Classified but not yet plumbed through the walker.
//! * [`MppPlanShape::JoinOnly`] — bare join, no aggregate.
//! * [`MppPlanShape::Ineligible`] — non-MPP serial fallback.

use crate::scan::info::RowEstimate;

/// MPP topology shape. Note `TopKGroupByAggOnBinaryJoin` is never produced
/// by [`classify`] — the logical plan doesn't reveal whether DataFusion's
/// physical planner will fuse `Sort[fetch=k]` over the aggregate. The
/// walker's dispatcher upgrades to it when the physical plan shows that
/// fusion; the variant lives here so the rest of the chain can `match` on
/// one enum.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MppPlanShape {
    ScalarAggOnBinaryJoin,
    GroupByAggOnBinaryJoin,
    /// `GroupByAggOnBinaryJoin` + outer `ORDER BY agg LIMIT k`. Topology is
    /// scalar-style: post-Partial gathers every participant to participant 0
    /// (`FixedTargetPartitioner(0)`); the leader wraps with
    /// `FinalPartitioned + SortExec[fetch=k]` to resolve the global Top-K.
    /// Workers emit zero rows.
    TopKGroupByAggOnBinaryJoin,
    GroupByAggSingleTable,
    JoinOnly,
    Ineligible,
}

impl MppPlanShape {
    /// Shapes that shuffle both sides of a binary join, hence have a
    /// "broadcast candidate" (the smaller side) where a row-estimate cost
    /// gate makes sense — if the smaller side is small, broadcast-join
    /// beats MPP. Single-table shapes have no candidate and skip the gate.
    pub fn is_binary_join(&self) -> bool {
        matches!(
            self,
            MppPlanShape::JoinOnly
                | MppPlanShape::ScalarAggOnBinaryJoin
                | MppPlanShape::GroupByAggOnBinaryJoin
        )
    }
}

/// Broadcast-side MPP cost gate. Returns `Some(n)` (caller skips MPP) when
/// the smallest known-side estimate is below `min_rows`; `None` when the
/// gate is disabled (`min_rows <= 0`), no side has a `Known` estimate
/// (un-ANALYZE'd tables shouldn't silently bypass MPP), or the smallest
/// known estimate meets the threshold. Only meaningful for binary joins.
pub fn broadcast_side_gate(estimates: &[RowEstimate], min_rows: i32) -> Option<u64> {
    if min_rows <= 0 {
        return None;
    }
    let threshold = min_rows as u64;
    let smallest_known = estimates
        .iter()
        .filter_map(|e| match e {
            RowEstimate::Known(n) => Some(*n),
            RowEstimate::Unknown => None,
        })
        .min()?;
    (smallest_known < threshold).then_some(smallest_known)
}

/// Shape-classification inputs. Kept as plain fields rather than a reference
/// to a larger state so callers can construct it from whatever they have
/// (RelNode walk, AggregateCSClause inspection, test synthetic data).
#[derive(Debug, Clone)]
pub struct ClassifyInputs {
    /// Number of tables in the join. 0 = no join, 1 = single table,
    /// 2 = binary join, >=3 = multi-table join.
    pub n_join_tables: usize,
    /// True if the query has at least one GROUP BY expression the MPP
    /// shuffle can hash-partition on. Callers must exclude group-by
    /// expressions whose value is not stable across workers (volatile
    /// functions, session-dependent casts) — those break the "each group
    /// lives on exactly one worker" invariant.
    pub has_group_by: bool,
    /// True if every aggregate function in the targetlist is one of
    /// COUNT/SUM/MIN/MAX/AVG/BOOL_*/STDDEV_*/VAR_* — the set with a safe
    /// Partial/Final split. `false` for COUNT(DISTINCT), ARRAY_AGG,
    /// STRING_AGG, ordered-set, hypothetical-set aggregates. When
    /// `has_aggregate=false`, this field is irrelevant and callers
    /// conventionally pass `true`.
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

    #[test]
    fn is_binary_join_covers_all_join_shapes() {
        assert!(MppPlanShape::JoinOnly.is_binary_join());
        assert!(MppPlanShape::ScalarAggOnBinaryJoin.is_binary_join());
        assert!(MppPlanShape::GroupByAggOnBinaryJoin.is_binary_join());
        assert!(!MppPlanShape::GroupByAggSingleTable.is_binary_join());
        assert!(!MppPlanShape::Ineligible.is_binary_join());
    }

    mod broadcast_gate {
        use super::super::broadcast_side_gate;
        use crate::scan::info::RowEstimate;

        #[test]
        fn skips_mpp_when_smallest_side_below_threshold() {
            let est = [RowEstimate::Known(500), RowEstimate::Known(1_000_000)];
            assert_eq!(broadcast_side_gate(&est, 10_000), Some(500));
        }

        #[test]
        fn allows_mpp_when_all_sides_meet_threshold() {
            let est = [RowEstimate::Known(50_000), RowEstimate::Known(1_000_000)];
            assert_eq!(broadcast_side_gate(&est, 10_000), None);
        }

        #[test]
        fn allows_mpp_at_exact_threshold() {
            let est = [RowEstimate::Known(10_000), RowEstimate::Known(1_000_000)];
            assert_eq!(broadcast_side_gate(&est, 10_000), None);
        }

        #[test]
        fn disabled_when_threshold_is_zero() {
            let est = [RowEstimate::Known(1), RowEstimate::Known(2)];
            assert_eq!(broadcast_side_gate(&est, 0), None);
        }

        #[test]
        fn disabled_when_threshold_is_negative() {
            let est = [RowEstimate::Known(1)];
            assert_eq!(broadcast_side_gate(&est, -1), None);
        }

        #[test]
        fn allows_mpp_when_no_known_estimates() {
            // Un-ANALYZE'd tables: don't silently gate MPP off.
            let est = [RowEstimate::Unknown, RowEstimate::Unknown];
            assert_eq!(broadcast_side_gate(&est, 10_000), None);
        }

        #[test]
        fn ignores_unknown_when_some_sides_known() {
            let est = [RowEstimate::Unknown, RowEstimate::Known(100)];
            assert_eq!(broadcast_side_gate(&est, 10_000), Some(100));
        }

        #[test]
        fn empty_estimates_returns_none() {
            assert_eq!(broadcast_side_gate(&[], 10_000), None);
        }
    }
}
