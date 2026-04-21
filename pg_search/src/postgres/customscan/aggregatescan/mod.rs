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
pub mod limit_offset;
pub mod orderby;
pub mod privdat;
pub mod scan_state;
pub mod searchquery;
pub mod targetlist;

// Re-export commonly used types for easier access
pub use aggregate_type::AggregateType;
pub use groupby::GroupingColumn;
pub use targetlist::TargetListEntry;

use crate::api::agg_funcoid;
use crate::api::SortDirection;
use crate::gucs;

use crate::aggregate::{NULL_SENTINEL_MAX, NULL_SENTINEL_MIN};
use crate::customscan::aggregatescan::build::AggregateCSClause;
use crate::postgres::customscan::aggregatescan::datafusion_build::{
    all_have_bm25_index, collect_join_agg_sources, extract_join_tree_from_parse, has_any_bm25_index,
};
use crate::postgres::customscan::aggregatescan::datafusion_exec::{
    build_join_aggregate_plan, create_aggregate_session_context,
    create_aggregate_session_context_mpp,
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
use crate::postgres::customscan::dsm as mpp_dsm;
use crate::postgres::customscan::dsm::ParallelQueryCapable;
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::joinscan::scan_state::{build_physical_plan, build_task_context};
use crate::postgres::customscan::limit_offset::LimitOffset;
use crate::postgres::customscan::projections::{create_placeholder_targetlist, placeholder_procid};
use crate::postgres::customscan::solve_expr::SolvePostgresExpressions;
use crate::postgres::customscan::{range_table, CreateUpperPathsHookArgs, CustomScan};
use crate::postgres::rel_get_bm25_index;
use crate::postgres::types::{is_datetime_type, TantivyValue};
use crate::postgres::utils::{add_vars_to_tlist, is_unnest_func, make_text_const};
use crate::postgres::PgSearchRelation;
use chrono::{DateTime as ChronoDateTime, Utc};
use pgrx::{pg_sys, PgList, PgMemoryContexts, PgTupleDesc};
use std::ffi::CStr;
use tantivy::schema::OwnedValue;

#[derive(Default)]
pub struct AggregateScan;

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
            BeginCustomScan: Some(crate::postgres::customscan::exec::begin_custom_scan::<Self>),
            ExecCustomScan: Some(crate::postgres::customscan::exec::exec_custom_scan::<Self>),
            EndCustomScan: Some(crate::postgres::customscan::exec::end_custom_scan::<Self>),
            ReScanCustomScan: Some(crate::postgres::customscan::exec::rescan_custom_scan::<Self>),
            MarkPosCustomScan: None,
            RestrPosCustomScan: None,
            // MPP DSM slots. These are only invoked when the path is flagged
            // parallel-safe in `create_custom_path`; they wire the MPP
            // lifecycle when `customscan_glue::mpp_is_active()`.
            EstimateDSMCustomScan: Some(mpp_dsm::estimate_dsm_custom_scan::<Self>),
            InitializeDSMCustomScan: Some(mpp_dsm::initialize_dsm_custom_scan::<Self>),
            ReInitializeDSMCustomScan: Some(mpp_dsm::reinitialize_dsm_custom_scan::<Self>),
            InitializeWorkerCustomScan: Some(mpp_dsm::initialize_worker_custom_scan::<Self>),
            ShutdownCustomScan: Some(
                crate::postgres::customscan::exec::shutdown_custom_scan::<Self>,
            ),
            ExplainCustomScan: Some(crate::postgres::customscan::exec::explain_custom_scan::<Self>),
        }
    }

    fn create_custom_path(builder: CustomPathBuilder<Self>) -> Vec<pg_sys::CustomPath> {
        let has_paradedb_agg = unsafe {
            let parse = builder.args().root().parse;
            !parse.is_null()
                && crate::postgres::customscan::hook::query_has_paradedb_agg(parse, true)
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

            // Check if the query has pathkeys (e.g. GROUP BY emits group_pathkeys
            // into query_pathkeys) before consuming builder.
            let root = builder.args().root;
            let has_pathkeys = unsafe {
                !(*root).query_pathkeys.is_null() && pg_sys::list_length((*root).query_pathkeys) > 0
            };
            // Separately check for an explicit SQL ORDER BY clause. GROUP BY
            // without ORDER BY sets query_pathkeys (group_pathkeys) but leaves
            // parse->sortClause empty — and for the MPP path we can aggref-
            // replace plan.targetlist in that case, since no Sort node will
            // be placed above us to care about the Aggrefs.
            let has_sort_clause = unsafe {
                let parse = (*root).parse;
                !parse.is_null()
                    && !(*parse).sortClause.is_null()
                    && pg_sys::list_length((*parse).sortClause) > 0
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
                    // Non-MPP, no-pathkeys case: replace Aggrefs at plan
                    // time with `pdb.agg_fn(...)` placeholders. The non-MPP
                    // CustomScan executes aggregation internally (without
                    // a Gather above), so placing FuncExprs here is fine —
                    // they never get evaluated because our ExecCustomScan
                    // produces finished tuples directly.
                    let plan = &mut cscan.scan.plan;
                    replace_aggrefs_in_target_list(plan);
                }
                // For the MPP (parallel_aware) path we leave Aggrefs in
                // plan.targetlist. The Gather above us has a tlist with
                // structurally-equal Aggrefs (from `rel->reltarget`); at
                // setrefs time `fix_upper_expr` matches them via `equal()`
                // and rewrites to `Var(OUTER_VAR, N)`. CustomScan's own
                // plan.targetlist Aggrefs get rewritten to
                // `Var(INDEX_VAR, N)` against `custom_scan_tlist`. Any
                // Result added above a Sort for ORDER BY also sees
                // matching Aggrefs in its subplan. Replacement here would
                // break all three of those matches under ORDER BY.
                // When has_pathkeys AND !parallel_aware (non-MPP ORDER BY
                // path): aggrefs stay in plan.targetlist so Postgres's
                // `make_sort_from_pathkeys` can find them. Replacement is
                // deferred to `create_custom_scan_state` (execution time).
                let _ = has_sort_clause; // previously gated on this; now unused
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
                builder.custom_state().aggregate_clause = *aggregate_clause;
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
                    join_level_predicates,
                    multi_table_predicates,
                    custom_exprs,
                    custom_scan_tlist,
                    having_filter,
                    runtime: None,
                    stream: None,
                    current_batch: None,
                    batch_row_idx: 0,
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
                    let groups: Vec<String> = df_state
                        .targetlist
                        .group_columns
                        .iter()
                        .map(|gc| gc.field_name.clone())
                        .collect();
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
                                a.field_refs.iter().map(|(_, _, n)| n.as_str()).collect();
                            format!("{}({})", a.agg_kind, fields.join(", "))
                        }
                    })
                    .collect();
                if !aggs.is_empty() {
                    explainer.add_text("Aggregates", aggs.join(", "));
                }
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
            // DataFusion backend: create scan slot for result projection
            unsafe {
                let planstate = state.planstate();
                let scan_slot = pg_sys::MakeTupleTableSlot(
                    (*planstate).ps_ResultTupleDesc,
                    &pg_sys::TTSOpsVirtual,
                );
                state.custom_state_mut().scan_slot = Some(scan_slot);
            }

            // MPP plan-bytes stash: serialize the DataFusion logical plan
            // at BeginCustomScan time so the DSM hooks can read `.len()`
            // and the bytes without rebuilding the plan. Skip for
            // `EXPLAIN` without `ANALYZE` (EXEC_FLAG_EXPLAIN_ONLY set) —
            // users running bare EXPLAIN don't want to pay plan-build +
            // serialize cost that's purely for parallel init. Also gated
            // on `mpp_is_active()` so non-MPP queries pay zero cost.
            // Failures inside the helper are non-fatal (log + fall back).
            let explain_only = (eflags & pg_sys::EXEC_FLAG_EXPLAIN_ONLY as i32) != 0;
            if !explain_only && crate::postgres::customscan::mpp::customscan_glue::mpp_is_active() {
                Self::maybe_stash_mpp_plan_bytes(state);
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

    fn shutdown_custom_scan(_state: &mut CustomScanStateWrapper<Self>) {}

    fn end_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        // Explicitly drop DataFusion resources (runtime, stream, batches) at the
        // intended lifecycle boundary rather than relying on Postgres to drop the
        // state wrapper later. Mirrors JoinScan::end_custom_scan.
        if let Some(mut df_state) = state.custom_state_mut().datafusion_state.take() {
            df_state.stream = None;
            df_state.current_batch = None;
            df_state.runtime = None;
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
    /// Build the DataFusion logical plan now, serialize it via
    /// `PgSearchExtensionCodec`, and stash the bytes on `AggregateScanState`
    /// for the MPP DSM hooks to consume. Called from `begin_custom_scan`
    /// when `mpp_is_active()` is true and the DataFusion backend is active.
    ///
    /// Failures (plan-build, serialize) are logged via `mpp_log!` and leave
    /// `logical_plan_bytes` as `None` — MPP then silently falls back to the
    /// non-MPP path at `exec_datafusion_aggregate` rather than aborting the
    /// query. This is the most ops-friendly failure mode: a misconfigured
    /// MPP setting shouldn't take down a query that would otherwise succeed.
    ///
    /// Uses a throwaway single-thread tokio runtime because
    /// `build_join_aggregate_plan` / `build_physical_plan` are async-signature
    /// even though their bodies don't need an I/O runtime. The
    /// `exec_datafusion_aggregate` path does the same dance — we pay the
    /// plan-build cost twice when MPP is active (once here, once at exec
    /// time). A future optimization is to cache the plan, but the
    /// complexity of keeping logical + physical in sync across parallel
    /// workers isn't worth it for milestone 1.
    fn maybe_stash_mpp_plan_bytes(state: &mut CustomScanStateWrapper<Self>) {
        let Some(df_state) = state.custom_state().datafusion_state.as_ref() else {
            return;
        };
        let plan = df_state.plan.clone();
        let targetlist = df_state.targetlist.clone();
        let topk = df_state.topk.clone();
        let jlp = df_state.join_level_predicates.clone();
        let custom_exprs = df_state.custom_exprs;
        let custom_scan_tlist = df_state.custom_scan_tlist;
        let having = df_state.having_filter.clone();

        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                crate::mpp_log!("mpp: failed to build tokio runtime for plan stash: {e}");
                return;
            }
        };

        let ctx = create_aggregate_session_context();
        let plan_and_shape: datafusion::common::Result<(
            bytes::Bytes,
            crate::postgres::customscan::mpp::shape::MppPlanShape,
        )> = rt.block_on(async {
            let logical = build_join_aggregate_plan(
                &plan,
                &targetlist,
                topk.as_ref(),
                &jlp,
                custom_exprs,
                custom_scan_tlist,
                having.as_ref(),
                &ctx,
            )
            .await?;
            let shape = Self::classify_logical_plan(&logical);
            let bytes = crate::scan::codec::serialize_logical_plan(&logical)?;
            Ok((bytes, shape))
        });

        match plan_and_shape {
            Ok((bytes, shape)) => {
                // Wrap the raw logical plan in `MppPlanBroadcast` and
                // bincode-serialize *now* so `logical_plan_bytes.len()` equals
                // the exact bytes that will later be copied into DSM. If we
                // instead stashed the raw logical plan bytes here and
                // re-wrapped at DSM init time, `estimate_dsm_custom_scan`
                // would under-report by `MppPlanBroadcast`'s framing overhead
                // (~6 bytes: version byte + varint length prefix +
                // total_participants + profile tag). That mismatch causes the
                // DSM init path to overrun the PG-allocated `shm_toc` region
                // and corrupt adjacent DSA control blocks → crash at
                // `dsa_release_in_place` during parallel-context teardown.
                let n = crate::postgres::customscan::mpp::customscan_glue::mpp_worker_count();
                let broadcast = crate::postgres::customscan::mpp::session::MppPlanBroadcast::new(
                    bytes.to_vec(),
                    n,
                    crate::postgres::customscan::mpp::session::MppSessionProfile::Aggregate,
                );
                let broadcast_bytes = match broadcast.serialize() {
                    Ok(b) => b,
                    Err(e) => {
                        crate::mpp_log!(
                            "mpp: MppPlanBroadcast serialize failed, MPP will fall back: {e}"
                        );
                        return;
                    }
                };
                crate::mpp_log!(
                    "mpp: stashed plan-broadcast bytes (shape={shape:?}) on AggregateScanState"
                );
                state.custom_state_mut().logical_plan_bytes =
                    Some(bytes::Bytes::from(broadcast_bytes));
                state.custom_state_mut().mpp_shape = Some(shape);
            }
            Err(e) => {
                crate::mpp_log!(
                    "mpp: plan build/serialize failed, MPP will fall back to serial path: {e}"
                );
            }
        }
    }

    /// Walk a built `LogicalPlan` tree and derive [`MppPlanShape`] inputs.
    /// Counts table scans, detects GROUP BY / aggregate presence, and
    /// inspects aggregate function names for splittability. Runs once per
    /// MPP-eligible query; keep side-effect free so it can be called during
    /// plan-stash.
    fn classify_logical_plan(
        plan: &datafusion::logical_expr::LogicalPlan,
    ) -> crate::postgres::customscan::mpp::shape::MppPlanShape {
        use datafusion::common::tree_node::TreeNode;
        use datafusion::logical_expr::LogicalPlan;
        let mut n_table_scans: usize = 0;
        let mut has_aggregate = false;
        let mut has_group_by = false;
        let mut all_aggregates_splittable = true;

        plan.apply(|node| {
            match node {
                LogicalPlan::TableScan(_) => {
                    n_table_scans += 1;
                }
                LogicalPlan::Aggregate(a) => {
                    has_aggregate = true;
                    if !a.group_expr.is_empty() {
                        has_group_by = true;
                    }
                    for expr in &a.aggr_expr {
                        if !Self::is_splittable_aggregate_expr(expr) {
                            all_aggregates_splittable = false;
                        }
                    }
                }
                _ => {}
            }
            Ok(datafusion::common::tree_node::TreeNodeRecursion::Continue)
        })
        .ok();

        crate::postgres::customscan::mpp::shape::classify(
            &crate::postgres::customscan::mpp::shape::ClassifyInputs {
                n_join_tables: n_table_scans,
                has_group_by,
                all_aggregates_splittable,
                has_aggregate,
            },
        )
    }

    /// Early-path classification used at `create_custom_path` time, before
    /// we have a DataFusion logical plan. Mirrors [`Self::classify_logical_plan`]
    /// but reads from the PG-side `sources` + `JoinAggregateTargetList` that
    /// are already available at this stage.
    ///
    /// Returns the cheap-but-correct shape input triple; feeds directly into
    /// [`mpp::shape::classify`]. Conservative: any aggregate not on the
    /// partial-safe allow-list downgrades the whole query to `Ineligible`.
    fn classify_path_shape(
        sources: &[crate::postgres::customscan::aggregatescan::datafusion_build::JoinAggSource],
        targetlist: &crate::postgres::customscan::aggregatescan::join_targetlist::JoinAggregateTargetList,
    ) -> crate::postgres::customscan::mpp::shape::MppPlanShape {
        use crate::postgres::customscan::aggregatescan::join_targetlist::AggKind;
        let n_join_tables = sources.len();
        let has_group_by = !targetlist.group_columns.is_empty();
        let has_aggregate = !targetlist.aggregates.is_empty();
        let all_aggregates_splittable = targetlist.aggregates.iter().all(|a| {
            matches!(
                a.agg_kind,
                AggKind::CountStar
                    | AggKind::Count
                    | AggKind::Sum
                    | AggKind::Avg
                    | AggKind::Min
                    | AggKind::Max
                    | AggKind::BoolAnd
                    | AggKind::BoolOr
                    | AggKind::StddevSamp
                    | AggKind::StddevPop
                    | AggKind::VarSamp
                    | AggKind::VarPop
            ) && !a.distinct
        });
        crate::postgres::customscan::mpp::shape::classify(
            &crate::postgres::customscan::mpp::shape::ClassifyInputs {
                n_join_tables,
                has_group_by,
                all_aggregates_splittable,
                has_aggregate,
            },
        )
    }

    /// If the query is MPP-eligible, flip the path's `parallel_safe` /
    /// `parallel_aware` / `parallel_workers` on the builder so PG launches
    /// workers for this scan. Called from `try_build_datafusion_aggregate_path`
    /// just before `.build(...)`.
    ///
    /// Returns the builder unchanged when MPP is off, the worker count is
    /// below 2, or the shape classifies as `Ineligible`.
    fn maybe_flip_mpp_parallel(
        builder: CustomPathBuilder<Self>,
        sources: &[crate::postgres::customscan::aggregatescan::datafusion_build::JoinAggSource],
        targetlist: &crate::postgres::customscan::aggregatescan::join_targetlist::JoinAggregateTargetList,
    ) -> CustomPathBuilder<Self> {
        use crate::postgres::customscan::mpp::shape::MppPlanShape;
        if !crate::postgres::customscan::mpp::customscan_glue::mpp_is_active() {
            return builder;
        }
        let shape = Self::classify_path_shape(sources, targetlist);

        // `GroupByAggSingleTable` and `JoinOnly` shapes' bridges haven't
        // landed yet; they still classify correctly but fall through to
        // the serial path here.
        let activate = matches!(
            shape,
            MppPlanShape::ScalarAggOnBinaryJoin | MppPlanShape::GroupByAggOnBinaryJoin
        );
        if !activate {
            return builder;
        }
        let n = crate::postgres::customscan::mpp::customscan_glue::mpp_worker_count() as usize;
        // `mpp_worker_count` clamps to >= 2, but be defensive.
        let nworkers = n.saturating_sub(1).max(1);
        crate::mpp_log!(
            "mpp: flipping AggregateScan path parallel_workers={} for shape {:?}",
            nworkers,
            shape
        );
        builder.set_parallel(nworkers)
    }

    /// Conservative allow-list of aggregate function names that are safe to
    /// split into Partial/FinalPartitioned. Anything outside this list marks
    /// the shape Ineligible so MPP falls back to the serial path.
    ///
    /// The outer expression may be wrapped in `Alias` (e.g., `COUNT(*) AS cnt`)
    /// — unwrap before checking. Any other wrapping (casts, arithmetic) is
    /// treated as unsafe to be conservative.
    fn is_splittable_aggregate_expr(expr: &datafusion::logical_expr::Expr) -> bool {
        use datafusion::logical_expr::Expr;
        let unwrapped = match expr {
            Expr::Alias(a) => a.expr.as_ref(),
            other => other,
        };
        let name = match unwrapped {
            Expr::AggregateFunction(af) => af.func.name().to_ascii_lowercase(),
            // Non-aggregate in the aggr_expr slot — assume unsafe.
            _ => return false,
        };
        matches!(
            name.as_str(),
            "count"
                | "sum"
                | "min"
                | "max"
                | "avg"
                | "bool_and"
                | "bool_or"
                | "stddev"
                | "stddev_samp"
                | "stddev_pop"
                | "var"
                | "var_samp"
                | "var_pop"
        )
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
        let Some((_table, index)) = rel_get_bm25_index(unsafe { (*heap_rte).relid }) else {
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

        // For M1, all tables must have BM25 indexes (DataFusion scans all via PgSearchTableProvider)
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
        let targetlist = unsafe { extract_aggregate_targetlist(builder.args(), &sources) }
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

        // Extract HAVING clause if present
        let having_filter = unsafe {
            let parse = builder.args().root().parse;
            if !parse.is_null() && !(*parse).havingQual.is_null() {
                privdat::FilterExpr::from_pg_node(
                    (*parse).havingQual,
                    &datafusion_build::FilterExprBuildContext {
                        targetlist: Some(&targetlist),
                        sources: None,
                    },
                )
            } else {
                None
            }
        };

        // Flip parallel-safe with the configured worker count when the
        // shape is MPP-eligible. Uses a cheap pre-classification based on
        // `sources` + `targetlist` — the authoritative shape is re-derived
        // from the DataFusion logical plan in `maybe_stash_mpp_plan_bytes`,
        // but we must commit to parallel-safe here (before `.build()`)
        // because PG's path-builder freezes the parallel flags at this
        // point. Downstream failure to stash the plan falls back to the
        // serial path via the `mpp_state.is_none()` guard in
        // `exec_datafusion_aggregate`.
        let builder = Self::maybe_flip_mpp_parallel(builder, &sources, &targetlist);

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

            fill_slot_from_row(slot, &tupdesc, &row, &state.custom_state().aggregate_clause);

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
        // Correctness fence: if we're a parallel worker and `mpp_state` is
        // None even though MPP is supposed to be active, the worker DSM
        // attach didn't run. Silently running the non-MPP path here would
        // double-count at the final gather. Abort loudly instead.
        // `IsParallelWorker` is a C macro (`ParallelWorkerNumber >= 0`);
        // pgrx exposes the underlying int but not the macro, so inline it.
        let is_parallel_worker = unsafe { pg_sys::ParallelWorkerNumber } >= 0;
        if is_parallel_worker
            && crate::postgres::customscan::mpp::customscan_glue::mpp_is_active()
            && state.custom_state().mpp_state.is_none()
        {
            pgrx::error!(
                "mpp: parallel worker reached exec_datafusion_aggregate without \
                 MppExecutionState — worker DSM attach did not run. Wrong-result \
                 scenario averted via loud abort."
            );
        }

        // Grab the scan_slot pointer before entering the mutable borrow
        let scan_slot = state
            .custom_state()
            .scan_slot
            .expect("scan_slot must be initialized in begin_custom_scan");

        // Decide whether we need the MPP rewrite *before* we take the
        // `datafusion_state` mutable borrow below — the MPP branch needs
        // `custom_state_mut().mpp_state`, which aliases the same
        // `custom_state_mut()` borrow. We resolve the alias here by reading
        // the shape first (immutable read) then taking the mpp_state
        // mutable borrow separately from the `datafusion_state` one.
        let mpp_shape = if state.custom_state().mpp_state.is_some() {
            Some(match state.custom_state().mpp_shape {
                Some(s) => s,
                None => pgrx::error!(
                    "mpp: exec_datafusion_aggregate has mpp_state but no mpp_shape — \
                     begin_custom_scan must populate both or neither"
                ),
            })
        } else {
            None
        };

        // First call: build and execute the DataFusion plan.
        //
        // We need to reach both `state.custom_state_mut().datafusion_state`
        // and `state.custom_state_mut().mpp_state` inside this block, so
        // we don't take the long-lived `df_state` borrow up front; instead,
        // we build the plan + stream locally and store into the fields at
        // the end in a short borrow.
        let runtime_is_none = state
            .custom_state()
            .datafusion_state
            .as_ref()
            .map(|d| d.runtime.is_none())
            .unwrap_or(false);
        if runtime_is_none {
            // Always current-thread. PG FFI is single-threaded — pgrx's
            // `thread_id_check` panics with "postgres FFI may not be
            // called from multiple threads" on any PG call from a tokio
            // worker thread. `PgSearchTableProvider::scan` and its
            // downstream reach into PG during the lazy scan, so a
            // multi-thread runtime is off the table both for MPP and
            // non-MPP paths.
            //
            // The MPP `shm_mq_send` stall (the reason the earlier
            // multi-thread experiment existed) is broken instead by
            // making the sender cooperative: on would-block, call
            // `DrainHandle::poll_drain_pass` on the same mesh's drain
            // to consume inbound rows — that frees peers' outbound
            // queues and lets their send-to-us un-stall. See
            // `MppSender::send_batch` + `mesh::ShmMqSender::try_send_bytes`.
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap_or_else(|e| pgrx::error!("Failed to create tokio runtime: {}", e));

            // When MPP is active we build the session with the join-layer
            // bug-fixes applied: skip `SortMergeJoinEnforcer` (which would
            // collapse hash-partitioned streams to one partition) and disable
            // `enable_join_dynamic_filter_pushdown` (which races with the
            // exchange producer on the probe-side scan and drops rows before
            // the dynamic filter stabilizes). Without this, MPP AggregateScan
            // silently loses ~83% of probe-side rows at high cardinality.
            let ctx = match state.custom_state().mpp_state.as_ref() {
                Some(mpp_state) if mpp_state.participant_config().total_participants > 1 => {
                    let cfg = mpp_state.participant_config();
                    let ctx = create_aggregate_session_context_mpp(cfg);
                    ctx.state_ref()
                        .write()
                        .config_mut()
                        .set_extension(std::sync::Arc::new(
                            crate::postgres::customscan::mpp::MppShardConfig {
                                participant_index: cfg.participant_index,
                                total_participants: cfg.total_participants,
                            },
                        ));
                    ctx
                }
                _ => create_aggregate_session_context(),
            };

            // Build the logical → standard physical plan in a short
            // immutable-ish scope that doesn't block the later
            // `mpp_state` mutable borrow.
            let standard_plan = {
                let df_state = state
                    .custom_state_mut()
                    .datafusion_state
                    .as_mut()
                    .expect("DataFusion state must be initialized");
                let custom_exprs = df_state.custom_exprs;
                let custom_scan_tlist = df_state.custom_scan_tlist;
                let result = runtime.block_on(async {
                    let logical = build_join_aggregate_plan(
                        &df_state.plan,
                        &df_state.targetlist,
                        df_state.topk.as_ref(),
                        &df_state.join_level_predicates,
                        custom_exprs,
                        custom_scan_tlist,
                        df_state.having_filter.as_ref(),
                        &ctx,
                    )
                    .await?;
                    build_physical_plan(&ctx, logical).await
                });
                match result {
                    Ok(p) => p,
                    Err(e) => pgrx::error!("Failed to build DataFusion aggregate plan: {}", e),
                }
            };

            // If MPP state was wired up by the DSM hooks, rewrite the
            // standard plan into a shape-specific MPP plan. The non-MPP
            // branch stays byte-identical to the previous behavior.
            let physical_plan = if let Some(shape) = mpp_shape {
                let mpp_state = state
                    .custom_state_mut()
                    .mpp_state
                    .as_mut()
                    .expect("mpp_shape set ⇒ mpp_state must also be Some");
                crate::mpp_log!(
                    "mpp: routing exec_datafusion_aggregate through shape {:?} (is_leader={})",
                    shape,
                    mpp_state.is_leader()
                );
                let _guard = runtime.enter();
                match crate::postgres::customscan::mpp::exec_bridge::build_mpp_physical_plan(
                    standard_plan,
                    mpp_state,
                    shape,
                ) {
                    Ok(p) => p,
                    Err(e) => pgrx::error!("mpp: build_mpp_physical_plan failed: {e}"),
                }
            } else {
                standard_plan
            };

            let task_ctx = build_task_context(
                &ctx,
                &physical_plan,
                unsafe { pg_sys::work_mem as usize * 1024 },
                unsafe { pg_sys::hash_mem_multiplier },
            );
            let stream = {
                let _guard = runtime.enter();
                match physical_plan.execute(0, task_ctx) {
                    Ok(s) => s,
                    Err(e) => pgrx::error!("Failed to execute DataFusion aggregate plan: {}", e),
                }
            };

            let df_state = state
                .custom_state_mut()
                .datafusion_state
                .as_mut()
                .expect("DataFusion state must be initialized");
            df_state.runtime = Some(runtime);
            df_state.stream = Some(stream);
        }

        let df_state = state
            .custom_state_mut()
            .datafusion_state
            .as_mut()
            .expect("DataFusion state must be initialized");

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
                    let result = unsafe {
                        project_aggregate_row_to_slot(scan_slot, batch, row_idx, targetlist)
                    };
                    df_state.batch_row_idx += 1;
                    return result;
                }
                // Current batch exhausted
                df_state.current_batch = None;
            }

            // Fetch next batch from stream
            let runtime = df_state.runtime.as_ref().unwrap();
            let stream = df_state.stream.as_mut().unwrap();

            let next = runtime.block_on(async {
                use futures::StreamExt;
                stream.next().await
            });

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
) -> Option<pg_sys::Datum> {
    if row.is_empty() {
        return agg_type.nullish().value.and_then(|value| {
            TantivyValue(OwnedValue::F64(value))
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
    exec::aggregate_result_to_datum(next_aggregate, agg_type, target_typoid)
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
) {
    let natts = (*(*slot).tts_tupleDescriptor).natts as usize;
    let datums = std::slice::from_raw_parts_mut((*slot).tts_values, natts);
    let isnull = std::slice::from_raw_parts_mut((*slot).tts_isnull, natts);

    let mut aggregates = row.aggregates.clone().into_iter();
    let mut natts_processed = 0;

    // Fill in values according to the target list
    for (i, entry) in aggregate_clause.entries().enumerate() {
        let attr = tupdesc.get(i).expect("missing attribute");
        let expected_typoid = attr.type_oid().value();

        let datum = match entry {
            TargetListEntry::GroupingColumn(gc_idx) => {
                if row.is_empty() {
                    None
                } else {
                    group_key_to_datum(row.group_keys[*gc_idx].clone(), expected_typoid)
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
) -> Option<pg_sys::Datum> {
    // Check if this is a NULL sentinel (handles both MIN and MAX sentinels).
    // U64 uses string sentinel for MIN (since 0 is valid); u64::MAX for MAX.
    // Bool uses string sentinels for both MIN and MAX.
    // DateTime columns don't have a missing sentinel (NULLs are excluded).
    let is_null_sentinel = match &key.0 {
        OwnedValue::Str(s) => s == NULL_SENTINEL_MIN || s == NULL_SENTINEL_MAX,
        OwnedValue::I64(v) => *v == i64::MAX || *v == i64::MIN,
        OwnedValue::U64(v) => *v == u64::MAX,
        OwnedValue::F64(v) => *v == f64::MAX || *v == f64::MIN,
        _ => false,
    };
    if is_null_sentinel {
        return None;
    }

    if !is_datetime_type(expected_typoid) {
        return key
            .try_into_datum(pgrx::PgOid::from(expected_typoid))
            .expect("should be able to convert to datum");
    }

    // For datetime types, Tantivy's terms aggregation returns the date as
    // an ISO 8601 string (e.g., "2025-12-26T00:00:00Z"). We need to parse
    // this string and convert it to the appropriate PostgreSQL date type.
    match &key.0 {
        OwnedValue::Str(date_str) => match date_str.parse::<ChronoDateTime<Utc>>() {
            Ok(chrono_dt) => {
                // Convert to nanoseconds since epoch for Tantivy DateTime
                let nanos = chrono_dt.timestamp_nanos_opt().unwrap_or(0);
                let datetime = tantivy::DateTime::from_timestamp_nanos(nanos);
                TantivyValue(OwnedValue::Date(datetime))
                    .try_into_datum(pgrx::PgOid::from(expected_typoid))
                    .expect("should be able to convert datetime to datum")
            }
            Err(e) => {
                pgrx::error!("Failed to parse datetime string '{}': {}", date_str, e);
            }
        },
        OwnedValue::I64(nanos) => {
            // Fallback for I64 (nanoseconds timestamp)
            let datetime = tantivy::DateTime::from_timestamp_nanos(*nanos);
            TantivyValue(OwnedValue::Date(datetime))
                .try_into_datum(pgrx::PgOid::from(expected_typoid))
                .expect("should be able to convert datetime to datum")
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

    // Must have a LIMIT for TopK to matter
    let limit_offset = LimitOffset::from_parse(parse);
    let limit = limit_offset.limit()? as usize;
    let offset = limit_offset.offset().unwrap_or(0) as usize;
    let k = limit + offset;

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
/// Mutator that replaces Aggref nodes with a placeholder FuncExpr and UNNEST
/// FuncExprs with a generic placeholder of their result type. Used to clean
/// up Plan targetlists (see [`replace_aggrefs_in_target_list`]) on the
/// non-parallel path — the MPP path keeps Aggrefs intact so PG's setrefs
/// matches them structurally between Gather/Sort/Result and the CustomScan.
#[pgrx::pg_guard]
pub(crate) unsafe extern "C-unwind" fn aggref_mutator(
    node: *mut pg_sys::Node,
    context: *mut core::ffi::c_void,
) -> *mut pg_sys::Node {
    if node.is_null() {
        return std::ptr::null_mut();
    }

    if (*node).type_ == pg_sys::NodeTag::T_Aggref {
        let aggref = node as *mut pg_sys::Aggref;
        return make_placeholder_func_expr(aggref) as *mut pg_sys::Node;
    }

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

    #[cfg(not(any(feature = "pg16", feature = "pg17", feature = "pg18")))]
    {
        let fnptr = aggref_mutator as usize as *const ();
        let mutator: unsafe extern "C-unwind" fn() -> *mut pg_sys::Node =
            std::mem::transmute(fnptr);
        pg_sys::expression_tree_mutator(node, Some(mutator), context)
    }

    #[cfg(any(feature = "pg16", feature = "pg17", feature = "pg18"))]
    {
        pg_sys::expression_tree_mutator_impl(node, Some(aggref_mutator), context)
    }
}

unsafe fn replace_aggrefs_in_target_list(plan: *mut pg_sys::Plan) {
    if (*plan).targetlist.is_null() {
        return;
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
    // Try to get the function name from the catalog
    let funcid = (*aggref).aggfnoid;
    if funcid == agg_funcoid() {
        return "pdb.agg".to_string();
    }
    let proc_tuple =
        pg_sys::SearchSysCache1(pg_sys::SysCacheIdentifier::PROCOID as _, funcid.into());

    if !proc_tuple.is_null() {
        let proc_form = pg_sys::GETSTRUCT(proc_tuple) as *mut pg_sys::FormData_pg_proc;
        let name_data = &(*proc_form).proname;

        let name_str = pgrx::name_data_to_str(name_data);

        pg_sys::ReleaseSysCache(proc_tuple);

        // Add (*) for COUNT(*) or star aggregates
        if (*aggref).aggstar {
            format!("{}(*)", name_str.to_uppercase())
        } else {
            name_str.to_uppercase()
        }
    } else {
        "UNKNOWN".to_string()
    }
}

// ----------------------------------------------------------------------------
// ParallelQueryCapable for AggregateScan — delegation surface for the MPP
// lifecycle hooks. `create_custom_path` flags the path parallel-safe via
// `CustomPathBuilder::set_parallel(nworkers)` when `mpp_eligible_for_aggregate()`
// is true; `begin_custom_scan` serializes the logical plan and stashes the
// bytes on `AggregateScanState`; `exec_datafusion_aggregate` routes through
// `mpp::plan_build::build_mpp_aggregate_plan` when `mpp_state` is `Some`.
// ----------------------------------------------------------------------------
impl ParallelQueryCapable for AggregateScan {
    fn estimate_dsm_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        _pcxt: *mut pg_sys::ParallelContext,
    ) -> pg_sys::Size {
        if !crate::postgres::customscan::mpp::customscan_glue::mpp_is_active() {
            return 0;
        }
        let Some(plan_bytes) = state.custom_state().logical_plan_bytes.as_ref() else {
            crate::mpp_log!(
                "mpp: AggregateScan::estimate_dsm_custom_scan but logical_plan_bytes \
                 is None (plan serialization failed earlier?); returning 0"
            );
            return 0;
        };
        let shape = match state.custom_state().mpp_shape {
            Some(s) => s,
            None => {
                crate::mpp_log!("mpp: estimate_dsm but mpp_shape is None; returning 0");
                return 0;
            }
        };
        let num_meshes = crate::postgres::customscan::mpp::shape::shuffles_required(shape);
        if num_meshes == 0 {
            crate::mpp_log!("mpp: shape {:?} requires no shuffle; returning 0", shape);
            return 0;
        }
        match crate::postgres::customscan::mpp::customscan_glue::leader_estimate_dsm(
            plan_bytes.len(),
            num_meshes,
        ) {
            Ok(sz) => sz,
            Err(e) => {
                crate::mpp_log!("mpp: leader_estimate_dsm failed: {e}; returning 0");
                0
            }
        }
    }

    fn initialize_dsm_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        pcxt: *mut pg_sys::ParallelContext,
        coordinate: *mut std::os::raw::c_void,
    ) {
        debug_assert!(state.custom_state().mpp_state.is_none());
        if !crate::postgres::customscan::mpp::customscan_glue::mpp_is_active() {
            return;
        }
        if coordinate.is_null() {
            // Estimate returned 0 (plan bytes unavailable) — PG will pass
            // NULL coordinate. Silently fall back to non-MPP path.
            return;
        }
        let Some(plan_bytes) = state.custom_state().logical_plan_bytes.as_ref() else {
            return;
        };
        let plan_bytes = plan_bytes.to_vec();
        let shape = match state.custom_state().mpp_shape {
            Some(s) => s,
            None => return,
        };
        let num_meshes = crate::postgres::customscan::mpp::shape::shuffles_required(shape);
        if num_meshes == 0 {
            return;
        }
        let seg = unsafe { (*pcxt).seg };
        let ctx = unsafe {
            crate::postgres::customscan::mpp::customscan_glue::leader_init_dsm(
                coordinate, plan_bytes, num_meshes, seg,
            )
        };
        match ctx {
            Ok(leader_ctx) => {
                crate::mpp_log!(
                    "mpp: AggregateScan leader initialized DSM for {} participants",
                    leader_ctx.participant_config().total_participants
                );
                state.custom_state_mut().mpp_state = Some(leader_ctx);
            }
            Err(e) => {
                // DSM init is in the hot path of parallel-query startup —
                // a partial init leaves the region in an undefined state,
                // so ERROR rather than silently degrade.
                pgrx::error!("mpp: leader_init_dsm failed: {e}");
            }
        }
    }

    fn reinitialize_dsm_custom_scan(
        _state: &mut CustomScanStateWrapper<Self>,
        _pcxt: *mut pg_sys::ParallelContext,
        _coordinate: *mut std::os::raw::c_void,
    ) {
        // Rescan is out of scope. A rescanned MPP aggregate would need a
        // full mesh teardown + rebuild.
    }

    fn initialize_worker_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        _toc: *mut pg_sys::shm_toc,
        coordinate: *mut std::os::raw::c_void,
    ) {
        debug_assert!(state.custom_state().mpp_state.is_none());
        if !crate::postgres::customscan::mpp::customscan_glue::mpp_is_active() {
            return;
        }
        if coordinate.is_null() {
            return;
        }

        // Worker-side attach. PG passes `shm_toc` (not a full
        // `ParallelContext`), so we don't have access to `pcxt->seg`
        // directly. We pass NULL for the seg to `worker_init_dsm`:
        // `shm_mq_attach` handles NULL seg by skipping its
        // `on_dsm_detach` callback and letting process-exit clean up
        // the shm_mq handles. Safe for parallel-worker lifetimes where
        // the process dies with the query.
        //
        // For region_total, we read the header's own `region_total`
        // field (the first 8 bytes of the header's final u64). This
        // loses the independence of the check but is the only source
        // available without a seg pointer. A defense-in-depth
        // independent check can land later via `dsm_find_mapping` +
        // `dsm_segment_map_length`.
        let region_total = unsafe {
            let header = std::ptr::read_unaligned(
                coordinate as *const crate::postgres::customscan::mpp::worker::MppDsmHeader,
            );
            header.region_total
        };
        let worker_number = unsafe { pg_sys::ParallelWorkerNumber };
        let ctx = unsafe {
            crate::postgres::customscan::mpp::customscan_glue::worker_init_dsm(
                coordinate,
                region_total,
                worker_number,
                std::ptr::null_mut(), // seg unknown to worker init; shm_mq_attach handles NULL
            )
        };
        match ctx {
            Ok(worker_ctx) => {
                let _ = region_total;
                crate::mpp_log!(
                    "mpp: AggregateScan worker {} attached to DSM ({} participants)",
                    worker_number,
                    worker_ctx.participant_config().total_participants
                );
                state.custom_state_mut().mpp_state = Some(worker_ctx);
            }
            Err(e) => {
                // Worker-side DSM attach failure is fatal for the same
                // reason the leader side is: the worker now has no valid
                // MPP state but the leader expects MPP partials.
                // Aborting here lets the leader's query fail cleanly
                // with a diagnostic rather than silently returning
                // wrong answers.
                pgrx::error!("mpp: worker_init_dsm failed on worker {worker_number}: {e}");
            }
        }
    }
}

/// Eligibility predicate for firing the MPP path from `create_custom_path`.
///
/// Rules:
///   * `paradedb.enable_mpp` ON AND `paradedb.mpp_worker_count` >= 2
///   * Aggregate is over a JOIN (RELOPT_JOINREL), not a single table
///   * Only COUNT / SUM / MIN / MAX / AVG / BOOL_* / STDDEV_* / VAR_*
///     aggregates — no DISTINCT, ARRAY_AGG, STRING_AGG
///   * GROUP BY keys are simple column references
///
/// Kept here so the eligibility rule has one canonical home.
#[allow(dead_code)]
pub fn mpp_eligible_for_aggregate(input_rel_kind: pg_sys::RelOptKind::Type) -> bool {
    if !crate::postgres::customscan::mpp::customscan_glue::mpp_is_active() {
        return false;
    }
    // Milestone 1: only fire MPP on aggregate-over-JOIN. Single-table
    // aggregates already run serially fast enough that the shuffle cost
    // would dominate. RELOPT_OTHER_JOINREL covers partition-wise join
    // rollups — same semantics, just a different RelOptKind tag.
    matches!(
        input_rel_kind,
        pg_sys::RelOptKind::RELOPT_JOINREL | pg_sys::RelOptKind::RELOPT_OTHER_JOINREL
    )
}
