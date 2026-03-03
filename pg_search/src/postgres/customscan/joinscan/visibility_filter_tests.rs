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

//! Unit tests for the VisibilityFilter optimizer rule.
//!
//! These tests live in a separate file (rather than inline `#[cfg(test)]`)
//! to keep the main `visibility_filter.rs` module focused on implementation.

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use std::collections::BTreeSet;

    use datafusion::common::Result;
    use datafusion::logical_expr::{col, lit, LogicalPlan, LogicalPlanBuilder};
    use datafusion::optimizer::optimizer::OptimizerContext;
    use datafusion::optimizer::OptimizerRule;
    use pgrx::pg_sys;
    use pgrx::prelude::*;

    use crate::postgres::customscan::joinscan::build::{JoinCSClause, JoinSource};
    use crate::postgres::customscan::joinscan::visibility_filter::{
        VisibilityFilterNode, VisibilityFilterOptimizerRule,
    };
    use crate::scan::ScanInfo;

    const TEST_RTI: pg_sys::Index = 1;

    fn make_rule() -> VisibilityFilterOptimizerRule {
        let test_heap_oid = pg_sys::Oid::from(42);
        let source = JoinSource {
            scan_info: ScanInfo {
                heap_rti: TEST_RTI,
                heaprelid: test_heap_oid,
                ..Default::default()
            },
        };
        VisibilityFilterOptimizerRule::new(JoinCSClause::new().add_source(source))
    }

    fn make_ctid_plan() -> Result<LogicalPlan> {
        LogicalPlanBuilder::values(vec![vec![lit(1_u64)]])?
            .project(vec![
                col("column1").alias(crate::scan::ctid_column_name(TEST_RTI))
            ])?
            .build()
    }

    fn count_visibility_nodes(plan: &LogicalPlan) -> usize {
        let current = match plan {
            LogicalPlan::Extension(ext)
                if ext
                    .node
                    .as_any()
                    .downcast_ref::<VisibilityFilterNode>()
                    .is_some() =>
            {
                1
            }
            _ => 0,
        };
        current
            + plan
                .inputs()
                .iter()
                .map(|child| count_visibility_nodes(child))
                .sum::<usize>()
    }

    fn limit_child_is_visibility(plan: &LogicalPlan) -> bool {
        let LogicalPlan::Limit(limit) = plan else {
            return false;
        };
        let LogicalPlan::Extension(ext) = limit.input.as_ref() else {
            return false;
        };
        ext.node
            .as_any()
            .downcast_ref::<VisibilityFilterNode>()
            .is_some()
    }

    /// Returns true if the first child of a barrier node is a VisibilityFilterNode.
    fn first_child_is_visibility(plan: &LogicalPlan) -> bool {
        let children = plan.inputs();
        let Some(child) = children.first() else {
            return false;
        };
        let LogicalPlan::Extension(ext) = child else {
            return false;
        };
        ext.node
            .as_any()
            .downcast_ref::<VisibilityFilterNode>()
            .is_some()
    }

    /// Helper to assert barrier injection + idempotency.
    fn assert_barrier_injection(plan: LogicalPlan) -> Result<()> {
        let config = OptimizerContext::new();
        let rule = make_rule();

        let first = rule.rewrite(plan, &config)?;
        assert!(first.transformed, "first pass should transform");
        assert_eq!(count_visibility_nodes(&first.data), 1);
        assert!(
            first_child_is_visibility(&first.data),
            "visibility should be inserted below barrier"
        );

        let second = rule.rewrite(first.data.clone(), &config)?;
        assert!(!second.transformed, "second pass should be idempotent");
        assert_eq!(count_visibility_nodes(&second.data), 1);
        assert_eq!(first.data, second.data);
        Ok(())
    }

    #[pg_test]
    fn root_injection_is_idempotent() -> Result<()> {
        let config = OptimizerContext::new();
        let rule = make_rule();
        let plan = make_ctid_plan()?;

        let first = rule.rewrite(plan, &config)?;
        assert!(first.transformed);
        assert_eq!(count_visibility_nodes(&first.data), 1);

        let second = rule.rewrite(first.data.clone(), &config)?;
        assert!(!second.transformed);
        assert_eq!(count_visibility_nodes(&second.data), 1);
        assert_eq!(first.data, second.data);
        Ok(())
    }

    #[pg_test]
    fn inserts_visibility_below_limit_barrier() -> Result<()> {
        let config = OptimizerContext::new();
        let rule = make_rule();
        let plan = LogicalPlanBuilder::from(make_ctid_plan()?)
            .limit(0, Some(5))?
            .build()?;

        let first = rule.rewrite(plan, &config)?;
        assert!(first.transformed);
        assert!(limit_child_is_visibility(&first.data));
        assert_eq!(count_visibility_nodes(&first.data), 1);

        let second = rule.rewrite(first.data.clone(), &config)?;
        assert!(!second.transformed);
        assert!(limit_child_is_visibility(&second.data));
        assert_eq!(count_visibility_nodes(&second.data), 1);
        assert_eq!(first.data, second.data);
        Ok(())
    }

    #[pg_test]
    fn inserts_visibility_below_aggregate_barrier() -> Result<()> {
        use datafusion::functions_aggregate::count::count;
        let plan = LogicalPlanBuilder::from(make_ctid_plan()?)
            .aggregate(
                Vec::<datafusion::logical_expr::Expr>::new(),
                vec![count(col(crate::scan::ctid_column_name(TEST_RTI)))],
            )?
            .build()?;
        assert_barrier_injection(plan)
    }

    #[pg_test]
    fn inserts_visibility_below_distinct_barrier() -> Result<()> {
        let plan = LogicalPlanBuilder::from(make_ctid_plan()?)
            .distinct()?
            .build()?;
        assert_barrier_injection(plan)
    }

    #[pg_test]
    fn inserts_visibility_below_sort_with_fetch_barrier() -> Result<()> {
        let plan = LogicalPlanBuilder::from(make_ctid_plan()?)
            .sort_with_limit(
                vec![col(crate::scan::ctid_column_name(TEST_RTI)).sort(true, false)],
                Some(10),
            )?
            .build()?;
        assert_barrier_injection(plan)
    }

    #[pg_test]
    fn sort_without_fetch_is_not_barrier() -> Result<()> {
        let config = OptimizerContext::new();
        let rule = make_rule();
        let plan = LogicalPlanBuilder::from(make_ctid_plan()?)
            .sort(vec![
                col(crate::scan::ctid_column_name(TEST_RTI)).sort(true, false)
            ])?
            .build()?;

        let result = rule.rewrite(plan, &config)?;
        assert!(result.transformed);
        // Visibility should be at root, not below the sort.
        assert_eq!(count_visibility_nodes(&result.data), 1);
        assert!(!first_child_is_visibility(&result.data));
        Ok(())
    }

    #[pg_test]
    fn multi_relation_join() -> Result<()> {
        let config = OptimizerContext::new();

        const RTI_A: pg_sys::Index = 1;
        const RTI_B: pg_sys::Index = 2;
        let oid_a = pg_sys::Oid::from(42);
        let oid_b = pg_sys::Oid::from(43);

        let rule = VisibilityFilterOptimizerRule::new(
            JoinCSClause::new()
                .add_source(JoinSource {
                    scan_info: ScanInfo {
                        heap_rti: RTI_A,
                        heaprelid: oid_a,
                        ..Default::default()
                    },
                })
                .add_source(JoinSource {
                    scan_info: ScanInfo {
                        heap_rti: RTI_B,
                        heaprelid: oid_b,
                        ..Default::default()
                    },
                }),
        );

        // Build two leaf plans and join them (inner join = not a barrier).
        let left = LogicalPlanBuilder::values(vec![vec![lit(1_u64)]])?
            .project(vec![
                col("column1").alias(crate::scan::ctid_column_name(RTI_A))
            ])?
            .build()?;
        let right = LogicalPlanBuilder::values(vec![vec![lit(2_u64)]])?
            .project(vec![
                col("column1").alias(crate::scan::ctid_column_name(RTI_B))
            ])?
            .build()?;

        let plan = LogicalPlanBuilder::from(left).cross_join(right)?.build()?;

        let first = rule.rewrite(plan, &config)?;
        assert!(first.transformed);
        // Both RTIs should get visibility — single node at root covers both.
        assert_eq!(count_visibility_nodes(&first.data), 1);

        // Extract the VisibilityFilterNode and check it covers both RTIs.
        if let LogicalPlan::Extension(ext) = &first.data {
            let vf = ext
                .node
                .as_any()
                .downcast_ref::<VisibilityFilterNode>()
                .expect("root should be VisibilityFilterNode");
            let rtis: BTreeSet<pg_sys::Index> = vf.rti_oids.iter().map(|(r, _)| *r).collect();
            assert!(rtis.contains(&RTI_A));
            assert!(rtis.contains(&RTI_B));
        } else {
            panic!("expected root to be VisibilityFilterNode");
        }

        // Idempotent.
        let second = rule.rewrite(first.data.clone(), &config)?;
        assert!(!second.transformed);
        assert_eq!(count_visibility_nodes(&second.data), 1);
        Ok(())
    }

    #[pg_test]
    fn left_join_injects_per_side_visibility() -> Result<()> {
        let config = OptimizerContext::new();

        const RTI_A: pg_sys::Index = 1;
        const RTI_B: pg_sys::Index = 2;
        let oid_a = pg_sys::Oid::from(42);
        let oid_b = pg_sys::Oid::from(43);

        let rule = VisibilityFilterOptimizerRule::new(
            JoinCSClause::new()
                .add_source(JoinSource {
                    scan_info: ScanInfo {
                        heap_rti: RTI_A,
                        heaprelid: oid_a,
                        ..Default::default()
                    },
                })
                .add_source(JoinSource {
                    scan_info: ScanInfo {
                        heap_rti: RTI_B,
                        heaprelid: oid_b,
                        ..Default::default()
                    },
                }),
        );

        let left = LogicalPlanBuilder::values(vec![vec![lit(1_u64)]])?
            .project(vec![
                col("column1").alias(crate::scan::ctid_column_name(RTI_A))
            ])?
            .build()?;
        let right = LogicalPlanBuilder::values(vec![vec![lit(2_u64)]])?
            .project(vec![
                col("column1").alias(crate::scan::ctid_column_name(RTI_B))
            ])?
            .build()?;

        // LEFT join is a barrier — visibility must be injected per-side.
        let plan = LogicalPlanBuilder::from(left)
            .join_on(
                right,
                datafusion::common::JoinType::Left,
                vec![col(crate::scan::ctid_column_name(RTI_A))
                    .eq(col(crate::scan::ctid_column_name(RTI_B)))],
            )?
            .build()?;

        let first = rule.rewrite(plan, &config)?;
        assert!(first.transformed);
        // LEFT join is a barrier, so visibility filters are injected into
        // each child that has unverified RTIs — 2 total.
        assert_eq!(count_visibility_nodes(&first.data), 2);

        // Idempotent.
        let second = rule.rewrite(first.data.clone(), &config)?;
        assert!(!second.transformed);
        assert_eq!(count_visibility_nodes(&second.data), 2);
        Ok(())
    }
}
