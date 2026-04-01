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

//! Physical optimizer rule that replaces `SortExec → GlobalLimitExec` (or
//! `SortExec(fetch=K)`) above an `AggregateExec` with a single
//! [`TopKAggregateExec`] node.
//!
//! This avoids a full sort of aggregate results when only the top-K groups
//! are needed, using partial sort (`select_nth_unstable_by`) instead.
//!
//! # Plan Transformation
//!
//! ```text
//! Pattern 1 (limit pushed into sort):
//!   SortExec(fetch=K, sort=[agg_col DESC])
//!     └─ AggregateExec(...)
//!   =>
//!   TopKAggregateExec(k=K, sort=[agg_col DESC])
//!     └─ AggregateExec(...)
//!
//! Pattern 2 (separate limit):
//!   GlobalLimitExec(skip=0, fetch=K)
//!     └─ SortExec(sort=[agg_col DESC])
//!          └─ AggregateExec(...)
//!   =>
//!   TopKAggregateExec(k=K, sort=[agg_col DESC])
//!     └─ AggregateExec(...)
//! ```

use std::sync::Arc;

use datafusion::common::config::ConfigOptions;
use datafusion::common::tree_node::{Transformed, TreeNode};
use datafusion::common::Result;
use datafusion::physical_optimizer::PhysicalOptimizerRule;
use datafusion::physical_plan::aggregates::AggregateExec;
use datafusion::physical_plan::limit::GlobalLimitExec;
use datafusion::physical_plan::sorts::sort::SortExec;
use datafusion::physical_plan::ExecutionPlan;

use crate::scan::topk_aggregate_exec::TopKAggregateExec;

/// Physical optimizer rule that fuses sort + limit into a TopK selection
/// when the input is an aggregate.
#[derive(Debug)]
pub struct TopKAggregateRule;

impl PhysicalOptimizerRule for TopKAggregateRule {
    fn name(&self) -> &str {
        "TopKAggregate"
    }

    fn schema_check(&self) -> bool {
        true
    }

    fn optimize(
        &self,
        plan: Arc<dyn ExecutionPlan>,
        _config: &ConfigOptions,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        plan.transform_up(|node| {
            // Pattern 1: GlobalLimitExec → SortExec → ... AggregateExec ...
            if let Some(limit_exec) = node.as_any().downcast_ref::<GlobalLimitExec>() {
                if let Some(fetch) = limit_exec.fetch() {
                    let skip = limit_exec.skip();
                    let total_k = skip + fetch;
                    let child = limit_exec.input();

                    if let Some(sort_exec) = child.as_any().downcast_ref::<SortExec>() {
                        let sort_input = sort_exec.input();
                        if has_aggregate_as_child(sort_input) {
                            let sort_exprs = sort_exec.expr().clone();
                            let topk =
                                TopKAggregateExec::new(Arc::clone(sort_input), sort_exprs, total_k);
                            if skip > 0 {
                                // Keep GlobalLimitExec for offset handling
                                let new_limit =
                                    GlobalLimitExec::new(Arc::new(topk), skip, Some(fetch));
                                return Ok(Transformed::yes(
                                    Arc::new(new_limit) as Arc<dyn ExecutionPlan>
                                ));
                            }
                            return Ok(Transformed::yes(Arc::new(topk) as Arc<dyn ExecutionPlan>));
                        }
                    }
                }
            }

            // Pattern 2: SortExec(fetch=K) → ... AggregateExec ...
            if let Some(sort_exec) = node.as_any().downcast_ref::<SortExec>() {
                if let Some(k) = sort_exec.fetch() {
                    let sort_input = sort_exec.input();
                    if has_aggregate_as_child(sort_input) {
                        let sort_exprs = sort_exec.expr().clone();
                        let topk = TopKAggregateExec::new(Arc::clone(sort_input), sort_exprs, k);
                        return Ok(Transformed::yes(Arc::new(topk) as Arc<dyn ExecutionPlan>));
                    }
                }
            }

            Ok(Transformed::no(node))
        })
        .map(|t| t.data)
    }
}

/// Check if the immediate child (or child behind a CoalescePartitionsExec) is an `AggregateExec`.
///
/// Only matches direct parent-child relationships to avoid incorrectly fusing
/// a sort+limit with an aggregate deep inside a join or subquery.
fn has_aggregate_as_child(plan: &Arc<dyn ExecutionPlan>) -> bool {
    if plan.as_any().downcast_ref::<AggregateExec>().is_some() {
        return true;
    }
    // Look through CoalescePartitionsExec which DataFusion inserts between
    // partitioned AggregateExec and the sort.
    if plan
        .as_any()
        .downcast_ref::<datafusion::physical_plan::coalesce_partitions::CoalescePartitionsExec>()
        .is_some()
    {
        return plan
            .children()
            .first()
            .map(|c| c.as_any().downcast_ref::<AggregateExec>().is_some())
            .unwrap_or(false);
    }
    false
}
