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

mod build;
mod explain;
mod memory;
mod planner;
mod planning;
mod predicate;
mod privdat;
mod scan_state;
mod translator;
pub mod visibility_filter;

pub use self::build::CtidColumn;
use self::build::{JoinCSClause, RelNode, RelationAlias};
use self::explain::{format_join_level_expr, get_attname_safe};
use self::memory::create_memory_pool;
use self::planning::{
    collect_join_sources, collect_required_fields, ensure_score_bubbling,
    expr_uses_scores_from_source, extract_join_conditions, extract_orderby, get_score_func_rti,
    order_by_columns_are_fast_fields, pathkey_uses_scores_from_source,
};
use self::predicate::{extract_join_level_conditions, is_column_fast_field};
use self::privdat::PrivateData;

use self::scan_state::{
    build_joinscan_logical_plan, build_joinscan_physical_plan, create_execution_session_context,
    JoinScanState,
};
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
use crate::scan::codec::{
    deserialize_logical_plan, deserialize_logical_plan_parallel, serialize_logical_plan,
};
use datafusion::execution::runtime_env::RuntimeEnvBuilder;
use datafusion::execution::TaskContext;
use datafusion::physical_plan::displayable;
use datafusion::physical_plan::metrics::MetricValue;
use datafusion::physical_plan::{DisplayFormatType, ExecutionPlan};
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
        // The leader uses these in exec_custom_scan to inject them into the codec.
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
    /// Keyed by plan_position (not indexrelid) so self-joins with the same index
    /// but different segment sets (partitioned vs replicated) are correctly
    /// disambiguated. If this were keyed only by indexrelid, one source could
    /// inject the other source's segment set and make packed DocAddresses resolve
    /// against the wrong segment ordering.
    ///
    /// Workers use frozen segment IDs from DSM to match the leader's segment set.
    /// Leader/serial uses manifests captured with the same snapshot.
    fn build_index_segment_ids(
        state: &mut CustomScanStateWrapper<Self>,
        join_clause: &JoinCSClause,
        plan_sources: &[&build::JoinSource],
    ) -> crate::api::HashMap<usize, crate::api::HashSet<tantivy::index::SegmentId>> {
        let mut map = crate::api::HashMap::default();
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
                        map.insert(i, ids);
                    }
                } else if let Some(ids) = non_partitioning_segs.get(np_counter) {
                    map.insert(i, ids.clone());
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
                    map.insert(i, ids);
                }
            }
        }
        map
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
                .map(|s| {
                    RelationAlias::new(s.scan_info.alias.as_deref())
                        .warning_context(s.scan_info.heaprelid)
                })
                .collect();

            let join_conditions = extract_join_conditions(extra, &all_sources);

            // The minimum requirement for considering the join scan is that a search predicate
            // is used.
            if !all_sources.iter().any(|s| s.scan_info.has_search_predicate)
                && !join_conditions.has_search_predicate
            {
                // Join does not use our operator anywhere: do not trigger.
                return Vec::new();
            }

            //
            // After this point the join is considered interesting: all returns should have
            // `add_planner_warning` calls to explain themselves.
            //

            // All tables must have bm25 indexes.
            if all_sources
                .iter()
                .any(|s| s.scan_info.indexrelid == pg_sys::InvalidOid)
            {
                Self::add_planner_warning(
                    "JoinScan not used: all join sources must have a BM25 index",
                    &aliases,
                );
                return Vec::new();
            }

            // TODO: Add support for Aggregate functions
            // Bail out if the query has aggregates — LIMIT applies to aggregate
            // output not to the rows feeding the aggregate, so pushing LIMIT into
            // DataFusion would produce wrong results until we add support for Agg functions
            if (*(*root).parse).hasAggs {
                Self::add_planner_warning(
                    "JoinScan not used: queries with aggregate functions are not supported",
                    (),
                );
                return Vec::new();
            }

            // TODO(join-types): Currently INNER, SEMI, and ANTI joins are supported.
            // Future work should add:
            // - LEFT JOIN: Return NULL for non-matching right rows; track matched left rows
            // - RIGHT JOIN: Swap left/right sides, then use LEFT logic
            // - FULL OUTER JOIN: Track unmatched rows on both sides; two-pass or marking approach
            //
            // WARNING: If enabling other join types, you MUST review the parallel partitioning
            // strategy documentation in `pg_search/src/postgres/customscan/joinscan/scan_state.rs`.
            // The current "Partition Outer / Replicate Inner" strategy is incorrect for Right/Full joins.

            // JoinScan requires a LIMIT clause. This restriction exists because we gain a
            // significant benefit from using the column store when it enables late-materialization
            // of heap tuples _after_ the join has run.
            let limit_offset = LimitOffset::from_root(root);
            if limit_offset.limit.is_none() {
                Self::add_planner_warning("JoinScan not used: query must have a LIMIT clause", ());
                return Vec::new();
            }

            // Require equi-join keys for JoinScan.
            // Without equi-join keys, we'd have a cross join requiring O(N*M) comparisons
            // where join complexity explodes. PostgreSQL's native join
            // handles cartesian products more efficiently.
            if join_conditions.equi_keys.is_empty() {
                Self::add_planner_warning(
                    "JoinScan not used: at least one equi-join key (e.g., a.id = b.id) is required",
                    &aliases,
                );
                return Vec::new();
            }

            // Detect DISTINCT and validate columns are fast fields
            let has_distinct = !(*(*root).parse).distinctClause.is_null();
            if has_distinct && !distinct_columns_are_fast_fields(root, &all_sources) {
                Self::add_planner_warning(
                    "JoinScan not used: all DISTINCT columns must be fast fields in the BM25 index",
                    &aliases,
                );
                return Vec::new();
            }

            // Check if all ORDER BY columns are fast fields
            // JoinScan requires fast field access for efficient sorting
            if !order_by_columns_are_fast_fields(root, &all_sources) {
                Self::add_planner_warning(
                    "JoinScan not used: all ORDER BY columns must be fast fields in the BM25 index",
                    &aliases,
                );
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
                            Self::add_planner_warning(
                                "JoinScan not used: join key columns must be fast fields",
                                &aliases,
                            );
                            return Vec::new();
                        }
                    }
                    _ => return Vec::new(), // Should not happen if extraction logic is correct
                }
            }

            // Add current level keys
            join_keys.extend(join_conditions.equi_keys.clone());

            let parsed_jointype = match build::JoinType::try_from(jointype) {
                Ok(jt) => jt,
                Err(e) => {
                    Self::add_planner_warning(e.to_string(), ());
                    return Vec::new();
                }
            };

            let plan = RelNode::Join(Box::new(build::JoinNode {
                join_type: parsed_jointype,
                left: outer_node,
                right: inner_node,
                equi_keys: join_conditions.equi_keys,
                filter: None,
            }));

            let unsupported = plan.unsupported_join_types();
            if !unsupported.is_empty() {
                Self::add_detailed_planner_warning(
                    "JoinScan not used: only INNER, ANTI, and SEMI JOIN are currently supported",
                    &aliases,
                    unsupported
                        .iter()
                        .map(|t| t.to_string().to_uppercase())
                        .collect::<Vec<_>>(),
                );
                return Vec::new();
            }

            let mut join_clause = JoinCSClause::new(plan)
                .with_limit(limit_offset.limit)
                .with_offset(limit_offset.offset)
                .with_distinct(has_distinct);

            for source in join_clause.plan.sources_mut() {
                // Check if paradedb.score() is used anywhere in the query for each side.
                // This includes ORDER BY, SELECT list, or any other expression.
                // We need to check ALL sides because scores come from the pre-materialized search results.
                let score_in_tlist =
                    expr_uses_scores_from_source((*root).processed_tlist.cast(), source);

                let score_in_pathkey = pathkey_uses_scores_from_source(root, source);

                if score_in_tlist || score_in_pathkey {
                    // Record score_needed for each side
                    ensure_score_bubbling(source);
                }
            }

            // The current parallel strategy partitions exactly one source and replicates all
            // others. For SEMI JOIN and ANTI JOIN correctness, the partitioned source MUST be
            // the left side to avoid duplicate emissions from replicated workers.
            // TODO: Because we force the left side to be partitioned, we will fully replicate
            // (broadcast) the right side even if it is significantly larger. This reduces
            // performance but ensures correctness. We can remove this limitation once we transition
            // away from a broadcast strategy to true plan partitioning:
            // https://github.com/paradedb/paradedb/issues/4152
            if join_clause.plan.has_semi_or_anti() {
                if join_clause.partitioning_source_index() != 0 {
                    pgrx::warning!(
                        "For SEMI/ANTI join correctness, JoinScan needs to use a suboptimal \
                        parallel partitioning strategy for this query. See \
                        https://github.com/paradedb/paradedb/issues/4152"
                    );
                }
                join_clause = join_clause.with_forced_partitioning(0);
            }

            let current_sources = join_clause.plan.sources();

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
                        Self::add_planner_warning(
                                "JoinScan not used: failed to extract join-level conditions (ensure all referenced columns are fast fields)",
                                &aliases,
                            );
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
            let output_rtis = join_clause.plan.output_rtis();
            let order_by = match extract_orderby(root, &current_sources_after_cond, &output_rtis) {
                Some(ob) => ob,
                None => {
                    Self::add_planner_warning(
                            "JoinScan not used: ORDER BY column is not available in the joined output schema",
                            &aliases,
                        );
                    return Vec::new();
                }
            };
            join_clause = join_clause.with_order_by(order_by);

            // Use simple fixed costs since we force the path anyway.
            // Cost estimation is deferred to DataFusion integration.
            let startup_cost = crate::DEFAULT_STARTUP_COST;
            let total_cost = startup_cost + 1.0;
            let mut result_rows = limit_offset.limit.map(|l| l as f64).unwrap_or(1000.0);

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

                let processes = processes as u64;
                let partitioning_idx = join_clause.partitioning_source_index();
                for (idx, source) in join_clause.plan.sources_mut().into_iter().enumerate() {
                    if let crate::scan::info::RowEstimate::Known(n) = source.scan_info.estimate {
                        if idx == partitioning_idx {
                            source.scan_info.estimated_rows_per_worker = Some(n / processes);
                        } else {
                            source.scan_info.estimated_rows_per_worker = Some(n);
                        }
                    }
                }
            } else {
                for source in join_clause.plan.sources_mut() {
                    if let crate::scan::info::RowEstimate::Known(n) = source.scan_info.estimate {
                        source.scan_info.estimated_rows_per_worker = Some(n);
                    }
                }
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
            Self::mark_contexts_successful(&aliases);

            vec![custom_path]
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
                    let rti = (*var).varno as pg_sys::Index;
                    let attno = (*var).varattno;
                    let plan_position = private_data
                        .join_clause
                        .plan_position(root.into(), rti, attno)
                        .unwrap_or_else(|| {
                            panic!(
                                "Failed to resolve output Var to plan_position (rti={}, attno={})",
                                rti, attno
                            )
                        });
                    output_columns.push(privdat::OutputColumnInfo {
                        plan_position,
                        rti,
                        original_attno: attno,
                        is_score: false,
                    });
                } else {
                    let mut is_score = false;
                    let mut rti = 0;
                    let mut plan_position = 0usize;
                    for source in private_data.join_clause.plan.sources() {
                        if expr_uses_scores_from_source((*te).expr.cast(), source) {
                            // This expression contains paradedb.score()
                            is_score = true;
                            rti = get_score_func_rti((*te).expr.cast()).unwrap_or(0);
                            plan_position = source.plan_position;
                            break;
                        }
                    }
                    // Non-Var, non-score expression - mark as null (attno = 0)
                    output_columns.push(privdat::OutputColumnInfo {
                        plan_position,
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
                serialize_logical_plan(&logical_plan)
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
                        OrderByFeature::Score { .. } => {
                            format!("pdb.score() {}", oi.direction.as_ref())
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
            // For plain EXPLAIN, reconstruct the plan using the execution context
            // so that VisibilityFilterExec nodes appear in the displayed plan,
            // matching what actually runs during EXPLAIN ANALYZE.
            let snapshot = unsafe { pg_sys::GetActiveSnapshot() };
            let join_clause = &state.custom_state().join_clause;
            let ctx = create_execution_session_context(join_clause, snapshot);
            let runtime = tokio::runtime::Builder::new_current_thread()
                .build()
                .expect("Failed to create tokio runtime");
            let expr_context = crate::postgres::utils::ExprContextGuard::new();
            let logical_plan = deserialize_logical_plan(
                logical_plan,
                &ctx.task_ctx(),
                None,
                Some(expr_context.as_ptr()),
                None,
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

                // Deserialize the logical plan and convert to execution plan
                let planstate = state.planstate();
                // Clone plan_bytes to release the immutable borrow on `state`
                // before the mutable borrow in ensure_source_manifests below.
                let plan_bytes = state
                    .custom_state()
                    .logical_plan
                    .clone()
                    .expect("Logical plan is required");

                let index_segment_ids =
                    Self::build_index_segment_ids(state, &join_clause, &plan_sources);

                let ctx = create_execution_session_context(&join_clause, snapshot);
                let logical_plan = deserialize_logical_plan_parallel(
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
                    .block_on(build_joinscan_physical_plan(&ctx, logical_plan))
                    .expect("Failed to create execution plan");

                let memory_pool = create_memory_pool(
                    &plan,
                    pg_sys::work_mem as usize * 1024,
                    pg_sys::hash_mem_multiplier,
                );

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
        let mut fetched_sources = crate::api::HashSet::default();

        // Fetch tuples for all RTIs referenced in the output columns
        for col_info in &output_columns {
            if col_info.rti != 0 && !fetched_sources.contains(&col_info.plan_position) {
                // Get the CTID for this RTI from the DataFusion result batch
                let ctid = {
                    let batch = state.custom_state().current_batch.as_ref()?;
                    let rel_state = state
                        .custom_state()
                        .relations
                        .get(&col_info.plan_position)?;
                    let ctid_col = batch.column(rel_state.ctid_col_idx?);
                    ctid_col
                        .as_any()
                        .downcast_ref::<arrow_array::UInt64Array>()
                        .expect("ctid should be u64")
                        .value(row_idx)
                };
                // Fetch the tuple from the heap using the CTID
                let rel_state = state
                    .custom_state_mut()
                    .relations
                    .get_mut(&col_info.plan_position)?;
                if !rel_state
                    .visibility_checker
                    .fetch_tuple_direct(ctid, rel_state.fetch_slot)
                {
                    return None;
                }
                // Make sure slots have all attributes deformed
                pg_sys::slot_getallattrs(rel_state.fetch_slot);
                fetched_sources.insert(col_info.plan_position);
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
            let rel_state = state
                .custom_state()
                .relations
                .get(&col_info.plan_position)?;
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
