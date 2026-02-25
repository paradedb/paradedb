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

//! Physical optimizer rule that injects `SegmentedTopKExec` below
//! `TantivyLookupExec` when a `SortExec(TopK)` sorts by a deferred
//! (late-materialized) string/bytes column.
//!
//! # Plan Transformation
//!
//! ```text
//! BEFORE:
//!   SortExec(fetch=K, sort=[val ASC])
//!     └─ TantivyLookupExec(decode=[val])
//!          └─ Child
//!
//! AFTER:
//!   SortExec(fetch=K, sort=[val ASC])
//!     └─ TantivyLookupExec(decode=[val])
//!          └─ SegmentedTopKExec(col=val, k=K, ASC)
//!               └─ Child
//! ```
//!
//! The rule walks the plan tree top-down. When it finds a `SortExec` with
//! `fetch` (TopK mode), it searches its descendants for a `TantivyLookupExec`.
//! If the primary sort key matches one of the deferred string/bytes fields,
//! it injects a `SegmentedTopKExec` as the new child of `TantivyLookupExec`.

use std::sync::Arc;

use datafusion::common::config::ConfigOptions;
use datafusion::common::Result;
use datafusion::physical_expr::expressions::Column;
use datafusion::physical_optimizer::PhysicalOptimizerRule;
use datafusion::physical_plan::sorts::sort::SortExec;
use datafusion::physical_plan::ExecutionPlan;

use crate::gucs;
use crate::scan::segmented_topk_exec::SegmentedTopKExec;
use crate::scan::tantivy_lookup_exec::TantivyLookupExec;

#[derive(Debug)]
pub struct SegmentedTopKRule;

impl PhysicalOptimizerRule for SegmentedTopKRule {
    fn name(&self) -> &str {
        "SegmentedTopK"
    }

    fn schema_check(&self) -> bool {
        false
    }

    fn optimize(
        &self,
        plan: Arc<dyn ExecutionPlan>,
        _config: &ConfigOptions,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        if !gucs::enable_segmented_topk() {
            return Ok(plan);
        }
        rewrite_plan(plan)
    }
}

/// Recursively rewrite the plan tree, injecting `SegmentedTopKExec` below
/// `TantivyLookupExec` when a `SortExec(TopK)` sorts by a deferred column.
fn rewrite_plan(plan: Arc<dyn ExecutionPlan>) -> Result<Arc<dyn ExecutionPlan>> {
    // First, recursively rewrite all children.
    let children = plan.children();
    if !children.is_empty() {
        let mut new_children = Vec::with_capacity(children.len());
        let mut children_changed = false;
        for child in &children {
            let new_child = rewrite_plan(Arc::clone(child))?;
            if !Arc::ptr_eq(child, &new_child) {
                children_changed = true;
            }
            new_children.push(new_child);
        }
        let plan = if children_changed {
            plan.with_new_children(new_children)?
        } else {
            plan
        };
        return try_inject_at_sort(plan);
    }

    Ok(plan)
}

/// If `plan` is a `SortExec(TopK)` sorting by a deferred column, inject
/// `SegmentedTopKExec` below the `TantivyLookupExec` in its subtree.
fn try_inject_at_sort(plan: Arc<dyn ExecutionPlan>) -> Result<Arc<dyn ExecutionPlan>> {
    let Some(sort_exec) = plan.as_any().downcast_ref::<SortExec>() else {
        return Ok(plan);
    };

    let Some(k) = sort_exec.fetch() else {
        return Ok(plan);
    };

    // Extract the primary sort key's column name and direction.
    let sort_exprs = sort_exec.expr();
    let first_sort = &sort_exprs[0];
    let Some(col) = first_sort.expr.as_any().downcast_ref::<Column>() else {
        return Ok(plan);
    };
    let sort_col_name = col.name();
    let descending = first_sort.options.descending;

    // Walk down from SortExec to find TantivyLookupExec.
    match try_inject_below_lookup(&plan, sort_col_name, k, descending)? {
        Some(rewritten) => Ok(rewritten),
        None => Ok(plan),
    }
}

/// Recursively search below `plan` for a `TantivyLookupExec` whose deferred
/// fields include `sort_col_name`. If found, inject a `SegmentedTopKExec`
/// as its new child and rebuild the plan tree up to `plan`.
fn try_inject_below_lookup(
    plan: &Arc<dyn ExecutionPlan>,
    sort_col_name: &str,
    k: usize,
    descending: bool,
) -> Result<Option<Arc<dyn ExecutionPlan>>> {
    let children = plan.children();

    for (child_idx, child) in children.iter().enumerate() {
        if let Some(lookup) = child.as_any().downcast_ref::<TantivyLookupExec>() {
            // Check if the sort column is one of the deferred fields.
            let matching_field = lookup
                .deferred_fields()
                .iter()
                .find(|d| d.field_name == sort_col_name);

            if let Some(field) = matching_field {
                // Found a match. Inject SegmentedTopKExec below TantivyLookupExec.
                let lookup_child = &lookup.children()[0];

                // Resolve the sort column index in the pre-lookup schema.
                let input_schema = lookup_child.schema();
                let sort_col_idx = match input_schema.column_with_name(sort_col_name) {
                    Some((idx, _)) => idx,
                    None => return Ok(None),
                };

                let segmented_topk = Arc::new(SegmentedTopKExec::new(
                    Arc::clone(lookup_child),
                    sort_col_name.to_string(),
                    sort_col_idx,
                    field.ff_index,
                    Arc::clone(lookup.ffhelper()),
                    k,
                    descending,
                    field.is_bytes,
                ));

                // Rebuild TantivyLookupExec with the new child.
                let new_lookup = Arc::clone(child).with_new_children(vec![segmented_topk])?;

                // Rebuild the parent with the updated child.
                let mut new_children: Vec<Arc<dyn ExecutionPlan>> =
                    children.iter().map(|c| Arc::clone(c)).collect();
                new_children[child_idx] = new_lookup;
                return Ok(Some(plan.clone().with_new_children(new_children)?));
            }
        }

        // Recurse into intermediate nodes (ProjectionExec, CoalescePartitionsExec, etc.)
        if let Some(rewritten) = try_inject_below_lookup(child, sort_col_name, k, descending)? {
            let mut new_children: Vec<Arc<dyn ExecutionPlan>> =
                children.iter().map(|c| Arc::clone(c)).collect();
            new_children[child_idx] = rewritten;
            return Ok(Some(plan.clone().with_new_children(new_children)?));
        }
    }

    Ok(None)
}
