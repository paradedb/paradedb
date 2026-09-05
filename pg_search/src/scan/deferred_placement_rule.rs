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

//! Physical optimizer rule that places the two halves of a deferred string lookup per source.
//!
//! The logical rule decides whether a string column leaves its scan as a union at all. This
//! rule decides where each half of the lookup runs, from the shape of the plan between the
//! scan and the decode point:
//!
//! - The fetch (doc address to term ordinal) reads a fast-field column, which is cheapest in
//!   doc order. It stays deferred while the rows reach the decode point in that order and no
//!   join multiplies them. Otherwise the scan resolves the ordinals itself, in doc order.
//! - The decode (term ordinal to string) costs the same per row wherever it runs, so it stays
//!   deferred unless a join multiplies the rows on the way and nothing above bounds them. In
//!   that case the scan decodes the column and no union is carried at all.
//!
//! Row multiplication is read from the join keys, not from cardinality estimates: a source's
//! rows fan out through an equi-join when the other side's key is not that side's unique key
//! field, and through any non-equi or cross join. Estimates move between machines and would
//! flip plans between runs; the key shape does not.
//!
//! The shape of the model follows Liu et al., "Selective Late Materialization in Modern
//! Analytical Databases" (PVLDB 2025): each attribute picks its own point between its scan
//! and its first consumer, a fetch costs more once the row ids stop arriving in storage order
//! (a hash join's build side, a sort, a hash repartition) and grows with the row count at the
//! point, and carrying a narrow stand-in through hash tables and shuffles is what deferral
//! buys. It differs in what it measures. The paper trains a fetch and memory-copy cost model
//! and takes cardinalities from the optimizer. Here both signals come from the plan's shape:
//! the fetch of an ordinal is a sequential column read that costs little whichever way the
//! decision goes, the decode is a per-row dictionary lookup whose cost is set by the row
//! count alone, and the paper found that points in the middle of a pipeline rarely pay, so
//! the scan and the consumer are the only candidates.

use std::sync::Arc;

use datafusion::common::config::ConfigOptions;
use datafusion::common::{JoinType, Result};
use datafusion::physical_expr::expressions::Column;
use datafusion::physical_optimizer::PhysicalOptimizerRule;
use datafusion::physical_plan::coalesce_partitions::CoalescePartitionsExec;
use datafusion::physical_plan::coop::CooperativeExec;
use datafusion::physical_plan::joins::{CrossJoinExec, HashJoinExec, NestedLoopJoinExec};
use datafusion::physical_plan::limit::{GlobalLimitExec, LocalLimitExec};
use datafusion::physical_plan::projection::ProjectionExec;
use datafusion::physical_plan::repartition::RepartitionExec;
use datafusion::physical_plan::sorts::sort::SortExec;
use datafusion::physical_plan::sorts::sort_preserving_merge::SortPreservingMergeExec;
use datafusion::physical_plan::{
    ChildrenPropertiesMode, ExecutionPlan, Partitioning, ReplaceChildrenOptions,
};
use pgrx::pg_sys;

use crate::api::HashMap;
use crate::gucs::{self, DeferredPlacement};
use crate::postgres::rel::PgSearchRelation;
use crate::scan::deferred_lookup::PhysicalDeferredField;
use crate::scan::execution_plan::PgSearchScanPlan;
use crate::scan::filter_passthrough_exec::FilterPassthroughExec;
use crate::scan::tantivy_decode_exec::TantivyDecodeExec;
use crate::scan::tantivy_fetch_exec::TantivyFetchExec;

#[derive(Debug)]
pub struct DeferredPlacementRule;

impl PhysicalOptimizerRule for DeferredPlacementRule {
    fn name(&self) -> &str {
        "DeferredPlacement"
    }

    fn schema_check(&self) -> bool {
        true
    }

    fn optimize(
        &self,
        plan: Arc<dyn ExecutionPlan>,
        _config: &ConfigOptions,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let mut ctx = Context {
            fetch_auto: gucs::defer_column_fetch() == DeferredPlacement::Auto,
            decode_auto: gucs::defer_string_decode() == DeferredPlacement::Auto,
            key_fields: HashMap::default(),
            decisions: HashMap::default(),
        };
        if !ctx.fetch_auto && !ctx.decode_auto {
            return Ok(plan);
        }
        collect_decisions(&plan, false, &mut ctx);
        if ctx.decisions.values().all(|d| !d.moves()) {
            return Ok(plan);
        }
        rewrite(plan, &ctx.decisions)
    }
}

/// Whether a source's rows are multiplied on their way up to the decode point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Expansion {
    No,
    Yes,
    Unknown,
}

impl Expansion {
    fn join(self, other: Expansion) -> Expansion {
        match (self, other) {
            (Expansion::Yes, _) | (_, Expansion::Yes) => Expansion::Yes,
            (Expansion::Unknown, _) | (_, Expansion::Unknown) => Expansion::Unknown,
            _ => Expansion::No,
        }
    }
}

/// What the path from a scan up to its decode point does to the scan's rows.
#[derive(Debug)]
struct PathSummary {
    out_of_order: bool,
    expansion: Expansion,
}

/// Where a source's deferred columns end up. Both flags false keeps them at the decode point.
#[derive(Clone, Copy, Debug, Default)]
struct Decision {
    fetch_at_scan: bool,
    eager: bool,
}

impl Decision {
    fn moves(&self) -> bool {
        self.fetch_at_scan || self.eager
    }

    /// Two scans of one index share a helper layout, so they share a decision; the deferred
    /// choice wins because it is the one both paths were planned for.
    fn merge(self, other: Decision) -> Decision {
        Decision {
            fetch_at_scan: self.fetch_at_scan && other.fetch_at_scan,
            eager: self.eager && other.eager,
        }
    }
}

struct Context {
    fetch_auto: bool,
    decode_auto: bool,
    /// Key field per index, or `None` when the index cannot be opened (a placeholder scan).
    key_fields: HashMap<u32, Option<String>>,
    decisions: HashMap<u32, Decision>,
}

impl Context {
    fn key_field(&mut self, indexrelid: u32) -> Option<String> {
        self.key_fields
            .entry(indexrelid)
            .or_insert_with(|| {
                if indexrelid == 0 {
                    return None;
                }
                let rel = PgSearchRelation::open(pg_sys::Oid::from(indexrelid));
                Some(rel.options().key_field_name().to_string())
            })
            .clone()
    }
}

/// One step of the path from a decode point down to a scan: the node and the child that
/// leads toward the scan.
type PathStep = (Arc<dyn ExecutionPlan>, usize);

/// Walks the plan top-down. At each decode point, every source scan it decodes gets a
/// decision from the path between the two. `bounded` says whether the nearest consumer above
/// the current node stops after a fixed number of rows, which makes a deferred decode cheap
/// whatever the path did to the row count.
fn collect_decisions(node: &Arc<dyn ExecutionPlan>, bounded: bool, ctx: &mut Context) {
    if let Some(decode) = node.downcast_ref::<TantivyDecodeExec>() {
        let wanted: Vec<u32> = decode
            .deferred_fields()
            .iter()
            .map(|f| f.canonical.indexrelid)
            .collect();
        let mut scans = Vec::new();
        collect_scans(node, &mut Vec::new(), &wanted, &mut scans);
        for (scan, path) in scans {
            let summary = summarize_path(&path, ctx);
            let decision = decide(&summary, bounded, ctx);
            let merged = match ctx.decisions.get(&scan.indexrelid) {
                Some(existing) => existing.merge(decision),
                None => decision,
            };
            ctx.decisions.insert(scan.indexrelid, merged);
            pgrx::debug1!(
                "DeferredPlacement: {} out_of_order={} expansion={:?} bounded={} -> fetch_at_scan={} eager={}",
                scan.table_alias,
                summary.out_of_order,
                summary.expansion,
                bounded,
                merged.fetch_at_scan,
                merged.eager
            );
        }
    }

    let bounded_below = is_bounded_consumer(node) || (bounded && is_transparent(node));
    for child in node.children() {
        collect_decisions(child, bounded_below, ctx);
    }
}

/// Collects the scans under `node` whose deferred columns the decode point reads, each with
/// the path of `(node, child index)` steps that leads down to it.
fn collect_scans(
    node: &Arc<dyn ExecutionPlan>,
    path: &mut Vec<PathStep>,
    wanted: &[u32],
    out: &mut Vec<(Arc<PgSearchScanPlan>, Vec<PathStep>)>,
) {
    if let Some(scan) = node.downcast_ref::<PgSearchScanPlan>() {
        if scan.has_deferred_fields() && wanted.contains(&scan.indexrelid) {
            // The scan is cloned through its `Clone`, which shares the readers.
            out.push((Arc::new(scan.clone()), path.clone()));
        }
        return;
    }
    for (idx, child) in node.children().into_iter().enumerate() {
        path.push((Arc::clone(node), idx));
        collect_scans(child, path, wanted, out);
        path.pop();
    }
}

fn summarize_path(path: &[PathStep], ctx: &mut Context) -> PathSummary {
    let mut summary = PathSummary {
        out_of_order: false,
        expansion: Expansion::No,
    };
    for (node, child_idx) in path {
        if let Some(join) = node.downcast_ref::<HashJoinExec>() {
            let on_left = *child_idx == 0;
            // The build side comes back out in probe order.
            if on_left {
                summary.out_of_order = true;
            }
            let other = if on_left { join.right() } else { join.left() };
            let other_keys = join.on().iter().map(|(l, r)| if on_left { r } else { l });
            let expansion = match join.join_type() {
                JoinType::LeftSemi
                | JoinType::LeftAnti
                | JoinType::LeftMark
                | JoinType::RightSemi
                | JoinType::RightAnti
                | JoinType::RightMark => Expansion::No,
                _ => equi_join_expansion(other, other_keys, ctx),
            };
            summary.expansion = summary.expansion.join(expansion);
        } else if let Some(join) = node.downcast_ref::<NestedLoopJoinExec>() {
            if *child_idx == 0 {
                summary.out_of_order = true;
            }
            let expansion = match join.join_type() {
                JoinType::LeftSemi
                | JoinType::LeftAnti
                | JoinType::LeftMark
                | JoinType::RightSemi
                | JoinType::RightAnti
                | JoinType::RightMark => Expansion::No,
                _ => Expansion::Yes,
            };
            summary.expansion = summary.expansion.join(expansion);
        } else if node.is::<CrossJoinExec>() {
            if *child_idx == 0 {
                summary.out_of_order = true;
            }
            summary.expansion = summary.expansion.join(Expansion::Yes);
        } else if node.is::<SortExec>() {
            summary.out_of_order = true;
        } else if let Some(repartition) = node.downcast_ref::<RepartitionExec>() {
            // A hash repartition hands each partition a strided slice of the column, so a
            // batch no longer covers a contiguous run of doc ids; round-robin keeps batches.
            if matches!(repartition.partitioning(), Partitioning::Hash(_, _)) {
                summary.out_of_order = true;
            }
        } else if node.children().len() > 1 {
            summary.expansion = summary.expansion.join(Expansion::Unknown);
        }
    }
    summary
}

/// A source's rows fan out through an equi-join unless one of the other side's keys is that
/// side's unique key field. A key that cannot be traced to a scan (the other side is itself a
/// join, or the key is an expression) leaves the answer open.
fn equi_join_expansion<'a>(
    other: &Arc<dyn ExecutionPlan>,
    other_keys: impl Iterator<Item = &'a Arc<dyn datafusion::physical_expr::PhysicalExpr>>,
    ctx: &mut Context,
) -> Expansion {
    let mut traced_any = false;
    for key in other_keys {
        let Some(col) = key.downcast_ref::<Column>() else {
            continue;
        };
        let Some((indexrelid, field)) = trace_to_scan(other, col.index()) else {
            continue;
        };
        traced_any = true;
        if ctx.key_field(indexrelid).as_deref() == Some(field.as_str()) {
            return Expansion::No;
        }
    }
    if traced_any {
        Expansion::Yes
    } else {
        Expansion::Unknown
    }
}

/// Follows an output column down to the scan it comes from, through projections and
/// schema-preserving nodes. Stops at a join, whose output no longer has a single source.
fn trace_to_scan(plan: &Arc<dyn ExecutionPlan>, col: usize) -> Option<(u32, String)> {
    if let Some(scan) = plan.downcast_ref::<PgSearchScanPlan>() {
        let schema = plan.schema();
        return schema
            .fields()
            .get(col)
            .map(|f| (scan.indexrelid, f.name().clone()));
    }
    if let Some(proj) = plan.downcast_ref::<ProjectionExec>() {
        let expr = &proj.expr().get(col)?.expr;
        let column = expr.downcast_ref::<Column>()?;
        return trace_to_scan(proj.input(), column.index());
    }
    let children = plan.children();
    if children.len() == 1 && same_columns(plan, children[0]) {
        return trace_to_scan(children[0], col);
    }
    None
}

fn same_columns(a: &Arc<dyn ExecutionPlan>, b: &Arc<dyn ExecutionPlan>) -> bool {
    let (sa, sb) = (a.schema(), b.schema());
    sa.fields().len() == sb.fields().len()
        && sa
            .fields()
            .iter()
            .zip(sb.fields().iter())
            .all(|(fa, fb)| fa.name() == fb.name())
}

fn decide(summary: &PathSummary, bounded: bool, ctx: &Context) -> Decision {
    let eager = ctx.decode_auto && summary.expansion == Expansion::Yes && !bounded;
    let fetch_at_scan =
        ctx.fetch_auto && !eager && (summary.out_of_order || summary.expansion == Expansion::Yes);
    Decision {
        fetch_at_scan,
        eager,
    }
}

/// A consumer that stops after a fixed number of rows: a deferred decode below it only ever
/// runs for those rows.
fn is_bounded_consumer(node: &Arc<dyn ExecutionPlan>) -> bool {
    node.downcast_ref::<SortExec>()
        .is_some_and(|sort| sort.fetch().is_some())
        || node.is::<GlobalLimitExec>()
        || node.is::<LocalLimitExec>()
}

/// Nodes that hand a bounded consumer's limit through to their child unchanged.
fn is_transparent(node: &Arc<dyn ExecutionPlan>) -> bool {
    node.is::<ProjectionExec>()
        || node.is::<CoalescePartitionsExec>()
        || node.is::<SortPreservingMergeExec>()
        || node.is::<CooperativeExec>()
        || node.is::<RepartitionExec>()
        || node.is::<FilterPassthroughExec>()
}

fn moved(decisions: &HashMap<u32, Decision>, field: &PhysicalDeferredField) -> Option<Decision> {
    decisions
        .get(&field.canonical.indexrelid)
        .copied()
        .filter(Decision::moves)
}

/// Applies the decisions bottom-up: a scan takes its columns over, and the fetch and decode
/// above it drop them. The nodes in between are rebuilt so an eager column's new type reaches
/// the decode point.
fn rewrite(
    node: Arc<dyn ExecutionPlan>,
    decisions: &HashMap<u32, Decision>,
) -> Result<Arc<dyn ExecutionPlan>> {
    if let Some(scan) = node.downcast_ref::<PgSearchScanPlan>() {
        let Some(decision) = decisions.get(&scan.indexrelid).filter(|d| d.moves()) else {
            return Ok(node);
        };
        let names: Vec<String> = scan
            .deferred_fields()
            .iter()
            .map(|d| d.name.clone())
            .collect();
        return if decision.eager {
            Ok(scan.with_deferred_placement(&[], &names)?)
        } else {
            Ok(scan.with_deferred_placement(&names, &[])?)
        };
    }

    let children = node.children();
    let mut new_children = Vec::with_capacity(children.len());
    let mut changed = false;
    for child in children {
        let new_child = rewrite(Arc::clone(child), decisions)?;
        changed |= !Arc::ptr_eq(child, &new_child);
        new_children.push(new_child);
    }

    if let Some(fetch) = node.downcast_ref::<TantivyFetchExec>() {
        let keep: Vec<PhysicalDeferredField> = fetch
            .fetch_fields()
            .iter()
            .filter(|f| moved(decisions, f).is_none())
            .cloned()
            .collect();
        let input = new_children.remove(0);
        if keep.len() == fetch.fetch_fields().len() && !changed {
            return Ok(node);
        }
        if keep.is_empty() && fetch.ctid_columns().is_empty() {
            return Ok(input);
        }
        return Ok(Arc::new(fetch.with_input_and_fields(input, keep)?));
    }

    if let Some(decode) = node.downcast_ref::<TantivyDecodeExec>() {
        let keep: Vec<PhysicalDeferredField> = decode
            .deferred_fields()
            .iter()
            .filter(|f| !moved(decisions, f).is_some_and(|d| d.eager))
            .cloned()
            .collect();
        let input = new_children.remove(0);
        if keep.len() == decode.deferred_fields().len() && !changed {
            return Ok(node);
        }
        if keep.is_empty() {
            return Ok(input);
        }
        return Ok(Arc::new(decode.with_input_and_fields(input, keep)?));
    }

    if changed {
        node.replace_children(
            new_children,
            ReplaceChildrenOptions::new(ChildrenPropertiesMode::Recompute),
        )
    } else {
        Ok(node)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(fetch_auto: bool, decode_auto: bool) -> Context {
        Context {
            fetch_auto,
            decode_auto,
            key_fields: HashMap::default(),
            decisions: HashMap::default(),
        }
    }

    fn summary(out_of_order: bool, expansion: Expansion) -> PathSummary {
        PathSummary {
            out_of_order,
            expansion,
        }
    }

    #[test]
    fn expansion_joins_toward_the_worse_answer() {
        assert_eq!(Expansion::No.join(Expansion::No), Expansion::No);
        assert_eq!(Expansion::No.join(Expansion::Unknown), Expansion::Unknown);
        assert_eq!(Expansion::Unknown.join(Expansion::Yes), Expansion::Yes);
        assert_eq!(Expansion::Yes.join(Expansion::No), Expansion::Yes);
    }

    #[test]
    fn in_order_rows_that_do_not_fan_out_stay_deferred() {
        let d = decide(&summary(false, Expansion::No), false, &ctx(true, true));
        assert!(!d.fetch_at_scan && !d.eager);
        let d = decide(&summary(false, Expansion::Unknown), false, &ctx(true, true));
        assert!(!d.fetch_at_scan && !d.eager);
    }

    #[test]
    fn a_build_side_fetches_in_the_scan_but_still_decodes_late() {
        let d = decide(&summary(true, Expansion::No), false, &ctx(true, true));
        assert!(d.fetch_at_scan && !d.eager);
    }

    #[test]
    fn a_fan_out_decodes_in_the_scan_unless_something_above_bounds_it() {
        let d = decide(&summary(false, Expansion::Yes), false, &ctx(true, true));
        assert!(d.eager && !d.fetch_at_scan);
        let d = decide(&summary(false, Expansion::Yes), true, &ctx(true, true));
        assert!(d.fetch_at_scan && !d.eager);
    }

    #[test]
    fn a_pinned_half_is_left_alone() {
        let d = decide(&summary(true, Expansion::Yes), false, &ctx(false, true));
        assert!(d.eager && !d.fetch_at_scan);
        let d = decide(&summary(true, Expansion::Yes), false, &ctx(true, false));
        assert!(d.fetch_at_scan && !d.eager);
        let d = decide(&summary(true, Expansion::Yes), false, &ctx(false, false));
        assert!(!d.moves());
    }

    #[test]
    fn two_scans_of_one_index_only_move_when_both_agree() {
        let stay = Decision::default();
        let go = Decision {
            fetch_at_scan: true,
            eager: true,
        };
        assert!(!go.merge(stay).moves());
        assert!(go.merge(go).eager);
    }
}
