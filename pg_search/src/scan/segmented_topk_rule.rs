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
//! `fetch` (Top K mode), it searches its descendants for a `TantivyLookupExec`.
//! If the primary sort key matches one of the deferred string/bytes fields,
//! it injects a `SegmentedTopKExec` as the new child of `TantivyLookupExec`.

use std::sync::Arc;

use datafusion::common::config::ConfigOptions;
use datafusion::common::Result;
use datafusion::physical_expr::expressions::Column;
use datafusion::physical_expr::{LexOrdering, PhysicalExpr, PhysicalSortExpr};
use datafusion::physical_optimizer::PhysicalOptimizerRule;
use datafusion::physical_plan::sorts::sort::SortExec;
use datafusion::physical_plan::sorts::sort_preserving_merge::SortPreservingMergeExec;
use datafusion::physical_plan::ExecutionPlan;

use crate::gucs;
use crate::index::fast_fields_helper::FFHelper;
use crate::scan::execution_plan::PgSearchScanPlan;
use crate::scan::filter_passthrough_exec::FilterPassthroughExec;
use crate::scan::segmented_topk_exec::{SegmentedThresholds, SegmentedTopKExec};
use crate::scan::tantivy_lookup_exec::TantivyLookupExec;

#[derive(Debug)]
pub struct SegmentedTopKRule;

impl PhysicalOptimizerRule for SegmentedTopKRule {
    fn name(&self) -> &str {
        "SegmentedTopK"
    }

    fn schema_check(&self) -> bool {
        true
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

/// If `plan` is a `SortExec(TopK)` sorting by at least one deferred column, inject
/// `SegmentedTopKExec` below the `TantivyLookupExec` in its subtree.
fn try_inject_at_sort(plan: Arc<dyn ExecutionPlan>) -> Result<Arc<dyn ExecutionPlan>> {
    let Some(sort_exec) = plan.as_any().downcast_ref::<SortExec>() else {
        return Ok(plan);
    };

    let Some(k) = sort_exec.fetch() else {
        return Ok(plan);
    };

    let sort_exprs = sort_exec.expr();

    // Walk down from SortExec to find TantivyLookupExec.
    match try_inject_below_lookup(&plan, sort_exprs.clone(), k)? {
        Some(rewritten) => Ok(rewritten),
        None => Ok(plan),
    }
}

/// Recursively search below `plan` for a `TantivyLookupExec` whose deferred
/// fields include `sort_col_name`. If found, inject a `SegmentedTopKExec`
/// as its new child and rebuild the plan tree up to `plan`.
fn try_inject_below_lookup(
    plan: &Arc<dyn ExecutionPlan>,
    sort_exprs: LexOrdering,
    k: usize,
) -> Result<Option<Arc<dyn ExecutionPlan>>> {
    let children = plan.children();

    for (child_idx, child) in children.iter().enumerate() {
        if let Some(lookup) = child.as_any().downcast_ref::<TantivyLookupExec>() {
            // Check if ANY sort column is one of the deferred fields.
            // If so, we will inject SegmentedTopKExec and collect all deferred
            // fields in the sort expressions to pass them down.
            let has_deferred_sort_col = sort_exprs.iter().any(|expr| {
                if let Some(col) = expr.expr.as_any().downcast_ref::<Column>() {
                    lookup
                        .deferred_fields()
                        .iter()
                        .any(|d| d.col_idx == col.index())
                } else {
                    false
                }
            });

            if has_deferred_sort_col {
                let lookup_child = &lookup.children()[0];
                // Wrap blocking nodes (e.g. SortPreservingMergeExec) so that
                // the second FilterPushdown(Post) pass can push
                // SegmentedTopKExec's DynamicFilterPhysicalExpr down to PgSearchScan.
                let lookup_child = &wrap_blocking_nodes(Arc::clone(lookup_child))?;
                let input_schema = lookup_child.schema();

                // Collect all deferred columns found in the sort expressions.
                let mut deferred_columns = Vec::new();
                for expr in &sort_exprs {
                    if let Some(col) = expr.expr.as_any().downcast_ref::<Column>() {
                        if let Some(field) = lookup
                            .deferred_fields()
                            .iter()
                            .find(|d| d.col_idx == col.index())
                        {
                            deferred_columns.push(
                                crate::scan::segmented_topk_exec::DeferredSortColumn {
                                    sort_col_idx: col.index(),
                                    canonical: field.canonical.clone(),
                                },
                            );
                        }
                    }
                }

                // If the sort requires deferred columns from multiple different indexes (tables),
                // we cannot push the threshold down, because a single segment scanner cannot evaluate
                // the threshold across multiple tables (it only sees its own base table).
                // E.g. `ORDER BY f.title ASC, d.category DESC` is a multi-dimensional bound that
                // spans across the HashJoin. We must gracefully fall back to a standard SortExec.
                // TODO: Add support for SegmentedTopK executing the TopK, but without pushing down
                // thresholds: see https://github.com/paradedb/paradedb/issues/4347
                let first_indexrelid = deferred_columns.first().map(|d| d.canonical.indexrelid);
                if let Some(id) = first_indexrelid {
                    if deferred_columns
                        .iter()
                        .any(|d| d.canonical.indexrelid != id)
                    {
                        pgrx::warning!("SegmentedTopK: ORDER BY includes string columns from multiple tables, which is not currently supported. Falling back to default execution.");
                        return Ok(None);
                    }
                }

                let target_indexrelid = first_indexrelid.unwrap_or(0);
                let ffhelper = match lookup.ffhelper(target_indexrelid) {
                    Some(helper) => Arc::clone(helper),
                    None => return Ok(None),
                };

                // The sort_exprs were extracted from SortExec, which is evaluated against
                // a schema further up the plan (often after a ProjectionExec or AggregateExec).
                // We must rewrite the Column expressions to match the input_schema of this node.
                let mut rewritten_sort_exprs = Vec::with_capacity(sort_exprs.len());
                for sort_expr in &sort_exprs {
                    use datafusion::common::tree_node::{Transformed, TreeNode};
                    let rewritten_expr = sort_expr.expr.clone().transform(|node| {
                        if let Some(col) = node.as_any().downcast_ref::<Column>() {
                            if let Ok(new_idx) = input_schema.index_of(col.name()) {
                                let new_col = Column::new(col.name(), new_idx);
                                return Ok(Transformed::yes(
                                    Arc::new(new_col) as Arc<dyn PhysicalExpr>
                                ));
                            }
                        }
                        Ok(Transformed::no(node))
                    })?;
                    rewritten_sort_exprs.push(PhysicalSortExpr {
                        expr: rewritten_expr.data,
                        options: sort_expr.options,
                    });
                }

                let rewritten_lex_ordering =
                    LexOrdering::new(rewritten_sort_exprs).unwrap_or(sort_exprs.clone());

                let thresholds = Arc::new(SegmentedThresholds::new());

                let segmented_topk = Arc::new(SegmentedTopKExec::new(
                    Arc::clone(lookup_child),
                    rewritten_lex_ordering,
                    deferred_columns.clone(),
                    Arc::clone(&ffhelper),
                    k,
                    Arc::clone(&thresholds),
                ));

                // Wire thresholds to the PgSearchScanPlan in the subtree.
                // We match by both deferred column presence AND FFHelper identity to ensure
                // we only wire to the scan for the same relation (in joins,
                // multiple PgSearchScanPlans exist but only one shares the
                // same Arc<FFHelper> as the TantivyLookupExec).
                wire_thresholds_to_scan(
                    lookup_child.as_ref(),
                    &thresholds,
                    &deferred_columns,
                    &ffhelper,
                );

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
        if let Some(rewritten) = try_inject_below_lookup(child, sort_exprs.clone(), k)? {
            let mut new_children: Vec<Arc<dyn ExecutionPlan>> =
                children.iter().map(|c| Arc::clone(c)).collect();
            new_children[child_idx] = rewritten;
            return Ok(Some(plan.clone().with_new_children(new_children)?));
        }
    }

    Ok(None)
}

/// Recursively wrap `SortPreservingMergeExec` nodes with [`FilterPassthroughExec`]
/// so that dynamic filters from `SegmentedTopKExec` can be pushed through them
/// during the `FilterPushdown(Post)` pass.
///
/// Other DataFusion built-in nodes in the path (`ProjectionExec`, `CooperativeExec`)
/// already implement filter passthrough natively.
fn wrap_blocking_nodes(plan: Arc<dyn ExecutionPlan>) -> Result<Arc<dyn ExecutionPlan>> {
    let children = plan.children();
    if children.is_empty() {
        return Ok(plan);
    }

    let mut changed = false;
    let mut new_children = Vec::with_capacity(children.len());
    for child in &children {
        let new_child = wrap_blocking_nodes(Arc::clone(child))?;
        if !Arc::ptr_eq(child, &new_child) {
            changed = true;
        }
        new_children.push(new_child);
    }

    let plan = if changed {
        plan.with_new_children(new_children)?
    } else {
        plan
    };

    if plan.as_any().is::<SortPreservingMergeExec>() {
        return Ok(Arc::new(FilterPassthroughExec::new(plan)));
    }

    Ok(plan)
}

/// Walk the plan tree to find a `PgSearchScanPlan` and wire the shared
/// thresholds into it. This handles both direct children and plans behind
/// intermediate nodes like `SortPreservingMergeExec`.
///
/// Matching is done by both ensuring that at least one deferred sort column belongs
/// to the scan's deferred fields, and `expected_ffhelper` (`Arc` pointer equality ensures we
/// wire to the scan for the same relation, not a different table in a join).
fn wire_thresholds_to_scan(
    plan: &dyn ExecutionPlan,
    thresholds: &Arc<SegmentedThresholds>,
    deferred_columns: &[crate::scan::segmented_topk_exec::DeferredSortColumn],
    expected_ffhelper: &Arc<FFHelper>,
) {
    if let Some(scan) = plan.as_any().downcast_ref::<PgSearchScanPlan>() {
        let same_relation = scan
            .ffhelper_if_deferred()
            .is_some_and(|ff| Arc::ptr_eq(ff, expected_ffhelper));
        // Check if ANY deferred column in the sort expressions belongs to this scan
        let has_target = deferred_columns.iter().any(|sort_col| {
            scan.deferred_fields()
                .iter()
                .any(|d| d.canonical == sort_col.canonical)
        });
        if same_relation && has_target {
            scan.set_segmented_thresholds(Arc::clone(thresholds));
        }
        return;
    }
    for child in plan.children() {
        wire_thresholds_to_scan(
            child.as_ref(),
            thresholds,
            deferred_columns,
            expected_ffhelper,
        );
    }
}
