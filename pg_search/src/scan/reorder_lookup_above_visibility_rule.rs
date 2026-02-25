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

//! Physical optimizer rule that moves `TantivyLookupExec` above `VisibilityFilterExec`.
//!
//! After `LateMaterializationRule` runs, the plan may look like:
//!
//! ```text
//! SortExec
//!   └─ VisibilityFilterExec
//!        └─ TantivyLookupExec(text)
//!             └─ HashJoinExec
//! ```
//!
//! This wastes dictionary lookups on rows that will be filtered out by visibility.
//! This rule swaps the order so visibility filtering happens first:
//!
//! ```text
//! SortExec
//!   └─ TantivyLookupExec(text)
//!        └─ VisibilityFilterExec
//!             └─ HashJoinExec
//! ```

use std::sync::Arc;

use datafusion::common::config::ConfigOptions;
use datafusion::common::Result;
use datafusion::physical_optimizer::PhysicalOptimizerRule;
use datafusion::physical_plan::ExecutionPlan;

use crate::postgres::customscan::joinscan::visibility_filter::VisibilityFilterExec;
use crate::scan::tantivy_lookup_exec::TantivyLookupExec;

#[derive(Debug)]
pub struct ReorderLookupAboveVisibilityRule;

impl PhysicalOptimizerRule for ReorderLookupAboveVisibilityRule {
    fn name(&self) -> &str {
        "ReorderLookupAboveVisibility"
    }

    fn schema_check(&self) -> bool {
        // Disabled because TantivyLookupExec changes column types
        // (UnionArray → Utf8View/BinaryView).
        false
    }

    fn optimize(
        &self,
        plan: Arc<dyn ExecutionPlan>,
        _config: &ConfigOptions,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        reorder(plan)
    }
}

fn reorder(plan: Arc<dyn ExecutionPlan>) -> Result<Arc<dyn ExecutionPlan>> {
    // Recurse into children first (bottom-up)
    let children = plan.children();
    let new_children = children
        .into_iter()
        .map(|c| reorder(Arc::clone(c)))
        .collect::<Result<Vec<_>>>()?;
    let plan = if new_children.is_empty() {
        plan
    } else {
        plan.with_new_children(new_children)?
    };

    // Only act on VisibilityFilterExec
    if plan
        .as_any()
        .downcast_ref::<VisibilityFilterExec>()
        .is_none()
    {
        return Ok(plan);
    }

    // Collect chain of TantivyLookupExec(s) immediately below.
    // This swap is safe because TantivyLookupExec only replaces UnionArray columns
    // with materialized text/bytes — it never adds, removes, or reorders rows.
    // It therefore commutes with VisibilityFilterExec (which only removes rows).
    let mut lookups = Vec::new();
    let mut current = Arc::clone(plan.children()[0]);
    while current
        .as_any()
        .downcast_ref::<TantivyLookupExec>()
        .is_some()
    {
        let child = Arc::clone(current.children()[0]);
        lookups.push(current);
        current = child;
    }

    if lookups.is_empty() {
        return Ok(plan);
    }

    // Swap: VisibilityFilterExec wraps the node below all lookups
    let new_vis = plan.with_new_children(vec![current])?;

    // Stack TantivyLookupExecs on top (reverse to preserve original order)
    let mut result = new_vis;
    for lookup in lookups.into_iter().rev() {
        result = lookup.with_new_children(vec![result])?;
    }
    Ok(result)
}
