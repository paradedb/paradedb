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
//! - The fetch (doc address to term ordinal) reads a columnar field, which is cheapest in
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
//! common case, though only as far as the pushed-down keys prune. The case that stays open
//! is a join whose other side is the selective one: the scan decodes rows the join then
//! drops. An estimate would not close it today either. No scan publishes a distinct count
//! for a join key, and without one DataFusion reports an equi-join's output as its smaller
//! input, which is the one number that says nothing about the fan-out.
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
use datafusion::common::tree_node::Transformed;
use datafusion::common::{JoinType, Result};
use datafusion::physical_expr::LexOrdering;
use datafusion::physical_expr::expressions::Column;
use datafusion::physical_optimizer::PhysicalOptimizerRule;
use datafusion::physical_plan::aggregates::{AggregateExec, AggregateMode};
use datafusion::physical_plan::coalesce_partitions::CoalescePartitionsExec;
use datafusion::physical_plan::coop::CooperativeExec;
use datafusion::physical_plan::filter::FilterExec;
use datafusion::physical_plan::joins::{CrossJoinExec, HashJoinExec, NestedLoopJoinExec};
use datafusion::physical_plan::limit::{GlobalLimitExec, LocalLimitExec};
use datafusion::physical_plan::projection::ProjectionExec;
use datafusion::physical_plan::repartition::RepartitionExec;
use datafusion::physical_plan::sorts::sort::SortExec;
use datafusion::physical_plan::sorts::sort_preserving_merge::SortPreservingMergeExec;
use datafusion::physical_plan::{ExecutionPlan, Partitioning};
use pgrx::pg_sys;

use crate::api::{HashMap, HashSet};
use crate::gucs::{self, DeferredPlacement};
use crate::index::fast_fields_helper::FFIndex;
use crate::postgres::customscan::joinscan::visibility_filter::VisibilityFilterExec;
use crate::postgres::rel::PgSearchRelation;
use crate::scan::deferred_lookup::PhysicalDeferredField;
use crate::scan::execution_plan::PgSearchScanPlan;
use crate::scan::filter_passthrough_exec::FilterPassthroughExec;
use crate::scan::plan_walk::{PathStep, transform_up_with_children, visit_with_path};
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

/// One deferred column of one scan. The index is not enough on its own: a self-join reads
/// one index through two scans, and each reaches its decode point by its own path.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct DeferredSource {
    heap_rti: u32,
    ff_index: FFIndex,
}

impl DeferredSource {
    fn of(field: &PhysicalDeferredField) -> Self {
        Self {
            heap_rti: field.heap_rti,
            ff_index: field.canonical.ff_index,
        }
    }
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

    /// One column can still be read at two decode points. The deferred choice wins, because
    /// it is the one both paths were planned for.
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
    /// Per column of per scan, since each one has its own consumer and its own path, so its
    /// own point to stop at.
    decisions: HashMap<DeferredSource, Decision>,
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
            .map(|f| f.heap_rti)
            .collect();
        let mut scans = Vec::new();
        collect_scans(node, &wanted, &mut scans);
        for ((heap_rti, alias), path) in scans {
            let summary = summarize_path(&path, ctx);
            let decision = decide(&summary, bounded, ctx);
            for field in decode
                .deferred_fields()
                .iter()
                .filter(|f| f.heap_rti == heap_rti)
            {
                let source = DeferredSource::of(field);
                let merged = match ctx.decisions.get(&source) {
                    Some(existing) => existing.merge(decision),
                    None => decision,
                };
                ctx.decisions.insert(source, merged);
                pgrx::debug1!(
                    "DeferredPlacement: {}.{} out_of_order={} expansion={:?} eager_safe={} bounded={} -> fetch_at_scan={} eager={}",
                    alias,
                    field.display_name,
                    summary.out_of_order,
                    summary.expansion,
                    summary.eager_safe,
                    bounded,
                    merged.fetch_at_scan,
                    merged.eager
                );
            }
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
/// its range table index and the path of steps that leads down to it.
fn collect_scans(
    node: &Arc<dyn ExecutionPlan>,
    wanted: &[u32],
    out: &mut Vec<((u32, String), Vec<PathStep>)>,
) {
    visit_with_path(node, &mut |node, path| {
        // Every deferred column of one scan carries that scan's range table index.
        if let Some(scan) = node.downcast_ref::<PgSearchScanPlan>()
            && let Some(field) = scan.deferred_fields().first()
            && wanted.contains(&field.heap_rti)
        {
            out.push(((field.heap_rti, scan.table_alias.clone()), path.to_vec()));
        }
    });
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
    let mut read_any = false;
    for key in other_keys {
        let Some(col) = key.downcast_ref::<Column>() else {
            continue;
        };
        read_any = true;
        if is_unique(other, col.index(), ctx) {
            return Expansion::No;
        }
    }
    if read_any {
        Expansion::Yes
    } else {
        Expansion::Unknown
    }
}

/// Whether `col` holds at most one row per value where `plan` emits it.
///
/// Read from the plan's shape alone, never from a row count. A scan's key field is unique, a
/// group key is unique because the aggregate emits one row per group, and a node that only
/// drops rows passes uniqueness through. A join keeps one side's uniqueness exactly while the
/// other side's join keys are unique on that side, which is this same question one level
/// down, so a three-table join gets an answer instead of a shrug.
fn is_unique(plan: &Arc<dyn ExecutionPlan>, col: usize, ctx: &mut Context) -> bool {
    if let Some(scan) = plan.downcast_ref::<PgSearchScanPlan>() {
        let schema = plan.schema();
        return schema.fields().get(col).is_some_and(|field| {
            ctx.key_field(scan.indexrelid).as_deref() == Some(field.name().as_str())
        });
    }

    if let Some(proj) = plan.downcast_ref::<ProjectionExec>() {
        // Only a plain reference carries uniqueness over; an expression can map two values
        // onto one.
        return proj
            .expr()
            .get(col)
            .and_then(|e| e.expr.downcast_ref::<Column>())
            .is_some_and(|column| is_unique(proj.input(), column.index(), ctx));
    }

    if let Some(agg) = plan.downcast_ref::<AggregateExec>() {
        // A partial aggregate emits one row per group per partition, so its keys repeat.
        return matches!(
            agg.mode(),
            AggregateMode::Single | AggregateMode::Final | AggregateMode::FinalPartitioned
        ) && agg.group_expr().is_single()
            && col < agg.group_expr().expr().len();
    }

    if let Some(join) = plan.downcast_ref::<HashJoinExec>() {
        return join_keeps_uniqueness(join, col, ctx);
    }

    let children = plan.children();
    children.len() == 1 && same_columns(plan, children[0]) && is_unique(children[0], col, ctx)
}

/// Whether `col` of a hash join's output is still unique. A semi or anti join reads one side
/// and multiplies nothing. Any other join repeats a row of one side once per match on the
/// other, so that side keeps its uniqueness only while the other side's keys are unique.
fn join_keeps_uniqueness(join: &HashJoinExec, col: usize, ctx: &mut Context) -> bool {
    let col = match &join.projection {
        Some(projection) => match projection.get(col) {
            Some(mapped) => *mapped,
            None => return false,
        },
        None => col,
    };
    let left_width = join.left().schema().fields().len();
    let (side, col, other, other_is_left) = match join.join_type() {
        JoinType::LeftSemi | JoinType::LeftAnti | JoinType::LeftMark => {
            return is_unique(join.left(), col, ctx);
        }
        JoinType::RightSemi | JoinType::RightAnti | JoinType::RightMark => {
            return is_unique(join.right(), col, ctx);
        }
        _ if col < left_width => (join.left(), col, join.right(), false),
        _ => (join.right(), col - left_width, join.left(), true),
    };
    if !is_unique(side, col, ctx) {
        return false;
    }
    join.on().iter().all(|(left, right)| {
        let key = if other_is_left { left } else { right };
        key.downcast_ref::<Column>()
            .is_some_and(|c| is_unique(other, c.index(), ctx))
    })
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

fn moved(
    decisions: &HashMap<DeferredSource, Decision>,
    field: &PhysicalDeferredField,
) -> Option<Decision> {
    decisions
        .get(&DeferredSource::of(field))
        .copied()
        .filter(Decision::moves)
}

/// Applies the decisions bottom-up: a scan takes its columns over, and the fetch and decode
/// above it drop them. The nodes in between are rebuilt so an eager column's new type reaches
/// the decode point.
fn rewrite(
    plan: Arc<dyn ExecutionPlan>,
    decisions: &HashMap<DeferredSource, Decision>,
) -> Result<Arc<dyn ExecutionPlan>> {
    let mut step = |node: Arc<dyn ExecutionPlan>, children: &[Arc<dyn ExecutionPlan>]| {
        if let Some(scan) = node.downcast_ref::<PgSearchScanPlan>() {
            let mut fetch_at_scan: Vec<String> = Vec::new();
            let mut eager: Vec<String> = Vec::new();
            for field in scan.deferred_fields() {
                let source = DeferredSource {
                    heap_rti: field.heap_rti,
                    ff_index: field.canonical.ff_index,
                };
                match decisions.get(&source).filter(|d| d.moves()) {
                    Some(decision) if decision.eager => eager.push(field.name.clone()),
                    Some(_) => fetch_at_scan.push(field.name.clone()),
                    None => {}
                }
            }
            if fetch_at_scan.is_empty() && eager.is_empty() {
                return Ok(Transformed::no(node));
            }
            return Ok(Transformed::yes(
                scan.with_deferred_placement(&fetch_at_scan, &eager)?,
            ));
        }

        if let Some(proj) = node.downcast_ref::<ProjectionExec>() {
            if Arc::ptr_eq(&children[0], proj.input()) {
                return Ok(Transformed::no(node));
            }
            // The projector caches the schema it was built against, so putting the new input
            // back through `replace_children` would keep an eager column's old type.
            return Ok(Transformed::yes(Arc::new(ProjectionExec::try_new(
                proj.expr().to_vec(),
                Arc::clone(&children[0]),
            )?)));
        }

        if let Some(fetch) = node.downcast_ref::<TantivyFetchExec>() {
            let keep: Vec<PhysicalDeferredField> = fetch
                .fetch_fields()
                .iter()
                .filter(|f| moved(decisions, f).is_none())
                .cloned()
                .collect();
            if keep.len() == fetch.fetch_fields().len() {
                return Ok(Transformed::no(node));
            }
            let input = Arc::clone(&children[0]);
            if keep.is_empty() && fetch.ctid_columns().is_empty() {
                return Ok(Transformed::yes(input));
            }
            return Ok(Transformed::yes(Arc::new(
                fetch.with_input_and_fields(input, keep)?,
            )));
        }

        if let Some(decode) = node.downcast_ref::<TantivyDecodeExec>() {
            let keep: Vec<PhysicalDeferredField> = decode
                .deferred_fields()
                .iter()
                .filter(|f| !moved(decisions, f).is_some_and(|d| d.eager))
                .cloned()
                .collect();
            if keep.len() == decode.deferred_fields().len() {
                return Ok(Transformed::no(node));
            }
            let input = Arc::clone(&children[0]);
            if keep.is_empty() {
                return Ok(Transformed::yes(input));
            }
            return Ok(Transformed::yes(Arc::new(
                decode.with_input_and_fields(input, keep)?,
            )));
        }

        Ok(Transformed::no(node))
    };
    transform_up_with_children(plan, &mut step).map(|rewritten| rewritten.data)
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use crate::index::fast_fields_helper::{CanonicalColumn, FFHelper};
    use crate::query::SearchQueryInput;
    use crate::scan::deferred_encode::deferred_field;
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

    /// Builds a scan of `indexrelid` whose schema is `id` then `title`.
    fn scan_of(indexrelid: u32, heap_rti: u32) -> Arc<dyn ExecutionPlan> {
        Arc::new(PgSearchScanPlan::new(
            None,
            Arc::new(Schema::new(vec![
                Field::new("id", DataType::Int64, true),
                Field::new("title", DataType::Utf8View, true),
            ])),
            SearchQueryInput::All,
            None,
            vec![DeferredField {
                name: "title".into(),
                is_bytes: false,
                canonical: CanonicalColumn {
                    indexrelid,
                    ff_index: 1,
                },
                heap_rti,
                plan_position: None,
                rebuild: None,
                fetch_at_scan: false,
            }],
            Some(Arc::new(FFHelper::empty())),
            indexrelid,
            None,
            1,
            None,
            None,
        )) as Arc<dyn ExecutionPlan>
    }

    fn ctx_with_key_field(indexrelid: u32) -> Context {
        let mut ctx = ctx(true, true);
        ctx.key_fields.insert(indexrelid, Some("id".to_string()));
        ctx
    }

    #[pg_test]
    fn a_scan_is_unique_only_on_its_key_field() {
        let scan = scan_of(7, 1);
        let mut ctx = ctx_with_key_field(7);
        assert!(is_unique(&scan, 0, &mut ctx), "the key field is unique");
        assert!(!is_unique(&scan, 1, &mut ctx), "a text column is not");
    }

    /// The answer a join asks of its other side is the same answer one level down, so a key
    /// that a join below already multiplied stops counting as unique.
    #[pg_test]
    fn a_join_keeps_uniqueness_only_while_the_other_side_has_it() {
        let mut ctx = ctx_with_key_field(7);
        let left = scan_of(7, 1);
        let right = scan_of(7, 2);

        let on_key = vec![(
            Arc::new(Column::new("id", 0)) as Arc<dyn datafusion::physical_expr::PhysicalExpr>,
            Arc::new(Column::new("id", 0)) as Arc<dyn datafusion::physical_expr::PhysicalExpr>,
        )];
        let on_text = vec![(
            Arc::new(Column::new("id", 0)) as Arc<dyn datafusion::physical_expr::PhysicalExpr>,
            Arc::new(Column::new("title", 1)) as Arc<dyn datafusion::physical_expr::PhysicalExpr>,
        )];

        let key_join = hash_join(Arc::clone(&left), Arc::clone(&right), on_key);
        assert!(
            is_unique(&key_join, 0, &mut ctx),
            "the left key field survives a join on the right key field"
        );

        let text_join = hash_join(left, right, on_text);
        assert!(
            !is_unique(&text_join, 0, &mut ctx),
            "the right side's key is not unique, so the left rows repeat"
        );
    }

    fn hash_join(
        left: Arc<dyn ExecutionPlan>,
        right: Arc<dyn ExecutionPlan>,
        on: Vec<(
            Arc<dyn datafusion::physical_expr::PhysicalExpr>,
            Arc<dyn datafusion::physical_expr::PhysicalExpr>,
        )>,
    ) -> Arc<dyn ExecutionPlan> {
        Arc::new(
            HashJoinExec::try_new(
                left,
                right,
                on,
                None,
                &JoinType::Inner,
                None,
                datafusion::physical_plan::joins::PartitionMode::CollectLeft,
                datafusion::common::NullEquality::NullEqualsNothing,
                false,
            )
            .unwrap(),
        ) as Arc<dyn ExecutionPlan>
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
                deferred_field("title"),
            ])),
            SearchQueryInput::All,
            None,
            vec![DeferredField {
                name: "title".into(),
                is_bytes: false,
                canonical: canonical.clone(),
                heap_rti: 1,
                plan_position: None,
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
        ffhelpers.insert((None, indexrelid), Arc::new(FFHelper::empty()));
        let decode = Arc::new(
            TantivyDecodeExec::new(
                projection,
                vec![PhysicalDeferredField {
                    col_idx: 0,
                    display_name: "title".into(),
                    is_bytes: false,
                    canonical: canonical.clone(),
                    heap_rti: 1,
                    plan_position: None,
                    rebuild: None,
                }],
                ffhelpers,
            )
            .unwrap(),
        ) as Arc<dyn ExecutionPlan>;

        let mut decisions = HashMap::default();
        decisions.insert(
            DeferredSource {
                heap_rti: 1,
                ff_index: canonical.ff_index,
            },
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
