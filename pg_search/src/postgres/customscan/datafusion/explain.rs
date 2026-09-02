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
//! `get_attname_safe` operates on raw PostgreSQL relation OIDs and attribute numbers;
//! `format_join_level_expr` formats a `JoinLevelExpr` for JoinScan EXPLAIN output,
//! using PostgreSQL's active `ExplainState` deparsing context when available.

use crate::postgres::customscan::explain::ExplainFormat;
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::joinscan::build::{JoinCSClause, JoinLevelExpr, RelationAlias};
use datafusion::physical_plan::metrics::MetricValue;
use datafusion::physical_plan::{DisplayFormatType, ExecutionPlan};
use datafusion_distributed::{DistributedExec, display_plan_ascii};
use pgrx::pg_sys;
use std::sync::Arc;

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
pub fn format_join_level_expr(
    expr: &JoinLevelExpr,
    join_clause: &JoinCSClause,
    explainer: Option<&Explainer>,
) -> String {
    match expr {
        JoinLevelExpr::SingleTablePredicate {
            plan_position,
            predicate,
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
            format!("{}:{}", label, predicate.query.explain_format())
        }
        JoinLevelExpr::MultiTablePredicate { predicate } => unsafe {
            let Ok(c_str) = std::ffi::CString::new(predicate.pg_node_string.as_str()) else {
                return format!("heap:{}", predicate.pg_node_string);
            };
            let node = pg_sys::stringToNode(c_str.as_ptr().cast_mut());
            if node.is_null() {
                return format!("heap:{}", predicate.pg_node_string);
            }
            if let Some(explainer) = explainer
                && let Some(deparsed) = explainer.deparse_expr(node.cast())
            {
                return format!("heap:{}", deparsed);
            }
            format!("heap:{}", predicate.pg_node_string)
        },
        JoinLevelExpr::And(children) => {
            let parts: Vec<_> = children
                .iter()
                .map(|c| format_join_level_expr(c, join_clause, explainer))
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
                .map(|c| format_join_level_expr(c, join_clause, explainer))
                .collect();
            if parts.len() == 1 {
                parts.into_iter().next().unwrap()
            } else {
                format!("({})", parts.join(" OR "))
            }
        }
        JoinLevelExpr::Not(child) => {
            format!(
                "NOT {}",
                format_join_level_expr(child, join_clause, explainer)
            )
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
            if let Some(explainer) = explainer
                && let Some(deparsed) = explainer.deparse_expr(node.cast())
            {
                return deparsed;
            }
            pg_node_string.clone()
        },
    }
}

/// Recursively formats a DataFusion physical plan as a string, appending
/// collected metrics.  When `include_timing` is false, timing metrics
/// (`elapsed_compute`, named `Time` values) are stripped so that regression
/// test output remains stable.  Pass `true` (e.g. for EXPLAIN ANALYZE VERBOSE)
/// to include everything.
// NOTE: PG parallel workers each run their own `exec_custom_scan` with their
// own plan instance, so these metrics only cover the leader's share.
fn render_plan_with_metrics(
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
pub fn explain_physical_plan(plan: &Arc<dyn ExecutionPlan>, explainer: &mut Explainer) {
    explainer.add_text("DataFusion Physical Plan", "");
    if plan.is::<DistributedExec>() {
        // TODO: `display_plan_ascii` does not currently support stripping `elapsed_compute`
        // or other timing data, so MPP EXPLAIN ANALYZE will include timing metrics even when
        // `TIMING OFF` is requested. This makes regression tests flake. We should upstream
        // a feature to `datafusion-distributed` to optionally strip these metrics.
        let rendered = display_plan_ascii(plan.as_ref(), explainer.is_analyze());
        for line in rendered.lines() {
            explainer.add_text("  ", line);
        }
    } else if explainer.is_analyze() {
        let mut lines = Vec::new();
        render_plan_with_metrics(plan.as_ref(), 0, explainer.is_verbose(), &mut lines);
        for line in lines {
            explainer.add_text("  ", line);
        }
    } else {
        let rendered = datafusion::physical_plan::displayable(plan.as_ref())
            .indent(false)
            .to_string();
        for line in rendered.lines() {
            explainer.add_text("  ", line);
        }
    }
}

/// Merges MPP worker metrics if we are the leader. If the merge times out or fails,
/// it appends a warning to the explainer and returns the original plan.
pub fn get_plan_with_merged_metrics(
    plan: &Arc<dyn ExecutionPlan>,
    is_leader: bool,
    has_runtime: bool,
    explainer: &mut Explainer,
) -> Arc<dyn ExecutionPlan> {
    if is_leader && has_runtime {
        if let Some(merged) = crate::postgres::customscan::mpp::glue::merge_worker_metrics(plan) {
            return merged;
        }
        explainer.add_text(
            "  (worker metrics incomplete; a worker may not have reported)",
            "",
        );
    }
    Arc::clone(plan)
}
