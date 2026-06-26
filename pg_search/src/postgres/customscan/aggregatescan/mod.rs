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

pub mod aggregate_type;
pub mod build;
pub mod datafusion_build;
pub mod datafusion_exec;
pub mod datafusion_project;
pub mod exec;
pub mod filterquery;
pub mod groupby;
pub mod join_targetlist;
pub mod json_rewrite;
pub mod limit_offset;
pub mod orderby;
use crate::postgres::customscan::orderby::validate_topk_compatibility;
pub mod privdat;
pub mod scan_state;
pub mod searchquery;
pub mod targetlist;

// Re-export commonly used types for easier access
pub use aggregate_type::AggregateType;
pub use groupby::GroupingColumn;
pub use targetlist::TargetListEntry;

use std::sync::Arc;

use crate::postgres::catalog::is_ltree_oid;

use datafusion::execution::TaskContext;
use datafusion::physical_plan::ExecutionPlan;
use datafusion_distributed::{display_plan_ascii, DistributedExec, DistributedTaskContext};

use datafusion_distributed::shm::MppMesh;

use crate::postgres::customscan::mpp::glue::mpp_is_active;
use crate::postgres::customscan::mpp::interrupt::block_on_next;

use crate::api::agg_funcoid;
use crate::api::SortDirection;
use crate::gucs;

use crate::aggregate::{NULL_SENTINEL_MAX, NULL_SENTINEL_MIN};
use crate::customscan::aggregatescan::build::AggregateCSClause;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexManifest;
use crate::postgres::customscan::aggregatescan::datafusion_build::{
    all_have_bm25_index, collect_join_agg_sources, extract_join_tree_from_parse, has_any_bm25_index,
};
use crate::postgres::customscan::aggregatescan::datafusion_exec::{
    build_join_aggregate_plan, create_aggregate_session_context, MppPlanContext,
};
use crate::postgres::customscan::aggregatescan::datafusion_project::project_aggregate_row_to_slot;
use crate::postgres::customscan::aggregatescan::exec::{
    aggregation_results_iter, AggregateResult, AggregationResultsRow,
};
use crate::postgres::customscan::aggregatescan::groupby::GroupByClause;
use crate::postgres::customscan::aggregatescan::join_targetlist::extract_aggregate_targetlist;
use crate::postgres::customscan::aggregatescan::privdat::PrivateData;
use crate::postgres::customscan::aggregatescan::scan_state::{
    AggregateScanState, ExecutionState, WrappedAggregateProjection,
};
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::builders::custom_state::{
    CustomScanStateBuilder, CustomScanStateWrapper,
};
use crate::postgres::customscan::exec::{
    begin_custom_scan, end_custom_scan, exec_custom_scan, explain_custom_scan, rescan_custom_scan,
    shutdown_custom_scan,
};
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::hook::query_has_paradedb_agg;
use crate::postgres::customscan::joinscan::scan_state::{build_physical_plan, build_task_context};
use crate::postgres::customscan::limit_offset::LimitOffset;
use crate::postgres::customscan::orderby::is_collation_pushdown_safe;
use crate::postgres::customscan::projections::{create_placeholder_targetlist, placeholder_procid};
use crate::postgres::customscan::solve_expr::SolvePostgresExpressions;
use crate::postgres::customscan::{range_table, CreateUpperPathsHookArgs, CustomScan};
use crate::postgres::datetime::PostgresDateTime;
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::postgres::rel_get_bm25_index;
use crate::postgres::types::{is_datetime_type, TantivyValue};
use crate::postgres::utils::{add_vars_to_tlist, is_unnest_func, make_text_const};
use crate::postgres::ParallelScanArgs;
use crate::postgres::PgSearchRelation;
use crate::scan::codec::serialize_logical_plan;
use pgrx::{pg_sys, PgList, PgMemoryContexts, PgTupleDesc};
use std::ffi::CStr;

#[derive(Default)]
pub struct AggregateScan;

/// A collection of index information that is necessary for making result-rewriting decisions
pub struct AggIndexInfo {
    pub created_by_version: Option<crate::api::version::Version>,
    pub schema: crate::schema::SearchIndexSchema,
}
impl From<&crate::postgres::rel::PgSearchRelation> for AggIndexInfo {
    fn from(value: &crate::postgres::rel::PgSearchRelation) -> Self {
        Self {
            created_by_version: value.created_by_version(),
            schema: value.schema().expect("schema should be initialized by now"),
        }
    }
}

/// Why the DataFusion aggregate path declined to produce a custom path.
///
/// Mirrors `JoinScan`'s `JoinPathDecline` shape: `Quiet` is for early gates
/// that filter out non-candidate inputs, `Warn` is for validation failures
/// past the "candidate" boundary that owe the planner a NOTICE.
enum AggregatePathDecline {
    Quiet,
    Warn(AggregateDeclineReason),
}

/// Specific reason a `Warn` decline was raised. Each variant maps 1:1 to a
/// planner-warning string the inline code used to emit.
enum AggregateDeclineReason {
    NotAllBm25,
    NonEquiJoinQuals,
    CrossJoin,
    /// Errors carrying a free-form message (parse-tree extraction, target-list
    /// extraction, fast-field population) — the underlying helper already
    /// produces a contextual string.
    Other(String),
}

impl AggregateDeclineReason {
    fn emit(&self) {
        let alias = "join".to_string();
        match self {
            AggregateDeclineReason::NotAllBm25 => AggregateScan::add_planner_warning(
                "Aggregate Scan (DataFusion) not used: all tables in the join must have BM25 indexes",
                alias,
            ),
            AggregateDeclineReason::NonEquiJoinQuals => AggregateScan::add_planner_warning(
                "Aggregate Scan (DataFusion) not used: join has non-equi quals that cannot be pushed to individual table scans",
                alias,
            ),
            AggregateDeclineReason::CrossJoin => AggregateScan::add_planner_warning(
                "Aggregate Scan (DataFusion) not used: CROSS JOINs are not supported (no equi-join keys)",
                alias,
            ),
            AggregateDeclineReason::Other(msg) => AggregateScan::add_planner_warning(
                format!("Aggregate Scan (DataFusion) not used: {}", msg),
                alias,
            ),
        }
    }
}

impl CustomScan for AggregateScan {
    const NAME: &'static CStr = c"ParadeDB Aggregate Scan";
    type Args = CreateUpperPathsHookArgs;
    type State = AggregateScanState;
    type PrivateData = PrivateData;

    fn exec_methods() -> pg_sys::CustomExecMethods {
        pg_sys::CustomExecMethods {
            CustomName: Self::NAME.as_ptr(),
            BeginCustomScan: Some(begin_custom_scan::<Self>),
            ExecCustomScan: Some(exec_custom_scan::<Self>),
            EndCustomScan: Some(end_custom_scan::<Self>),
            ReScanCustomScan: Some(rescan_custom_scan::<Self>),
            MarkPosCustomScan: None,
            RestrPosCustomScan: None,
            // No PG parallel callbacks: MPP launches its own workers via `mpp::launch`, and the
            // aggregate node has no non-MPP parallel mode, so it never runs under a Gather.
            EstimateDSMCustomScan: None,
            InitializeDSMCustomScan: None,
            ReInitializeDSMCustomScan: None,
            InitializeWorkerCustomScan: None,
            ShutdownCustomScan: Some(shutdown_custom_scan::<Self>),
            ExplainCustomScan: Some(explain_custom_scan::<Self>),
        }
    }

    fn create_custom_path(builder: CustomPathBuilder<Self>) -> Vec<pg_sys::CustomPath> {
        let has_paradedb_agg = unsafe {
            let parse = builder.args().root().parse;
            !parse.is_null() && query_has_paradedb_agg(parse, true)
        };

        let input_rel = builder.args().input_rel();

        match input_rel.reloptkind {
            pg_sys::RelOptKind::RELOPT_BASEREL => {
                let use_datafusion = unsafe {
                    // If the estimated number of groups exceeds Tantivy's bucket limit,
                    // fall back to DataFusion which has no such limit.
                    let estimated_groups = builder.args().output_rel().rows;
                    let max_buckets = gucs::max_term_agg_buckets() as f64;
                    if estimated_groups > max_buckets {
                        true
                    } else {
                        // ORDER BY aggregate + LIMIT: route to DataFusion which has
                        // no bucket cap and provides native TopK via SortExec(fetch=K).
                        build::has_aggregate_orderby_with_limit(builder.args())
                    }
                };
                if use_datafusion {
                    if !gucs::enable_aggregate_custom_scan() && !has_paradedb_agg {
                        return Vec::new();
                    }
                    return Self::build_datafusion_aggregate_path(builder);
                }
                Self::build_tantivy_aggregate_path(builder, has_paradedb_agg)
            }
            pg_sys::RelOptKind::RELOPT_JOINREL => {
                if !gucs::enable_aggregate_custom_scan() && !has_paradedb_agg {
                    return Vec::new();
                }
                Self::build_datafusion_aggregate_path(builder)
            }
            _ => Vec::new(),
        }
    }

    fn plan_custom_path(mut builder: CustomScanBuilder<Self>) -> pg_sys::CustomScan {
        // Extract values from private data before the match to avoid borrow conflicts.
        let (is_tantivy, heap_rti_val, should_replace_val, clause_count_val) =
            match builder.custom_private() {
                PrivateData::Tantivy {
                    heap_rti,
                    aggregate_clause,
                    ..
                } => (
                    true,
                    *heap_rti,
                    aggregate_clause.planner_should_replace_aggrefs(),
                    0usize,
                ),
                PrivateData::DataFusion {
                    multi_table_clause_count,
                    ..
                } => (false, 0, false, *multi_table_clause_count),
            };

        if is_tantivy {
            builder.set_scanrelid(heap_rti_val);
            if should_replace_val {
                unsafe {
                    let mut cscan = builder.build();
                    let plan = &mut cscan.scan.plan;
                    replace_aggrefs_in_target_list(plan);
                    cscan
                }
            } else {
                builder.build()
            }
        } else {
            // For join aggregates, scanrelid=0 (no single base relation)
            builder.set_scanrelid(0);

            // Check if the query has pathkeys (ORDER BY) before consuming builder.
            let root = builder.args().root;
            let has_pathkeys = unsafe {
                !(*root).query_pathkeys.is_null() && pg_sys::list_length((*root).query_pathkeys) > 0
            };

            let clause_count = clause_count_val;
            let best_path = builder.args().best_path;

            unsafe {
                let mut cscan = builder.build();

                // Set custom_scan_tlist so Postgres can resolve variable references
                // when Sort/Limit nodes are placed above this scanrelid=0 CustomScan.
                // This is a copy of the original targetlist (with Aggrefs intact) —
                // setrefs.c uses it to create INDEX_VAR references in parent nodes.
                let original_tlist = cscan.scan.plan.targetlist;
                cscan.custom_scan_tlist =
                    pg_sys::copyObjectImpl(original_tlist.cast()).cast::<pg_sys::List>();

                // Move raw PG Expr pointers from custom_private to custom_exprs
                // so setrefs transforms their Var nodes to INDEX_VAR references.
                if clause_count > 0 {
                    // Before moving to custom_exprs, ensure all Vars referenced
                    // in the predicate clauses are present in custom_scan_tlist.
                    // setrefs needs them there to create INDEX_VAR references.
                    let path_private_full =
                        PgList::<pg_sys::Node>::from_pg((*best_path).custom_private);
                    let mut tlist = PgList::<pg_sys::TargetEntry>::from_pg(cscan.custom_scan_tlist);
                    // Skip index 0 (PrivateData JSON)
                    for i in 1..path_private_full.len() {
                        if let Some(node_ptr) = path_private_full.get_ptr(i) {
                            add_vars_to_tlist(node_ptr, &mut tlist);
                        }
                    }
                    cscan.custom_scan_tlist = tlist.into_pg();

                    let path_private_full =
                        PgList::<pg_sys::Node>::from_pg((*best_path).custom_private);
                    let mut custom_exprs_list = PgList::<pg_sys::Node>::from_pg(cscan.custom_exprs);
                    // Skip index 0 (PrivateData JSON)
                    for i in 1..path_private_full.len() {
                        if let Some(node_ptr) = path_private_full.get_ptr(i) {
                            custom_exprs_list.push(node_ptr);
                        }
                    }
                    cscan.custom_exprs = custom_exprs_list.into_pg();
                }

                let parallel_aware = (*best_path).path.parallel_aware;
                if !has_pathkeys && !parallel_aware {
                    // Non-MPP, no-pathkeys: safe to replace Aggrefs at plan
                    // time. The customscan emits final aggregate rows, no
                    // Gather above us, no setrefs match needed.
                    let plan = &mut cscan.scan.plan;
                    replace_aggrefs_in_target_list(plan);
                }
                // MPP path (parallel_aware): leave Aggrefs in
                // `plan.targetlist`. PG's `set_plan_refs` walks the
                // partial-worker tlist looking for `equal()` matches to
                // wire the Gather's projection — observed behaviour is
                // that replacing the Aggrefs with `pdb.agg_fn(...)`
                // placeholders breaks every match and the planner falls
                // back to `Single Copy: true`. We haven't traced the
                // exact upstream code path that does the rejection (it
                // doesn't necessarily go through Partial+Final aggregate
                // insertion), only the symptom. Either way the workaround
                // is the same: keep the Aggrefs through path/plan
                // construction and replace them at execution time in
                // `create_custom_scan_state`.
                //
                // When has_pathkeys: same reason — `make_sort_from_pathkeys`
                // needs to see the original Aggrefs. Replacement deferred
                // to `create_custom_scan_state` (execution time) for both
                // MPP-on and MPP-off+ORDER-BY cases.
                cscan
            }
        }
    }

    fn create_custom_scan_state(
        mut builder: CustomScanStateBuilder<Self, Self::PrivateData>,
    ) -> *mut CustomScanStateWrapper<Self> {
        match builder.custom_private().clone() {
            PrivateData::Tantivy {
                indexrelid,
                aggregate_clause,
                ..
            } => {
                if !aggregate_clause.planner_should_replace_aggrefs() {
                    unsafe {
                        let cscan = builder.args().cscan;
                        let plan = &mut (*cscan).scan.plan;
                        replace_aggrefs_in_target_list(plan);
                    }
                }
                builder.custom_state().indexrelid = indexrelid;
                builder.custom_state().execution_rti =
                    unsafe { (*builder.args().cscan).scan.scanrelid as pg_sys::Index };
                builder.custom_state().aggregate_clause = *aggregate_clause.clone();
                builder.custom_state().base_aggregate_clause = Some(*aggregate_clause);
                builder.build()
            }
            PrivateData::DataFusion {
                plan,
                targetlist,
                topk,
                join_level_predicates,
                multi_table_predicates,
                having_filter,
                ..
            } => {
                // Replace Aggrefs for DataFusion path too
                let (custom_exprs, custom_scan_tlist) = unsafe {
                    let cscan = builder.args().cscan;
                    let pg_plan = &mut (*cscan).scan.plan;
                    replace_aggrefs_in_target_list(pg_plan);
                    ((*cscan).custom_exprs, (*cscan).custom_scan_tlist)
                };
                builder.custom_state().datafusion_state = Some(scan_state::DataFusionAggState {
                    plan,
                    targetlist,
                    topk,
                    base_join_level_predicates: Some(join_level_predicates.clone()),
                    join_level_predicates,
                    multi_table_predicates,
                    custom_exprs,
                    custom_scan_tlist,
                    having_filter,
                    runtime: None,
                    physical_plan: None,
                    stream: None,
                    current_batch: None,
                    batch_row_idx: 0,
                    group_df_indices: Vec::new(),
                    mpp: None,
                    mpp_plan_bytes: None,
                });
                builder.build()
            }
        }
    }

    fn explain_custom_scan(
        state: &CustomScanStateWrapper<Self>,
        _ancestors: *mut pg_sys::List,
        explainer: &mut Explainer,
    ) {
        if state.custom_state().is_datafusion_backend() {
            explainer.add_text("Backend", "DataFusion");
            if explainer.is_analyze() {
                if let Some(ref df_state) = state.custom_state().datafusion_state {
                    Self::render_executed_plan_with_metrics(df_state, explainer);
                }
            }
            if let Some(ref df_state) = state.custom_state().datafusion_state {
                // Show indexes from the join tree sources
                let indexes: Vec<String> = df_state
                    .plan
                    .sources()
                    .iter()
                    .map(|s| {
                        let alias = s.scan_info.alias.as_deref().unwrap_or("unknown");
                        format!(
                            "{} ({})",
                            PgSearchRelation::open(s.scan_info.indexrelid).name(),
                            alias
                        )
                    })
                    .collect();
                if !indexes.is_empty() {
                    explainer.add_text("Indexes", indexes.join(", "));
                }

                // Show GROUP BY columns
                if !df_state.targetlist.group_columns.is_empty() {
                    // TODO: When grouping on expressions with the same underlying input columns,
                    // it's possible to get dupes here. We should consider rendering the expression
                    // instead, but for now we dedupe.
                    let mut groups: Vec<String> = df_state
                        .targetlist
                        .group_columns
                        .iter()
                        .map(|gc| gc.field_name.clone())
                        .collect();
                    groups.sort();
                    groups.dedup();
                    explainer.add_text("Group By", groups.join(", "));
                }

                // Show join-level search predicates (cross-table WHERE filters)
                if !df_state.join_level_predicates.is_empty() {
                    let preds: Vec<String> = df_state
                        .join_level_predicates
                        .iter()
                        .map(|p| p.display_string.clone())
                        .collect();
                    explainer.add_text("Search Filter", preds.join(" AND "));
                }

                // Show multi-table predicates (non-@@@ cross-table filters)
                if !df_state.multi_table_predicates.is_empty() {
                    let preds: Vec<String> = df_state
                        .multi_table_predicates
                        .iter()
                        .map(|p| p.description.clone())
                        .collect();
                    explainer.add_text("Multi-Table Filter", preds.join(" AND "));
                }

                // Show aggregates
                let aggs: Vec<String> = df_state
                    .targetlist
                    .aggregates
                    .iter()
                    .map(|a| {
                        if a.field_refs.is_empty() {
                            // CountStar displays as "COUNT(*)" — no extra wrapping needed.
                            // Other no-arg aggregates (none currently) also use Display directly.
                            a.agg_kind.to_string()
                        } else {
                            let fields: Vec<&str> =
                                a.field_refs.iter().map(|r| r.field_name.as_str()).collect();
                            format!("{}({})", a.agg_kind, fields.join(", "))
                        }
                    })
                    .collect();
                if !aggs.is_empty() {
                    explainer.add_text("Aggregates", aggs.join(", "));
                }

                // Rebuild and render the DataFusion physical plan so
                // reviewers can see the join/aggregate/network shape PG
                // is about to execute. Matches the joinscan EXPLAIN
                // behaviour. For plain EXPLAIN we build with the leader's
                // session context (distributed planner included) but
                // skip installing the runtime mesh — `create_physical_plan`
                // only needs `WorkerTransport` for boundary node
                // construction, not actual execute calls. Build failures
                // here shouldn't crash EXPLAIN; surface them as a note.
                Self::render_df_physical_plan(df_state, explainer);
            }
            return;
        }

        explainer.add_text("Index", state.custom_state().indexrel().name());
        explainer.add_query(state.custom_state().aggregate_clause.query());
        state
            .custom_state()
            .aggregate_clause
            .add_to_explainer(explainer);

        // Add note about recursive cost estimation if GUC is enabled
        if gucs::explain_recursive_estimates() && explainer.is_verbose() {
            explainer.add_text(
                "Recursive Query Estimates",
                "(not yet implemented for aggregate scans)",
            );
        }
    }

    fn begin_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        estate: *mut pg_sys::EState,
        eflags: i32,
    ) {
        if state.custom_state().is_datafusion_backend() {
            // The agg-on-join path runs entirely inside DataFusion and
            // never reaches the standard `init_expr_context` block below.
            // Allocate an ExprContext here so per-relation HeapFilter
            // queries (e.g. `=` on a `pdb.literal`-cast column) have a
            // live evaluation context - except under EXPLAIN_ONLY, where
            // no expressions run and the allocation is just dead weight
            // until the per-query context tears down.
            unsafe {
                let planstate = state.planstate();
                if eflags & (pg_sys::EXEC_FLAG_EXPLAIN_ONLY as i32) == 0 {
                    pg_sys::ExecAssignExprContext(estate, planstate);
                    state.runtime_context = state.csstate.ss.ps.ps_ExprContext;
                }

                let scan_slot = pg_sys::MakeTupleTableSlot(
                    (*planstate).ps_ResultTupleDesc,
                    &pg_sys::TTSOpsVirtual,
                );
                state.custom_state_mut().scan_slot = Some(scan_slot);
            }
            // MPP: serialize the logical plan so estimate_dsm/initialize_dsm
            // can write it into the DSM region. Only the leader runs
            // this branch (`ParallelWorkerNumber == -1` in the leader
            // backend); workers read the bytes back from DSM in
            // initialize_worker_custom_scan.
            if mpp_is_active() && unsafe { pg_sys::ParallelWorkerNumber } == -1 {
                Self::stash_mpp_plan_bytes(state);
            }
            return;
        }

        unsafe {
            let rte = pg_sys::exec_rt_fetch(state.custom_state().execution_rti, estate);
            assert!(!rte.is_null());
            let lockmode = (*rte).rellockmode as pg_sys::LOCKMODE;
            let planstate = state.planstate();
            // TODO: Opening of the index could be deduped between custom scans: see
            // `BaseScanState::open_relations`.
            state.custom_state_mut().open_relations(lockmode);

            state
                .custom_state_mut()
                .init_expr_context(estate, planstate);
            state.runtime_context = state.csstate.ss.ps.ps_ExprContext;

            // Create a reusable tuple slot for aggregate results
            // This avoids per-row MakeTupleTableSlot calls which leak memory
            let scan_slot =
                pg_sys::MakeTupleTableSlot((*planstate).ps_ResultTupleDesc, &pg_sys::TTSOpsVirtual);
            state.custom_state_mut().scan_slot = Some(scan_slot);

            // Set up placeholder targetlist for wrapped aggregate expression projection.
            let plan_targetlist = (*(*planstate).plan).targetlist;
            // This creates a copy of the plan's targetlist with FuncExpr placeholders replaced
            // by Const nodes. The Const nodes will be mutated with actual aggregate values
            // before each ExecBuildProjectionInfo call in exec_custom_scan (basescan pattern).
            let (placeholder_tlist, const_nodes, needs_projection) =
                create_placeholder_targetlist(plan_targetlist);
            if needs_projection && !placeholder_tlist.is_null() {
                state.custom_state_mut().wrapped_projection = Some(WrappedAggregateProjection {
                    targetlist: placeholder_tlist,
                    const_nodes,
                });
                // Note: projection is built per-row in exec_custom_scan, not here
            }
        }
    }

    fn rescan_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        state.custom_state_mut().state = ExecutionState::NotStarted;
        // Reset DataFusion state so rescan rebuilds the plan and stream.
        // Drop stream before runtime to avoid tokio panics.
        if let Some(ref mut df_state) = state.custom_state_mut().datafusion_state {
            df_state.stream = None;
            df_state.current_batch = None;
            df_state.batch_row_idx = 0;
            df_state.runtime = None;
        }
    }

    fn exec_custom_scan(state: &mut CustomScanStateWrapper<Self>) -> *mut pg_sys::TupleTableSlot {
        if state.custom_state().is_datafusion_backend() {
            Self::exec_datafusion_aggregate(state)
        } else {
            Self::exec_tantivy_aggregate(state)
        }
    }

    fn shutdown_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        // Drop the gather stream first (fires the leader-inbox detach on an early-terminated LIMIT
        // query so blocked producers stop), then wait for the workers to finish and flush their
        // `TaskMetrics`: `recv` blocks until every worker detaches its completion queue, which it
        // does only after sending metrics. The builder-launched workers need this explicit join so
        // the metrics land before the EXPLAIN render (which runs before end_custom_scan, where the
        // context is finally destroyed). Harmless when the gather already reached EOF.
        if let Some(df_state) = state.custom_state_mut().datafusion_state.as_mut() {
            df_state.stream = None;
            // Release the DSM-backed control senders before `recv`. A producer's `work_mem`
            // overflow (or any worker error) is re-raised in the leader from inside `recv`, which
            // longjmps out of this hook; a release placed after it would never run, leaving the
            // senders to drop at xact commit, past the DSM's lifetime, where their `fetch_sub`
            // faults. The query is done producing here, so the senders aren't needed.
            if let Some(leader) = df_state.mpp.as_ref() {
                leader.release_control_senders();
            }
            if let Some(leader) = df_state.mpp.as_mut() {
                if let Some(finish) = leader.finish.as_mut() {
                    let _ = finish.recv();
                }
            }
            // PG destroys the parallel DSM right after this hook, so drain the workers' metrics
            // frames off the mesh now (the EXPLAIN hook runs after teardown and only reads the
            // store).
            if let Some(leader) = df_state.mpp.as_ref() {
                if let Some(plan) = df_state.physical_plan.as_ref() {
                    crate::postgres::customscan::mpp::glue::drain_worker_metrics(
                        plan,
                        &leader.mesh,
                    );
                }
            }
        }
    }

    fn end_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        // Explicitly drop DataFusion resources (runtime, stream, batches) at the
        // intended lifecycle boundary rather than relying on Postgres to drop the
        // state wrapper later. Mirrors JoinScan::end_custom_scan.
        if let Some(mut df_state) = state.custom_state_mut().datafusion_state.take() {
            // Pull the builder handle out so we can join the workers and destroy the parallel
            // context once nothing references the ring mesh anymore. The leader value (and its
            // own mesh handle) drops at the end of this match arm.
            let finish = match df_state.mpp.take() {
                Some(mut leader) => leader.finish.take(),
                _ => None,
            };
            // Drop the stream first (tokio + mesh), then everything else holding a mesh reference
            // (physical plan, session) via `drop(df_state)`, before destroying the DSM below.
            df_state.stream = None;
            df_state.current_batch = None;
            df_state.runtime = None;
            drop(df_state);
            if let Some(finish) = finish {
                // The gather already drained, or early-terminated and the deadlock fix let the
                // producers stop, so the workers have finished and detached; this returns promptly.
                finish.wait_for_finish();
            }
        }

        // Clean up the reusable scan slot
        if let Some(slot) = state.custom_state().scan_slot {
            unsafe {
                pg_sys::ExecDropSingleTupleTableSlot(slot);
            }
        }
    }
}

pub enum CustomScanBuildError {
    NotInteresting,
    Incompatible(String),
}

impl From<String> for CustomScanBuildError {
    fn from(s: String) -> Self {
        CustomScanBuildError::Incompatible(s)
    }
}

impl From<&str> for CustomScanBuildError {
    fn from(s: &str) -> Self {
        CustomScanBuildError::Incompatible(s.to_string())
    }
}

/// Return the alias name of a Postgres `RangeTblEntry`, or `"unknown"` if the
/// entry has no alias attached. Used by planner-warning messages where we want
/// a stable user-visible label for the rejected relation.
unsafe fn rte_alias_or_unknown(rte: *mut pg_sys::RangeTblEntry) -> String {
    if !(*rte).eref.is_null() && !(*(*rte).eref).aliasname.is_null() {
        std::ffi::CStr::from_ptr((*(*rte).eref).aliasname)
            .to_string_lossy()
            .into_owned()
    } else {
        "unknown".to_string()
    }
}

pub trait CustomScanClause<CS: CustomScan> {
    type Args;

    fn from_pg(
        args: &CS::Args,
        heap_rti: pg_sys::Index,
        index: &PgSearchRelation,
    ) -> Result<Self, CustomScanBuildError>
    where
        Self: Sized;

    fn add_to_custom_path(&self, builder: CustomPathBuilder<CS>) -> CustomPathBuilder<CS>;

    fn explain_output(&self) -> Box<dyn Iterator<Item = (String, String)>> {
        Box::new(std::iter::empty())
    }

    fn add_to_explainer(&self, explainer: &mut Explainer) {
        for (key, value) in self.explain_output() {
            explainer.add_text(&format!("  {}", key), &value);
        }
    }

    fn build(
        builder: CustomPathBuilder<CS>,
        heap_rti: pg_sys::Index,
        index: &PgSearchRelation,
    ) -> Result<(CustomPathBuilder<CS>, Self), CustomScanBuildError>
    where
        Self: Sized,
    {
        let clause = Self::from_pg(builder.args(), heap_rti, index)?;
        let builder = clause.add_to_custom_path(builder);
        Ok((builder, clause))
    }
}

impl AggregateScan {
    /// Capture per-source `SearchIndexManifest`s for every PgSearchScan
    /// reachable from the aggregate's `RelNode` plan tree. Mirrors
    /// `JoinScan::ensure_source_manifests`. Required at DSM-init time so
    /// `ParallelScanState::size_of` can size the shared region; the buffer
    /// pins must also outlive the scan itself, hence storing on
    /// `AggregateScanState` rather than as a local.
    fn ensure_source_manifests(state: &mut CustomScanStateWrapper<Self>) {
        if !state.custom_state().source_manifests.is_empty() {
            return;
        }
        let Some(df_state) = state.custom_state().datafusion_state.as_ref() else {
            return;
        };
        let manifests: Vec<SearchIndexManifest> = df_state
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

    /// Pick the partitioning source — the one whose segment list workers
    /// claim from. Largest by total live doc count, falling back to
    /// segment count, then position. (Earlier this was just
    /// `max_by_key(segment_count)`, which under ties returns the *last*
    /// element — for a 2-table JOIN with both indexes at 10 segments,
    /// that put the smaller table on the partitioning side and forced
    /// the all-gather to cache the larger one. Doc count breaks the tie
    /// in the right direction.)
    fn partitioning_source_idx(state: &CustomScanStateWrapper<Self>) -> usize {
        state
            .custom_state()
            .source_manifests
            .iter()
            .enumerate()
            .max_by_key(|(_, m)| (m.total_doc_count(), m.segment_count()))
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    /// Serialize the leader's logical plan (already on `df_state`) and stash
    /// the bytes on `df_state.mpp_plan_bytes`. `estimate_dsm_custom_scan`
    /// reads the length to size the DSM region; `initialize_dsm_custom_scan`
    /// hands the bytes to `glue::leader_setup` which copies them into DSM
    /// for workers.
    fn stash_mpp_plan_bytes(state: &mut CustomScanStateWrapper<Self>) {
        // Capture source manifests + partitioning_source_idx BEFORE building
        // the logical plan. Each `PgSearchTableProvider` needs to know whether
        // it's the partitioning source (uses `parallel_state.checkout_segment`
        // to slice work across PG parallel workers) or non-partitioning
        // (replicated view via canonical segment IDs). These flags are baked
        // into the serialized plan; workers re-derive them via the codec.
        Self::ensure_source_manifests(state);
        let partitioning_idx = Self::partitioning_source_idx(state);
        let mpp_ctx = MppPlanContext {
            partitioning_plan_position: partitioning_idx,
        };
        state.custom_state_mut().mpp_partitioning_source_idx = Some(partitioning_idx);

        let Some(df_state) = state.custom_state_mut().datafusion_state.as_mut() else {
            return;
        };
        // Build a logical plan eagerly. The DataFusion exec path normally
        // builds it lazily on first `exec_custom_scan` call; for MPP we
        // need it before estimate_dsm fires.
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                pgrx::warning!("mpp: tokio runtime build failed: {e}; skipping MPP");
                return;
            }
        };
        let ctx = create_aggregate_session_context();
        let custom_exprs = df_state.custom_exprs;
        let custom_scan_tlist = df_state.custom_scan_tlist;
        let logical = runtime.block_on(async {
            build_join_aggregate_plan(
                &df_state.plan,
                &df_state.targetlist,
                df_state.topk.as_ref(),
                &df_state.join_level_predicates,
                custom_exprs,
                custom_scan_tlist,
                df_state.having_filter.as_ref(),
                &ctx,
                None,
                None,
                Some(mpp_ctx),
            )
            .await
        });
        let logical = match logical {
            Ok((lp, _group_df_indices)) => lp,
            Err(e) => {
                pgrx::warning!("mpp: build_join_aggregate_plan failed: {e}; skipping MPP");
                return;
            }
        };
        let bytes = match serialize_logical_plan(&logical) {
            Ok(b) => b,
            Err(e) => {
                pgrx::warning!("mpp: serialize_logical_plan failed: {e}; skipping MPP");
                return;
            }
        };
        df_state.mpp_plan_bytes = Some(bytes.to_vec());
    }

    /// First-exec MPP launch. The leader spawns its producer workers through the builder and, on
    /// success, installs the leader state so the consumer plan reads from the mesh. A short launch
    /// (or any setup fallback) leaves `mpp` unset and the query runs serially.
    fn maybe_launch_mpp(state: &mut CustomScanStateWrapper<Self>) {
        if !mpp_is_active() {
            return;
        }
        let Some(plan_bytes) = state
            .custom_state_mut()
            .datafusion_state
            .as_mut()
            .and_then(|d| d.mpp_plan_bytes.take())
        else {
            return;
        };

        // Manifests + partitioning index were captured in `stash_mpp_plan_bytes` (begin); `ensure`
        // is idempotent.
        Self::ensure_source_manifests(state);
        let partitioning_idx = state
            .custom_state()
            .mpp_partitioning_source_idx
            .unwrap_or_else(|| Self::partitioning_source_idx(state));

        let all_sources: Vec<&[tantivy::SegmentReader]> = state
            .custom_state()
            .source_manifests
            .iter()
            .map(|m| m.segment_readers())
            .collect();
        let args = ParallelScanArgs {
            all_sources,
            partitioning_source_idx: partitioning_idx,
            query: vec![],
            with_aggregates: false,
        };

        let Some(leader) = crate::postgres::customscan::mpp::launch::launch_mpp_aggregate(
            plan_bytes,
            args,
            partitioning_idx,
        ) else {
            return;
        };
        if let Some(df_state) = state.custom_state_mut().datafusion_state.as_mut() {
            df_state.mpp = Some(leader);
        }
    }

    /// Build the leader's distributed session context for this AggregateScan query. Thin
    /// wrapper over the shape-agnostic [`crate::postgres::customscan::mpp::exec_worker::
    /// build_mpp_session_context`] that seeds with `create_aggregate_session_context()`.
    /// `mesh = None` is the EXPLAIN-time path. See the shared helper's doc.
    fn build_mpp_session_context(
        mesh: Option<Arc<MppMesh>>,
    ) -> datafusion::prelude::SessionContext {
        crate::postgres::customscan::mpp::exec_worker::build_mpp_session_context(
            create_aggregate_session_context(),
            mesh,
        )
    }

    /// Rebuild and render the DataFusion physical plan into the explainer.
    /// Used only by plain EXPLAIN (no ANALYZE); EXPLAIN ANALYZE would cache
    /// the executing plan separately.
    ///
    /// When `mpp_is_active()` we rebuild with the same distributed planner
    /// the leader will run with, attached to a drain-less stub mesh — the
    /// planner only consults the mesh's worker count, not the actual
    /// `shm_mq` queues, so the stub is enough to produce a `DistributedExec`
    /// root. That lets us render via `datafusion_distributed::display_plan_ascii`
    /// and surface the boxed `Stage N — Tasks: t0:[p0..pN]` topology the
    /// executor will actually run. When MPP is off we fall back to the
    /// serial context and the standard `displayable().indent(false)` tree.
    ///
    /// Failures here go to a single explainer line rather than crashing
    /// EXPLAIN; the failure mode is non-load-bearing diagnostics.
    fn render_df_physical_plan(
        df_state: &scan_state::DataFusionAggState,
        explainer: &mut Explainer,
    ) {
        let custom_exprs = df_state.custom_exprs;
        let custom_scan_tlist = df_state.custom_scan_tlist;
        let ctx = if mpp_is_active() {
            // EXPLAIN-time: skip the shm_mq transport install (no execution, no `open()` call).
            // The shared session-context builder takes `mesh = None` and derives `n_workers`
            // from `producer_worker_count()`, so the planner still emits a `DistributedExec`
            // root with the right stage sizing for display.
            Self::build_mpp_session_context(None)
        } else {
            create_aggregate_session_context()
        };
        let Ok(runtime) = tokio::runtime::Builder::new_current_thread().build() else {
            explainer.add_text("DataFusion Plan", "(tokio runtime unavailable)");
            return;
        };
        let plan_result = runtime.block_on(async {
            let (logical, _) = build_join_aggregate_plan(
                &df_state.plan,
                &df_state.targetlist,
                df_state.topk.as_ref(),
                &df_state.join_level_predicates,
                custom_exprs,
                custom_scan_tlist,
                df_state.having_filter.as_ref(),
                &ctx,
                None,
                None,
                None,
            )
            .await?;
            build_physical_plan(&ctx, logical).await
        });
        match plan_result {
            Ok(plan) => {
                explainer.add_text("DataFusion Physical Plan", "");
                for line in Self::render_plan_for_explain(plan.as_ref()).lines() {
                    explainer.add_text("  ", line);
                }
            }
            Err(e) => {
                explainer.add_text(
                    "DataFusion Plan",
                    format!("(rebuild failed during EXPLAIN: {e})"),
                );
            }
        }
    }

    /// EXPLAIN ANALYZE: merge the worker metrics that arrived over the mesh into the executed
    /// plan and render it. The leader's own nodes already carry their metrics; the worker
    /// fragments reported theirs as `TaskMetrics` frames when they finished.
    fn render_executed_plan_with_metrics(
        df_state: &scan_state::DataFusionAggState,
        explainer: &mut Explainer,
    ) {
        let Some(plan) = df_state.physical_plan.clone() else {
            return;
        };
        let rendered = match (df_state.mpp.as_ref(), df_state.runtime.as_ref()) {
            (Some(_), Some(_runtime)) => {
                match crate::postgres::customscan::mpp::glue::merge_worker_metrics(&plan) {
                    Some(merged) => display_plan_ascii(merged.as_ref(), true),
                    None => {
                        explainer.add_text(
                            "DataFusion Physical Plan",
                            "(worker metrics incomplete; a worker may not have reported)",
                        );
                        return;
                    }
                }
            }
            // Serial fallback: no workers, the plain metrics display tells the whole story.
            _ => Self::render_plan_for_explain(plan.as_ref()),
        };
        explainer.add_text("DataFusion Physical Plan", "");
        for line in rendered.lines() {
            explainer.add_text("  ", line);
        }
    }

    /// Render a physical plan for EXPLAIN. `DistributedExec` roots go through
    /// `display_plan_ascii` for the boxed-stage rendering; serial plans keep
    /// the standard `displayable().indent(false)` tree so non-MPP expected
    /// outputs are stable.
    fn render_plan_for_explain(plan: &dyn ExecutionPlan) -> String {
        if plan.is::<DistributedExec>() {
            display_plan_ascii(plan, false)
        } else {
            datafusion::physical_plan::displayable(plan)
                .indent(false)
                .to_string()
        }
    }

    /// Existing single-table Tantivy aggregate path.
    fn build_tantivy_aggregate_path(
        builder: CustomPathBuilder<Self>,
        has_paradedb_agg: bool,
    ) -> Vec<pg_sys::CustomPath> {
        let parent_relids = builder.args().input_rel().relids;
        let Some(heap_rti) = (unsafe { range_table::bms_exactly_one_member(parent_relids) }) else {
            return Vec::new();
        };
        let heap_rte = unsafe {
            range_table::get_rte(
                builder.args().root().simple_rel_array_size as usize,
                builder.args().root().simple_rte_array,
                heap_rti,
            )
        };
        let Some(heap_rte) = heap_rte else {
            return Vec::new();
        };

        // If it's not a plain relation (e.g. it's a partitioned table), we can't do Tantivy agg directly.
        // Parent partitioned tables are not yet supported for aggregate pushdown.
        let Some(heap_relid) = (unsafe { range_table::get_plain_relation_relid(heap_rte) }) else {
            if has_paradedb_agg {
                Self::add_planner_warning(
                    "Aggregate Scan not used: unsupported relation type (e.g., partitioned table or view)",
                    unsafe { rte_alias_or_unknown(heap_rte) },
                );
            }
            return Vec::new();
        };

        let Some((_table, index)) = rel_get_bm25_index(heap_relid) else {
            if has_paradedb_agg {
                Self::add_planner_warning(
                    "Aggregate Scan not used: table must have a BM25 index",
                    unsafe { rte_alias_or_unknown(heap_rte) },
                );
            }
            return Vec::new();
        };

        match AggregateCSClause::build(builder, heap_rti, &index) {
            Ok((builder, aggregate_clause)) => {
                Self::mark_contexts_successful(unsafe { rte_alias_or_unknown(heap_rte) });

                vec![builder.build(PrivateData::Tantivy {
                    heap_rti,
                    indexrelid: index.oid(),
                    aggregate_clause: Box::new(aggregate_clause),
                })]
            }
            Err(CustomScanBuildError::Incompatible(e)) => {
                if has_paradedb_agg
                    || (gucs::enable_aggregate_custom_scan() && gucs::check_aggregate_scan())
                {
                    let warning_msg = if has_paradedb_agg {
                        format!("Aggregate Scan not used: {}", e)
                    } else {
                        format!(
                            "Aggregate Scan not used: {}. \
                             To disable this warning: SET paradedb.check_aggregate_scan = false",
                            e,
                        )
                    };
                    Self::add_planner_warning(warning_msg, _table.name().to_string());
                }
                Vec::new()
            }
            Err(CustomScanBuildError::NotInteresting) => Vec::new(),
        }
    }

    /// New DataFusion-backed aggregate path for JOINs.
    fn build_datafusion_aggregate_path(
        builder: CustomPathBuilder<Self>,
    ) -> Vec<pg_sys::CustomPath> {
        match Self::try_build_datafusion_aggregate_path(builder) {
            Ok(path) => vec![path],
            Err(AggregatePathDecline::Quiet) => Vec::new(),
            Err(AggregatePathDecline::Warn(reason)) => {
                reason.emit();
                Vec::new()
            }
        }
    }

    /// Body of [`Self::build_datafusion_aggregate_path`] in `?`-style.
    /// Mirrors the JoinScan `try_build_join_custom_path` shape: `Quiet` for
    /// silent gates that don't qualify as a join we'd accelerate, and
    /// `Warn(reason)` for validation failures past the "candidate" boundary
    /// that owe the planner a NOTICE.
    fn try_build_datafusion_aggregate_path(
        builder: CustomPathBuilder<Self>,
    ) -> Result<pg_sys::CustomPath, AggregatePathDecline> {
        let root = builder.args().root;
        let input_rel = builder.args().input_rel();

        // Silent gates: no sources, or no BM25 index at all → not a candidate.
        let sources = unsafe { collect_join_agg_sources(root, input_rel) };
        if sources.is_empty() {
            return Err(AggregatePathDecline::Quiet);
        }
        if !has_any_bm25_index(&sources) {
            return Err(AggregatePathDecline::Quiet);
        }

        // Below this line every Err carries a planner warning.
        let warn = |reason| AggregatePathDecline::Warn(reason);

        // Check if any RTI was dropped by collect_join_agg_sources (e.g., subqueries)
        let expected_rtis = unsafe { pgrx::pg_sys::bms_num_members(input_rel.relids) } as usize;
        if sources.len() != expected_rtis {
            let rtis: Vec<pgrx::pg_sys::Index> = unsafe {
                crate::postgres::customscan::range_table::bms_iter(input_rel.relids).collect()
            };
            for rti in rtis {
                if !sources.iter().any(|s| s.rti == rti) {
                    let rte = unsafe {
                        crate::postgres::customscan::range_table::get_rte(
                            (*root).simple_rel_array_size as usize,
                            (*root).simple_rte_array,
                            rti,
                        )
                    };
                    if let Some(rte_ptr) = rte {
                        let rtekind = unsafe { (*rte_ptr).rtekind };
                        // RTE_JOIN represents the join itself, not a base table we'd scan
                        if rtekind == pgrx::pg_sys::RTEKind::RTE_JOIN {
                            continue;
                        }

                        // Silent decline for subqueries with limits, as they are
                        // handled efficiently by the BaseScan TopK pushdown natively.
                        // We check recursively because the limit might be nested inside CTEs
                        // or UNION arms within the subquery.
                        if rtekind == pgrx::pg_sys::RTEKind::RTE_SUBQUERY {
                            let subquery = unsafe { (*rte_ptr).subquery };
                            if !subquery.is_null() && unsafe { query_will_use_topk(subquery) } {
                                return Err(AggregatePathDecline::Quiet);
                            }
                        }

                        return Err(warn(AggregateDeclineReason::Other(format!(
                            "RTI {} is not a plain relation (rtekind: {:?}), which is not supported",
                            rti, rtekind
                        ))));
                    } else {
                        return Err(warn(AggregateDeclineReason::Other(format!(
                            "RTI {} not found in simple_rte_array",
                            rti
                        ))));
                    }
                }
            }
        }

        // All tables must have BM25 indexes (DataFusion scans all via PgSearchTableProvider).
        if !all_have_bm25_index(&sources) {
            return Err(warn(AggregateDeclineReason::NotAllBm25));
        }

        // Reject joins with non-equi quals (OR across tables, cross-table
        // filters, non-@@@ conditions). Check both the cheapest path's
        // joinrestrictinfo AND the parse tree's WHERE quals for cross-table
        // references that our DataFusion backend can't apply.
        if unsafe { datafusion_build::has_non_equi_join_quals(input_rel, &sources) } {
            return Err(warn(AggregateDeclineReason::NonEquiJoinQuals));
        }

        // Extract the join tree from the parse tree
        let (mut plan, join_level_predicates, multi_table_predicates, multi_table_clauses) =
            unsafe { extract_join_tree_from_parse(root, &sources, builder.args().input_rel()) }
                .map_err(|e| warn(AggregateDeclineReason::Other(e)))?;

        // Extract aggregate target list (GROUP BY + aggregates)
        let targetlist = unsafe { extract_aggregate_targetlist(builder.args(), &sources, &plan) }
            .map_err(|e| warn(AggregateDeclineReason::Other(e)))?;

        // Reject plans with any join node that has no equi-keys (CROSS JOIN).
        // Without join keys, PgSearchTableProvider has no Named fields,
        // producing empty RecordBatches or DataFusion "join condition should
        // not be empty" errors. Single-table scans (sources.len() == 1) have
        // no join keys by definition and are allowed — they reach this path
        // when routed from RELOPT_BASEREL (e.g., max_buckets overflow or
        // ORDER BY aggregate + LIMIT).
        if sources.len() > 1 && plan.has_join_without_keys() {
            return Err(warn(AggregateDeclineReason::CrossJoin));
        }

        // Populate the fast fields on each source so PgSearchTableProvider exposes them.
        // This fails if join key fields aren't indexed as fast fields.
        unsafe {
            datafusion_build::populate_required_fields(&mut plan, &targetlist, &multi_table_clauses)
        }
        .map_err(|e| warn(AggregateDeclineReason::Other(e)))?;

        // Detect ORDER BY on aggregate + LIMIT for TopK pushdown into DataFusion.
        // DataFusion's SortExec(fetch=K) uses a bounded TopK heap internally.
        // We do NOT declare pathkeys to Postgres because scanrelid=0 CustomScans
        // cannot resolve pathkey items through setrefs.c. Postgres may add a
        // redundant Sort above us, which is correct (just wasteful on K rows).
        let topk = unsafe { detect_join_aggregate_topk(builder.args(), &targetlist) };

        // Extract HAVING clause if present.
        //
        // HAVING produces `AggRef`/`GroupRef`, never `ColumnRef`, so the
        // FILTER `T_Var` arm is unreachable from here. But the HAVING
        // matchers need the plan tree anyway to recover a source's rti
        // from `plan_position` when matching parse-tree Vars to extracted
        // targetlist refs: the plan is the system of record for rti, the
        // targetlist refs themselves don't carry it.
        let outer_root_id =
            crate::postgres::customscan::joinscan::build::PlannerRootId::from(builder.args().root);
        let having_filter = unsafe {
            let parse = builder.args().root().parse;
            if !parse.is_null() && !(*parse).havingQual.is_null() {
                privdat::FilterExpr::from_pg_node(
                    (*parse).havingQual,
                    &datafusion_build::FilterExprBuildContext::Having {
                        targetlist: &targetlist,
                        plan: &plan,
                        outer_root_id,
                    },
                )
            } else {
                None
            }
        };

        // MPP launches its own producer workers from `exec_custom_scan` through the builder, so the
        // path stays serial to PG. Marking it parallel-aware would make PG plan a Gather over it and
        // spawn a redundant second worker set whose serial aggregates duplicate the result.

        // Build the custom path with DataFusion private data
        let multi_table_clause_count = multi_table_clauses.len();
        let mut custom_path = builder.build(PrivateData::DataFusion {
            plan,
            targetlist,
            topk,
            join_level_predicates,
            multi_table_predicates,
            multi_table_clause_count,
            having_filter,
        });

        // Append raw PG Expr pointers to custom_private after the serialized
        // PrivateData. Structure: [PrivateData JSON, expr_1, expr_2, ...]
        // These will be moved to custom_exprs in plan_custom_path so that
        // setrefs transforms their Var nodes to INDEX_VAR references.
        if !multi_table_clauses.is_empty() {
            unsafe {
                let mut private_list = PgList::<pg_sys::Node>::from_pg(custom_path.custom_private);
                for clause in multi_table_clauses {
                    private_list.push(clause.cast());
                }
                custom_path.custom_private = private_list.into_pg();
            }
        }

        Ok(custom_path)
    }

    /// Execute the Tantivy aggregate path: drive the existing
    /// `aggregation_results_iter` one row at a time, fill the scan slot, and
    /// optionally project wrapped aggregates on top.
    fn exec_tantivy_aggregate(
        state: &mut CustomScanStateWrapper<Self>,
    ) -> *mut pg_sys::TupleTableSlot {
        let Some(row) = Self::advance_tantivy_state(state) else {
            return std::ptr::null_mut();
        };

        unsafe {
            let tupdesc = PgTupleDesc::from_pg_unchecked((*state.planstate()).ps_ResultTupleDesc);
            // Use the reusable slot created in begin_custom_scan to avoid per-row memory leaks
            let slot = state
                .custom_state()
                .scan_slot
                .expect("scan_slot should be initialized in begin_custom_scan");
            pg_sys::ExecClearTuple(slot);

            fill_slot_from_row(
                slot,
                &tupdesc,
                &row,
                &state.custom_state().aggregate_clause,
                state
                    .custom_state()
                    .precomputed_index_info()
                    .expect("should be initialized by now"),
            );

            Self::project_wrapped_aggregates(state, slot, &row)
        }
    }

    /// Drive the Tantivy execution state machine: lazily kick off the
    /// aggregate iterator on the first call, return the next row on
    /// subsequent calls, and transition to `Completed` when the iterator
    /// is exhausted.
    fn advance_tantivy_state(
        state: &mut CustomScanStateWrapper<Self>,
    ) -> Option<AggregationResultsRow> {
        let row = match &mut state.custom_state_mut().state {
            ExecutionState::Completed => return None,
            ExecutionState::NotStarted => {
                // Execute the aggregate, and change the state to Emitting.
                let mut row_iter = aggregation_results_iter(state);
                let first = row_iter.next();
                state.custom_state_mut().state = ExecutionState::Emitting(row_iter);
                first
            }
            ExecutionState::Emitting(row_iter) => row_iter.next(),
        };
        if row.is_none() {
            state.custom_state_mut().state = ExecutionState::Completed;
        }
        row
    }

    /// If `wrapped_projection` is set on the scan state, mutate the
    /// pre-baked Const nodes with the row's native aggregate values, switch
    /// into the per-tuple memory context, and `ExecProject` to materialize
    /// the wrapped expressions. Returns the projected slot. When no wrapped
    /// projection is configured, returns the input slot unchanged.
    unsafe fn project_wrapped_aggregates(
        state: &mut CustomScanStateWrapper<Self>,
        slot: *mut pg_sys::TupleTableSlot,
        row: &AggregationResultsRow,
    ) -> *mut pg_sys::TupleTableSlot {
        // Snapshot the projection state into locals so the immutable borrow
        // on `state.custom_state()` ends before the mutable `state.planstate()`
        // call below. The targetlist is a raw pointer (Copy) and the const-node
        // vec is small (one entry per output column).
        let projection_snapshot: Option<(*mut pg_sys::List, Vec<Option<*mut pg_sys::Const>>)> =
            state
                .custom_state()
                .wrapped_projection
                .as_ref()
                .map(|w| (w.targetlist, w.const_nodes.clone()));
        let Some((placeholder_tlist, const_nodes)) = projection_snapshot else {
            return slot;
        };

        let planstate = state.planstate();
        let expr_context = (*planstate).ps_ExprContext;

        // Switch to per-tuple memory context and reset it to avoid memory leaks
        // from ExecBuildProjectionInfo allocations and wrapper functions
        let mut per_tuple_context = PgMemoryContexts::For((*expr_context).ecxt_per_tuple_memory);
        per_tuple_context.reset();

        // Read the slot's already-filled datums for the GroupingColumn fallback
        // arm in the loop below.
        let natts = (*(*slot).tts_tupleDescriptor).natts as usize;
        let datums = std::slice::from_raw_parts((*slot).tts_values, natts);
        let isnull = std::slice::from_raw_parts((*slot).tts_isnull, natts);

        // Mutate Const nodes with values directly from the row results.
        // We DON'T use the slot's datums for aggregates because those were converted
        // using the output tuple descriptor's types (e.g., TEXT for jsonb_pretty output),
        // but we need the native aggregate type (e.g., JSONB for pdb.agg).
        // This matches basescan's approach of setting Const values directly.
        let mut agg_iter = row.aggregates.iter();
        let aggregate_clause = &state.custom_state().aggregate_clause;
        for (i, entry) in aggregate_clause.entries().enumerate() {
            let Some(const_node) = const_nodes.get(i).copied().flatten() else {
                // No Const node for this entry, skip the aggregate iterator if
                // it's an aggregate that occupies a slot in `row.aggregates`
                // (doc-count aggregates do not — see `uses_doc_count_path`).
                if let TargetListEntry::Aggregate(agg_type) = entry {
                    if !uses_doc_count_path(agg_type, aggregate_clause) {
                        agg_iter.next();
                    }
                }
                continue;
            };

            let (datum, is_null) = match entry {
                TargetListEntry::Aggregate(agg_type) => {
                    // Use the native aggregate result type (from the Const node),
                    // not the output tuple descriptor's type — those would have
                    // been converted to e.g. TEXT for jsonb_pretty output, but
                    // the wrapped projection wants the raw JSONB / numeric.
                    //
                    // Only advance the aggregate iterator for entries that
                    // actually occupy a slot in `row.aggregates` (see
                    // `uses_doc_count_path`).
                    let next_aggregate =
                        if row.is_empty() || uses_doc_count_path(agg_type, aggregate_clause) {
                            None
                        } else {
                            agg_iter.next().and_then(|v| v.clone())
                        };
                    match aggregate_value_to_datum(
                        agg_type,
                        row,
                        (*const_node).consttype,
                        aggregate_clause,
                        next_aggregate,
                        state
                            .custom_state()
                            .precomputed_index_info()
                            .expect("should be initialized by now"),
                    ) {
                        Some(datum) => (datum, false),
                        None => (pg_sys::Datum::null(), true),
                    }
                }
                TargetListEntry::GroupingColumn(_) => {
                    debug_assert!(
                        i < natts,
                        "aggregate clause entry index out of bounds for tuple descriptor"
                    );
                    (datums[i], isnull[i])
                }
            };

            (*const_node).constvalue = datum;
            (*const_node).constisnull = is_null;
        }

        // Set the scan tuple for expression evaluation context
        (*expr_context).ecxt_scantuple = slot;

        // Build projection and execute in per-tuple memory context (basescan pattern)
        // This ensures ExecBuildProjectionInfo allocations are cleaned up each row
        per_tuple_context.switch_to(|_| {
            let proj_info = pg_sys::ExecBuildProjectionInfo(
                placeholder_tlist,
                expr_context,
                (*planstate).ps_ResultTupleSlot,
                planstate,
                (*slot).tts_tupleDescriptor,
            );
            pg_sys::ExecProject(proj_info)
        })
    }

    /// Execute the DataFusion aggregate path: build plan, consume Arrow batches,
    /// project each row to a Postgres TupleTableSlot.
    fn exec_datafusion_aggregate(
        state: &mut CustomScanStateWrapper<Self>,
    ) -> *mut pg_sys::TupleTableSlot {
        // Grab the scan_slot pointer before entering the mutable borrow
        let scan_slot = state
            .custom_state()
            .scan_slot
            .expect("scan_slot must be initialized in begin_custom_scan");

        // Capture before the mutable borrow on `datafusion_state`. Threaded
        // down to each `PgSearchTableProvider` so HeapFilter queries (`=`
        // on a `pdb.literal`-cast column, etc.) can resolve their runtime
        // expressions - the same plumbing single-table aggregates get from
        // `state.runtime_context` directly.
        let runtime_expr_context =
            (!state.runtime_context.is_null()).then_some(state.runtime_context);
        let ps = state.planstate();
        let runtime_planstate = (!ps.is_null()).then_some(ps);

        // First exec call: the leader launches its producer workers and picks the count, so a
        // short launch falls back to serial instead of hanging the query. Done before the
        // df_state borrow below because it needs `state` for the source manifests.
        let first_call = state
            .custom_state()
            .datafusion_state
            .as_ref()
            .is_some_and(|d| d.runtime.is_none());
        if first_call {
            Self::maybe_launch_mpp(state);
        }

        let df_state = state
            .custom_state_mut()
            .datafusion_state
            .as_mut()
            .expect("DataFusion state must be initialized");

        // First call: build and execute the DataFusion plan
        if df_state.runtime.is_none() {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap_or_else(|e| pgrx::error!("Failed to create tokio runtime: {}", e));

            // MPP leader: install the mesh + DF-D fork's distributed planner so
            // `create_physical_plan` produces a `DistributedExec` whose
            // `NetworkShuffleExec`s use our `ShmMqWorkerTransport` to read
            // from worker queues at execute time. Otherwise: existing serial
            // session context.
            let ctx = match df_state.mpp.as_ref() {
                Some(leader) => {
                    // Workers are launched and draining their inboxes (the launcher already
                    // verified the full producer set came up, else it fell back to serial); ship
                    // each fragment's plan frame now.
                    if let Err(e) =
                        crate::postgres::customscan::mpp::glue::deliver_set_plans(leader)
                    {
                        pgrx::error!("mpp aggregate: plan delivery failed: {e}");
                    }
                    Self::build_mpp_session_context(Some(Arc::clone(&leader.mesh)))
                }
                _ => create_aggregate_session_context(),
            };

            let custom_exprs = df_state.custom_exprs;
            let custom_scan_tlist = df_state.custom_scan_tlist;
            let physical_plan = runtime.block_on(async {
                let (logical, group_df_indices) = build_join_aggregate_plan(
                    &df_state.plan,
                    &df_state.targetlist,
                    df_state.topk.as_ref(),
                    &df_state.join_level_predicates,
                    custom_exprs,
                    custom_scan_tlist,
                    df_state.having_filter.as_ref(),
                    &ctx,
                    runtime_expr_context,
                    runtime_planstate,
                    None,
                )
                .await?;
                df_state.group_df_indices = group_df_indices;
                build_physical_plan(&ctx, logical).await
            });

            let physical_plan = match physical_plan {
                Ok(p) => p,
                Err(e) => pgrx::error!("Failed to build DataFusion aggregate plan: {}", e),
            };

            let task_ctx = build_task_context(
                &ctx,
                &physical_plan,
                unsafe { pg_sys::work_mem as usize * 1024 },
                unsafe { pg_sys::hash_mem_multiplier },
            );
            // Install `DistributedTaskContext` explicitly so the top boundary sees the leader's
            // `(task_index=0, task_count=1)` identity. Skipping this would let the fork's
            // `from_ctx` default kick in, which silently hides any planner shape that emits a
            // different `consumer_tc` at the top boundary. Matches the worker dispatcher's setup.
            let task_ctx = {
                let cfg = task_ctx.session_config().clone().with_extension(Arc::new(
                    DistributedTaskContext {
                        task_index: 0,
                        task_count: 1,
                    },
                ));
                Arc::new(
                    TaskContext::default()
                        .with_session_config(cfg)
                        .with_runtime(task_ctx.runtime_env().clone()),
                )
            };
            let stream = {
                let _guard = runtime.enter();
                match physical_plan.execute(0, task_ctx) {
                    Ok(s) => s,
                    Err(e) => pgrx::error!("Failed to execute DataFusion aggregate plan: {}", e),
                }
            };

            df_state.runtime = Some(runtime);
            df_state.physical_plan = Some(physical_plan);
            df_state.stream = Some(stream);
        }

        // Consume batches row-by-row
        loop {
            // Try current batch
            if let Some(ref batch) = df_state.current_batch {
                if df_state.batch_row_idx < batch.num_rows() {
                    unsafe {
                        pg_sys::ExecClearTuple(scan_slot);
                    }
                    let row_idx = df_state.batch_row_idx;
                    let targetlist = &df_state.targetlist;
                    let group_df_indices = &df_state.group_df_indices;
                    let result = unsafe {
                        project_aggregate_row_to_slot(
                            scan_slot,
                            batch,
                            row_idx,
                            targetlist,
                            group_df_indices,
                        )
                    };
                    df_state.batch_row_idx += 1;
                    return result;
                }
                // Current batch exhausted
                df_state.current_batch = None;
            }

            // Fetch next batch from stream
            let next = block_on_next(
                df_state.runtime.as_ref().unwrap(),
                df_state.stream.as_mut().unwrap(),
            );

            match next {
                Some(Ok(batch)) => {
                    df_state.current_batch = Some(batch);
                    df_state.batch_row_idx = 0;
                }
                Some(Err(e)) => {
                    pgrx::error!("DataFusion aggregate execution failed: {}", e);
                }
                None => {
                    // Stream exhausted — no more results
                    return std::ptr::null_mut();
                }
            }
        }
    }
}

/// True when an aggregate entry is served by the bucket's `doc_count` rather
/// than by an entry in `row.aggregates`. Matches the filter in
/// `CollectFlat<AggregateType, MetricsWithGroupBy>` in `build.rs`, which omits
/// these aggregates from the Tantivy request — so they must NOT be advanced
/// past when walking the result iterator.
fn uses_doc_count_path(agg_type: &AggregateType, aggregate_clause: &AggregateCSClause) -> bool {
    agg_type.can_use_doc_count() && !aggregate_clause.has_filter() && aggregate_clause.has_groupby()
}

/// Convert one aggregate target-list entry to a Postgres datum.
///
/// Handles three branches in order:
/// 1. **Empty result row** → use the agg type's `nullish` fallback (always F64).
/// 2. **`can_use_doc_count` fast path** → forward `row.doc_count()` directly,
///    bypassing the aggregate iterator. Requires no FILTER and a GROUP BY.
/// 3. **Otherwise** → call into [`exec::aggregate_result_to_datum`] with the
///    `next_aggregate` value the caller supplies from its iterator.
///
/// The caller is responsible for advancing its aggregate iterator exactly when
/// branch 3 fires — see [`uses_doc_count_path`]. Doc-count aggregates have no
/// slot in `row.aggregates`, so advancing past them would shift subsequent
/// aggregate results by one.
///
/// Used by both `fill_slot_from_row` (which targets the tupdesc type) and
/// `project_wrapped_aggregates` (which targets the Const node's native type).
/// Returns `None` when the result should be NULL.
unsafe fn aggregate_value_to_datum(
    agg_type: &AggregateType,
    row: &AggregationResultsRow,
    target_typoid: pg_sys::Oid,
    aggregate_clause: &AggregateCSClause,
    next_aggregate: Option<AggregateResult>,
    index_info: &AggIndexInfo,
) -> Option<pg_sys::Datum> {
    if row.is_empty() {
        return agg_type.nullish().value.and_then(|value| {
            TantivyValue(PdbOwnedValue::F64(value))
                .try_into_datum(target_typoid.into())
                .unwrap()
        });
    }
    if uses_doc_count_path(agg_type, aggregate_clause) {
        return row
            .doc_count()
            .try_into_datum(pgrx::PgOid::from(target_typoid))
            .ok()
            .flatten();
    }
    exec::aggregate_result_to_datum(next_aggregate, agg_type, target_typoid, index_info)
}

/// Fill the scan slot's `tts_values` / `tts_isnull` arrays from a single
/// `AggregationResultsRow`. Walks the aggregate clause's target list once and
/// dispatches to one of four shapes:
///
/// 1. `GroupingColumn` with a non-empty row → decode the group key (handles
///    NULL sentinels and ISO-8601 datetime parsing) via [`group_key_to_datum`].
/// 2. `GroupingColumn` with an empty row → NULL.
/// 3. `Aggregate` (any row) → delegate to [`aggregate_value_to_datum`].
///
/// Finalizes by setting `tts_flags` and `tts_nvalid` so the slot is in the
/// "virtual tuple stored" state.
unsafe fn fill_slot_from_row(
    slot: *mut pg_sys::TupleTableSlot,
    tupdesc: &PgTupleDesc<'_>,
    row: &AggregationResultsRow,
    aggregate_clause: &AggregateCSClause,
    index_info: &AggIndexInfo,
) {
    let natts = (*(*slot).tts_tupleDescriptor).natts as usize;
    let datums = std::slice::from_raw_parts_mut((*slot).tts_values, natts);
    let isnull = std::slice::from_raw_parts_mut((*slot).tts_isnull, natts);

    let mut aggregates = row.aggregates.clone().into_iter();
    let mut natts_processed = 0;

    let grouping_columns = aggregate_clause.grouping_columns();

    // Fill in values according to the target list
    for (i, entry) in aggregate_clause.entries().enumerate() {
        let attr = tupdesc.get(i).expect("missing attribute");
        let expected_typoid = attr.type_oid().value();

        let datum = match entry {
            TargetListEntry::GroupingColumn(gc_idx) => {
                if row.is_empty() {
                    None
                } else {
                    group_key_to_datum(
                        row.group_keys[*gc_idx].clone(),
                        expected_typoid,
                        grouping_columns[*gc_idx].original_type_oid,
                    )
                }
            }
            TargetListEntry::Aggregate(agg_type) => {
                // Doc-count aggregates don't occupy a slot in `row.aggregates`
                // (see `uses_doc_count_path` and the matching filter in
                // `CollectFlat<..., MetricsWithGroupBy>` in build.rs), so the
                // iterator must only be advanced for aggregates that do.
                let next_aggregate =
                    if row.is_empty() || uses_doc_count_path(agg_type, aggregate_clause) {
                        None
                    } else {
                        aggregates.next().and_then(|v| v)
                    };
                aggregate_value_to_datum(
                    agg_type,
                    row,
                    expected_typoid,
                    aggregate_clause,
                    next_aggregate,
                    index_info,
                )
            }
        };

        if let Some(datum) = datum {
            datums[i] = datum;
            isnull[i] = false;
        } else {
            datums[i] = pg_sys::Datum::null();
            isnull[i] = true;
        }

        natts_processed += 1;
    }

    assert_eq!(natts, natts_processed, "target list length mismatch",);

    // Simple finalization - just set the flags and return the slot (no
    // ExecStoreVirtualTuple needed). Note: we don't set TTS_FLAG_SHOULDFREE
    // since we're reusing this slot across rows.
    (*slot).tts_flags &= !(pg_sys::TTS_FLAG_EMPTY as u16);
    (*slot).tts_nvalid = natts as i16;
}

/// Decodes internal binary fast-field representations for known-safe type casts.
///
/// Because `AggregateScan` maps grouping columns directly to `INDEX_VAR`s pointing at
/// the final scan slot, Postgres expects the slot to contain the Datum of the
/// *cast* type (e.g., `TEXTOID`), not the base column type. `try_into_datum` will
/// use this cast type to decode the `TantivyValue`.
///
/// For types that use an internal binary encoding in Tantivy (like `ltree` using `\0`
/// separators), casting directly to `TEXTOID` would expose the binary encoding.
/// This function intercepts known-safe conversions and decodes the internal
/// representation back into a standard `TantivyValue` string before it is cast.
///
/// This approach is only valid when the grouping and comparison semantics of the
/// binary-encoded fast field value are strictly equivalent to the semantics of
/// the projected value.
fn decode_safe_cast(
    mut key: TantivyValue,
    original_typoid: pg_sys::Oid,
    expected_typoid: pg_sys::Oid,
) -> TantivyValue {
    if original_typoid == expected_typoid {
        return key;
    }

    match (original_typoid, expected_typoid) {
        (orig, pg_sys::TEXTOID) if is_ltree_oid(orig) => {
            if let PdbOwnedValue::Str(ref s) = key.0 {
                key = TantivyValue(PdbOwnedValue::Str(
                    crate::postgres::catalog::facet_encoded_str_to_ltree_text(s),
                ));
            }
            key
        }
        // Add other safe `original_typoid` -> `expected_typoid` conversions here
        _ => key,
    }
}

/// Convert a Tantivy group key to a Postgres datum, handling NULL sentinels
/// (used for I64/U64/F64/Bool when the aggregator omits a row) and the
/// datetime decoding path (Tantivy returns ISO-8601 strings; we parse them
/// with chrono and re-pack as `tantivy::DateTime` before round-tripping
/// through `try_into_datum`).
///
/// Returns `None` for NULL sentinels; otherwise the converted datum.
unsafe fn group_key_to_datum(
    key: TantivyValue,
    expected_typoid: pg_sys::Oid,
    original_typoid: pg_sys::Oid,
) -> Option<pg_sys::Datum> {
    // Check if this is a NULL sentinel (handles both MIN and MAX sentinels).
    // U64 uses string sentinel for MIN (since 0 is valid); u64::MAX for MAX.
    // Bool uses string sentinels for both MIN and MAX.
    // DateTime columns don't have a missing sentinel (NULLs are excluded).
    let is_null_sentinel = match &key.0 {
        PdbOwnedValue::Str(s) => s == NULL_SENTINEL_MIN || s == NULL_SENTINEL_MAX,
        PdbOwnedValue::I64(v) => *v == i64::MAX || *v == i64::MIN,
        PdbOwnedValue::U64(v) => *v == u64::MAX,
        PdbOwnedValue::F64(v) => *v == f64::MAX || *v == f64::MIN,
        _ => false,
    };
    if is_null_sentinel {
        return None;
    }

    let key = decode_safe_cast(key, original_typoid, expected_typoid);

    if !is_datetime_type(expected_typoid) {
        return key
            .try_into_datum(pgrx::PgOid::from(expected_typoid))
            .expect("should be able to convert to datum");
    }

    // For datetime types, Tantivy's terms aggregation returns the date as
    // an ISO 8601 string (e.g., "2025-12-26T00:00:00Z"). We need to parse
    // this string and convert it to the appropriate PostgreSQL date type.
    match &key.0 {
        PdbOwnedValue::Str(date_str) => {
            let pgdt = match PostgresDateTime::try_from(date_str.as_str()) {
                Ok(ts) => ts,
                Err(e) => pgrx::error!("Failed to parse datetime string '{}': {}", date_str, e),
            };
            TantivyValue(PdbOwnedValue::Date(pgdt))
                .try_into_datum(expected_typoid.into())
                .expect("should be able to convert into datum")
        }
        PdbOwnedValue::I64(pg_micros) => {
            // (i64-storage) indexes: bucket key is already PG-epoch micros.
            let pgdt = PostgresDateTime::try_from_raw(*pg_micros)
                .expect("We should never see an invalid timestamp coming back from tantivy");
            TantivyValue(PdbOwnedValue::Date(pgdt))
                .try_into_datum(expected_typoid.into())
                .expect("should be able to convert into datum")
        }
        _ => key
            .try_into_datum(pgrx::PgOid::from(expected_typoid))
            .expect("should be able to convert to datum"),
    }
}

/// Detects ORDER BY + LIMIT for join aggregate queries and returns a
/// [`DataFusionTopK`] describing the sort target, direction, and K.
///
/// Supports two patterns:
/// - **ORDER BY aggregate LIMIT K** (e.g., `ORDER BY COUNT(*) DESC LIMIT 5`)
/// - **ORDER BY group_column LIMIT K** (e.g., `ORDER BY category LIMIT 5`)
///
/// Pushing sort+limit into the DataFusion plan enables three optimizations
/// depending on the sort target (handled by DataFusion's built-in
/// `TopKAggregation` optimizer rule and `SortExec(fetch=K)`):
/// - GROUP BY key ordering → early termination after K groups
/// - MIN/MAX ordering → PriorityMap-based group pruning during aggregation
/// - COUNT/SUM/AVG ordering → `SortExec(fetch=K)` bounded heap
unsafe fn detect_join_aggregate_topk(
    args: &CreateUpperPathsHookArgs,
    targetlist: &join_targetlist::JoinAggregateTargetList,
) -> Option<privdat::DataFusionTopK> {
    let parse = args.root().parse;
    if parse.is_null() || (*parse).sortClause.is_null() {
        return None;
    }

    // Must have a LIMIT for TopK to matter. We require a STATIC value here
    // because DataFusion's TopK rule needs a concrete K at planning time.
    // Parameterized LIMIT is left as a regular sort+limit pipeline.
    let limit_offset = LimitOffset::from_parse(parse)?;
    let static_fetch = limit_offset.static_fetch()?;
    let k = static_fetch;

    // Only single sort clause for TopK
    let sort_clauses = PgList::<pg_sys::SortGroupClause>::from_pg((*parse).sortClause);
    if sort_clauses.len() != 1 {
        return None;
    }

    let sort_clause_ptr = sort_clauses.get_ptr(0)?;
    let sort_expr = pg_sys::get_sortgroupclause_expr(sort_clause_ptr, (*parse).targetList);

    let direction =
        SortDirection::from_sort_op((*sort_clause_ptr).sortop, (*sort_clause_ptr).nulls_first)?;

    // Find matching position in output_rel target using structural equality
    let reltarget = args.output_rel().reltarget;
    if reltarget.is_null() {
        return None;
    }
    let target_exprs = PgList::<pg_sys::Expr>::from_pg((*reltarget).exprs);

    let mut match_pos = None;
    for (pos, target_expr) in target_exprs.iter_ptr().enumerate() {
        if pg_sys::equal(
            sort_expr as *const core::ffi::c_void,
            target_expr as *const core::ffi::c_void,
        ) {
            match_pos = Some(pos);
            break;
        }
    }
    let pos = match_pos?;

    // Try aggregate target: ORDER BY COUNT(*), SUM(x), MIN(x), etc.
    if let Some(agg_idx) = targetlist
        .aggregates
        .iter()
        .position(|a| a.output_index == pos)
    {
        // The sort expression must BE an aggregate, not merely contain one.
        // e.g. ORDER BY ABS(SUM(score)) wraps the aggregate — ABS breaks
        // monotonicity so DataFusion's ordering wouldn't match Postgres.
        if targetlist::find_single_aggref_in_expr(sort_expr)
            .is_none_or(|a| a as *mut pg_sys::Node != sort_expr)
        {
            return None;
        }
        return Some(privdat::DataFusionTopK {
            sort_target: privdat::TopKSortTarget::Aggregate(agg_idx),
            direction,
            k,
        });
    }

    // Try group column: ORDER BY category, ORDER BY name, etc.
    if let Some(gc_idx) = targetlist
        .group_columns
        .iter()
        .position(|gc| gc.output_index == pos)
    {
        // The sort expression must be a simple Var (group column reference).
        if (*sort_expr).type_ != pg_sys::NodeTag::T_Var {
            return None;
        }

        // If the collation for this pathkey isn't "safe" (C-like), then we can't pushdown as Tantivy uses byte ordering
        let collation = pg_sys::exprCollation(sort_expr);
        if !is_collation_pushdown_safe(collation) {
            return None;
        }

        return Some(privdat::DataFusionTopK {
            sort_target: privdat::TopKSortTarget::GroupColumn(gc_idx),
            direction,
            k,
        });
    }

    None
}

/// Replace any T_Aggref expressions in the target list with T_FuncExpr placeholders
/// This is called at execution time to avoid "Aggref found in non-Agg plan node" errors
/// Uses expression_tree_mutator to handle nested Aggrefs (e.g., COALESCE(COUNT(*), 0))
unsafe fn replace_aggrefs_in_target_list(plan: *mut pg_sys::Plan) {
    use pgrx::pg_guard;

    if (*plan).targetlist.is_null() {
        return;
    }

    // Mutator function to replace Aggref nodes with placeholder FuncExpr
    #[pg_guard]
    unsafe extern "C-unwind" fn aggref_mutator(
        node: *mut pg_sys::Node,
        context: *mut core::ffi::c_void,
    ) -> *mut pg_sys::Node {
        if node.is_null() {
            return std::ptr::null_mut();
        }

        // If this is an Aggref, replace it with a placeholder FuncExpr
        if (*node).type_ == pg_sys::NodeTag::T_Aggref {
            let aggref = node as *mut pg_sys::Aggref;
            return make_placeholder_func_expr(aggref) as *mut pg_sys::Node;
        }

        // If this is an UNNEST FuncExpr, replace it with a placeholder FuncExpr of its result type.
        // This is safe because AggregateScan handles the unnesting via Tantivy's terms aggregation.
        if (*node).type_ == pg_sys::NodeTag::T_FuncExpr {
            let func_expr = node as *mut pg_sys::FuncExpr;
            if is_unnest_func((*func_expr).funcid) {
                return make_placeholder_func_expr_internal(
                    (*func_expr).funcresulttype,
                    (*func_expr).inputcollid,
                    (*func_expr).location,
                    "UNNEST",
                ) as *mut pg_sys::Node;
            }
        }

        // For all other nodes, use the standard mutator to walk children
        #[cfg(not(any(feature = "pg16", feature = "pg17", feature = "pg18")))]
        {
            let fnptr = aggref_mutator as *const ();
            let mutator: unsafe extern "C-unwind" fn() -> *mut pg_sys::Node =
                std::mem::transmute(fnptr);
            pg_sys::expression_tree_mutator(node, Some(mutator), context)
        }

        #[cfg(any(feature = "pg16", feature = "pg17", feature = "pg18"))]
        {
            pg_sys::expression_tree_mutator_impl(node, Some(aggref_mutator), context)
        }
    }

    let targetlist = PgList::<pg_sys::TargetEntry>::from_pg((*plan).targetlist);

    // Check if there are any Aggref or UNNEST nodes anywhere in the target list
    let has_unpushable = targetlist.iter_ptr().any(|te| {
        !te.is_null()
            && !(*te).expr.is_null()
            && (expr_contains_aggref((*te).expr as *mut pg_sys::Node)
                || expr_contains_unnest((*te).expr as *mut pg_sys::Node))
    });

    if !has_unpushable {
        return;
    }

    // Build a new target list with Aggrefs replaced by placeholders and UNNEST stripped
    let mut new_targetlist: *mut pg_sys::List = std::ptr::null_mut();
    for te in targetlist.iter_ptr() {
        let new_te = pg_sys::flatCopyTargetEntry(te);

        // Use the mutator to replace any Aggref or UNNEST nodes in the expression
        let new_expr = aggref_mutator((*te).expr as *mut pg_sys::Node, std::ptr::null_mut());
        (*new_te).expr = new_expr as *mut pg_sys::Expr;

        new_targetlist = pg_sys::lappend(new_targetlist, new_te.cast());
    }

    (*plan).targetlist = new_targetlist;
}

/// Check if an expression tree contains any UNNEST nodes
unsafe fn expr_contains_unnest(node: *mut pg_sys::Node) -> bool {
    use pgrx::pg_guard;
    use std::ptr::addr_of_mut;

    #[pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        context: *mut core::ffi::c_void,
    ) -> bool {
        if node.is_null() {
            return false;
        }

        if (*node).type_ == pg_sys::NodeTag::T_FuncExpr {
            let func_expr = node as *mut pg_sys::FuncExpr;
            if is_unnest_func((*func_expr).funcid) {
                let ctx = &mut *(context as *mut bool);
                *ctx = true;
                return true; // Stop walking
            }
        }

        pg_sys::expression_tree_walker(node, Some(walker), context)
    }

    let mut found = false;
    walker(node, addr_of_mut!(found).cast());
    found
}

/// Check if an expression tree contains any Aggref nodes
unsafe fn expr_contains_aggref(node: *mut pg_sys::Node) -> bool {
    use pgrx::pg_guard;
    use std::ptr::addr_of_mut;

    #[pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        context: *mut core::ffi::c_void,
    ) -> bool {
        if node.is_null() {
            return false;
        }

        if (*node).type_ == pg_sys::NodeTag::T_Aggref {
            let ctx = &mut *(context as *mut bool);
            *ctx = true;
            return true; // Stop walking
        }

        pg_sys::expression_tree_walker(node, Some(walker), context)
    }

    let mut found = false;
    walker(node, addr_of_mut!(found).cast());
    found
}

/// Creates a placeholder `FuncExpr` for a PostgreSQL `Aggref`.
///
/// The placeholder is used during execution to avoid "Aggref found in non-Agg plan node" errors.
unsafe fn make_placeholder_func_expr(aggref: *mut pg_sys::Aggref) -> *mut pg_sys::FuncExpr {
    let agg_name = get_aggregate_name(aggref);
    make_placeholder_func_expr_internal(
        (*aggref).aggtype,
        (*aggref).inputcollid,
        (*aggref).location,
        &agg_name,
    )
}

/// Creates a placeholder `FuncExpr` with the specified result type and label.
///
/// This is used both for `Aggref` nodes and for `UNNEST` calls which are handled
/// internally by the custom aggregate scan.
unsafe fn make_placeholder_func_expr_internal(
    result_type: pg_sys::Oid,
    input_collid: pg_sys::Oid,
    location: i32,
    label: &str,
) -> *mut pg_sys::FuncExpr {
    let paradedb_funcexpr: *mut pg_sys::FuncExpr =
        pg_sys::palloc0(size_of::<pg_sys::FuncExpr>()).cast();
    (*paradedb_funcexpr).xpr.type_ = pg_sys::NodeTag::T_FuncExpr;
    (*paradedb_funcexpr).funcid = placeholder_procid();
    (*paradedb_funcexpr).funcresulttype = result_type;
    (*paradedb_funcexpr).funcretset = false;
    (*paradedb_funcexpr).funcvariadic = false;
    (*paradedb_funcexpr).funcformat = pg_sys::CoercionForm::COERCE_EXPLICIT_CALL;
    (*paradedb_funcexpr).funccollid = pg_sys::InvalidOid;
    (*paradedb_funcexpr).inputcollid = input_collid;
    (*paradedb_funcexpr).location = location;

    // Create a string argument with the label for better EXPLAIN output
    let label_const = make_text_const(label);
    let mut args = PgList::<pg_sys::Node>::new();
    args.push(label_const.cast());
    (*paradedb_funcexpr).args = args.into_pg();

    paradedb_funcexpr
}

/// Get a human-readable name for the aggregate function
unsafe fn get_aggregate_name(aggref: *mut pg_sys::Aggref) -> String {
    let funcid = (*aggref).aggfnoid;
    if funcid == agg_funcoid() {
        return "pdb.agg".to_string();
    }

    let name_str =
        crate::postgres::catalog::lookup_func_name(funcid).unwrap_or_else(|| "UNKNOWN".to_string());

    // Add (*) for COUNT(*) or star aggregates
    if (*aggref).aggstar {
        format!("{}(*)", name_str.to_uppercase())
    } else {
        name_str.to_uppercase()
    }
}

/// Check if the query (or any subquery/CTE within it) will trigger TopK pushdown.
///
/// AggregateScan currently does not support extracting `RTE_SUBQUERY` nodes and will typically
/// emit a WARNING when it encounters one. However, if a subquery is a TopK query (has a `LIMIT`
/// and an `ORDER BY` on a BM25 index), we want to silently decline it instead. This is because
/// `BaseScan` will natively optimize the subquery, meaning we can safely step aside without
/// bothering the user with a planner warning.
unsafe fn query_will_use_topk(parse: *mut pgrx::pg_sys::Query) -> bool {
    if parse.is_null() {
        return false;
    }

    // If there is an explicit LIMIT, check if TopK pushdown will natively optimize it
    if !(*parse).limitCount.is_null()
        && crate::gucs::enable_custom_scan()
        && validate_topk_compatibility(parse)
    {
        return true;
    }

    // Check subqueries in RTEs
    if !(*parse).rtable.is_null() {
        let rtable = pgrx::list::PgList::<pgrx::pg_sys::RangeTblEntry>::from_pg((*parse).rtable);
        for rte in rtable.iter_ptr() {
            if (*rte).rtekind == pgrx::pg_sys::RTEKind::RTE_SUBQUERY
                && !(*rte).subquery.is_null()
                && query_will_use_topk((*rte).subquery)
            {
                return true;
            }
        }
    }

    // Check CTEs (Common Table Expressions)
    if !(*parse).cteList.is_null() {
        let ctelist =
            pgrx::list::PgList::<pgrx::pg_sys::CommonTableExpr>::from_pg((*parse).cteList);
        for cte in ctelist.iter_ptr() {
            if !(*cte).ctequery.is_null() && query_will_use_topk((*cte).ctequery.cast()) {
                return true;
            }
        }
    }

    // Check setOperations (UNION/INTERSECT/EXCEPT arms)
    // The query block for the set operations usually references the arms in rtable,
    // but just in case, we can also descend into the setOperations tree if we needed,
    // but since they are in rtable as RTE_SUBQUERY, iterating rtable is sufficient.

    false
}
