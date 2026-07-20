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

//! EXPLAIN output helpers shared by the DataFusion-backed custom scans.
//!
//! `format_expr_for_explain` and `get_attname_safe` operate on raw PostgreSQL
//! nodes/oids; `format_join_level_expr` formats a `JoinLevelExpr` whose
//! definition currently lives in `joinscan::build` (and which AggregateScan
//! consumes by absolute path). None of these helpers are join-specific in
//! shape — they live here so JoinScan and any future aggregate-on-join
//! predicate-tree EXPLAIN can both reach them.

use crate::postgres::customscan::explain::ExplainFormat;
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::joinscan::build::{JoinCSClause, JoinLevelExpr, RelationAlias};
use crate::postgres::deparse::node_to_string_fallback;
use datafusion::physical_plan::metrics::MetricValue;
use datafusion::physical_plan::{DisplayFormatType, ExecutionPlan};
use datafusion_distributed::{display_plan_ascii, DistributedExec};
use pgrx::pg_sys;
use std::sync::Arc;

/// Format a PostgreSQL expression node for EXPLAIN output.
/// Returns a human-readable description of the expression.
///
/// Note: This uses nodeToString which produces a debug representation.
/// For proper SQL deparsing with table/column names, we'd need a PlannerContext
/// which isn't available during EXPLAIN. The debug output is still useful for
/// understanding the expression structure.
pub unsafe fn format_expr_for_explain(node: *mut pg_sys::Node) -> String {
    node_to_string_fallback(node)
}

/// Get the column name for an attribute, with fallback to "relname.attno" if lookup fails.
pub fn get_attname_safe(
    heaprelid: Option<pg_sys::Oid>,
    attno: pg_sys::AttrNumber,
    rel_name: &str,
) -> String {
    let Some(oid) = heaprelid else {
        return format!("{}.{}", rel_name, attno);
    };

    unsafe {
        let attname_ptr = pg_sys::get_attname(oid, attno, true); // missing_ok = true
        if attname_ptr.is_null() {
            format!("{}.{}", rel_name, attno)
        } else {
            let attname = std::ffi::CStr::from_ptr(attname_ptr)
                .to_str()
                .unwrap_or("?");
            format!("{}.{}", rel_name, attname)
        }
    }
}

/// Format a join-level expression tree for EXPLAIN output.
pub fn format_join_level_expr(expr: &JoinLevelExpr, join_clause: &JoinCSClause) -> String {
    match expr {
        JoinLevelExpr::SingleTablePredicate {
            plan_position,
            predicate_idx,
        } => {
            let label = join_clause
                .plan
                .sources()
                .iter()
                .find(|source| source.plan_position == *plan_position)
                .map(|source| {
                    RelationAlias::new(source.scan_info.alias.as_deref())
                        .display(source.plan_position)
                })
                .unwrap_or_else(|| RelationAlias::new(None).display(*plan_position));
            if let Some(pred) = join_clause.join_level_predicates.get(*predicate_idx) {
                format!("{}:{}", label, pred.query.explain_format())
            } else {
                format!("{}:?", label)
            }
        }
        JoinLevelExpr::MultiTablePredicate { predicate_idx } => {
            if let Some(cond) = join_clause.multi_table_predicates.get(*predicate_idx) {
                format!("heap:{}", cond.description)
            } else {
                "heap:?".to_string()
            }
        }
        JoinLevelExpr::And(children) => {
            let parts: Vec<_> = children
                .iter()
                .map(|c| format_join_level_expr(c, join_clause))
                .collect();
            if parts.len() == 1 {
                parts.into_iter().next().unwrap()
            } else {
                format!("({})", parts.join(" AND "))
            }
        }
        JoinLevelExpr::Or(children) => {
            let parts: Vec<_> = children
                .iter()
                .map(|c| format_join_level_expr(c, join_clause))
                .collect();
            if parts.len() == 1 {
                parts.into_iter().next().unwrap()
            } else {
                format!("({})", parts.join(" OR "))
            }
        }
        JoinLevelExpr::Not(child) => {
            format!("NOT {}", format_join_level_expr(child, join_clause))
        }
        JoinLevelExpr::MarkOrNull { is_anti, .. } => {
            if *is_anti {
                "(mark = false OR col IS NULL)".to_string()
            } else {
                "(mark = true OR col IS NULL)".to_string()
            }
        }
        JoinLevelExpr::PgExpression { pg_node_string, .. } => unsafe {
            let Ok(c_str) = std::ffi::CString::new(pg_node_string.as_str()) else {
                return pg_node_string.clone();
            };
            let node = pg_sys::stringToNode(c_str.as_ptr().cast_mut());
            if node.is_null() {
                return pg_node_string.clone();
            }
            format_expr_for_explain(node.cast())
        },
    }
}

/// Recursively formats a DataFusion physical plan as a string, appending
/// collected metrics.  When `include_timing` is false, timing metrics
/// (`elapsed_compute`, named `Time` values) are stripped so that regression
/// test output remains stable.  Pass `true` (e.g. for EXPLAIN ANALYZE VERBOSE)
/// to include everything.
pub fn render_plan_with_metrics(
    plan: &dyn ExecutionPlan,
    indent: usize,
    include_timing: bool,
    lines: &mut Vec<String>,
) {
    use std::fmt::Write;

    let mut line = format!("{:indent$}", "", indent = indent * 2);

    struct Fmt<'a>(&'a dyn ExecutionPlan);
    impl std::fmt::Display for Fmt<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            self.0.fmt_as(DisplayFormatType::Default, f)
        }
    }
    write!(line, "{}", Fmt(plan)).unwrap();

    if let Some(metrics) = plan.metrics() {
        let aggregated = metrics
            .aggregate_by_name()
            .sorted_for_display()
            .timestamps_removed();
        let parts: Vec<String> = aggregated
            .iter()
            .filter(|m| {
                include_timing
                    || !matches!(
                        m.value(),
                        MetricValue::ElapsedCompute(_) | MetricValue::Time { .. }
                    )
            })
            .map(|m| m.to_string())
            .collect();
        if !parts.is_empty() {
            write!(line, ", metrics=[{}]", parts.join(", ")).unwrap();
        }
    }

    lines.push(line);
    for child in plan.children() {
        render_plan_with_metrics(child.as_ref(), indent + 1, include_timing, lines);
    }
}

/// Renders a physical plan for EXPLAIN.
/// For EXPLAIN ANALYZE, renders with metrics. For plain EXPLAIN, renders with
/// ASCII boxes for DistributedExec or a tree format for non-distributed plans.
pub fn explain_physical_plan(plan: Arc<dyn ExecutionPlan>, explainer: &mut Explainer) {
    explainer.add_text("DataFusion Physical Plan", "");
    if explainer.is_analyze() {
        let mut lines = Vec::new();
        render_plan_with_metrics(plan.as_ref(), 0, explainer.is_verbose(), &mut lines);
        for line in lines {
            explainer.add_text("  ", line);
        }
    } else {
        let rendered = if plan.is::<DistributedExec>() {
            display_plan_ascii(plan.as_ref(), false)
        } else {
            datafusion::physical_plan::displayable(plan.as_ref())
                .indent(false)
                .to_string()
        };
        for line in rendered.lines() {
            explainer.add_text("  ", line);
        }
    }
}
