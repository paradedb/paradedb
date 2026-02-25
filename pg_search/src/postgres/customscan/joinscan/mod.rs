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
//! 3. Only access the PostgreSQL heap (materialize) for the final result rows (top N).
//!
//! This strategy requires that all data needed for the join, filter, and sort phases
//! resides in fast fields, and that the result set size is small enough (via LIMIT)
//! that the random heap access cost doesn't outweigh the join benefit.
//!
//! 1. **GUC enabled**: `paradedb.enable_join_custom_scan = on` (default: on)
//!
//! 2. **Join type**: Only `INNER JOIN` is currently supported
//!    - LEFT, RIGHT, FULL, SEMI, and ANTI joins are planned for future work
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
//!    - **Ordering side**: Streams results from Tantivy (search results).
//!    - **Non-ordering side**: Scans the other relation (or search results).
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
mod planner;
mod planning;
mod predicate;
mod privdat;
mod scan_state;
mod translator;

use self::build::{JoinCSClause, RelNode};
use self::explain::{format_join_level_expr, get_attname_safe};
use self::memory::PanicOnOOMMemoryPool;
use self::planning::{
    collect_join_sources, collect_required_fields, ensure_score_bubbling,
    expr_uses_scores_from_source, extract_join_conditions, extract_orderby, extract_score_pathkey,
    get_score_func_rti, order_by_columns_are_fast_fields,
};
use self::predicate::{extract_join_level_conditions, is_column_fast_field};
use self::privdat::PrivateData;

use self::scan_state::{
    build_joinscan_logical_plan, build_joinscan_physical_plan, create_session_context,
    JoinScanState,
};
use crate::api::OrderByFeature;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::customscan::builders::custom_path::{CustomPathBuilder, Flags};
use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::builders::custom_state::{
    CustomScanStateBuilder, CustomScanStateWrapper,
};
use crate::postgres::customscan::dsm::ParallelQueryCapable;
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::parallel::compute_nworkers;
use crate::postgres::customscan::{CustomScan, JoinPathlistHookArgs};
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::ParallelScanState;
use crate::scan::PgSearchExtensionCodec;
use datafusion::execution::runtime_env::RuntimeEnvBuilder;
use datafusion::execution::TaskContext;
use datafusion::physical_plan::displayable;
use datafusion::physical_plan::metrics::MetricValue;
use datafusion::physical_plan::{DisplayFormatType, ExecutionPlan};
use datafusion_proto::bytes::{
    logical_plan_from_bytes_with_extension_codec, logical_plan_to_bytes_with_extension_codec,
};
use futures::StreamExt;
use pgrx::{pg_sys, PgList};
use std::ffi::{c_void, CStr};
use std::sync::Arc;

#[derive(Default)]
pub struct JoinScan;

impl ParallelQueryCapable for JoinScan {
    fn estimate_dsm_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        _pcxt: *mut pg_sys::ParallelContext,
    ) -> pg_sys::Size {
        // We only partition the largest source
        let join_clause = &state.custom_state().join_clause;
        let source = join_clause.partitioning_source();
        let segment_count = source.scan_info.segment_count;

        // JoinScan doesn't currently support aggregates in the scan
        ParallelScanState::size_of(segment_count, &[], false)
    }

    fn initialize_dsm_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        _pcxt: *mut pg_sys::ParallelContext,
        coordinate: *mut c_void,
    ) {
        let pscan_state = coordinate.cast::<ParallelScanState>();
        assert!(!pscan_state.is_null(), "coordinate is null");

        // Initialize shared state with segments from the largest source
        let join_clause = &state.custom_state().join_clause;
        let source = join_clause.partitioning_source();
        let index_relid = source.scan_info.indexrelid;
        let query = source.scan_info.query.clone();

        let index_rel = PgSearchRelation::open(index_relid);
        let reader = SearchIndexReader::open_with_context(
            &index_rel,
            query,
            false,
            MvccSatisfies::Snapshot,
            None,
            None,
        )
        .expect("Failed to open reader for DSM initialization");

        let args = crate::postgres::ParallelScanArgs {
            segment_readers: reader.segment_readers(),
            query: vec![], // We don't need to pass query bytes for JoinScan (handled by plan)
            with_aggregates: false,
        };

        unsafe {
            (*pscan_state).create_and_populate(args);
            state.custom_state_mut().parallel_state = Some(pscan_state);
        }
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

        // We don't need to deserialize query from parallel state for JoinScan
        // because the full plan (including query) is serialized in PrivateData
        // and available to the worker via the plan.
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
            let args = builder.args();
            let root = args.root;
            let jointype = args.jointype;
            let outerrel = args.outerrel;
            let innerrel = args.innerrel;
            let extra = args.extra;

            let (outer_node, mut join_keys) =
                if let Some(res) = collect_join_sources(root, outerrel) {
                    res
                } else {
                    return Vec::new();
                };
            let (inner_node, inner_keys) = if let Some(res) = collect_join_sources(root, innerrel) {
                res
            } else {
                return Vec::new();
            };

            join_keys.extend(inner_keys);

            let mut all_sources = outer_node.sources();
            all_sources.extend(inner_node.sources());

            // Collect aliases for warnings
            let aliases: Vec<String> = all_sources
                .iter()
                .enumerate()
                .map(|(i, s)| s.execution_alias(i))
                .collect();

            // A join is "potentially interesting" if at least one side has a BM25 index and a search predicate.
            // We use this flag to decide whether to emit user-friendly warnings explaining why the JoinScan
            // wasn't chosen (e.g., missing LIMIT, missing fast fields). If the user hasn't tried to search,
            // we don't want to spam them with warnings about standard Postgres joins.
            let is_interesting = all_sources.iter().any(|s| s.scan_info.has_search_predicate);

            if all_sources
                .iter()
                .any(|s| s.scan_info.indexrelid == pg_sys::InvalidOid)
            {
                if is_interesting {
                    Self::add_planner_warning(
                        "JoinScan not used: all join sources must have a BM25 index",
                        &aliases,
                    );
                }
                return Vec::new();
            }

            // TODO(join-types): Currently only INNER JOIN is supported.
            // Future work should add:
            // - LEFT JOIN: Return NULL for non-matching non-ordering rows; track matched ordering rows
            // - RIGHT JOIN: Swap ordering/non-ordering sides, then use LEFT logic
            // - FULL OUTER JOIN: Track unmatched rows on both sides; two-pass or marking approach
            // - SEMI JOIN: Stop after first match per ordering row (benefits EXISTS queries)
            // - ANTI JOIN: Return only ordering rows with no matches (benefits NOT EXISTS)
            //
            // WARNING: If enabling other join types, you MUST review the parallel partitioning
            // strategy documentation in `pg_search/src/postgres/customscan/joinscan/scan_state.rs`.
            // The current "Partition Outer / Replicate Inner" strategy is incorrect for Right/Full joins.
            if jointype != pg_sys::JoinType::JOIN_INNER && jointype != pg_sys::JoinType::JOIN_SEMI {
                let is_user_visible_jointype = jointype <= pg_sys::JoinType::JOIN_ANTI;
                if is_interesting && is_user_visible_jointype {
                    Self::add_planner_warning(
                            format!(
                                "JoinScan not used: only INNER/SEMI JOIN is currently supported, got {:?}",
                                jointype
                            ),
                            &aliases,
                        );
                }
                return Vec::new();
            }

            // JoinScan requires a LIMIT clause. This restriction exists because we gain a
            // significant benefit from using the column store when it enables late-materialization
            // of heap tuples _after_ the join has run.
            let limit = if (*root).limit_tuples > -1.0 {
                Some((*root).limit_tuples as usize)
            } else {
                if is_interesting {
                    Self::add_planner_warning(
                        "JoinScan not used: query must have a LIMIT clause",
                        (),
                    );
                }
                return Vec::new();
            };
            let join_conditions = extract_join_conditions(extra, &all_sources);

            // Require equi-join keys for JoinScan.
            // Without equi-join keys, we'd have a cross join requiring O(N*M) comparisons
            // where join complexity explodes. PostgreSQL's native join
            // handles cartesian products more efficiently.
            if join_conditions.equi_keys.is_empty() {
                if is_interesting {
                    Self::add_planner_warning(
                        "JoinScan not used: at least one equi-join key (e.g., a.id = b.id) is required",
                        &aliases,
                    );
                }
                return Vec::new();
            }

            // Check if all ORDER BY columns are fast fields
            // JoinScan requires fast field access for efficient sorting
            if !order_by_columns_are_fast_fields(root, &all_sources) {
                if is_interesting {
                    Self::add_planner_warning(
                        "JoinScan not used: all ORDER BY columns must be fast fields in the BM25 index",
                        (),
                    );
                }
                return Vec::new();
            }

            // Validate ONLY the new keys added at this level (the recursive ones were validated during collection)
            for jk in &join_conditions.equi_keys {
                // All equi-join key columns must be fast fields in their respective BM25 indexes
                // We need to find the source for each RTI involved in the join key
                let outer_source = all_sources.iter().find(|s| s.contains_rti(jk.outer_rti));
                let inner_source = all_sources.iter().find(|s| s.contains_rti(jk.inner_rti));

                match (outer_source, inner_source) {
                    (Some(outer), Some(inner)) => {
                        if !is_column_fast_field(
                            outer.scan_info.heaprelid,
                            outer.scan_info.indexrelid,
                            jk.outer_attno,
                        ) || !is_column_fast_field(
                            inner.scan_info.heaprelid,
                            inner.scan_info.indexrelid,
                            jk.inner_attno,
                        ) {
                            if is_interesting {
                                Self::add_planner_warning(
                                    "JoinScan not used: join key columns must be fast fields",
                                    &aliases,
                                );
                            }
                            return Vec::new();
                        }
                    }
                    _ => return Vec::new(), // Should not happen if extraction logic is correct
                }
            }

            // Add current level keys
            join_keys.extend(join_conditions.equi_keys.clone());

            let plan = RelNode::Join(Box::new(build::JoinNode {
                join_type: jointype.into(),
                left: outer_node,
                right: inner_node,
                equi_keys: join_conditions.equi_keys,
                filter: None,
            }));

            let mut join_clause = JoinCSClause::new(plan).with_limit(limit);

            // Determine ordering side index
            let ordering_idx = join_clause.ordering_side_index();
            let score_pathkey = if let Some(side) = join_clause.ordering_side() {
                extract_score_pathkey(root, &side)
            } else {
                None
            };

            for (i, source) in join_clause.plan.sources_mut().into_iter().enumerate() {
                // Check if paradedb.score() is used anywhere in the query for each side.
                // This includes ORDER BY, SELECT list, or any other expression.
                // We need to check ALL sides because:
                // - Ordering side: scores come from the streaming executor
                // - Non-ordering side: scores come from the pre-materialized search results
                let score_in_tlist =
                    expr_uses_scores_from_source((*root).processed_tlist.cast(), source);

                let score_needed = if let Some(ord_idx) = ordering_idx {
                    (i == ord_idx && score_pathkey.is_some()) || score_in_tlist
                } else {
                    score_in_tlist
                };

                if score_needed {
                    // Record score_needed for each side
                    ensure_score_bubbling(source);
                }
            }

            let current_sources = join_clause.plan.sources();

            // The current parallel strategy partitions exactly one source and replicates all
            // others. For SEMI JOIN correctness, the partitioned source must be the left side.
            // We currently enforce a conservative subset: binary base-table joins only.
            if jointype == pg_sys::JoinType::JOIN_SEMI {
                if current_sources.len() > 2 {
                    if is_interesting {
                        Self::add_planner_warning(
                            "JoinScan not used: SEMI JOIN currently supports only binary base-table joins",
                            &aliases,
                        );
                    }
                    return Vec::new();
                }

                let partitioning_idx = join_clause.partitioning_source_index();
                if partitioning_idx != 0 {
                    if is_interesting {
                        Self::add_planner_warning(
                            "JoinScan not used: SEMI JOIN requires the left side to be the largest source",
                            &aliases,
                        );
                    }
                    return Vec::new();
                }
            }

            // Extract join-level predicates (search predicates and heap conditions)
            // This builds an expression tree that can reference:
            // - Predicate nodes: Tantivy search queries
            // - MultiTablePredicate nodes: PostgreSQL expressions
            // Returns the updated join_clause and a list of heap condition clause pointers
            let (mut join_clause, multi_table_predicate_clauses) =
                match extract_join_level_conditions(
                    root,
                    extra,
                    &current_sources,
                    &join_conditions.other_conditions,
                    join_clause.clone(),
                ) {
                    Ok(result) => result,
                    Err(_err) => {
                        if is_interesting {
                            Self::add_planner_warning(
                                "JoinScan not used: failed to extract join-level conditions (ensure all referenced columns are fast fields)",
                                &aliases,
                            );
                        }
                        return Vec::new();
                    }
                };

            let current_sources_after_cond = join_clause.plan.sources();

            // Check if this is a valid join for JoinScan
            // We need at least one side with a BM25 index AND a search predicate,
            // OR successfully extracted join-level predicates.
            let has_side_predicate = current_sources_after_cond
                .iter()
                .any(|s| s.has_search_predicate());
            let has_join_level_predicates = !join_clause.join_level_predicates.is_empty();

            if !has_side_predicate && !has_join_level_predicates {
                return Vec::new();
            }

            // Note: Multi-table predicates (conditions like `a.price > b.price`) are allowed
            // only if all referenced columns are fast fields. The check happens during
            // predicate extraction in predicate.rs - if any column is not a fast field,
            // the predicate extraction returns None and JoinScan won't be proposed.

            // Extract ORDER BY info for DataFusion execution
            let order_by = extract_orderby(root, &current_sources_after_cond, ordering_idx);
            join_clause = join_clause.with_order_by(order_by);

            // Use simple fixed costs since we force the path anyway.
            // Cost estimation is deferred to DataFusion integration.
            let startup_cost = crate::DEFAULT_STARTUP_COST;
            let total_cost = startup_cost + 1.0;
            let mut result_rows = limit.map(|l| l as f64).unwrap_or(1000.0);

            // Calculate parallel workers based on the largest source, which we will partition.
            let (segment_count, row_estimate) = {
                let largest_source = join_clause.partitioning_source();
                let segment_count = largest_source.scan_info.segment_count;

                let row_estimate = largest_source.scan_info.estimate;

                (segment_count, row_estimate)
            };

            let nworkers = if (*outerrel).consider_parallel {
                // JoinScan always has a limit (required).
                // It declares sorted output if there is an ORDER BY clause.
                let declares_sorted_output = !join_clause.order_by.is_empty();
                // We pass `contains_external_var = false` because we handle joins internally
                // and don't want to suppress parallelism based on standard Postgres join logic rules.
                // We pass `contains_correlated_param = false` for now (TODO: check this).
                compute_nworkers(
                    declares_sorted_output,
                    limit.map(|l| l as f64),
                    row_estimate,
                    segment_count,
                    false,
                    false,
                    true,
                )
            } else {
                0
            };

            // Force the path to be chosen when we have a valid join opportunity.
            // TODO: Once cost model is well-tuned, consider removing Flags::Force
            // to let PostgreSQL make cost-based decisions.
            let mut builder = builder
                .set_flag(Flags::Force)
                .set_startup_cost(startup_cost)
                .set_total_cost(total_cost);

            if nworkers > 0 {
                builder = builder.set_parallel(nworkers);
                // Adjust result rows per worker for better costing
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
            }

            builder = builder.set_rows(result_rows);

            // Because JoinScan requires and handles the LIMIT, it must also satisfy the
            // full ORDER BY. If we determined during planning that all ORDER BY columns
            // are fast fields, we declare that this path satisfies the query pathkeys.
            if !join_clause.order_by.is_empty() {
                let query_pathkeys_len =
                    PgList::<pg_sys::PathKey>::from_pg((*root).query_pathkeys).len();
                if join_clause.order_by.len() == query_pathkeys_len {
                    builder = builder.set_pathkeys((*root).query_pathkeys);
                }
            }

            // TODO: Fix #4063 and mark this `set_parallel_safe(true)`.

            let private_data = PrivateData::new(join_clause);
            let mut custom_path = builder.build(private_data);

            // Store the restrictlist and heap condition clauses in custom_private
            // Structure: [PrivateData JSON, heap_cond_1, heap_cond_2, ...]
            let mut private_list = PgList::<pg_sys::Node>::from_pg(custom_path.custom_private);

            // Add heap condition clauses as subsequent elements
            for clause in multi_table_predicate_clauses {
                private_list.push(clause.cast());
            }
            custom_path.custom_private = private_list.into_pg();

            // We successfully created a JoinScan path for these tables, so we can clear any
            // "failure" warnings that might have been generated for them (e.g. from failed
            // attempts with different join orders or conditions).
            Self::clear_planner_warnings_for_contexts(&aliases);

            vec![custom_path]
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
            //
            // Note: We do NOT add restrictlist clauses to custom_exprs because setrefs would try
            // to resolve their Vars using the child plans' target lists, which may not have all
            // the needed columns. Instead, we keep the restrictlist in custom_private and handle
            // join condition evaluation manually during execution using the original Var references.

            // Extract the column mappings from the ORIGINAL targetlist (before we add restrictlist Vars).
            // The original_tlist has the SELECT's output columns, which is what ps_ResultTupleSlot is based on.
            // We store this mapping in PrivateData so build_result_tuple can use it during execution.
            let mut output_columns = Vec::new();
            let mut private_data = PrivateData::from(node.custom_private);
            let original_entries = PgList::<pg_sys::TargetEntry>::from_pg(original_tlist);

            for te in original_entries.iter_ptr() {
                if (*(*te).expr).type_ == pg_sys::NodeTag::T_Var {
                    let var = (*te).expr as *mut pg_sys::Var;
                    // Determine if this column comes from outer or inner relation
                    output_columns.push(privdat::OutputColumnInfo {
                        rti: (*var).varno as pg_sys::Index,
                        original_attno: (*var).varattno,
                        is_score: false,
                    });
                } else {
                    let mut is_score = false;
                    let mut rti = 0;
                    for source in private_data.join_clause.plan.sources() {
                        if expr_uses_scores_from_source((*te).expr.cast(), source) {
                            // This expression contains paradedb.score()
                            is_score = true;
                            rti = get_score_func_rti((*te).expr.cast()).unwrap_or(0);
                            break;
                        }
                    }
                    // Non-Var, non-score expression - mark as null (attno = 0)
                    output_columns.push(privdat::OutputColumnInfo {
                        rti,
                        original_attno: 0,
                        is_score,
                    });
                }
            }

            private_data.output_columns = output_columns;
            private_data.join_clause.output_projection = Some(
                private_data
                    .output_columns
                    .iter()
                    .map(|info| build::ChildProjection {
                        rti: info.rti,
                        attno: info.original_attno,
                        is_score: info.is_score,
                    })
                    .collect(),
            );

            // Add heap condition clauses to custom_exprs so they get transformed by set_customscan_references.
            // The Vars in these expressions will be converted to INDEX_VAR references into custom_scan_tlist.
            let path_private_full = PgList::<pg_sys::Node>::from_pg((*best_path).custom_private);
            let mut custom_exprs_list = PgList::<pg_sys::Node>::from_pg(node.custom_exprs);
            // Skip index 0 (PrivateData)
            for i in 1..path_private_full.len() {
                if let Some(node_ptr) = path_private_full.get_ptr(i) {
                    custom_exprs_list.push(node_ptr);
                }
            }
            node.custom_exprs = custom_exprs_list.into_pg();

            // Collect all required fields for execution
            collect_required_fields(
                &mut private_data.join_clause,
                &private_data.output_columns,
                node.custom_exprs,
            );

            // Build, serialize and store the logical plan
            let runtime = tokio::runtime::Builder::new_current_thread()
                .build()
                .expect("Failed to create tokio runtime");
            let logical_plan = runtime
                .block_on(build_joinscan_logical_plan(
                    &private_data.join_clause,
                    &private_data,
                    node.custom_exprs,
                ))
                .expect("Failed to build DataFusion logical plan");
            private_data.logical_plan = Some(
                logical_plan_to_bytes_with_extension_codec(
                    &logical_plan,
                    &PgSearchExtensionCodec::default(),
                )
                .expect("Failed to serialize DataFusion logical plan"),
            );

            // Convert PrivateData back to a list and preserve the restrictlist
            let mut new_private = PgList::<pg_sys::Node>::from_pg(PrivateData::into(private_data));
            let path_private_full = PgList::<pg_sys::Node>::from_pg((*best_path).custom_private);
            for i in 1..path_private_full.len() {
                if let Some(node_ptr) = path_private_full.get_ptr(i) {
                    new_private.push(node_ptr);
                }
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

        let join_keys = join_clause.plan.join_keys();
        let plan_sources = join_clause.plan.sources();

        if !join_keys.is_empty() {
            let keys_str: Vec<_> = join_keys
                .iter()
                .map(|k| {
                    let (outer_relid, outer_alias_name) = plan_sources
                        .iter()
                        .enumerate()
                        .find(|(_, s)| s.contains_rti(k.outer_rti))
                        .map(|(i, s)| (Some(s.scan_info.heaprelid), s.execution_alias(i)))
                        .expect("Outer source not found");

                    let (inner_relid, inner_alias_name) = plan_sources
                        .iter()
                        .enumerate()
                        .find(|(_, s)| s.contains_rti(k.inner_rti))
                        .map(|(i, s)| (Some(s.scan_info.heaprelid), s.execution_alias(i)))
                        .expect("Inner source not found");

                    format!(
                        "{} = {}",
                        get_attname_safe(outer_relid, k.outer_attno, &outer_alias_name),
                        get_attname_safe(inner_relid, k.inner_attno, &inner_alias_name)
                    )
                })
                .collect();
            explainer.add_text("Join Cond", keys_str.join(", "));
        }

        if let Some(expr) = join_clause.plan.join_level_expr() {
            explainer.add_text("Join Predicate", format_join_level_expr(expr, join_clause));
        }

        if let Some(limit) = join_clause.limit {
            explainer.add_text("Limit", limit.to_string());
        }

        if !join_clause.order_by.is_empty() {
            explainer.add_text(
                "Order By",
                join_clause
                    .order_by
                    .iter()
                    .map(|oi| match &oi.feature {
                        OrderByFeature::Field(f) => format!("{} {}", f, oi.direction.as_ref()),
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
                        OrderByFeature::Score => format!("pdb.score() {}", oi.direction.as_ref()),
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
            // For plain EXPLAIN, reconstruct the plan from the serialized logical plan.
            let ctx = create_session_context();
            let runtime = tokio::runtime::Builder::new_current_thread()
                .build()
                .expect("Failed to create tokio runtime");
            let logical_plan = logical_plan_from_bytes_with_extension_codec(
                logical_plan,
                &ctx.task_ctx(),
                &PgSearchExtensionCodec::default(),
            )
            .expect("Failed to deserialize logical plan");
            let physical_plan = runtime
                .block_on(build_joinscan_physical_plan(&ctx, logical_plan))
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
        _estate: *mut pg_sys::EState,
        eflags: i32,
    ) {
        if eflags & (pg_sys::EXEC_FLAG_EXPLAIN_ONLY as i32) == 0 {
            unsafe {
                state.custom_state_mut().max_memory = (pg_sys::work_mem as usize) * 1024;
                state.custom_state_mut().result_slot = Some(state.csstate.ss.ps.ps_ResultTupleSlot);
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
                    .build()
                    .unwrap();
                let join_clause = state.custom_state().join_clause.clone();
                let snapshot = state.csstate.ss.ps.state.as_ref().unwrap().es_snapshot;

                let mut base_relations = Vec::new();
                join_clause.collect_base_relations(&mut base_relations);
                for base in base_relations {
                    let rti = base.heap_rti;
                    let heaprelid = base.heaprelid;
                    let heaprel = PgSearchRelation::open(heaprelid);
                    let visibility_checker =
                        VisibilityChecker::with_rel_and_snap(&heaprel, snapshot);
                    let fetch_slot =
                        pg_sys::MakeTupleTableSlot(heaprel.rd_att, &pg_sys::TTSOpsBufferHeapTuple);
                    state.custom_state_mut().relations.insert(
                        rti,
                        scan_state::RelationState {
                            _heaprel: heaprel,
                            visibility_checker,
                            fetch_slot,
                            ctid_col_idx: None,
                        },
                    );
                }

                // Deserialize the logical plan and convert to execution plan
                let plan_bytes = state
                    .custom_state()
                    .logical_plan
                    .as_ref()
                    .expect("Logical plan is required");

                // Deserialize the logical plan
                let ctx = create_session_context();
                let codec = PgSearchExtensionCodec {
                    parallel_state: state.custom_state().parallel_state,
                };
                let logical_plan = logical_plan_from_bytes_with_extension_codec(
                    plan_bytes,
                    &ctx.task_ctx(),
                    &codec,
                )
                .expect("Failed to deserialize logical plan");

                // Convert logical plan to physical plan
                let plan = runtime
                    .block_on(build_joinscan_physical_plan(&ctx, logical_plan))
                    .expect("Failed to create execution plan");

                let memory_pool =
                    Arc::new(PanicOnOOMMemoryPool::new(state.custom_state().max_memory));
                let task_ctx = Arc::new(
                    TaskContext::default()
                        .with_session_config(ctx.state().config().clone())
                        .with_runtime(Arc::new(
                            RuntimeEnvBuilder::new()
                                .with_memory_pool(memory_pool)
                                .build()
                                .expect("Failed to create RuntimeEnv"),
                        )),
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
                    if let Some(stripped) = field.name().strip_prefix("ctid_") {
                        if let Ok(rti) = stripped.parse::<pg_sys::Index>() {
                            if let Some(rel_state) =
                                state.custom_state_mut().relations.get_mut(&rti)
                            {
                                rel_state.ctid_col_idx = Some(i);
                            }
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
    }
}

impl JoinScan {
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
        let mut fetched_rtis = crate::api::HashSet::default();

        // Fetch tuples for all RTIs referenced in the output columns
        for col_info in &output_columns {
            if col_info.rti != 0 && !fetched_rtis.contains(&col_info.rti) {
                let rti = col_info.rti;
                // Get the CTID for this RTI from the DataFusion result batch
                let ctid = {
                    let batch = state.custom_state().current_batch.as_ref()?;
                    let rel_state = state.custom_state().relations.get(&rti)?;
                    let ctid_col = batch.column(rel_state.ctid_col_idx?);
                    ctid_col
                        .as_any()
                        .downcast_ref::<arrow_array::UInt64Array>()
                        .expect("ctid should be u64")
                        .value(row_idx)
                };
                // Fetch the tuple from the heap using the CTID
                let rel_state = state.custom_state_mut().relations.get_mut(&rti)?;
                if !rel_state
                    .visibility_checker
                    .fetch_tuple_direct(ctid, rel_state.fetch_slot)
                {
                    return None;
                }
                // Make sure slots have all attributes deformed
                pg_sys::slot_getallattrs(rel_state.fetch_slot);
                fetched_rtis.insert(rti);
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
            // Handle score columns specially
            if col_info.is_score {
                let score_col = batch.column(i);
                let score = if let Some(score_array) = score_col
                    .as_any()
                    .downcast_ref::<arrow_array::Float32Array>()
                {
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
                continue;
            }

            if col_info.rti == 0 {
                // Non-Var, non-score expression - set null
                *nulls.add(i) = true;
                continue;
            }
            // Determine which slot to read from based on RTI
            let rel_state = state.custom_state().relations.get(&col_info.rti)?;
            let source_slot = rel_state.fetch_slot;
            let original_attno = col_info.original_attno;
            // Get the attribute value from the source slot using the original attribute number
            if original_attno <= 0
                || original_attno > (*(*source_slot).tts_tupleDescriptor).natts as i16
            {
                *nulls.add(i) = true;
                continue;
            }

            let mut is_null = false;
            *datums.add(i) = pg_sys::slot_getattr(source_slot, original_attno as i32, &mut is_null);
            *nulls.add(i) = is_null;
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
