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

//! JoinScan: Custom scan operator for optimizing joins with BM25 full-text search.
//!
//! JoinScan intercepts PostgreSQL join operations and executes them using Tantivy's
//! search capabilities combined with a join algorithm, providing significant
//! performance improvements for queries that combine full-text search with joins.
//!
//! # Activation Conditions
//!
//! JoinScan is proposed by the planner when **all** of the following conditions are met.
//! These restrictions ensure that we can execute the join efficiently using Tantivy's
//! columnar storage (fast fields) and minimize expensive heap access.
//!
//! The core strategy is **late materialization**:
//! 1. Execute the search and join using ONLY the index (fast fields).
//! 2. Apply sorting and limits on the joined index data.
//! 3. Only access the PostgreSQL heap (materialize) for the final result rows (Top K).
//!
//! This strategy requires that all data needed for the join, filter, and sort phases
//! resides in fast fields, and that the result set size is small enough (via LIMIT)
//! that the random heap access cost doesn't outweigh the join benefit.
//!
//! 1. **GUC enabled**: `paradedb.enable_join_custom_scan = on` (default: on)
//!
//! 2. **Join type**: INNER, SEMI, and ANTI joins are supported
//!    - LEFT, RIGHT, and FULL joins are planned for future work
//!
//! 3. **LIMIT clause**: Query must have a LIMIT clause
//!    - This ensures we only pay the cost of "late materialization" (random heap access)
//!      for a small number of rows. Without LIMIT, scanning the entire index and fetching
//!      all rows from the heap is often slower than PostgreSQL's native execution.
//!    - Future work will allow no-limit joins when both sides have search predicates.
//!
//! 4. **Search predicate**: At least one side must have:
//!    - A BM25 index on the table
//!    - A `@@@` search predicate in the WHERE clause
//!
//! 5. **Multi-level Joins**: JoinScan supports multi-level joins (e.g., `(A JOIN B) JOIN C`).
//!    It achieves this by reconstructing the join tree from PostgreSQL's plan or by nesting
//!    multiple JoinScan operators.
//!
//! 6. **Fast-field columns**: All columns used in the join must be fast fields in their
//!    respective BM25 indexes. This allows the join to be executed entirely within the index:
//!    - Equi-join keys (e.g., `a.id = b.id`) must be fast fields for join execution
//!    - Multi-table predicates (e.g., `a.price > b.min_price`) must reference fast fields
//!    - ORDER BY columns must be fast fields for efficient sorting
//!    - If any required column is not a fast field, we would need to access the heap
//!      during the join, breaking the late materialization strategy.
//!
//! 7. **Equi-join keys required**: At least one equi-join key (e.g., `a.id = b.id`) is
//!    required. Cross joins (cartesian products) fall back to PostgreSQL
//!
//! # Example Queries
//!
//! ```sql
//! -- JoinScan IS proposed (has LIMIT, has @@@ predicate)
//! SELECT p.name, s.name
//! FROM products p
//! JOIN suppliers s ON p.supplier_id = s.id
//! WHERE p.description @@@ 'wireless'
//! LIMIT 10;
//!
//! -- JoinScan is NOT proposed (no LIMIT)
//! SELECT p.name, s.name
//! FROM products p
//! JOIN suppliers s ON p.supplier_id = s.id
//! WHERE p.description @@@ 'wireless';
//!
//! -- JoinScan is NOT proposed (LEFT JOIN not supported)
//! SELECT p.name, s.name
//! FROM products p
//! LEFT JOIN suppliers s ON p.supplier_id = s.id
//! WHERE p.description @@@ 'wireless'
//! LIMIT 10;
//!
//! -- JoinScan IS proposed if price/min_price are fast fields in BM25 indexes
//! SELECT p.name, s.name
//! FROM products p
//! JOIN suppliers s ON p.supplier_id = s.id
//! WHERE p.description @@@ 'wireless' AND p.price > s.min_price
//! LIMIT 10;
//!
//! -- JoinScan is NOT proposed if price is NOT a fast field
//! -- (falls back to PostgreSQL's native join)
//! ```
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐     ┌──────────────────┐     ┌─────────────────────┐
//! │   PostgreSQL    │     │    JoinScan      │     │     DataFusion      │
//! │   Planner       │────▶│   Custom Scan    │────▶│   Execution Plan    │
//! │   (hook)        │     │   (planning +    │     │                     │
//! │                 │     │    execution)    │     │                     │
//! └─────────────────┘     └──────────────────┘     └─────────────────────┘
//!                                                             │
//!                                                             ▼
//!                                                  ┌─────────────────────┐
//!                                                  │      Tantivy        │
//!                                                  │    (Scan + Search)  │
//!                                                  └─────────────────────┘
//! ```
//!
//! ## Execution Strategy
//!
//! 1. **Planning**: During PostgreSQL planning, `JoinScan` hooks into the join path list.
//!    It identifies potential search joins (including reconstructing multi-level joins from
//!    PostgreSQL's optimal paths), extracts predicates, and builds a `JoinCSClause`.
//! 2. **Execution**: A DataFusion logical plan is constructed from the `JoinCSClause`.
//!    This plan defines the join, filters, sorts, and limits.
//! 3. **DataFusion**: The plan is executed by DataFusion, which chooses the best join algorithm.
//!    - Scans results from Tantivy for all relations, filtering by search predicates where applicable.
//! 4. **Result**: Joined tuples are returned to PostgreSQL via the Custom Scan interface.
//!
//! # Submodules
//!
//! - [`build`]: Data structures for planning serialization.
//! - [`planning`]: Cost estimation, condition extraction, field collection, pathkey handling.
//! - [`predicate`]: Transform PostgreSQL expressions to evaluable expression trees.
//! - [`scan_state`]: Execution state and DataFusion plan building.
//! - [`translator`]: Maps PostgreSQL expressions/columns to DataFusion expressions.
//! - [`privdat`]: Private data serialization between planning and execution.
//! - [`explain`]: EXPLAIN output formatting.

pub mod build;
#[allow(dead_code, deprecated)]
pub mod exchange;
#[allow(dead_code)]
pub mod parallel;
pub mod planner;
mod planning;
pub mod predicate;
pub mod privdat;
#[allow(dead_code, deprecated)]
pub mod sanitize;
pub mod scan_state;
pub mod transport;
pub mod visibility_filter;

pub use self::build::CtidColumn;
use self::build::{JoinCSClause, RelNode, RelationAlias};
use self::planning::{
    collect_join_sources, collect_join_sources_base_rel, collect_required_fields,
    ensure_score_bubbling, expr_uses_scores_from_source, extract_join_conditions, extract_orderby,
    extract_orderby_from_parse_sort_clause, get_score_func_rti, order_by_columns_are_fast_fields,
    pathkey_uses_scores_from_source,
};
use self::predicate::extract_join_level_conditions;
use self::privdat::PrivateData;
use crate::postgres::customscan::datafusion::explain::{format_join_level_expr, get_attname_safe};
use crate::postgres::customscan::pullup::resolve_fast_field;

use self::scan_state::{
    build_joinscan_logical_plan, build_physical_plan, build_task_context,
    create_datafusion_session_context, JoinScanState, SessionContextProfile,
};
use crate::api::HashSet;
use crate::api::OrderByFeature;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexManifest;
use crate::postgres::customscan::builders::custom_path::{CustomPathBuilder, Flags};
use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::builders::custom_state::{
    CustomScanStateBuilder, CustomScanStateWrapper,
};
use crate::postgres::customscan::dsm::ParallelQueryCapable;
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::joinscan::planning::distinct_columns_are_fast_fields;
use crate::postgres::customscan::limit_offset::LimitOffset;
use crate::postgres::customscan::parallel::compute_nworkers;
use crate::postgres::customscan::{CustomScan, JoinPathlistHookArgs};
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::ParallelScanState;
use crate::scan::codec::{deserialize_logical_plan_with_runtime, serialize_logical_plan};
use datafusion::physical_plan::displayable;
use datafusion::physical_plan::metrics::MetricValue;
use datafusion::physical_plan::{DisplayFormatType, ExecutionPlan};
use futures::StreamExt;
use pgrx::{pg_guard, pg_sys, PgList};
use std::ffi::{c_void, CStr};

#[derive(Default)]
pub struct JoinScan;

/// Output of [`JoinScan::try_build_join_custom_path`] when activation succeeds.
struct BuiltJoinPath {
    path: pg_sys::CustomPath,
    aliases: Vec<String>,
    multi_table_clauses: Vec<*mut pg_sys::Expr>,
}

/// Why the join scan declined to produce a custom path.
///
/// `Quiet` is for the early "this isn't even a candidate join" gates;
/// `Warn` is for validation failures past the "considered interesting"
/// boundary, where we owe the planner a `NOTICE`.
enum JoinPathDecline {
    Quiet,
    Warn {
        reason: JoinDeclineReason,
        aliases: Vec<String>,
    },
}

/// Specific reason a `JoinPathDecline::Warn` was raised. Each variant maps
/// 1:1 to a planner-warning message we used to emit inline.
enum JoinDeclineReason {
    NoEquiKeys,
    UnsupportedJoinType(Vec<String>),
    PrunedJoinKey,
    ActivationFailed,
    JoinLevelExtractionFailed,
    OrderByUnavailable,
    Other(String),
}

impl JoinDeclineReason {
    fn emit(&self, aliases: &[String]) {
        match self {
            JoinDeclineReason::NoEquiKeys => JoinScan::add_planner_warning(
                "JoinScan not used: at least one equi-join key (e.g., a.id = b.id) is required",
                aliases,
            ),
            JoinDeclineReason::UnsupportedJoinType(types) => {
                JoinScan::add_detailed_planner_warning(
                    "JoinScan not used: only INNER, ANTI, and SEMI JOIN are currently supported",
                    aliases,
                    types.clone(),
                )
            }
            JoinDeclineReason::PrunedJoinKey => JoinScan::add_planner_warning(
                "JoinScan not used: a semi/anti join prunes columns required by an outer join key and no equivalent output-visible column was found",
                aliases,
            ),
            JoinDeclineReason::ActivationFailed => JoinScan::add_planner_warning(
                "JoinScan not used: activation checks failed (LIMIT / BM25 index / fast fields / aggregates)",
                aliases,
            ),
            JoinDeclineReason::JoinLevelExtractionFailed => JoinScan::add_planner_warning(
                "JoinScan not used: failed to extract join-level conditions (ensure all referenced columns are fast fields)",
                aliases,
            ),
            JoinDeclineReason::OrderByUnavailable => JoinScan::add_planner_warning(
                "JoinScan not used: ORDER BY column is not available in the joined output schema",
                aliases,
            ),
            JoinDeclineReason::Other(msg) => {
                JoinScan::add_planner_warning(msg.clone(), ());
            }
        }
    }
}

/// Recursively walk an expression tree and collect the `plan_id` of every
/// `T_SubPlan` node found at any depth.  Uses Postgres's
/// `expression_tree_walker` so it handles all node types automatically.
unsafe fn collect_all_subplan_ids_from_expr(node: *mut pg_sys::Node, ids: &mut HashSet<i32>) {
    if node.is_null() {
        return;
    }

    #[pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        context: *mut std::ffi::c_void,
    ) -> bool {
        if node.is_null() {
            return false;
        }
        if (*node).type_ == pg_sys::NodeTag::T_SubPlan {
            let subplan = node as *mut pg_sys::SubPlan;
            let ids = &mut *(context as *mut HashSet<i32>);
            ids.insert((*subplan).plan_id);
        }
        pg_sys::expression_tree_walker(node, Some(walker), context)
    }

    walker(node, ids as *mut HashSet<i32> as *mut std::ffi::c_void);
}

/// Collect all SubPlan `plan_id`s present in `baserestrictinfo` of the
/// given base relations.
unsafe fn collect_all_subplan_ids_from_baserestrictinfo(
    root: *mut pg_sys::PlannerInfo,
    absorbed_rtis: &[pg_sys::Index],
) -> HashSet<i32> {
    let mut all_ids = HashSet::default();
    for rti in absorbed_rtis {
        let rel = pg_sys::find_base_rel(root, *rti as i32);
        let ri_list = PgList::<pg_sys::RestrictInfo>::from_pg((*rel).baserestrictinfo);
        for ri in ri_list.iter_ptr() {
            let clause = (*ri).clause as *mut pg_sys::Node;
            collect_all_subplan_ids_from_expr(clause, &mut all_ids);
        }
    }
    all_ids
}

/// Collect the `plan_id`s of SubPlans that JoinScan absorbed into
/// Semi/Anti/LeftMark join nodes in the `RelNode` tree.
fn collect_absorbed_subplan_ids(plan: &RelNode) -> HashSet<i32> {
    let mut ids = HashSet::default();
    walk_relnode_for_subplan_ids(plan, &mut ids);
    ids
}

fn walk_relnode_for_subplan_ids(node: &RelNode, ids: &mut HashSet<i32>) {
    match node {
        RelNode::Join(j) => {
            if let Some(plan_id) = j.subplan_id {
                ids.insert(plan_id);
            }
            walk_relnode_for_subplan_ids(&j.left, ids);
            walk_relnode_for_subplan_ids(&j.right, ids);
        }
        RelNode::Filter(f) => walk_relnode_for_subplan_ids(&f.input, ids),
        RelNode::Scan(_) => {}
    }
}

/// Check whether it is safe to push LIMIT into the JoinScan plan.
///
/// Returns `true` when ALL of:
/// 1. JoinScan absorbed every base relation in the query (no outer
///    relations that could add post-filters above JoinScan).
/// 2. Every SubPlan in `baserestrictinfo` of absorbed relations was also
///    absorbed into the `RelNode` tree (Semi/Anti/LeftMark joins).
///    Un-absorbed SubPlans would become Postgres post-filters above
///    the capped output.
/// 3. No volatile functions in `baserestrictinfo` of absorbed relations
///    (volatile functions can never be pushed into Tantivy).
unsafe fn is_limit_pushdown_safe(
    root: *mut pg_sys::PlannerInfo,
    join_clause: &JoinCSClause,
) -> bool {
    let absorbed_rtis: Vec<pg_sys::Index> = join_clause
        .plan
        .sources()
        .iter()
        .map(|s| s.scan_info.heap_rti)
        .collect();

    // 1. Did JoinScan absorb ALL base relations?
    #[cfg(feature = "pg15")]
    let all_rels = (*root).all_baserels;
    #[cfg(any(feature = "pg16", feature = "pg17", feature = "pg18"))]
    let all_rels = (*root).all_query_rels;

    let mut absorbed_bms: *mut pg_sys::Bitmapset = std::ptr::null_mut();
    for rti in &absorbed_rtis {
        absorbed_bms = pg_sys::bms_add_member(absorbed_bms, *rti as i32);
    }
    if !pg_sys::bms_is_subset(all_rels, absorbed_bms) {
        return false;
    }

    // 2. Every SubPlan in baserestrictinfo must have been absorbed.
    let all_subplan_ids = collect_all_subplan_ids_from_baserestrictinfo(root, &absorbed_rtis);
    let absorbed_subplan_ids = collect_absorbed_subplan_ids(&join_clause.plan);
    for id in &all_subplan_ids {
        if !absorbed_subplan_ids.contains(id) {
            return false;
        }
    }

    // 3. No volatile functions (these can never be absorbed).
    for rti in &absorbed_rtis {
        let rel = pg_sys::find_base_rel(root, *rti as i32);
        let ri_list = PgList::<pg_sys::RestrictInfo>::from_pg((*rel).baserestrictinfo);
        for ri in ri_list.iter_ptr() {
            let clause = (*ri).clause as *mut pg_sys::Node;
            if pg_sys::contain_volatile_functions(clause) {
                return false;
            }
        }
    }

    true
}

/// Try to create JoinScan `CustomPath`s for a single base relation that contains
/// SubPlan-based join opportunities (e.g. `col IN (SELECT ...) OR col IS NULL`).
///
/// Called from `set_rel_pathlist_hook` after BaseScan has been considered.
/// When PostgreSQL keeps a subquery as a SubPlan instead of flattening it into
/// a join, `set_join_pathlist_hook` never fires.  This function gives JoinScan
/// a chance to handle those patterns.
pub unsafe fn try_create_subplan_join_paths(
    root: *mut pg_sys::PlannerInfo,
    rel: *mut pg_sys::RelOptInfo,
    rti: pg_sys::Index,
) -> Vec<pg_sys::CustomPath> {
    use crate::postgres::customscan::range_table::bms_iter;

    if !crate::gucs::enable_join_custom_scan() {
        return Vec::new();
    }

    // Only consider base relations (single RTI).
    let relids = (*rel).relids;
    if relids.is_null() || pg_sys::bms_num_members(relids) != 1 {
        return Vec::new();
    }
    let base_rti = match bms_iter(relids).next() {
        Some(r) => r,
        None => return Vec::new(),
    };
    if base_rti != rti {
        return Vec::new();
    }

    // Quick pre-check: only proceed if baserestrictinfo contains an OR expression
    // with a SubPlan inside. This avoids interfering with normal queries where
    // set_join_pathlist_hook already handles SubPlans.
    {
        use crate::postgres::customscan::qual_inspect::is_subplan;
        let bri = PgList::<pg_sys::RestrictInfo>::from_pg((*rel).baserestrictinfo);
        let has_or_subplan = bri.iter_ptr().any(|ri| {
            let clause = (*ri).clause as *mut pg_sys::Node;
            if clause.is_null() {
                return false;
            }
            // Check if the clause itself is an OR BoolExpr containing a SubPlan.
            // Top-level SubPlans are handled by the normal join_pathlist hook.
            if (*clause).type_ == pg_sys::NodeTag::T_BoolExpr {
                let bexpr = clause as *mut pg_sys::BoolExpr;
                (*bexpr).boolop == pg_sys::BoolExprType::OR_EXPR && is_subplan(clause, root)
            } else {
                false
            }
        });
        if !has_or_subplan {
            return Vec::new();
        }
    }

    // Try to extract SubPlan-based joins from baserestrictinfo.
    let (plan, join_keys) = match collect_join_sources_base_rel(root, rel, rti) {
        Some(res) => res,
        None => return Vec::new(),
    };

    // Only proceed if the plan actually contains a join (from SubPlan extraction).
    if !plan.has_semi_or_anti() {
        return Vec::new();
    }

    // Phase 1: validate + build JoinCSClause.
    let has_distinct = !(*(*root).parse).distinctClause.is_null();
    let (join_clause, limit_offset) =
        match JoinScan::validate_and_build_clause(root, plan, &join_keys, has_distinct) {
            Some(res) => res,
            None => return Vec::new(),
        };

    // No join-level predicate extraction needed for SubPlan-based paths.

    // Phase 2: finalize into CustomPath.
    match JoinScan::finalize_clause_into_path(root, rel, join_clause, &limit_offset, false) {
        Some(path) => vec![path],
        None => Vec::new(),
    }
}

impl JoinScan {
    /// Phase 1: Validate a `RelNode` plan against JoinScan activation requirements
    /// and build a `JoinCSClause` with score bubbling and partitioning applied.
    ///
    /// Returns `None` if any activation check fails. The caller can then optionally
    /// perform join-level predicate extraction on the returned clause before
    /// passing it to [`finalize_clause_into_path`].
    unsafe fn validate_and_build_clause(
        root: *mut pg_sys::PlannerInfo,
        plan: RelNode,
        join_keys: &[build::JoinKeyPair],
        has_distinct: bool,
    ) -> Option<(JoinCSClause, LimitOffset)> {
        let all_sources = plan.sources();

        // --- Activation checks ---
        // NOTE: We do NOT check has_search_predicate here. The caller is
        // responsible for that check because the join-hook path also considers
        // join_conditions.has_search_predicate, which is not available to us.

        if all_sources
            .iter()
            .any(|s| s.scan_info.indexrelid == pg_sys::InvalidOid)
        {
            return None;
        }

        if (*(*root).parse).hasAggs {
            return None;
        }

        // Require LIMIT for top-level queries (without it, JoinScan's TopK
        // optimization has no bound). Subqueries are exempt because the parent
        // plan provides the cardinality constraint.
        let limit_offset = LimitOffset::from_root(root);
        let is_subquery = !(*root).parent_root.is_null();
        if limit_offset.limit.is_none() && !is_subquery {
            return None;
        }

        if join_keys.is_empty() {
            return None;
        }

        if has_distinct && distinct_columns_are_fast_fields(root, &all_sources).is_none() {
            return None;
        }

        if !order_by_columns_are_fast_fields(root, &all_sources, has_distinct) {
            return None;
        }

        for jk in join_keys {
            let outer_source = all_sources.iter().find(|s| s.contains_rti(jk.outer_rti));
            let inner_source = all_sources.iter().find(|s| s.contains_rti(jk.inner_rti));
            match (outer_source, inner_source) {
                (Some(outer), Some(inner)) => {
                    let outer_hr = PgSearchRelation::open(outer.scan_info.heaprelid);
                    let outer_ir = PgSearchRelation::open(outer.scan_info.indexrelid);
                    let inner_hr = PgSearchRelation::open(inner.scan_info.heaprelid);
                    let inner_ir = PgSearchRelation::open(inner.scan_info.indexrelid);
                    if resolve_fast_field(jk.outer_attno as i32, &outer_hr.tuple_desc(), &outer_ir)
                        .is_none()
                        || resolve_fast_field(
                            jk.inner_attno as i32,
                            &inner_hr.tuple_desc(),
                            &inner_ir,
                        )
                        .is_none()
                    {
                        return None;
                    }
                }
                _ => return None,
            }
        }

        // --- Build JoinCSClause ---

        let mut join_clause = JoinCSClause::new(plan)
            .with_limit(limit_offset.limit)
            .with_offset(limit_offset.offset)
            .with_distinct(has_distinct);

        for source in join_clause.plan.sources_mut() {
            let score_in_tlist =
                expr_uses_scores_from_source((*root).processed_tlist.cast(), source);
            let score_in_pathkey = pathkey_uses_scores_from_source(root, source);
            if score_in_tlist || score_in_pathkey {
                ensure_score_bubbling(source);
            }
        }

        if join_clause.plan.has_semi_or_anti() {
            if join_clause.partitioning_source_index() != 0 {
                pgrx::warning!(
                    "For SEMI/ANTI/LeftMark join correctness, JoinScan needs to use a suboptimal \
                     parallel partitioning strategy for this query. See \
                     https://github.com/paradedb/paradedb/issues/4152"
                );
            }
            join_clause = join_clause.with_forced_partitioning(0);
        }

        // Safety check: bail out if LIMIT pushdown is unsafe due to
        // un-absorbed relations, un-absorbed SubPlans, or volatile functions.
        if join_clause.limit_offset.limit.is_some() && !is_limit_pushdown_safe(root, &join_clause) {
            let aliases: Vec<String> = join_clause
                .plan
                .sources()
                .iter()
                .map(|s| {
                    RelationAlias::new(s.scan_info.alias.as_deref())
                        .warning_context(s.scan_info.heaprelid)
                })
                .collect();
            JoinScan::add_planner_warning(
                "JoinScan not used: LIMIT pushdown unsafe (un-absorbed \
                 relations, SubPlans, or volatile functions)",
                &aliases,
            );
            return None;
        }

        Some((join_clause, limit_offset))
    }

    /// Phase 2: Finalize a validated `JoinCSClause` into a `CustomPath` by
    /// extracting ORDER BY, computing costs/parallel workers, and building the
    /// `pg_sys::CustomPath` struct.
    ///
    /// Returns `None` if ORDER BY extraction fails.
    unsafe fn finalize_clause_into_path(
        root: *mut pg_sys::PlannerInfo,
        rel: *mut pg_sys::RelOptInfo,
        mut join_clause: JoinCSClause,
        limit_offset: &LimitOffset,
        consider_parallel: bool,
    ) -> Option<pg_sys::CustomPath> {
        let output_rtis = join_clause.plan.output_rtis();
        let current_sources = join_clause.plan.sources();
        let order_by = extract_orderby(
            root,
            &current_sources,
            &output_rtis,
            join_clause.has_distinct,
        )?;
        join_clause = join_clause.with_order_by(order_by);

        // --- Cost estimation ---

        let startup_cost = crate::DEFAULT_STARTUP_COST;
        let total_cost = startup_cost + 1.0;
        let mut result_rows = limit_offset.limit.map(|l| l as f64).unwrap_or(1000.0);

        let (segment_count, row_estimate) = {
            let src = join_clause.partitioning_source();
            (src.scan_info.segment_count, src.scan_info.estimate)
        };

        let nworkers = if consider_parallel {
            let declares_sorted_output = !join_clause.order_by.is_empty();
            compute_nworkers(
                declares_sorted_output,
                limit_offset.limit.map(|l| l as f64),
                row_estimate,
                segment_count,
                false,
                false,
                true,
            )
        } else {
            0
        };

        if nworkers > 0 {
            let processes = std::cmp::max(
                1,
                nworkers
                    + if pg_sys::parallel_leader_participation {
                        1
                    } else {
                        0
                    },
            );
            result_rows /= processes as f64;
            let processes = processes as u64;
            let partitioning_idx = join_clause.partitioning_source_index();
            for (idx, source) in join_clause.plan.sources_mut().into_iter().enumerate() {
                if let crate::scan::info::RowEstimate::Known(n) = source.scan_info.estimate {
                    source.scan_info.estimated_rows_per_worker = if idx == partitioning_idx {
                        Some(n / processes)
                    } else {
                        Some(n)
                    };
                }
            }
        } else {
            for source in join_clause.plan.sources_mut() {
                if let crate::scan::info::RowEstimate::Known(n) = source.scan_info.estimate {
                    source.scan_info.estimated_rows_per_worker = Some(n);
                }
            }
        }

        // Store planned worker count for MPP execution path.
        join_clause.planned_workers = nworkers;

        // --- Build CustomPath ---

        let has_order_by = !join_clause.order_by.is_empty();
        let order_by_len = join_clause.order_by.len();
        let relevant_pathkeys =
            planning::count_relevant_pathkeys(root, &join_clause.plan.sources());
        let private_data = PrivateData::new(join_clause);
        let mut custom_path = pg_sys::CustomPath {
            path: pg_sys::Path {
                type_: pg_sys::NodeTag::T_CustomPath,
                pathtype: pg_sys::NodeTag::T_CustomScan,
                parent: rel,
                pathtarget: (*rel).reltarget,
                param_info: pg_sys::get_baserel_parampathinfo(
                    root,
                    rel,
                    pg_sys::bms_copy((*rel).lateral_relids),
                ),
                rows: result_rows,
                startup_cost,
                total_cost,
                ..Default::default()
            },
            flags: Flags::Force as u32,
            methods: JoinScan::custom_path_methods(),
            custom_private: private_data.into(),
            custom_paths: std::ptr::null_mut(),
            ..Default::default()
        };

        if has_order_by && order_by_len == relevant_pathkeys {
            custom_path.path.pathkeys = (*root).query_pathkeys;
        }

        if nworkers > 0 {
            custom_path.path.parallel_aware = true;
            custom_path.path.parallel_safe = true;
            custom_path.path.parallel_workers =
                nworkers.try_into().expect("nworkers should be a valid i32");
        }

        Some(custom_path)
    }
}

impl ParallelQueryCapable for JoinScan {
    fn estimate_dsm_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        _pcxt: *mut pg_sys::ParallelContext,
    ) -> pg_sys::Size {
        // Size DSM from actual execution-time segment counts (via manifests) rather
        // than planning-time scan_info.segment_count, which can diverge under
        // concurrent inserts.
        Self::ensure_source_manifests(state);

        let join_clause = &state.custom_state().join_clause;
        let partitioning_idx = join_clause.partitioning_source_index();
        let all_nsegments: Vec<usize> = state
            .custom_state()
            .source_manifests
            .iter()
            .map(SearchIndexManifest::segment_count)
            .collect();

        ParallelScanState::size_of(&all_nsegments, partitioning_idx, &[], false)
    }

    fn initialize_dsm_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        _pcxt: *mut pg_sys::ParallelContext,
        coordinate: *mut c_void,
    ) {
        let pscan_state = coordinate.cast::<ParallelScanState>();
        assert!(!pscan_state.is_null(), "coordinate is null");

        Self::ensure_source_manifests(state);

        let join_clause = state.custom_state().join_clause.clone();
        let partitioning_idx = join_clause.partitioning_source_index();

        unsafe {
            let all_sources: Vec<&[tantivy::SegmentReader]> = state
                .custom_state()
                .source_manifests
                .iter()
                .map(|manifest| manifest.segment_readers())
                .collect();
            let args = crate::postgres::ParallelScanArgs {
                all_sources,
                partitioning_source_idx: partitioning_idx,
                query: vec![], // JoinScan passes query via PrivateData, not shared state
                with_aggregates: false,
            };
            (*pscan_state).create_and_populate(args);
        }

        // Read the canonical non-partitioning segment ID sets from shared memory.
        // The leader uses these in exec_custom_scan to populate the execution codec.
        let non_partitioning_segments = unsafe { (*pscan_state).non_partitioning_segment_ids() };

        state.custom_state_mut().parallel_state = Some(pscan_state);
        state.custom_state_mut().non_partitioning_segments = non_partitioning_segments;
    }

    fn reinitialize_dsm_custom_scan(
        _state: &mut CustomScanStateWrapper<Self>,
        _pcxt: *mut pg_sys::ParallelContext,
        coordinate: *mut c_void,
    ) {
        let pscan_state = coordinate.cast::<ParallelScanState>();
        assert!(!pscan_state.is_null(), "coordinate is null");
        unsafe {
            (*pscan_state).reset();
        }
    }

    fn initialize_worker_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        _toc: *mut pg_sys::shm_toc,
        coordinate: *mut c_void,
    ) {
        let pscan_state = coordinate.cast::<ParallelScanState>();
        assert!(!pscan_state.is_null(), "coordinate is null");

        state.custom_state_mut().parallel_state = Some(pscan_state);

        // Workers must wait for the leader to finish populating the segment pool.
        unsafe {
            (*pscan_state).wait_for_initialization();
        }

        // Read the canonical non-partitioning segment ID sets that the leader wrote to
        // shared memory.  These are used in exec_custom_scan to ensure every worker opens
        // each replicated source with MvccSatisfies::ParallelWorker, which pins the
        // visible segment list to exactly what the leader snapshotted.
        let non_partitioning_segments = unsafe { (*pscan_state).non_partitioning_segment_ids() };
        state.custom_state_mut().non_partitioning_segments = non_partitioning_segments;

        // We don't need to deserialize query from parallel state for JoinScan
        // because the full plan (including query) is serialized in PrivateData
        // and available to the worker via the plan.
    }
}

impl JoinScan {
    /// Capture lightweight segment manifests for all join sources.
    ///
    /// Uses `SearchIndexManifest::capture` instead of opening full `SearchIndexReader`s
    /// because this runs during DSM initialization (`estimate_dsm` / `initialize_dsm`),
    /// which is part of executor startup — before `begin_custom_scan` sets up executor
    /// state. Opening full readers would call `into_tantivy_query` on each source's
    /// `scan_info.query`, which fails for parameterized predicates (prepared statements,
    /// initplan-backed subqueries) that require a `PlanState` to evaluate.
    ///
    /// Manifests also provide consistent segment counts for both DSM sizing and DSM
    /// population, avoiding the divergence that can occur when planning-time counts
    /// (from `scan_info.segment_count`) differ from execution-time counts due to
    /// concurrent inserts.
    fn ensure_source_manifests(state: &mut CustomScanStateWrapper<Self>) {
        if !state.custom_state().source_manifests.is_empty() {
            return;
        }

        let manifests = state
            .custom_state()
            .join_clause
            .plan
            .sources()
            .iter()
            .map(|source| {
                let rel = PgSearchRelation::open(source.scan_info.indexrelid);
                SearchIndexManifest::capture(&rel, MvccSatisfies::Snapshot).unwrap_or_else(|e| {
                    panic!(
                        "Failed to capture source manifest for indexrelid {}: {e}",
                        source.scan_info.indexrelid
                    )
                })
            })
            .collect();

        state.custom_state_mut().source_manifests = manifests;
    }

    /// Build plan_position → canonical segment IDs map for SearchPredicateUDF.
    ///
    /// This is keyed by plan_position rather than indexrelid because it is a
    /// per-source contract, not just a per-index one. The same index can appear
    /// more than once in one JoinScan plan; in parallel execution those source
    /// copies can also carry different canonical segment sets (partitioned vs
    /// replicated). If this were keyed only by indexrelid, one source could
    /// inject another source's segment set and make packed DocAddresses resolve
    /// against the wrong segment ordering.
    ///
    /// Workers use frozen segment IDs from DSM to match the leader's segment set.
    /// Leader/serial uses manifests captured with the same snapshot.
    fn build_index_segment_ids(
        state: &mut CustomScanStateWrapper<Self>,
        join_clause: &JoinCSClause,
        plan_sources: &[&build::JoinSource],
    ) -> Vec<crate::api::HashSet<tantivy::index::SegmentId>> {
        let mut ids_by_pos = vec![None; plan_sources.len()];
        let partitioning_idx = join_clause.partitioning_source_index();
        let is_worker = unsafe { pg_sys::ParallelWorkerNumber >= 0 };

        if is_worker {
            let non_partitioning_segs = &state.custom_state().non_partitioning_segments;
            let mut np_counter = 0usize;
            for (i, _source) in plan_sources.iter().enumerate() {
                if i == partitioning_idx {
                    if let Some(ps) = state.custom_state().parallel_state {
                        let ids =
                            unsafe { crate::postgres::customscan::parallel::list_segment_ids(ps) };
                        ids_by_pos[i] = Some(ids);
                    }
                } else if let Some(ids) = non_partitioning_segs.get(np_counter) {
                    ids_by_pos[i] = Some(ids.clone());
                    np_counter += 1;
                }
            }
        } else {
            Self::ensure_source_manifests(state);
            for (i, _source) in plan_sources.iter().enumerate() {
                if let Some(manifest) = state.custom_state().source_manifests.get(i) {
                    let ids: crate::api::HashSet<_> = manifest
                        .segment_readers()
                        .iter()
                        .map(|r| r.segment_id())
                        .collect();
                    ids_by_pos[i] = Some(ids);
                }
            }
        }

        ids_by_pos
            .into_iter()
            .enumerate()
            .map(|(plan_position, ids)| {
                ids.unwrap_or_else(|| {
                    panic!(
                        "missing canonical segment IDs for join source at plan_position {}",
                        plan_position
                    )
                })
            })
            .collect()
    }

    fn source_queries_need_executor_state(join_clause: &JoinCSClause) -> bool {
        join_clause.plan.sources().iter().any(|source| {
            let mut query = source.scan_info.query.clone();
            query.has_postgres_expressions()
        })
    }
}

impl CustomScan for JoinScan {
    const NAME: &'static CStr = c"ParadeDB Join Scan";
    type Args = JoinPathlistHookArgs;
    type State = JoinScanState;
    type PrivateData = PrivateData;

    fn exec_methods() -> pg_sys::CustomExecMethods {
        pg_sys::CustomExecMethods {
            CustomName: Self::NAME.as_ptr(),
            BeginCustomScan: Some(crate::postgres::customscan::exec::begin_custom_scan::<Self>),
            ExecCustomScan: Some(crate::postgres::customscan::exec::exec_custom_scan::<Self>),
            EndCustomScan: Some(crate::postgres::customscan::exec::end_custom_scan::<Self>),
            ReScanCustomScan: Some(crate::postgres::customscan::exec::rescan_custom_scan::<Self>),
            MarkPosCustomScan: None,
            RestrPosCustomScan: None,
            EstimateDSMCustomScan: Some(
                crate::postgres::customscan::dsm::estimate_dsm_custom_scan::<Self>,
            ),
            InitializeDSMCustomScan: Some(
                crate::postgres::customscan::dsm::initialize_dsm_custom_scan::<Self>,
            ),
            ReInitializeDSMCustomScan: Some(
                crate::postgres::customscan::dsm::reinitialize_dsm_custom_scan::<Self>,
            ),
            InitializeWorkerCustomScan: Some(
                crate::postgres::customscan::dsm::initialize_worker_custom_scan::<Self>,
            ),
            ShutdownCustomScan: Some(
                crate::postgres::customscan::exec::shutdown_custom_scan::<Self>,
            ),
            ExplainCustomScan: Some(crate::postgres::customscan::exec::explain_custom_scan::<Self>),
        }
    }

    fn create_custom_path(builder: CustomPathBuilder<Self>) -> Vec<pg_sys::CustomPath> {
        unsafe {
            match Self::try_build_join_custom_path(&builder) {
                Ok(BuiltJoinPath {
                    path,
                    aliases,
                    multi_table_clauses,
                }) => {
                    let mut path = path;
                    if !multi_table_clauses.is_empty() {
                        let mut private_list = PgList::<pg_sys::Node>::from_pg(path.custom_private);
                        for clause in multi_table_clauses {
                            private_list.push(clause.cast());
                        }
                        path.custom_private = private_list.into_pg();
                    }
                    Self::mark_contexts_successful(&aliases);
                    vec![path]
                }
                Err(JoinPathDecline::Quiet) => Vec::new(),
                Err(JoinPathDecline::Warn { reason, aliases }) => {
                    reason.emit(&aliases);
                    Vec::new()
                }
            }
        }
    }

    fn plan_custom_path(mut builder: CustomScanBuilder<Self>) -> pg_sys::CustomScan {
        // For joins, scanrelid must be 0 (it's not scanning a single relation)
        builder.set_scanrelid(0);

        // Get best_path before builder is consumed
        let best_path = builder.args().best_path;
        let root = builder.args().root;

        let mut node = builder.build();

        unsafe {
            // For joins, we need to set custom_scan_tlist to describe the output columns.
            // Create a fresh copy of the target list to avoid corrupting the original.
            let original_tlist = node.scan.plan.targetlist;
            let copied_tlist = pg_sys::copyObjectImpl(original_tlist.cast()).cast::<pg_sys::List>();
            let tlist = PgList::<pg_sys::TargetEntry>::from_pg(copied_tlist);

            // For join custom scans, PostgreSQL doesn't pass clauses via the usual parameter.
            // We stored the restrictlist in custom_private during create_custom_path.
            //
            // Note: We do NOT add restrictlist clauses to custom_exprs because setrefs would try
            // to resolve their Vars using the child plans' target lists, which may not have all
            // the needed columns. Instead, we keep the restrictlist in custom_private and handle
            // join condition evaluation manually during execution using the original Var
            // references.

            // Extract the column mappings from the ORIGINAL targetlist (before we add restrictlist
            // Vars). The original_tlist has the SELECT's output columns, which is what
            // ps_ResultTupleSlot is based on. We store this mapping in PrivateData so
            // build_result_tuple can use it during execution.
            let mut private_data = PrivateData::from(node.custom_private);
            let original_entries = PgList::<pg_sys::TargetEntry>::from_pg(original_tlist);

            private_data.output_columns =
                compute_output_columns(&private_data.join_clause, original_tlist, root);

            build_output_projection(&mut private_data, &original_entries, root);

            // Add heap condition clauses to custom_exprs so they get transformed by
            // set_customscan_references. The Vars in these expressions will be converted to
            // INDEX_VAR references into custom_scan_tlist.
            node.custom_exprs = splice_path_private_into_list(node.custom_exprs, best_path);

            // Collect all required fields for execution
            collect_required_fields(
                &mut private_data.join_clause,
                &private_data.output_columns,
                node.custom_exprs,
            );

            bake_logical_plan(&mut private_data, node.custom_exprs);

            // Convert PrivateData back to a list and preserve the restrictlist.
            let private_list = PrivateData::into(private_data);
            node.custom_private = splice_path_private_into_list(private_list, best_path);

            // Set custom_scan_tlist with all needed columns
            node.custom_scan_tlist = tlist.into_pg();
        }
        node
    }

    fn create_custom_scan_state(
        mut builder: CustomScanStateBuilder<Self, Self::PrivateData>,
    ) -> *mut CustomScanStateWrapper<Self> {
        builder.custom_state().join_clause = builder.custom_private().join_clause.clone();
        builder.custom_state().output_columns = builder.custom_private().output_columns.clone();
        builder.custom_state().logical_plan = builder.custom_private().logical_plan.clone();
        builder.build()
    }

    fn explain_custom_scan(
        state: &CustomScanStateWrapper<Self>,
        _ancestors: *mut pg_sys::List,
        explainer: &mut Explainer,
    ) {
        let join_clause = &state.custom_state().join_clause;
        explainer.add_text("Relation Tree", join_clause.plan.explain());

        let mut base_relations = Vec::new();
        join_clause.collect_base_relations(&mut base_relations);

        fn collect_join_cond_strings(node: &RelNode, acc: &mut Vec<String>) {
            match node {
                RelNode::Scan(_) => {}
                RelNode::Filter(filter) => collect_join_cond_strings(&filter.input, acc),
                RelNode::Join(join) => {
                    for jk in &join.equi_keys {
                        let ((left_source, left_attno), (right_source, right_attno)) = jk
                            .resolve_against(&join.left, &join.right)
                            .unwrap_or_else(|| {
                                panic!(
                                    "Failed to resolve join key to current join sides: outer_rti={}, inner_rti={}",
                                    jk.outer_rti, jk.inner_rti
                                )
                            });

                        let (outer_source, outer_attno, inner_source, inner_attno) =
                            if join.left.contains_rti(jk.outer_rti)
                                && join.right.contains_rti(jk.inner_rti)
                            {
                                (left_source, left_attno, right_source, right_attno)
                            } else {
                                (right_source, right_attno, left_source, left_attno)
                            };

                        let outer_alias =
                            RelationAlias::new(outer_source.scan_info.alias.as_deref())
                                .display(outer_source.plan_position);
                        let inner_alias =
                            RelationAlias::new(inner_source.scan_info.alias.as_deref())
                                .display(inner_source.plan_position);

                        acc.push(format!(
                            "{} = {}",
                            get_attname_safe(
                                Some(outer_source.scan_info.heaprelid),
                                outer_attno,
                                &outer_alias
                            ),
                            get_attname_safe(
                                Some(inner_source.scan_info.heaprelid),
                                inner_attno,
                                &inner_alias
                            )
                        ));
                    }

                    collect_join_cond_strings(&join.left, acc);
                    collect_join_cond_strings(&join.right, acc);
                }
            }
        }

        let mut keys_str = Vec::new();
        collect_join_cond_strings(&join_clause.plan, &mut keys_str);
        if !keys_str.is_empty() {
            explainer.add_text("Join Cond", keys_str.join(", "));
        }

        if let Some(expr) = join_clause.plan.join_level_expr() {
            explainer.add_text("Join Predicate", format_join_level_expr(expr, join_clause));
        }

        if let Some(limit) = join_clause.limit_offset.limit {
            explainer.add_text("Limit", limit.to_string());
        }

        if let Some(offset) = join_clause.limit_offset.offset {
            if offset > 0 {
                explainer.add_text("Offset", offset.to_string());
            }
        }

        if join_clause.has_distinct {
            explainer.add_text("Distinct", "true");
        }

        if !join_clause.order_by.is_empty() {
            explainer.add_text(
                "Order By",
                join_clause
                    .order_by
                    .iter()
                    .map(|oi| match &oi.feature {
                        OrderByFeature::Field { name: f, .. } => {
                            format!("{} {}", f, oi.direction.as_ref())
                        }
                        OrderByFeature::Var { rti, attno, name } => {
                            if let Some(info) = base_relations.iter().find(|i| i.heap_rti == *rti) {
                                let col_name = get_attname_safe(
                                    Some(info.heaprelid),
                                    *attno,
                                    info.alias.as_deref().unwrap_or("?"),
                                );
                                format!("{} {}", col_name, oi.direction.as_ref())
                            } else {
                                format!(
                                    "{} {}",
                                    name.as_deref().unwrap_or("?"),
                                    oi.direction.as_ref()
                                )
                            }
                        }
                        other => {
                            format!("{other} {}", oi.direction.as_ref())
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        }

        if explainer.is_analyze() {
            // For EXPLAIN ANALYZE, render the plan with metrics inline.
            // VERBOSE includes timing; without VERBOSE, timing is stripped for stable output.
            if let Some(ref physical_plan) = state.custom_state().physical_plan {
                explainer.add_text("DataFusion Physical Plan", "");
                let mut lines = Vec::new();
                render_plan_with_metrics(
                    physical_plan.as_ref(),
                    0,
                    explainer.is_verbose(),
                    &mut lines,
                );
                for line in &lines {
                    explainer.add_text("  ", line);
                }
            }
        } else if let Some(ref logical_plan) = state.custom_state().logical_plan {
            // Plain EXPLAIN reconstructs the physical plan by deserializing the logical
            // plan and calling PgSearchTableProvider::scan(), but without executor state
            // (planstate=None). If any source query contains a PostgresExpression
            // (e.g., prepared-statement parameter), scan() would fail with "missing
            // planstate". Skip physical plan display for those cases.
            if Self::source_queries_need_executor_state(&state.custom_state().join_clause) {
                explainer.add_text(
                    "DataFusion Physical Plan",
                    "omitted for EXPLAIN because source queries require executor-time expression resolution",
                );
                return;
            }
            // For plain EXPLAIN, reconstruct the plan using the same session
            // configuration that execution uses so `VisibilityFilterExec`
            // appears in the displayed plan, matching EXPLAIN ANALYZE.
            let expr_context = crate::postgres::utils::ExprContextGuard::new();
            let ctx = create_datafusion_session_context(SessionContextProfile::Join);
            let runtime = tokio::runtime::Builder::new_current_thread()
                .build()
                .expect("Failed to create tokio runtime");
            let logical_plan = deserialize_logical_plan_with_runtime(
                logical_plan,
                &ctx.task_ctx(),
                None,
                Some(expr_context.as_ptr()),
                None,
                vec![],
                vec![],
            )
            .expect("Failed to deserialize logical plan");
            let physical_plan = runtime
                .block_on(build_physical_plan(&ctx, logical_plan))
                .expect("Failed to create execution plan");
            let displayable = displayable(physical_plan.as_ref());
            explainer.add_text("DataFusion Physical Plan", "");
            for line in displayable.indent(false).to_string().lines() {
                explainer.add_text("  ", line);
            }
        }
    }

    fn begin_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        estate: *mut pg_sys::EState,
        eflags: i32,
    ) {
        if eflags & (pg_sys::EXEC_FLAG_EXPLAIN_ONLY as i32) == 0 {
            unsafe {
                let planstate = state.planstate();
                // Always assign an ExprContext — heap filters, runtime
                // expressions, and pushed-down predicates may all need it.
                pg_sys::ExecAssignExprContext(estate, planstate);
                state.custom_state_mut().result_slot = Some(state.csstate.ss.ps.ps_ResultTupleSlot);
                state.runtime_context = state.csstate.ss.ps.ps_ExprContext;
            }
        }
    }

    fn rescan_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        state.custom_state_mut().reset();
    }

    fn exec_custom_scan(state: &mut CustomScanStateWrapper<Self>) -> *mut pg_sys::TupleTableSlot {
        unsafe {
            if state.custom_state().datafusion_stream.is_none() {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                let join_clause = state.custom_state().join_clause.clone();
                let snapshot = state.csstate.ss.ps.state.as_ref().unwrap().es_snapshot;

                let plan_sources = join_clause.plan.sources();
                for (plan_position, source) in plan_sources.iter().enumerate() {
                    let heaprelid = source.scan_info.heaprelid;
                    let heaprel = PgSearchRelation::open(heaprelid);
                    let visibility_checker =
                        VisibilityChecker::with_rel_and_snap(&heaprel, snapshot);
                    let fetch_slot =
                        pg_sys::MakeTupleTableSlot(heaprel.rd_att, &pg_sys::TTSOpsBufferHeapTuple);
                    state.custom_state_mut().relations.insert(
                        plan_position,
                        scan_state::RelationState {
                            _heaprel: heaprel,
                            visibility_checker,
                            fetch_slot,
                            ctid_col_idx: None,
                        },
                    );
                }

                let plan_bytes = state
                    .custom_state()
                    .logical_plan
                    .clone()
                    .expect("Logical plan is required");

                // MPP execution path: when the GUC is enabled and workers are planned,
                // use hash-partitioned MPP execution instead of broadcast-join.
                let _use_mpp = crate::gucs::enable_mpp_join() && join_clause.planned_workers > 0;

                // TODO(#4152): Wire the MPP execution path here. When _use_mpp is true:
                // 1. Call parallel::launch_join_workers() to set up workers + transport mesh
                // 2. Build physical plan with SessionContextProfile::JoinMpp
                // 3. Serialize and broadcast plan to workers
                // 4. Register DSM mesh for leader
                // 5. Spawn control service
                // 6. Execute partition 0 of the plan
                // For now, fall through to the broadcast-join path.

                // Deserialize the logical plan and convert to execution plan
                let planstate = state.planstate();

                let index_segment_ids =
                    Self::build_index_segment_ids(state, &join_clause, &plan_sources);

                let ctx = create_datafusion_session_context(SessionContextProfile::Join);
                let logical_plan = deserialize_logical_plan_with_runtime(
                    &plan_bytes,
                    &ctx.task_ctx(),
                    state.custom_state().parallel_state,
                    Some(state.runtime_context),
                    Some(planstate),
                    state.custom_state().non_partitioning_segments.clone(),
                    index_segment_ids,
                )
                .expect("Failed to deserialize logical plan");

                // Convert logical plan to physical plan
                let plan = runtime
                    .block_on(build_physical_plan(&ctx, logical_plan))
                    .expect("Failed to create execution plan");

                let task_ctx = build_task_context(
                    &ctx,
                    &plan,
                    pg_sys::work_mem as usize * 1024,
                    pg_sys::hash_mem_multiplier,
                );
                let stream = {
                    let _guard = runtime.enter();
                    plan.execute(0, task_ctx)
                        .expect("Failed to execute DataFusion plan")
                };

                // Retain the executed plan so EXPLAIN ANALYZE can extract metrics.
                state.custom_state_mut().physical_plan = Some(plan.clone());

                let schema = plan.schema();
                for (i, field) in schema.fields().iter().enumerate() {
                    if let Ok(ctid_col) = CtidColumn::try_from(field.name().as_str()) {
                        let plan_position = ctid_col.plan_position();
                        if let Some(rel_state) =
                            state.custom_state_mut().relations.get_mut(&plan_position)
                        {
                            rel_state.ctid_col_idx = Some(i);
                        }
                    }
                }
                state.custom_state_mut().runtime = Some(runtime);
                state.custom_state_mut().datafusion_stream = Some(stream);
            }

            loop {
                if let Some(batch) = &state.custom_state().current_batch {
                    if state.custom_state().batch_index < batch.num_rows() {
                        let idx = state.custom_state().batch_index;
                        state.custom_state_mut().batch_index += 1;
                        if let Some(slot) = Self::build_result_tuple(state, idx) {
                            return slot;
                        }
                        continue;
                    }
                    state.custom_state_mut().current_batch = None;
                }

                let next_batch = {
                    let custom_state = state.custom_state_mut();
                    custom_state.runtime.as_mut().unwrap().block_on(async {
                        custom_state
                            .datafusion_stream
                            .as_mut()
                            .unwrap()
                            .next()
                            .await
                    })
                };

                match next_batch {
                    Some(Ok(batch)) => {
                        state.custom_state_mut().current_batch = Some(batch);
                        state.custom_state_mut().batch_index = 0;
                    }
                    Some(Err(e)) => panic!("DataFusion execution failed: {}", e),
                    None => return std::ptr::null_mut(),
                }
            }
        }
    }

    fn shutdown_custom_scan(_state: &mut CustomScanStateWrapper<Self>) {}

    fn end_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        unsafe {
            // Drop tuple slots that we own.
            for rel_state in state.custom_state().relations.values() {
                pg_sys::ExecDropSingleTupleTableSlot(rel_state.fetch_slot);
            }
        }
        // Clean up resources
        state.custom_state_mut().relations.clear();
        state.custom_state_mut().result_slot = None;
        // Explicitly drop source manifests to release the Tantivy segment pins at the
        // intended lifetime boundary (end of scan), mirroring basescan's pattern of
        // explicitly dropping search_reader in end_custom_scan.
        drop(std::mem::take(
            &mut state.custom_state_mut().source_manifests,
        ));
    }
}

/// Walk a target list and classify each entry into the corresponding
/// [`privdat::OutputColumnInfo`]: a `Var` resolves to a plan position via the
/// join clause, a `paradedb.score()` call becomes a `Score` sentinel, and any
/// expression that cannot be located emits `Pruned` so the parent plan slot
/// stays NULL.
unsafe fn compute_output_columns(
    join_clause: &JoinCSClause,
    original_tlist: *mut pg_sys::List,
    root: *mut pg_sys::PlannerInfo,
) -> Vec<privdat::OutputColumnInfo> {
    let mut output_columns = Vec::new();
    let original_entries = PgList::<pg_sys::TargetEntry>::from_pg(original_tlist);

    for te in original_entries.iter_ptr() {
        if (*(*te).expr).type_ == pg_sys::NodeTag::T_Var {
            let var = (*te).expr as *mut pg_sys::Var;
            let rti = (*var).varno as pg_sys::Index;
            let attno = (*var).varattno;
            if let Some(plan_position) = join_clause.plan_position(root.into(), rti, attno) {
                output_columns.push(privdat::OutputColumnInfo::Var {
                    plan_position,
                    rti,
                    original_attno: attno,
                });
            } else {
                // Var references a relation pruned by an internal Semi/Anti
                // join (e.g., the inner side of a flattened EXISTS).
                // PostgreSQL's reltarget may include these Vars even though
                // they are not accessible after the Semi/Anti. Emit NULL;
                // the parent plan will not read this position.
                output_columns.push(privdat::OutputColumnInfo::Pruned);
            }
        } else {
            let mut found_score = false;
            for source in join_clause.plan.sources() {
                if expr_uses_scores_from_source((*te).expr.cast(), source) {
                    let rti = get_score_func_rti((*te).expr.cast()).unwrap_or(0);
                    output_columns.push(privdat::OutputColumnInfo::Score {
                        plan_position: source.plan_position,
                        rti,
                    });
                    found_score = true;
                    break;
                }
            }
            if !found_score {
                output_columns.push(privdat::OutputColumnInfo::Pruned);
            }
        }
    }

    output_columns
}

/// Build `private_data.join_clause.output_projection` from the scan target
/// list, picking one of two strategies:
///
/// 1. **Defer to parent** when the parse-tree DISTINCT clause is wider than
///    the scan target list. JoinScan strips DISTINCT, sorts by the parse
///    `sortClause`, and emits a passthrough projection so the parent plan can
///    re-evaluate the missing expressions.
/// 2. **Normal** when DISTINCT (if any) fits inside the scan target list.
///    Project each output column with metadata enriched from
///    `distinct_columns_are_fast_fields` so GROUP BY column matching works
///    against parse-tree varnos.
unsafe fn build_output_projection(
    private_data: &mut PrivateData,
    original_entries: &PgList<pg_sys::TargetEntry>,
    root: *mut pg_sys::PlannerInfo,
) {
    let parse = (*root).parse;
    let scan_tlist_len = original_entries.len();
    let distinct_list_len =
        if private_data.join_clause.has_distinct && !(*parse).distinctClause.is_null() {
            PgList::<pg_sys::SortGroupClause>::from_pg((*parse).distinctClause)
                .iter_ptr()
                .count()
        } else {
            0
        };

    // Custom scan tlist can be shorter than parse DISTINCT (parent evaluates some exprs).
    // Then DataFusion GROUP BY can't match all pathkeys; defer DISTINCT and sort only by
    // parse sortClause inside JoinScan.
    //
    // Limitation: if distinctClause and scan tlist happen to have equal length but
    // contain different expressions, this heuristic won't fire and the non-deferred
    // path runs.  In practice the scan tlist is a strict subset of distinctClause
    // columns, so a length mismatch is the reliable signal for "extra" expressions.
    let defer_distinct_to_parent =
        private_data.join_clause.has_distinct && distinct_list_len > scan_tlist_len;

    if defer_distinct_to_parent {
        private_data.join_clause.has_distinct = false;
        let output_rtis = private_data.join_clause.plan.output_rtis();
        let current_sources = private_data.join_clause.plan.sources();
        private_data.join_clause.order_by =
            extract_orderby_from_parse_sort_clause(root, &current_sources, &output_rtis)
                .unwrap_or_else(|| {
                    let sort_count = if (*parse).sortClause.is_null() {
                        0
                    } else {
                        PgList::<pg_sys::SortGroupClause>::from_pg((*parse).sortClause)
                            .iter_ptr()
                            .count()
                    };
                    panic!(
                        "JoinScan: ORDER BY from sortClause failed after DISTINCT/tlist \
                     mismatch (sortClause has {} entries, scan_tlist_len={}, \
                     distinct_list_len={})",
                        sort_count, scan_tlist_len, distinct_list_len
                    )
                });
        // Non-Var, non-score expressions have rti=0, attno=0 here.  That is safe:
        // has_distinct is cleared above, so DataFusion skips GROUP BY and
        // build_projection_expr yields NULL for these slots.  build_result_tuple
        // also treats rti=0 as NULL.  Postgres re-evaluates the real expressions
        // on top of the result slot, so the NULLs are overwritten.
        private_data.join_clause.output_projection = Some(
            private_data
                .output_columns
                .iter()
                .map(build::ChildProjection::from)
                .collect(),
        );
        return;
    }

    // Normal path: build output_projection, enriching expression entries with
    // metadata when DISTINCT is active.
    //
    // TODO(#4604): This is the second call to distinct_columns_are_fast_fields
    // in the same planning phase (first in validate_and_build_clause). Both
    // calls walk the same parse tree. Consider caching the result in a
    // planning-phase-scoped structure to avoid redundant work.
    let distinct_entries = if private_data.join_clause.has_distinct {
        let all_sources = private_data.join_clause.plan.sources();
        distinct_columns_are_fast_fields(root, &all_sources)
    } else {
        None
    };

    // Map ResolvedExpr to output columns by walking the parse tree's
    // target list (which has original expressions and valid ressortgroupref).
    // For ALL entries (Column, Score, Expression), use the parse-tree
    // varnos so that distinct_col_map keys are consistent with
    // extract_orderby's pathkey varnos.
    let mut entry_by_output_idx: crate::api::HashMap<usize, &planning::ResolvedExpr> =
        Default::default();
    if let Some(ref entries) = distinct_entries {
        let parse_tlist = PgList::<pg_sys::TargetEntry>::from_pg((*parse).targetList);
        let distinct_list = PgList::<pg_sys::SortGroupClause>::from_pg((*parse).distinctClause);

        for (clause_ptr, entry) in distinct_list.iter_ptr().zip(entries.iter()) {
            let tle_ref = (*clause_ptr).tleSortGroupRef;
            if let Some(parse_te) = parse_tlist
                .iter_ptr()
                .find(|te| (**te).ressortgroupref == tle_ref)
            {
                let output_idx = ((*parse_te).resno - 1) as usize;
                if output_idx < private_data.output_columns.len() {
                    entry_by_output_idx.insert(output_idx, entry);
                }
            }
        }
    }

    // Key `entry_by_output_idx` by each scan tlist entry's `resno`, not list position.
    let scan_target_entries: Vec<*mut pg_sys::TargetEntry> = original_entries.iter_ptr().collect();
    private_data.join_clause.output_projection = Some(
        private_data
            .output_columns
            .iter()
            .zip(scan_target_entries.iter().copied())
            .map(|(info, te)| {
                let output_idx = ((*te).resno - 1) as usize;
                match entry_by_output_idx.get(&output_idx) {
                    Some(planning::ResolvedExpr::Expression {
                        expr_node,
                        input_vars,
                        result_type,
                    }) => {
                        let expr_string = {
                            let node_str = pg_sys::nodeToString((*expr_node).cast());
                            std::ffi::CStr::from_ptr(node_str)
                                .to_string_lossy()
                                .into_owned()
                        };
                        let primary_rti = input_vars.first().map_or(0, |v| v.rti);
                        build::ChildProjection::Expression {
                            rti: primary_rti,
                            pg_expr_string: expr_string,
                            input_vars: input_vars.clone(),
                            result_type_oid: *result_type,
                        }
                    }
                    Some(planning::ResolvedExpr::Column { rti, attno }) => {
                        build::ChildProjection::Column {
                            rti: *rti,
                            attno: *attno,
                        }
                    }
                    Some(planning::ResolvedExpr::Score { rti }) => {
                        build::ChildProjection::Score { rti: *rti }
                    }
                    Some(planning::ResolvedExpr::IndexedExpression { rti }) => {
                        let attno = match info {
                            privdat::OutputColumnInfo::Var { original_attno, .. } => {
                                *original_attno
                            }
                            _ => 0,
                        };
                        build::ChildProjection::IndexedExpression { rti: *rti, attno }
                    }
                    None => info.into(),
                }
            })
            .collect(),
    );
}

/// Build the DataFusion logical plan for the JoinScan, serialize it, and store
/// the bytes inside `private_data.logical_plan` so the executor can rehydrate
/// it during scan startup.
fn bake_logical_plan(private_data: &mut PrivateData, custom_exprs: *mut pg_sys::List) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("Failed to create tokio runtime");
    let logical_plan = runtime
        .block_on(build_joinscan_logical_plan(
            &private_data.join_clause,
            &*private_data,
            custom_exprs,
        ))
        .expect("Failed to build DataFusion logical plan");
    private_data.logical_plan = Some(
        serialize_logical_plan(&logical_plan).expect("Failed to serialize DataFusion logical plan"),
    );
}

/// Append every entry in `best_path.custom_private` (skipping index 0, which
/// holds the serialized `PrivateData`) onto `list`. Used twice in
/// `plan_custom_path`: once to splice the trailing restrictlist clauses onto
/// `node.custom_exprs`, and once to preserve them when re-serializing
/// `PrivateData` back into `node.custom_private`.
unsafe fn splice_path_private_into_list(
    list: *mut pg_sys::List,
    best_path: *mut pg_sys::CustomPath,
) -> *mut pg_sys::List {
    let mut combined = PgList::<pg_sys::Node>::from_pg(list);
    let path_private_full = PgList::<pg_sys::Node>::from_pg((*best_path).custom_private);
    // Skip index 0 (PrivateData)
    for i in 1..path_private_full.len() {
        if let Some(node_ptr) = path_private_full.get_ptr(i) {
            combined.push(node_ptr);
        }
    }
    combined.into_pg()
}

impl JoinScan {
    /// Body of [`<Self as CustomScan>::create_custom_path`] in `?`-style.
    /// The Ok variant returns the assembled `CustomPath` plus the alias list
    /// (for the "successful" mark) and the trailing multi-table clauses to
    /// splice onto `custom_private`. The Err variants distinguish silent
    /// gates (`Quiet`) from validation failures that should emit a planner
    /// warning (`Warn { reason, aliases }`).
    unsafe fn try_build_join_custom_path(
        builder: &CustomPathBuilder<Self>,
    ) -> Result<BuiltJoinPath, JoinPathDecline> {
        let args = builder.args();
        let root = args.root;
        let jointype = args.jointype;
        let outerrel = args.outerrel;
        let innerrel = args.innerrel;
        let extra = args.extra;

        // Silent gates: collect outer/inner sources or bail without a warning.
        let (outer_node, mut join_keys) =
            collect_join_sources(root, outerrel).ok_or(JoinPathDecline::Quiet)?;
        let (inner_node, inner_keys) =
            collect_join_sources(root, innerrel).ok_or(JoinPathDecline::Quiet)?;
        join_keys.extend(inner_keys);

        let mut all_sources = outer_node.sources();
        all_sources.extend(inner_node.sources());

        let aliases: Vec<String> = all_sources
            .iter()
            .map(|s| {
                RelationAlias::new(s.scan_info.alias.as_deref())
                    .warning_context(s.scan_info.heaprelid)
            })
            .collect();

        let join_conditions = extract_join_conditions(extra, &all_sources);

        // The minimum requirement for considering the join scan is that a
        // search predicate is used — either in a source or in a join-level
        // condition. Below this gate, every Err carries a planner warning.
        if !all_sources.iter().any(|s| s.scan_info.has_search_predicate)
            && !join_conditions.has_search_predicate
        {
            return Err(JoinPathDecline::Quiet);
        }

        let warn = |reason| JoinPathDecline::Warn {
            reason,
            aliases: aliases.clone(),
        };

        if join_conditions.equi_keys.is_empty() {
            return Err(warn(JoinDeclineReason::NoEquiKeys));
        }

        join_keys.extend(join_conditions.equi_keys.clone());

        let parsed_jointype = build::JoinType::try_from(jointype)
            .map_err(|e| warn(JoinDeclineReason::Other(e.to_string())))?;

        let mut plan = RelNode::Join(Box::new(build::JoinNode {
            join_type: parsed_jointype,
            left: outer_node,
            right: inner_node,
            equi_keys: join_conditions.equi_keys,
            filter: None,
            subplan_id: None,
        }));

        let unsupported = plan.unsupported_join_types();
        if !unsupported.is_empty() {
            return Err(warn(JoinDeclineReason::UnsupportedJoinType(
                unsupported
                    .iter()
                    .map(|t| t.to_string().to_uppercase())
                    .collect(),
            )));
        }

        if !plan.rewrite_pruned_join_keys(root) {
            return Err(warn(JoinDeclineReason::PrunedJoinKey));
        }

        let has_distinct = !(*(*root).parse).distinctClause.is_null();

        // Phase 1: shared activation checks + JoinCSClause construction.
        let (mut join_clause, limit_offset) =
            Self::validate_and_build_clause(root, plan, &join_keys, has_distinct)
                .ok_or_else(|| warn(JoinDeclineReason::ActivationFailed))?;

        // --- Join-level predicate extraction (join-hook specific) ---
        // This builds an expression tree that can reference:
        // - Predicate nodes: Tantivy search queries
        // - MultiTablePredicate nodes: PostgreSQL expressions
        let current_sources = join_clause.plan.sources();
        let (join_clause_updated, multi_table_clauses) = extract_join_level_conditions(
            root,
            extra,
            &current_sources,
            &join_conditions.other_conditions,
            join_clause.clone(),
        )
        .map_err(|_| warn(JoinDeclineReason::JoinLevelExtractionFailed))?;
        join_clause = join_clause_updated;

        // Post-extraction check: need at least one side predicate OR join-level predicates.
        // This is a silent gate — the join is no longer interesting once predicates have
        // been pulled out.
        let current_sources_after_cond = join_clause.plan.sources();
        let has_side_predicate = current_sources_after_cond
            .iter()
            .any(|s| s.has_search_predicate());
        let has_join_level_predicates = !join_clause.join_level_predicates.is_empty();
        if !has_side_predicate && !has_join_level_predicates {
            return Err(JoinPathDecline::Quiet);
        }

        // Phase 2: shared ORDER BY + cost + CustomPath construction.
        let consider_parallel = (*outerrel).consider_parallel;
        let path = Self::finalize_clause_into_path(
            root,
            builder.args().joinrel,
            join_clause,
            &limit_offset,
            consider_parallel,
        )
        .ok_or_else(|| warn(JoinDeclineReason::OrderByUnavailable))?;

        Ok(BuiltJoinPath {
            path,
            aliases,
            multi_table_clauses,
        })
    }

    /// Build a result tuple from the current joined row.
    ///
    /// # Arguments
    /// * `state` - The custom scan state
    /// * `row_idx` - The index of the row in the current batch (for score lookup)
    unsafe fn build_result_tuple(
        state: &mut CustomScanStateWrapper<Self>,
        row_idx: usize,
    ) -> Option<*mut pg_sys::TupleTableSlot> {
        let result_slot = state.custom_state().result_slot?;
        let output_columns = state.custom_state().output_columns.clone();
        let mut fetched_sources = crate::api::HashSet::default();

        // Fetch tuples for all RTIs referenced in the output columns
        for col_info in &output_columns {
            let plan_position = match col_info {
                privdat::OutputColumnInfo::Var { plan_position, .. } => *plan_position,
                privdat::OutputColumnInfo::Score { plan_position, .. } => *plan_position,
                privdat::OutputColumnInfo::Pruned => continue,
            };
            if !fetched_sources.contains(&plan_position) {
                let ctid = {
                    let batch = state.custom_state().current_batch.as_ref()?;
                    let rel_state = state.custom_state().relations.get(&plan_position)?;
                    let ctid_col = batch.column(rel_state.ctid_col_idx?);
                    ctid_col
                        .as_any()
                        .downcast_ref::<arrow_array::UInt64Array>()
                        .expect("ctid should be u64")
                        .value(row_idx)
                };
                let rel_state = state.custom_state_mut().relations.get_mut(&plan_position)?;
                if !rel_state
                    .visibility_checker
                    .fetch_tuple_direct(ctid, rel_state.fetch_slot)
                {
                    return None;
                }
                pg_sys::slot_getallattrs(rel_state.fetch_slot);
                fetched_sources.insert(plan_position);
            }
        }
        // Get the result tuple descriptor from the result slot
        let result_tupdesc = (*result_slot).tts_tupleDescriptor;
        let natts = (*result_tupdesc).natts as usize;
        // Clear the result slot
        pg_sys::ExecClearTuple(result_slot);

        // Fill the result slot based on the output column mapping
        let datums = (*result_slot).tts_values;
        let nulls = (*result_slot).tts_isnull;
        let batch = state.custom_state().current_batch.as_ref()?;

        for (i, col_info) in output_columns.iter().enumerate() {
            if i >= natts {
                break;
            }
            match col_info {
                privdat::OutputColumnInfo::Score { .. } => {
                    let score_col = batch.column(i);
                    let score = if let Some(score_array) = score_col
                        .as_any()
                        .downcast_ref::<arrow_array::Float32Array>(
                    ) {
                        score_array.value(row_idx)
                    } else {
                        0.0
                    };
                    use pgrx::IntoDatum;
                    if let Some(datum) = score.into_datum() {
                        *datums.add(i) = datum;
                        *nulls.add(i) = false;
                    } else {
                        *nulls.add(i) = true;
                    }
                }
                privdat::OutputColumnInfo::Pruned => {
                    *nulls.add(i) = true;
                }
                privdat::OutputColumnInfo::Var {
                    plan_position,
                    original_attno,
                    ..
                } => {
                    let rel_state = state.custom_state().relations.get(plan_position)?;
                    let source_slot = rel_state.fetch_slot;
                    if *original_attno <= 0
                        || *original_attno > (*(*source_slot).tts_tupleDescriptor).natts as i16
                    {
                        *nulls.add(i) = true;
                        continue;
                    }
                    let mut is_null = false;
                    *datums.add(i) =
                        pg_sys::slot_getattr(source_slot, *original_attno as i32, &mut is_null);
                    *nulls.add(i) = is_null;
                }
            }
        }
        // Use ExecStoreVirtualTuple to properly mark the slot as containing a virtual tuple
        pg_sys::ExecStoreVirtualTuple(result_slot);
        Some(result_slot)
    }
}

/// Render a DataFusion physical plan tree with metrics.
///
/// Each node is rendered via its `DisplayAs` implementation, followed by
/// collected metrics.  When `include_timing` is false, timing metrics
/// (`elapsed_compute`, named `Time` values) are stripped so that regression
/// test output remains stable.  Pass `true` (e.g. for EXPLAIN ANALYZE VERBOSE)
/// to include everything.
///
/// TODO: In parallel mode each worker runs its own `exec_custom_scan` with its
/// own plan instances, so the metrics stored on the leader's plan only reflect
/// the leader's share of the work.  Once JoinScan parallelism is refactored
/// (#4152), aggregate these across workers.
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
