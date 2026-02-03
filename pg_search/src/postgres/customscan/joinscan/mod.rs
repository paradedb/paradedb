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
//! JoinScan is proposed by the planner when **all** of the following conditions are met:
//!
//! 1. **GUC enabled**: `paradedb.enable_join_custom_scan = on` (default: on)
//!
//! 2. **Join type**: Only `INNER JOIN` is currently supported
//!    - LEFT, RIGHT, FULL, SEMI, and ANTI joins are planned for future work
//!
//! 3. **LIMIT clause**: Query must have a LIMIT clause
//!    - This restriction exists because without LIMIT, scanning the entire index
//!      may not be more efficient than PostgreSQL's native join execution
//!    - Future work will allow no-limit joins when both sides have search predicates
//!
//! 4. **Search predicate**: At least one side must have:
//!    - A BM25 index on the table
//!    - A `@@@` search predicate in the WHERE clause
//!
//! 5. **Base relations**: Each side of the join must be a single base relation
//!    - Join trees (e.g., `(A JOIN B) JOIN C`) on one side are not yet supported
//!
//! 6. **Fast-field columns**: All columns used in the join must be fast fields in their
//!    respective BM25 indexes:
//!    - Equi-join keys (e.g., `a.id = b.id`) must be fast fields for join execution
//!    - Multi-table predicates (e.g., `a.price > b.min_price`) must reference fast fields
//!    - ORDER BY columns must be fast fields for efficient sorting
//!    - If any required column is not a fast field, the query falls back to PostgreSQL
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
//!    It identifies potential search joins, extracts predicates, and builds a `JoinCSClause`.
//! 2. **Execution**: A DataFusion logical plan is constructed from the `JoinCSClause`.
//!    This plan defines the join, filters, sorts, and limits.
//! 3. **DataFusion**: The plan is executed by DataFusion, which chooses the best join algorithm.
//!    - **Driving side**: Streams results from Tantivy (search results).
//!    - **Build side**: Scans the other relation (or search results).
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

mod build;
mod explain;
mod memory;
mod planning;
mod predicate;
mod privdat;
mod scan_state;
mod translator;
mod udf;

use self::build::{JoinCSClause, JoinType};
use self::explain::{format_join_level_expr, get_attname_safe};
use self::memory::PanicOnOOMMemoryPool;
use self::planning::{
    collect_required_fields, extract_join_conditions, extract_join_side_info, extract_orderby,
    extract_score_pathkey, order_by_columns_are_fast_fields,
};
use self::predicate::{extract_join_level_conditions, is_column_fast_field};
use self::privdat::{PrivateData, INNER_SCORE_ALIAS, OUTER_SCORE_ALIAS};
use self::scan_state::{build_joinscan_logical_plan, JoinScanState};
use crate::api::{OrderByFeature, OrderByInfo};
use crate::postgres::customscan::basescan::projections::score::uses_scores;
use crate::postgres::customscan::builders::custom_path::{CustomPathBuilder, Flags};
use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::builders::custom_state::{
    CustomScanStateBuilder, CustomScanStateWrapper,
};
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::score_funcoids;
use crate::postgres::customscan::{CustomScan, ExecMethod, JoinPathlistHookArgs, PlainExecCapable};
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
use datafusion::execution::runtime_env::RuntimeEnvBuilder;
use datafusion::execution::TaskContext;
use futures::StreamExt;
use pgrx::{pg_sys, PgList};
use std::ffi::CStr;
use std::sync::Arc;

#[derive(Default)]
pub struct JoinScan;

impl CustomScan for JoinScan {
    const NAME: &'static CStr = c"ParadeDB Join Scan";
    type Args = JoinPathlistHookArgs;
    type State = JoinScanState;
    type PrivateData = PrivateData;

    fn create_custom_path(builder: CustomPathBuilder<Self>) -> Option<pg_sys::CustomPath> {
        unsafe {
            let args = builder.args();
            let root = args.root;
            let jointype = args.jointype;
            let outerrel = args.outerrel;
            let innerrel = args.innerrel;
            let extra = args.extra;

            // TODO(join-types): Currently only INNER JOIN is supported.
            // Future work should add:
            // - LEFT JOIN: Return NULL for non-matching build rows; track matched driving rows
            // - RIGHT JOIN: Swap driving/build sides, then use LEFT logic
            // - FULL OUTER JOIN: Track unmatched rows on both sides; two-pass or marking approach
            // - SEMI JOIN: Stop after first match per driving row (benefits EXISTS queries)
            // - ANTI JOIN: Return only driving rows with no matches (benefits NOT EXISTS)
            if jointype != pg_sys::JoinType::JOIN_INNER {
                return None;
            }

            // TODO(no-limit): Currently requires LIMIT for JoinScan to be proposed.
            // This is overly restrictive. We should allow no-limit joins when:
            // 1. Both sides have search predicates (Aggregate Score pattern), OR
            // 2. Join-level predicates exist that benefit from index
            //
            // JoinScan currently requires a LIMIT clause. This restriction exists because
            // without a limit, scanning the entire index may not be more efficient than
            // PostgreSQL's native join execution.
            let limit = if (*root).limit_tuples > -1.0 {
                Some((*root).limit_tuples as usize)
            } else {
                return None;
            };

            // Extract information from both sides of the join
            let mut outer_side = extract_join_side_info(root, outerrel)?;
            let mut inner_side = extract_join_side_info(root, innerrel)?;

            // Extract join conditions from the restrict list
            let outer_rti = outer_side.heap_rti.unwrap_or(0);
            let inner_rti = inner_side.heap_rti.unwrap_or(0);
            let join_conditions = extract_join_conditions(extra, outer_rti, inner_rti);

            // Require equi-join keys for JoinScan.
            // Without equi-join keys, we'd have a cross join requiring O(N*M) comparisons
            // where join complexity explodes. PostgreSQL's native join
            // handles cartesian products more efficiently.
            let has_equi_join_keys = !join_conditions.equi_keys.is_empty();
            if !has_equi_join_keys {
                pgrx::debug1!("JoinScan: no equi-join keys (cross join), rejecting");
                return None;
            }

            // Check if all ORDER BY columns are fast fields
            // JoinScan requires fast field access for efficient sorting
            if !order_by_columns_are_fast_fields(root, &outer_side, &inner_side) {
                return None;
            }

            // Determine driving side: the side with a search predicate is the driving side
            let driving_side_is_outer = outer_side.has_search_predicate;
            let driving_side_rti = if driving_side_is_outer {
                outer_rti
            } else {
                inner_rti
            };

            // Check if paradedb.score() is used anywhere in the query for each side.
            // This includes ORDER BY, SELECT list, or any other expression.
            // We need to check BOTH sides, not just the driving side, because:
            // - Driving side: scores come from the streaming executor
            // - Build side: scores come from the pre-materialized search results
            let funcoids = score_funcoids();
            let score_pathkey = extract_score_pathkey(root, driving_side_rti as pg_sys::Index);

            // Check if outer side needs scores
            let outer_score_in_tlist =
                uses_scores((*root).processed_tlist.cast(), funcoids, outer_rti);
            let outer_score_needed = if driving_side_is_outer {
                score_pathkey.is_some() || outer_score_in_tlist
            } else {
                outer_score_in_tlist
            };

            // Check if inner side needs scores
            let inner_score_in_tlist =
                uses_scores((*root).processed_tlist.cast(), funcoids, inner_rti);
            let inner_score_needed = if !driving_side_is_outer {
                score_pathkey.is_some() || inner_score_in_tlist
            } else {
                inner_score_in_tlist
            };

            // Record score_needed for each side
            outer_side = outer_side.with_score_needed(outer_score_needed);
            inner_side = inner_side.with_score_needed(inner_score_needed);

            // Build the join clause with join keys
            let mut join_clause = JoinCSClause::new()
                .with_outer_side(outer_side.clone())
                .with_inner_side(inner_side.clone())
                .with_join_type(JoinType::from(jointype))
                .with_limit(limit);

            // Add extracted equi-join keys with type info
            // All equi-join key columns must be fast fields in their respective BM25 indexes
            for jk in join_conditions.equi_keys {
                // Check if outer join key column is a fast field
                if !is_column_fast_field(&outer_side, jk.outer_attno) {
                    pgrx::debug1!(
                        "JoinScan: outer equi-join key column (attno={}) is not a fast field, rejecting",
                        jk.outer_attno
                    );
                    return None;
                }
                // Check if inner join key column is a fast field
                if !is_column_fast_field(&inner_side, jk.inner_attno) {
                    pgrx::debug1!(
                        "JoinScan: inner equi-join key column (attno={}) is not a fast field, rejecting",
                        jk.inner_attno
                    );
                    return None;
                }
                join_clause = join_clause.add_join_key(
                    jk.outer_attno,
                    jk.inner_attno,
                    jk.type_oid,
                    jk.typlen,
                    jk.typbyval,
                );
            }

            // Extract join-level predicates (search predicates and heap conditions)
            // This builds an expression tree that can reference:
            // - Predicate nodes: Tantivy search queries
            // - MultiTablePredicate nodes: PostgreSQL expressions
            // Returns the updated join_clause and a list of heap condition clause pointers
            let multi_table_predicate_clauses: Vec<*mut pg_sys::Expr>;
            let (mut join_clause, multi_table_predicate_clauses) =
                match extract_join_level_conditions(
                    root,
                    extra,
                    &outer_side,
                    &inner_side,
                    &join_conditions.other_conditions,
                    join_clause,
                ) {
                    Ok(result) => result,
                    Err(err) => {
                        // Log the error for debugging - JoinScan won't be proposed for this query
                        pgrx::debug1!("JoinScan: failed to extract join-level conditions: {}", err);
                        return None;
                    }
                };

            // Check if this is a valid join for JoinScan
            // We need at least one side with a BM25 index AND a search predicate,
            // OR successfully extracted join-level predicates.
            let has_side_predicate = (outer_side.has_bm25_index()
                && outer_side.has_search_predicate)
                || (inner_side.has_bm25_index() && inner_side.has_search_predicate);
            let has_join_level_predicates = !join_clause.join_level_predicates.is_empty();

            if !has_side_predicate && !has_join_level_predicates {
                return None;
            }

            // Note: Multi-table predicates (conditions like `a.price > b.price`) are allowed
            // only if all referenced columns are fast fields. The check happens during
            // predicate extraction in predicate.rs - if any column is not a fast field,
            // the predicate extraction returns None and JoinScan won't be proposed.

            // Get the cheapest total paths from outer and inner relations
            // These are needed so PostgreSQL can resolve Vars in custom_scan_tlist
            let outer_path = (*outerrel).cheapest_total_path;
            let inner_path = (*innerrel).cheapest_total_path;

            // Use simple fixed costs since we force the path anyway.
            // Cost estimation is deferred to DataFusion integration.
            let startup_cost = crate::DEFAULT_STARTUP_COST;
            let total_cost = startup_cost + 1.0;
            let result_rows = limit.map(|l| l as f64).unwrap_or(1000.0);

            // Extract ORDER BY info for DataFusion execution
            let order_by = extract_orderby(root, &outer_side, &inner_side, driving_side_is_outer);
            join_clause = join_clause.with_order_by(order_by);

            // Create the private data with hints included
            let private_data = PrivateData::new(join_clause.clone());

            // Force the path to be chosen when we have a valid join opportunity.
            // TODO: Once cost model is well-tuned, consider removing Flags::Force
            // to let PostgreSQL make cost-based decisions.
            let mut builder = builder
                .set_flag(Flags::Force)
                .set_startup_cost(startup_cost)
                .set_total_cost(total_cost)
                .set_rows(result_rows)
                .add_custom_path(outer_path)
                .add_custom_path(inner_path);

            // Add pathkey if ORDER BY score detected for driving side
            if let Some(ref pathkey) = score_pathkey {
                builder = builder.add_path_key(pathkey);
            }

            let mut custom_path = builder.build(private_data);

            // Store the restrictlist and heap condition clauses in custom_private
            // Structure: [PrivateData JSON, restrictlist, heap_cond_1, heap_cond_2, ...]
            let mut private_list = PgList::<pg_sys::Node>::from_pg(custom_path.custom_private);

            // Add the restrictlist as the second element
            let restrictlist = (*extra).restrictlist;
            if !restrictlist.is_null() {
                private_list.push(restrictlist.cast());
            } else {
                private_list.push(std::ptr::null_mut());
            }

            // Add heap condition clauses as subsequent elements
            for clause in multi_table_predicate_clauses {
                private_list.push(clause.cast());
            }

            custom_path.custom_private = private_list.into_pg();

            Some(custom_path)
        }
    }

    fn plan_custom_path(mut builder: CustomScanBuilder<Self>) -> pg_sys::CustomScan {
        // For joins, scanrelid must be 0 (it's not scanning a single relation)
        builder.set_scanrelid(0);

        // Get best_path before builder is consumed
        let best_path = builder.args().best_path;

        let mut node = builder.build();

        unsafe {
            // For joins, we need to set custom_scan_tlist to describe the output columns.
            // Create a fresh copy of the target list to avoid corrupting the original
            let original_tlist = node.scan.plan.targetlist;
            let copied_tlist = pg_sys::copyObjectImpl(original_tlist.cast()).cast::<pg_sys::List>();
            let tlist = PgList::<pg_sys::TargetEntry>::from_pg(copied_tlist);

            // For join custom scans, PostgreSQL doesn't pass clauses via the usual parameter.
            // We stored the restrictlist in custom_private during create_custom_path
            let private_list = PgList::<pg_sys::Node>::from_pg((*best_path).custom_private);

            // Second element (if present) is the restrictlist we stored during create_custom_path
            // Note: We do NOT add restrictlist clauses to custom_exprs because setrefs would try
            // to resolve their Vars using the child plans' target lists, which may not have all
            // the needed columns. Instead, we keep the restrictlist in custom_private and handle
            // join condition evaluation manually during execution using the original Var references.

            // Extract the column mappings from the ORIGINAL targetlist (before we add restrictlist Vars).
            // The original_tlist has the SELECT's output columns, which is what ps_ResultTupleSlot is based on.
            // We store this mapping in PrivateData so build_result_tuple can use it during execution.
            let mut output_columns = Vec::new();

            // Get the outer and inner RTIs from PrivateData
            // Note: custom_private may have [PrivateData JSON, restrictlist]
            // We need to preserve the restrictlist when updating
            let current_private = PgList::<pg_sys::Node>::from_pg(node.custom_private);
            let restrictlist_node = if current_private.len() > 1 {
                current_private.get_ptr(1)
            } else {
                None
            };

            let mut private_data = PrivateData::from(node.custom_private);
            let outer_rti = private_data.join_clause.outer_side.heap_rti.unwrap_or(0);
            let inner_rti = private_data.join_clause.inner_side.heap_rti.unwrap_or(0);

            // Use the ORIGINAL targetlist to extract output_columns, NOT the extended tlist.
            // The original_tlist matches what ps_ResultTupleSlot is built from.
            let original_entries = PgList::<pg_sys::TargetEntry>::from_pg(original_tlist);
            let funcoids = score_funcoids();

            // Determine which RTI has the search predicate (score comes from that side)
            let driving_side_rti = if private_data.join_clause.outer_side.query.is_some() {
                outer_rti
            } else {
                inner_rti
            };

            for te in original_entries.iter_ptr() {
                if (*(*te).expr).type_ == pg_sys::NodeTag::T_Var {
                    let var = (*te).expr as *mut pg_sys::Var;
                    let varno = (*var).varno as pg_sys::Index;
                    let varattno = (*var).varattno;

                    // Determine if this column comes from outer or inner relation
                    let is_outer = varno == outer_rti;
                    output_columns.push(privdat::OutputColumnInfo {
                        is_outer,
                        original_attno: varattno,
                        is_score: false,
                    });
                } else if uses_scores((*te).expr.cast(), funcoids, outer_rti) {
                    // This expression contains paradedb.score() for the outer side
                    output_columns.push(privdat::OutputColumnInfo {
                        is_outer: true,
                        original_attno: 0,
                        is_score: true,
                    });
                } else if uses_scores((*te).expr.cast(), funcoids, inner_rti) {
                    // This expression contains paradedb.score() for the inner side
                    output_columns.push(privdat::OutputColumnInfo {
                        is_outer: false,
                        original_attno: 0,
                        is_score: true,
                    });
                } else {
                    // Non-Var, non-score expression - mark as null (attno = 0)
                    output_columns.push(privdat::OutputColumnInfo {
                        is_outer: false,
                        original_attno: 0,
                        is_score: false,
                    });
                }
            }

            // Update PrivateData with the output column mapping
            private_data.output_columns = output_columns;

            // Add heap condition clauses to custom_exprs so they get transformed by set_customscan_references.
            // The Vars in these expressions will be converted to INDEX_VAR references into custom_scan_tlist.
            // Heap condition clauses are stored in best_path.custom_private starting at index 2
            // (after PrivateData and restrictlist). Note: we read from best_path, not node, because
            // the builder only copies the PrivateData JSON to node.custom_private, not the full list.
            let path_private = PgList::<pg_sys::Node>::from_pg((*best_path).custom_private);
            let mut custom_exprs = PgList::<pg_sys::Node>::new();
            let num_multi_table_predicates = private_data.join_clause.multi_table_predicates.len();

            for i in 0..num_multi_table_predicates {
                // Index 0 = PrivateData, Index 1 = restrictlist, Index 2+ = heap condition clauses
                let clause_idx = 2 + i;
                if clause_idx < path_private.len() {
                    if let Some(clause_node) = path_private.get_ptr(clause_idx) {
                        if !clause_node.is_null() {
                            // Copy the clause to avoid modifying the original
                            let clause_copy = pg_sys::copyObjectImpl(clause_node.cast()).cast();
                            custom_exprs.push(clause_copy);
                        }
                    }
                }
            }
            node.custom_exprs = custom_exprs.into_pg();

            // Collect all required fields for execution
            collect_required_fields(
                &mut private_data.join_clause,
                &private_data.output_columns,
                node.custom_exprs,
            );

            // Convert PrivateData back to a list and preserve the restrictlist
            let private_data_list: *mut pg_sys::List = private_data.into();
            let mut new_private = PgList::<pg_sys::Node>::from_pg(private_data_list);
            if let Some(rl) = restrictlist_node {
                new_private.push(rl);
            }
            node.custom_private = new_private.into_pg();

            // Set custom_scan_tlist with all needed columns
            node.custom_scan_tlist = tlist.into_pg();
        }

        node
    }

    fn create_custom_scan_state(
        mut builder: CustomScanStateBuilder<Self, Self::PrivateData>,
    ) -> *mut CustomScanStateWrapper<Self> {
        // Transfer join clause and output column mapping to scan state.
        // Note: This clone happens once per query execution, not in a hot loop.
        // The builder API doesn't expose mutable access to custom_private,
        // so we can't use std::mem::take() here without changing the builder.
        builder.custom_state().join_clause = builder.custom_private().join_clause.clone();
        builder.custom_state().output_columns = builder.custom_private().output_columns.clone();
        builder.build()
    }

    fn explain_custom_scan(
        state: &CustomScanStateWrapper<Self>,
        _ancestors: *mut pg_sys::List,
        explainer: &mut Explainer,
    ) {
        let join_clause = &state.custom_state().join_clause;

        // Show join type
        let join_type_str = match join_clause.join_type {
            JoinType::Inner => "Inner",
            JoinType::Left => "Left",
            JoinType::Right => "Right",
            JoinType::Full => "Full",
            JoinType::Semi => "Semi",
            JoinType::Anti => "Anti",
        };
        explainer.add_text("Join Type", join_type_str);

        // Get relation names and aliases for display
        let outer_rel_name = join_clause
            .outer_side
            .heaprelid
            .map(|oid| PgSearchRelation::open(oid).name().to_string())
            .unwrap_or_else(|| "?".to_string());
        let inner_relid = join_clause
            .inner_side
            .heaprelid
            .unwrap_or(pg_sys::InvalidOid);
        let inner_rel_name = if inner_relid != pg_sys::InvalidOid {
            PgSearchRelation::open(inner_relid).name().to_string()
        } else {
            "?".to_string()
        };

        // Get aliases (use alias if available, otherwise use table name)
        let outer_alias = join_clause
            .outer_side
            .alias
            .as_ref()
            .cloned()
            .unwrap_or_else(|| outer_rel_name.clone());
        let inner_alias = join_clause
            .inner_side
            .alias
            .as_ref()
            .cloned()
            .unwrap_or_else(|| inner_rel_name.clone());

        // Show relation info for both sides (with alias in parentheses if different)
        let outer_display = if outer_rel_name != outer_alias {
            format!("{} ({})", outer_rel_name, outer_alias)
        } else {
            outer_rel_name
        };
        let inner_display = if inner_rel_name != inner_alias {
            format!("{} ({})", inner_rel_name, inner_alias)
        } else {
            inner_rel_name
        };
        explainer.add_text("Outer Relation", outer_display);
        explainer.add_text("Inner Relation", inner_display);

        // Show join keys (equi-join condition) with column names using aliases
        // Note: Cross joins are rejected during planning, so join_keys is never empty
        debug_assert!(
            !join_clause.join_keys.is_empty(),
            "JoinScan requires equi-join keys - cross joins should be rejected during planning"
        );
        let keys_str: Vec<String> = join_clause
            .join_keys
            .iter()
            .map(|k| {
                let outer_col = get_attname_safe(
                    join_clause.outer_side.heaprelid,
                    k.outer_attno,
                    &outer_alias,
                );
                let inner_col = get_attname_safe(
                    join_clause.inner_side.heaprelid,
                    k.inner_attno,
                    &inner_alias,
                );
                format!("{} = {}", outer_col, inner_col)
            })
            .collect();
        explainer.add_text("Join Cond", keys_str.join(", "));

        // Show if there are heap conditions (cross-relation filters)
        if join_clause.has_multi_table_predicates() {
            explainer.add_text(
                "Heap Conditions",
                join_clause.multi_table_predicates.len().to_string(),
            );
        }

        // Show side-level search predicates with clear labeling
        if join_clause.outer_side.has_search_predicate {
            if let Some(ref query) = join_clause.outer_side.query {
                explainer.add_explainable("Outer Tantivy Query", query);
            }
        }
        if join_clause.inner_side.has_search_predicate {
            if let Some(ref query) = join_clause.inner_side.query {
                explainer.add_explainable("Inner Tantivy Query", query);
            }
        }

        // Show join-level expression tree if present
        if let Some(ref expr) = join_clause.join_level_expr {
            let expr_str = format_join_level_expr(
                expr,
                &join_clause.join_level_predicates,
                &join_clause.multi_table_predicates,
            );
            explainer.add_text("Join Predicate", expr_str);
        }

        // Show limit if present
        if let Some(limit) = join_clause.limit {
            explainer.add_text("Limit", limit.to_string());
        }

        // Show Order By if present
        if !join_clause.order_by.is_empty() {
            explainer.add_text(
                "Order By",
                join_clause
                    .order_by
                    .iter()
                    .map(|oi| match oi {
                        OrderByInfo {
                            feature: OrderByFeature::Field(fieldname),
                            direction,
                            ..
                        } => {
                            format!("{} {}", fieldname, direction.as_ref())
                        }

                        OrderByInfo {
                            feature: OrderByFeature::Score,
                            direction,
                            ..
                        } => {
                            format!("pdb.score() {}", direction.as_ref())
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        }

        // For EXPLAIN ANALYZE, show the actual execution method used
        if explainer.is_analyze() {
            explainer.add_text("Exec Method", "DataFusion Join");
        }
    }

    fn begin_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        estate: *mut pg_sys::EState,
        eflags: i32,
    ) {
        unsafe {
            // For EXPLAIN-only (without ANALYZE), we don't need to do much
            if eflags & (pg_sys::EXEC_FLAG_EXPLAIN_ONLY as i32) != 0 {
                return;
            }

            // Clone the join clause to avoid borrow issues
            let join_clause = state.custom_state().join_clause.clone();

            // Initialize memory limit from work_mem (in KB, convert to bytes)
            let work_mem_bytes = (pg_sys::work_mem as usize) * 1024;
            state.custom_state_mut().max_memory = work_mem_bytes;

            // Use PostgreSQL's already-initialized result tuple slot (ps_ResultTupleSlot).
            // PostgreSQL sets this up in ExecInitCustomScan based on custom_scan_tlist
            // and the projection info. Don't create our own slot - use the one PostgreSQL
            // provides to ensure compatibility with the query executor's expectations.
            //
            // Note: After set_customscan_references, custom_scan_tlist contains Vars with
            // OUTER_VAR/INNER_VAR varnos. Using ExecTypeFromTL on that would create a
            // corrupt tuple descriptor.
            let result_slot = state.csstate.ss.ps.ps_ResultTupleSlot;
            state.custom_state_mut().result_slot = Some(result_slot);

            // Determine which side is driving (has search predicate) vs build
            let driving_is_outer = join_clause.driving_side_is_outer();
            state.custom_state_mut().driving_is_outer = driving_is_outer;
        }
    }

    fn rescan_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        // Reset state for rescanning
        state.custom_state_mut().reset();
    }

    fn exec_custom_scan(state: &mut CustomScanStateWrapper<Self>) -> *mut pg_sys::TupleTableSlot {
        unsafe {
            // Note: We don't enforce LIMIT here because JoinScan might be nested inside
            // another join (e.g., 3-table join). PostgreSQL's Limit node handles limiting.
            // The limit value in join_clause is used for cost estimation and executor hints.

            // Initialize plan if not already done
            if state.custom_state().datafusion_stream.is_none() {
                // We use block_on to execute the plan synchronously
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .build()
                    .unwrap();

                let join_clause = state.custom_state().join_clause.clone();
                let cscan = state.csstate.ss.ps.plan as *mut pg_sys::CustomScan;
                let snapshot = state.csstate.ss.ps.state.as_ref().unwrap().es_snapshot;

                // Initialize heap relations and visibility checkers for JoinScanState
                // These are needed for build_result_tuple to fetch the final tuples from ctids
                if let Some(heaprelid) = join_clause.driving_side().heaprelid {
                    let heaprel = PgSearchRelation::open(heaprelid);
                    state.custom_state_mut().driving_visibility_checker =
                        Some(VisibilityChecker::with_rel_and_snap(&heaprel, snapshot));
                    state.custom_state_mut().driving_fetch_slot = Some(pg_sys::MakeTupleTableSlot(
                        heaprel.rd_att,
                        &pg_sys::TTSOpsBufferHeapTuple,
                    ));
                    state.custom_state_mut().driving_heaprel = Some(heaprel);
                }

                if let Some(heaprelid) = join_clause.build_side().heaprelid {
                    let heaprel = PgSearchRelation::open(heaprelid);
                    state.custom_state_mut().build_visibility_checker =
                        Some(VisibilityChecker::with_rel_and_snap(&heaprel, snapshot));
                    state.custom_state_mut().build_scan_slot = Some(pg_sys::MakeTupleTableSlot(
                        heaprel.rd_att,
                        &pg_sys::TTSOpsBufferHeapTuple,
                    ));
                    state.custom_state_mut().build_heaprel = Some(heaprel);
                }

                let mut private_data = PrivateData::new(join_clause.clone());
                // output_columns is needed to map INDEX_VARs (from multi-table predicates)
                // back to their original columns during plan building.
                private_data.output_columns = state.custom_state().output_columns.clone();

                let plan = runtime
                    .block_on(build_joinscan_logical_plan(
                        &join_clause,
                        &private_data,
                        (*cscan).custom_exprs,
                    ))
                    .expect("Failed to build DataFusion plan");

                // Configure DataFusion memory management
                // We use a custom memory pool that panics on OOM to enforce work_mem
                // TODO: Support spilling to PostgreSQL temporary files when work_mem is exceeded
                let memory_limit = state.custom_state().max_memory;
                let memory_pool = Arc::new(PanicOnOOMMemoryPool::new(memory_limit));

                let runtime_env = RuntimeEnvBuilder::new()
                    .with_memory_pool(memory_pool)
                    .build()
                    .expect("Failed to create RuntimeEnv");

                let task_ctx = TaskContext::default().with_runtime(Arc::new(runtime_env));
                let task_ctx = Arc::new(task_ctx);

                let stream = {
                    let _guard = runtime.enter();
                    plan.execute(0, task_ctx)
                        .expect("Failed to execute DataFusion plan")
                };

                // Determine score column indices
                let schema = plan.schema();

                let mut outer_score_idx = None;
                let mut inner_score_idx = None;

                for (i, field) in schema.fields().iter().enumerate() {
                    if field.name() == OUTER_SCORE_ALIAS {
                        outer_score_idx = Some(i);
                    } else if field.name() == INNER_SCORE_ALIAS {
                        inner_score_idx = Some(i);
                    }
                }

                state.custom_state_mut().outer_score_col_idx = outer_score_idx;
                state.custom_state_mut().inner_score_col_idx = inner_score_idx;

                state.custom_state_mut().runtime = Some(runtime);
                state.custom_state_mut().datafusion_stream = Some(stream);
            }

            loop {
                // Check if we have a current batch to iterate
                if let Some(batch) = &state.custom_state().current_batch {
                    if state.custom_state().batch_index < batch.num_rows() {
                        let idx = state.custom_state().batch_index;

                        // build_joinscan_logical_plan ensures that CTIDs are at the following indices:
                        // - Outer CTID: index 0
                        // - Inner CTID: the last "ctid" column in the schema
                        let schema = batch.schema();
                        let outer_ctid_col = batch.column(0);
                        let inner_ctid_col_idx = schema
                            .fields()
                            .iter()
                            .rposition(|f| f.name() == "ctid")
                            .expect("Inner ctid column not found");
                        let inner_ctid_col = batch.column(inner_ctid_col_idx);

                        let outer_ctids = outer_ctid_col
                            .as_any()
                            .downcast_ref::<arrow_array::UInt64Array>()
                            .expect("ctid should be u64");
                        let inner_ctids = inner_ctid_col
                            .as_any()
                            .downcast_ref::<arrow_array::UInt64Array>()
                            .expect("ctid should be u64");

                        let outer_ctid = outer_ctids.value(idx);
                        let inner_ctid = inner_ctids.value(idx);

                        // Increment index for next call
                        state.custom_state_mut().batch_index += 1;

                        // Map outer/inner to driving/build
                        let (driving_ctid, build_ctid) = if state.custom_state().driving_is_outer {
                            (outer_ctid, inner_ctid)
                        } else {
                            (inner_ctid, outer_ctid)
                        };

                        // Set current driving ctid for `build_result_tuple`
                        state.custom_state_mut().current_driving_ctid = Some(driving_ctid);

                        if let Some(slot) = Self::build_result_tuple(state, build_ctid, idx) {
                            return slot;
                        } else {
                            // Tuple not visible (or deleted), skip
                            continue;
                        }
                    } else {
                        // Finished this batch
                        state.custom_state_mut().current_batch = None;
                    }
                }

                // Poll for next batch
                let custom_state = state.custom_state_mut();
                let runtime = custom_state.runtime.as_mut().unwrap();
                let stream = custom_state.datafusion_stream.as_mut().unwrap();

                let next_batch = runtime.block_on(async { stream.next().await });

                match next_batch {
                    Some(Ok(batch)) => {
                        state.custom_state_mut().current_batch = Some(batch);
                        state.custom_state_mut().batch_index = 0;
                        // Loop continues to process this batch
                    }
                    Some(Err(e)) => {
                        // TODO: Handle error properly (ereport)
                        panic!("DataFusion execution failed: {}", e);
                    }
                    None => {
                        // End of stream
                        return std::ptr::null_mut();
                    }
                }
            }
        }
    }

    fn shutdown_custom_scan(_state: &mut CustomScanStateWrapper<Self>) {}

    fn end_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        unsafe {
            // Drop tuple slots that we own.
            // Note: Don't drop result_slot - we borrowed it from PostgreSQL's ps_ResultTupleSlot.
            if let Some(slot) = state.custom_state().build_scan_slot {
                pg_sys::ExecDropSingleTupleTableSlot(slot);
            }
            if let Some(slot) = state.custom_state().driving_fetch_slot {
                pg_sys::ExecDropSingleTupleTableSlot(slot);
            }
        }

        // Clean up resources
        state.custom_state_mut().driving_heaprel = None;
        state.custom_state_mut().build_heaprel = None;
        state.custom_state_mut().build_scan_slot = None;
        state.custom_state_mut().driving_fetch_slot = None;
        state.custom_state_mut().result_slot = None;
    }
}

impl JoinScan {
    /// Build a result tuple from the current driving row and a build row.
    ///
    /// # Arguments
    /// * `state` - The custom scan state
    /// * `build_ctid` - The ctid of the build row to include in the result
    /// * `row_idx` - The index of the row in the current batch (for score lookup)
    unsafe fn build_result_tuple(
        state: &mut CustomScanStateWrapper<Self>,
        build_ctid: u64,
        row_idx: usize,
    ) -> Option<*mut pg_sys::TupleTableSlot> {
        let result_slot = state.custom_state().result_slot?;
        let driving_slot = state.custom_state().driving_fetch_slot?;
        let build_slot = state.custom_state().build_scan_slot?;
        let driving_ctid = state.custom_state().current_driving_ctid?;
        let driving_is_outer = state.custom_state().driving_is_outer;

        // Fetch driving tuple
        let driving_vis = state
            .custom_state_mut()
            .driving_visibility_checker
            .as_mut()?;
        if !driving_vis.fetch_tuple_direct(driving_ctid, driving_slot) {
            return None;
        }

        // Fetch build tuple using direct tuple fetch.
        // The build side ctids come from a sequential scan (datafusion scan), not from an index,
        // so we use fetch_tuple_direct which uses table_tuple_fetch_slot instead of
        // table_index_fetch_tuple. The latter is designed for index-derived ctids and may incorrectly
        // report tuples as "all_dead" when used with sequential scan ctids.
        let build_vis = match state.custom_state().build_visibility_checker.as_ref() {
            Some(vis) => vis,
            None => {
                return None;
            }
        };

        if !build_vis.fetch_tuple_direct(build_ctid, build_slot) {
            return None;
        }

        // Get the result tuple descriptor from the result slot
        let result_tupdesc = (*result_slot).tts_tupleDescriptor;
        let natts = (*result_tupdesc).natts as usize;

        // Clear the result slot
        pg_sys::ExecClearTuple(result_slot);

        // Make sure slots have all attributes deformed
        pg_sys::slot_getallattrs(driving_slot);
        pg_sys::slot_getallattrs(build_slot);

        // Map driving/build to outer/inner based on driving_is_outer
        let (outer_slot, inner_slot) = state.custom_state().outer_inner_slots();
        let outer_slot = outer_slot?;
        let inner_slot = inner_slot?;

        // Use the stored output_columns mapping to build the result tuple.
        // This was populated during planning (before setrefs transformed the Vars),
        // so it contains the original attribute numbers that work with our heap tuples.
        let output_columns = &state.custom_state().output_columns;

        // Fill the result slot based on the output column mapping
        let datums = (*result_slot).tts_values;
        let nulls = (*result_slot).tts_isnull;

        for (i, col_info) in output_columns.iter().enumerate() {
            if i >= natts {
                break;
            }

            // Handle score columns specially
            if col_info.is_score {
                // Use helper to determine which score (driving vs build) based on column's side
                let score = state
                    .custom_state()
                    .score_for_column(col_info.is_outer, row_idx);

                use pgrx::IntoDatum;
                if let Some(datum) = score.into_datum() {
                    *datums.add(i) = datum;
                    *nulls.add(i) = false;
                } else {
                    *nulls.add(i) = true;
                }
                continue;
            }

            // Determine which slot to read from based on is_outer
            let source_slot = if col_info.is_outer {
                outer_slot
            } else {
                inner_slot
            };
            let original_attno = col_info.original_attno;

            // Get the attribute value from the source slot using the original attribute number
            if original_attno <= 0 {
                // System attribute, whole-row reference, or non-Var expression - set null
                *nulls.add(i) = true;
                continue;
            }

            let source_natts = (*(*source_slot).tts_tupleDescriptor).natts as i16;
            if original_attno > source_natts {
                *nulls.add(i) = true;
                continue;
            }

            let mut is_null = false;
            let value = pg_sys::slot_getattr(source_slot, original_attno as i32, &mut is_null);
            *datums.add(i) = value;
            *nulls.add(i) = is_null;
        }

        // Use ExecStoreVirtualTuple to properly mark the slot as containing a virtual tuple
        // This is safer than manually setting tts_flags
        pg_sys::ExecStoreVirtualTuple(result_slot);

        Some(result_slot)
    }
}

impl ExecMethod for JoinScan {
    fn exec_methods() -> *const pg_sys::CustomExecMethods {
        <JoinScan as PlainExecCapable>::exec_methods()
    }
}

impl PlainExecCapable for JoinScan {}
