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
//! flip plans between runs; the key shape does not. The price of that is a join whose other
//! side is far more selective than the keys suggest: the scan then decodes rows the join
//! would have dropped. The join key `InList` pushed down into the probe scan covers the
//! common case, since the scan prunes on it before it reads any column.
//!
//! The shape of the model follows Liu et al., "Selective Late Materialization in Modern
//! Analytical Databases" (PVLDB 2025): each attribute picks its own point between its scan
//! and its first consumer, a fetch costs more once the row ids stop arriving in storage order
//! (a hash join's build side, a sort, a hash repartition) and grows with the row count at the
//! point, and carrying a narrow stand-in through hash tables and shuffles is what deferral
//! buys. It differs in what it measures. The paper trains a fetch and memory-copy cost model
//! and takes cardinalities from the optimizer. Here both signals come from the plan's shape:
//! an ordinal fetch is one sequential column read in the scan and becomes one random read
//! per joined row after a build side or a fan-out, and the decode is a per-row dictionary
//! lookup whose cost is set by the row count alone. The paper's Section 5.8 found that
//! points in the middle of a pipeline rarely pay, so the scan and the consumer are the only
//! candidates.

use std::sync::Arc;

use datafusion::common::config::ConfigOptions;
use datafusion::common::{JoinType, Result};
use datafusion::physical_expr::LexOrdering;
use datafusion::physical_expr::expressions::Column;
use datafusion::physical_optimizer::PhysicalOptimizerRule;
use datafusion::physical_plan::aggregates::AggregateExec;
use datafusion::physical_plan::coalesce_partitions::CoalescePartitionsExec;
use datafusion::physical_plan::coop::CooperativeExec;
use datafusion::physical_plan::filter::FilterExec;
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

use crate::api::{HashMap, HashSet};
use crate::gucs::{self, DeferredPlacement};
use crate::postgres::customscan::joinscan::visibility_filter::VisibilityFilterExec;
use crate::postgres::rel::PgSearchRelation;
use crate::scan::deferred_lookup::PhysicalDeferredField;
use crate::scan::execution_plan::PgSearchScanPlan;
use crate::scan::filter_passthrough_exec::FilterPassthroughExec;
use crate::scan::segmented_topk_rule::resolve_physical_index;
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
        collect_decisions(&plan, Bound::None, &mut ctx);
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
    fn worst(self, other: Expansion) -> Expansion {
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
    /// Every node on the path is one the rewrite knows how to rebuild with a changed column
    /// type; an eager decode changes the scan's output type and needs that.
    eager_safe: bool,
}

/// What the nearest consumer above a decode point does with the rows it gets.
#[derive(Clone)]
enum Bound {
    /// Consumes every row.
    None,
    /// A streaming limit: the decode below runs for about that many rows.
    Limit,
    /// A Top-K sort, which consumes every row itself. It bounds a decode only when the
    /// `SegmentedTopKRule` will take the sort over and prune before the decode.
    TopK(LexOrdering),
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
/// decision from the path between the two. `bound` says what the nearest consumer above the
/// current node does with its rows; one that stops early makes a deferred decode cheap
/// whatever the path did to the row count.
fn collect_decisions(node: &Arc<dyn ExecutionPlan>, bound: Bound, ctx: &mut Context) {
    if let Some(decode) = node.downcast_ref::<TantivyDecodeExec>() {
        let bounded = match &bound {
            Bound::None => false,
            Bound::Limit => true,
            Bound::TopK(order) => segmented_topk_takes(order, decode),
        };
        let wanted: Vec<u32> = decode
            .deferred_fields()
            .iter()
            .map(|f| f.canonical.indexrelid)
            .collect();
        let mut scans = Vec::new();
        collect_scans(node, &mut Vec::new(), &wanted, &mut scans);
        for ((indexrelid, alias), path) in scans {
            let summary = summarize_path(&path, ctx);
            let decision = decide(&summary, bounded, ctx);
            let merged = match ctx.decisions.get(&indexrelid) {
                Some(existing) => existing.merge(decision),
                None => decision,
            };
            ctx.decisions.insert(indexrelid, merged);
            pgrx::debug1!(
                "DeferredPlacement: {} out_of_order={} expansion={:?} eager_safe={} bounded={} -> fetch_at_scan={} eager={}",
                alias,
                summary.out_of_order,
                summary.expansion,
                summary.eager_safe,
                bounded,
                merged.fetch_at_scan,
                merged.eager
            );
        }
    }

    let below = if is_streaming_limit(node) {
        Bound::Limit
    } else if let Some(sort) = node.downcast_ref::<SortExec>()
        && sort.fetch().is_some()
    {
        Bound::TopK(sort.expr().clone())
    } else if is_transparent(node) {
        bound
    } else {
        Bound::None
    };
    for child in node.children() {
        collect_decisions(child, below.clone(), ctx);
    }
}

/// Whether `SegmentedTopKRule` will take a Top-K sort with this order over from `decode`:
/// at least one sort key is a deferred column of the decode, and every such key comes from
/// one index. Shares that rule's column resolution, so the two agree on a self-join.
fn segmented_topk_takes(order: &LexOrdering, decode: &TantivyDecodeExec) -> bool {
    if !gucs::enable_segmented_topk() {
        return false;
    }
    let input_schema = decode.children()[0].schema();
    let mut indexes: HashSet<u32> = HashSet::default();
    for sort in order.iter() {
        if let Some(col) = sort.expr.downcast_ref::<Column>()
            && let Some(idx) = resolve_physical_index(col, &input_schema)
            && let Some(field) = decode.deferred_fields().iter().find(|f| f.col_idx == idx)
        {
            indexes.insert(field.canonical.indexrelid);
        }
    }
    indexes.len() == 1
}

/// Collects the scans under `node` whose deferred columns the decode point reads, each with
/// the path of `(node, child index)` steps that leads down to it.
fn collect_scans(
    node: &Arc<dyn ExecutionPlan>,
    path: &mut Vec<PathStep>,
    wanted: &[u32],
    out: &mut Vec<((u32, String), Vec<PathStep>)>,
) {
    if let Some(scan) = node.downcast_ref::<PgSearchScanPlan>() {
        if scan.has_deferred_fields() && wanted.contains(&scan.indexrelid) {
            out.push(((scan.indexrelid, scan.table_alias.clone()), path.clone()));
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
        eager_safe: true,
    };
    for (node, child_idx) in path {
        if !rebuilds_with_new_types(node) {
            summary.eager_safe = false;
        }
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
            summary.expansion = summary.expansion.worst(expansion);
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
            summary.expansion = summary.expansion.worst(expansion);
        } else if node.is::<CrossJoinExec>() {
            if *child_idx == 0 {
                summary.out_of_order = true;
            }
            summary.expansion = summary.expansion.worst(Expansion::Yes);
        } else if node.is::<SortExec>() {
            summary.out_of_order = true;
        } else if let Some(repartition) = node.downcast_ref::<RepartitionExec>() {
            // A hash repartition hands each partition a strided slice of the column, so a
            // batch no longer covers a contiguous run of doc ids; round-robin keeps batches.
            if matches!(repartition.partitioning(), Partitioning::Hash(_, _)) {
                summary.out_of_order = true;
            }
        } else if node.children().len() > 1 {
            summary.expansion = summary.expansion.worst(Expansion::Unknown);
        }
    }
    summary
}

/// Nodes whose rebuild recomputes their schema from a child whose column type changed.
/// `ProjectionExec` is on the list because `rewrite` rebuilds it by hand.
fn rebuilds_with_new_types(node: &Arc<dyn ExecutionPlan>) -> bool {
    node.is::<HashJoinExec>()
        || node.is::<NestedLoopJoinExec>()
        || node.is::<CrossJoinExec>()
        || node.is::<FilterExec>()
        || node.is::<SortExec>()
        || node.is::<RepartitionExec>()
        || node.is::<CoalescePartitionsExec>()
        || node.is::<SortPreservingMergeExec>()
        || node.is::<CooperativeExec>()
        || node.is::<GlobalLimitExec>()
        || node.is::<LocalLimitExec>()
        || node.is::<ProjectionExec>()
        || node.is::<FilterPassthroughExec>()
        || node.is::<VisibilityFilterExec>()
        || node.is::<TantivyFetchExec>()
        || node.is::<TantivyDecodeExec>()
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
/// schema-preserving nodes. Stops at a join, whose output no longer has a single source, and
/// at an aggregate, whose groups are not the scan's rows.
fn trace_to_scan(plan: &Arc<dyn ExecutionPlan>, col: usize) -> Option<(u32, String)> {
    if plan.is::<AggregateExec>() {
        return None;
    }
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
    let eager =
        ctx.decode_auto && summary.expansion == Expansion::Yes && !bounded && summary.eager_safe;
    let fetch_at_scan =
        ctx.fetch_auto && !eager && (summary.out_of_order || summary.expansion == Expansion::Yes);
    Decision {
        fetch_at_scan,
        eager,
    }
}

/// A limit that stops pulling once it has its rows, so a deferred decode below it only ever
/// runs for about that many.
fn is_streaming_limit(node: &Arc<dyn ExecutionPlan>) -> bool {
    node.is::<GlobalLimitExec>()
        || node.is::<LocalLimitExec>()
        || node
            .downcast_ref::<SortPreservingMergeExec>()
            .is_some_and(|merge| merge.fetch().is_some())
}

/// Nodes a limit passes through, the same set `SegmentedTopKRule` descends through, so the
/// two rules see the same consumer.
fn is_transparent(node: &Arc<dyn ExecutionPlan>) -> bool {
    node.supports_limit_pushdown() || node.is::<FilterPassthroughExec>()
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

    if let Some(proj) = node.downcast_ref::<ProjectionExec>() {
        if !changed {
            return Ok(node);
        }
        // The projector caches its output schema, so a rebuild through `replace_children`
        // would keep the old column type.
        let input = new_children.remove(0);
        return Ok(Arc::new(ProjectionExec::try_new(
            proj.expr().to_vec(),
            input,
        )?));
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

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use crate::index::fast_fields_helper::{CanonicalColumn, FFHelper};
    use crate::query::SearchQueryInput;
    use crate::scan::deferred_encode::deferred_union_data_type;
    use crate::scan::late_materialization::DeferredField;
    use arrow_schema::{DataType, Field, Schema};
    use datafusion::physical_expr::projection::ProjectionExpr;
    use pgrx::prelude::*;

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
            eager_safe: true,
        }
    }

    #[test]
    fn expansion_combines_toward_the_worse_answer() {
        assert_eq!(Expansion::No.worst(Expansion::No), Expansion::No);
        assert_eq!(Expansion::No.worst(Expansion::Unknown), Expansion::Unknown);
        assert_eq!(Expansion::Unknown.worst(Expansion::Yes), Expansion::Yes);
        assert_eq!(Expansion::Yes.worst(Expansion::No), Expansion::Yes);
    }

    #[test]
    fn a_path_the_rewrite_cannot_retype_keeps_the_decode_deferred() {
        let mut path = summary(false, Expansion::Yes);
        path.eager_safe = false;
        let d = decide(&path, false, &ctx(true, true));
        assert!(d.fetch_at_scan && !d.eager);
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

    /// A projection between the scan and its decode point caches its output schema, so the
    /// eager rewrite must rebuild it for the scan's new column type to reach the root.
    #[pg_test]
    fn eager_rewrite_retypes_a_projection_above_the_scan() {
        let indexrelid = 42;
        let canonical = CanonicalColumn {
            indexrelid,
            ff_index: 1,
        };
        let scan = Arc::new(PgSearchScanPlan::new(
            None,
            Arc::new(Schema::new(vec![
                Field::new("id", DataType::Int64, true),
                Field::new("title", deferred_union_data_type(), true),
            ])),
            SearchQueryInput::All,
            None,
            vec![DeferredField {
                name: "title".into(),
                is_bytes: false,
                canonical: canonical.clone(),
                rebuild: None,
                fetch_at_scan: false,
            }],
            Some(Arc::new(FFHelper::empty())),
            indexrelid,
            None,
            1,
            None,
            None,
        )) as Arc<dyn ExecutionPlan>;
        let projection = Arc::new(
            ProjectionExec::try_new(
                vec![
                    ProjectionExpr::new(Arc::new(Column::new("title", 1)), "title"),
                    ProjectionExpr::new(Arc::new(Column::new("id", 0)), "id"),
                ],
                scan,
            )
            .unwrap(),
        ) as Arc<dyn ExecutionPlan>;
        let mut ffhelpers = HashMap::default();
        ffhelpers.insert(indexrelid, Arc::new(FFHelper::empty()));
        let decode = Arc::new(
            TantivyDecodeExec::new(
                projection,
                vec![PhysicalDeferredField {
                    col_idx: 0,
                    display_name: "title".into(),
                    is_bytes: false,
                    canonical,
                    rebuild: None,
                }],
                ffhelpers,
            )
            .unwrap(),
        ) as Arc<dyn ExecutionPlan>;

        let mut decisions = HashMap::default();
        decisions.insert(
            indexrelid,
            Decision {
                fetch_at_scan: false,
                eager: true,
            },
        );
        let rewritten = rewrite(decode, &decisions).unwrap();

        assert!(rewritten.is::<ProjectionExec>(), "the decode node must go");
        assert_eq!(rewritten.schema().field(0).data_type(), &DataType::Utf8View);
        assert_eq!(rewritten.schema().field(1).data_type(), &DataType::Int64);
        let scan = rewritten.children()[0]
            .downcast_ref::<PgSearchScanPlan>()
            .expect("the scan stays the leaf");
        assert!(!scan.has_deferred_fields());
    }
}
