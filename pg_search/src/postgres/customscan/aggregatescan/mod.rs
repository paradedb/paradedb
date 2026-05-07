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

use std::sync::Arc;

use datafusion::execution::SessionStateBuilder;
use datafusion::physical_plan::ExecutionPlan;
use datafusion_distributed::{DistributedExt, SessionStateBuilderExt};

use crate::postgres::customscan::mpp::dsm::MppDsmHeader;
use crate::postgres::customscan::mpp::exec::run_producer_fragment;
use crate::postgres::customscan::mpp::glue::{
    estimate_dsm_size, leader_setup, mpp_is_active, mpp_worker_count, producer_worker_count,
    worker_setup,
};
use crate::postgres::customscan::mpp::runtime::{
    LocalExecWorkerTransport, MppMesh, MppWorkerResolver, ShmMqWorkerTransport,
};
use crate::postgres::customscan::mpp::transport::MppSender;

use crate::api::agg_funcoid;
use crate::api::SortDirection;
use crate::gucs;

use crate::aggregate::{NULL_SENTINEL_MAX, NULL_SENTINEL_MIN};
use crate::api::HashSet;
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
use crate::postgres::customscan::dsm::{
    estimate_dsm_custom_scan, initialize_dsm_custom_scan, initialize_worker_custom_scan,
    reinitialize_dsm_custom_scan, ParallelQueryCapable,
};
use crate::postgres::customscan::exec::{
    begin_custom_scan, end_custom_scan, exec_custom_scan, explain_custom_scan, rescan_custom_scan,
    shutdown_custom_scan,
};
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::hook::query_has_paradedb_agg;
use crate::postgres::customscan::joinscan::scan_state::{build_physical_plan, build_task_context};
use crate::postgres::customscan::limit_offset::LimitOffset;
use crate::postgres::customscan::parallel::list_segment_ids;
use crate::postgres::customscan::projections::{create_placeholder_targetlist, placeholder_procid};
use crate::postgres::customscan::solve_expr::SolvePostgresExpressions;
use crate::postgres::customscan::{range_table, CreateUpperPathsHookArgs, CustomScan};
use crate::postgres::rel_get_bm25_index;
use crate::postgres::types::{is_datetime_type, TantivyValue};
use crate::postgres::utils::{add_vars_to_tlist, is_unnest_func, make_text_const};
use crate::postgres::PgSearchRelation;
use crate::postgres::{ParallelScanArgs, ParallelScanState};
use crate::scan::codec::{
    deserialize_logical_plan_with_runtime, serialize_logical_plan, PgSearchPhysicalCodecStub,
};
use chrono::{DateTime as ChronoDateTime, Utc};
use futures::StreamExt;
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
            BeginCustomScan: Some(begin_custom_scan::<Self>),
            ExecCustomScan: Some(exec_custom_scan::<Self>),
            EndCustomScan: Some(end_custom_scan::<Self>),
            ReScanCustomScan: Some(rescan_custom_scan::<Self>),
            MarkPosCustomScan: None,
            RestrPosCustomScan: None,
            EstimateDSMCustomScan: Some(estimate_dsm_custom_scan::<Self>),
            InitializeDSMCustomScan: Some(initialize_dsm_custom_scan::<Self>),
            ReInitializeDSMCustomScan: Some(reinitialize_dsm_custom_scan::<Self>),
            InitializeWorkerCustomScan: Some(initialize_worker_custom_scan::<Self>),
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
                // `plan.targetlist`. PG's parallel-aggregate machinery will
                // add Partial+Final Aggregate around our Gather; setrefs
                // uses `equal()`-matching to rewire the Aggrefs across the
                // Gather boundary, which only works if our targetlist still
                // carries them. Replacing them with `pdb.agg_fn(...)`
                // placeholders breaks every match and the planner rejects
                // the parallel path, falling back to `Single Copy: true`.
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
                    group_df_indices: Vec::new(),
                    mpp: None,
                    mpp_plan_bytes: None,
                    mpp_n_partitions: 1,
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
        _eflags: i32,
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

/// MPP DSM hook impl. The query's serialized worker-fragment plan bytes are
/// stashed on the customscan state by `begin_custom_scan` so estimate +
/// initialize see the same bytes; without that we'd have to re-build the
/// plan twice. The serialization itself happens via the
/// `PgSearchExtensionCodec` and the DF-D fork's `DistributedCodec` so
/// `NetworkShuffleExec` round-trips through the worker side.
/// DSM layout used by AggregateScan in MPP mode:
///
/// ```text
/// [0 .. 8)                     u64 mpp_offset            (offset to MPP region)
/// [8 .. 16)                    u64 partitioning_source_idx
/// [pscan_offset .. mpp_offset) ParallelScanState (variable size)
/// [mpp_offset .. total)        MPP region (header + queues + plan_bytes)
/// ```
///
/// Workers don't carry the source manifests the leader saw, so the
/// MPP-region offset and the partitioning-source index are stamped into
/// the first 16 bytes by the leader and read back by workers — neither
/// has to be re-derived.
#[repr(C)]
#[derive(Clone, Copy)]
struct MppAggDsmHeader {
    mpp_offset: u64,
    partitioning_source_idx: u64,
}

const MPP_AGG_DSM_HEADER_SIZE: usize = std::mem::size_of::<MppAggDsmHeader>();

fn mpp_align(n: usize) -> usize {
    let a = pg_sys::MAXIMUM_ALIGNOF as usize;
    n.next_multiple_of(a)
}

fn mpp_agg_pscan_offset() -> usize {
    mpp_align(MPP_AGG_DSM_HEADER_SIZE)
}

unsafe fn mpp_agg_read_header(coordinate: *const std::os::raw::c_void) -> MppAggDsmHeader {
    unsafe { *(coordinate as *const MppAggDsmHeader) }
}

unsafe fn mpp_agg_write_header(coordinate: *mut std::os::raw::c_void, header: MppAggDsmHeader) {
    unsafe {
        *(coordinate as *mut MppAggDsmHeader) = header;
    }
}

impl ParallelQueryCapable for AggregateScan {
    fn estimate_dsm_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        _pcxt: *mut pg_sys::ParallelContext,
    ) -> pg_sys::Size {
        let Some(df_state) = state.custom_state().datafusion_state.as_ref() else {
            return 0;
        };
        let Some(plan_bytes) = df_state.mpp_plan_bytes.as_ref() else {
            return 0;
        };
        let n_partitions = df_state.mpp_n_partitions;
        let plan_bytes_len = plan_bytes.len();

        // Capture source manifests so we can size the ParallelScanState block.
        Self::ensure_source_manifests(state);
        let all_nsegments: Vec<usize> = state
            .custom_state()
            .source_manifests
            .iter()
            .map(|m| m.segment_count())
            .collect();
        let partitioning_idx = Self::partitioning_source_idx(state);
        let pscan_size = ParallelScanState::size_of(&all_nsegments, partitioning_idx, &[], false);
        let mpp_offset = mpp_align(mpp_agg_pscan_offset() + pscan_size);

        // Number of non-partitioning sources gets a build-cache slot per worker.
        let n_cache_sources = state
            .custom_state()
            .source_manifests
            .len()
            .saturating_sub(1) as u32;
        // K peer meshes: 1 when post-agg shuffle is on (the post-aggregate
        // peer mesh), 0 otherwise. The runtime substrate is K-aware (Vec<MppPeerMesh>);
        // the planner side currently emits at most 1 nested cross-worker shuffle
        // (the `!has_shuffle_ancestor` guard in fork's `_distribute_plan`).
        let n_peer_meshes: u32 = if crate::gucs::enable_mpp_postagg_shuffle() {
            1
        } else {
            0
        };
        let mpp_size = match crate::postgres::customscan::mpp::glue::estimate_dsm_size(
            plan_bytes_len,
            n_partitions,
            n_cache_sources,
            n_peer_meshes,
        ) {
            Ok(sz) => sz,
            Err(e) => {
                pgrx::warning!("mpp: estimate_dsm failed: {e}; falling back to serial");
                return 0;
            }
        };

        (mpp_offset + mpp_size) as pg_sys::Size
    }

    fn initialize_dsm_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        pcxt: *mut pg_sys::ParallelContext,
        coordinate: *mut std::os::raw::c_void,
    ) {
        let Some(df_state) = state.custom_state_mut().datafusion_state.as_mut() else {
            return;
        };
        let Some(plan_bytes) = df_state.mpp_plan_bytes.take() else {
            return;
        };
        let n_partitions = df_state.mpp_n_partitions;
        let seg = unsafe { (*pcxt).seg };

        // Capture manifests + size the ParallelScanState block.
        Self::ensure_source_manifests(state);
        let partitioning_idx = Self::partitioning_source_idx(state);
        let all_nsegments: Vec<usize> = state
            .custom_state()
            .source_manifests
            .iter()
            .map(|m| m.segment_count())
            .collect();
        let pscan_size = ParallelScanState::size_of(&all_nsegments, partitioning_idx, &[], false);
        let pscan_offset = mpp_agg_pscan_offset();
        let mpp_offset = mpp_align(pscan_offset + pscan_size);

        // Stamp the MPP-region offset and partitioning-source index into
        // the DSM header so workers can skip past the ParallelScanState
        // block and key index_segment_ids the same way as the leader.
        unsafe {
            mpp_agg_write_header(
                coordinate,
                MppAggDsmHeader {
                    mpp_offset: mpp_offset as u64,
                    partitioning_source_idx: partitioning_idx as u64,
                },
            )
        };

        // Init the ParallelScanState at `pscan_offset`.
        let pscan_state =
            unsafe { (coordinate as *mut u8).add(pscan_offset) as *mut ParallelScanState };
        assert!(!pscan_state.is_null(), "MPP DSM coordinate is null");
        unsafe {
            let all_sources: Vec<&[tantivy::SegmentReader]> = state
                .custom_state()
                .source_manifests
                .iter()
                .map(|m| m.segment_readers())
                .collect();
            (*pscan_state).create_and_populate(ParallelScanArgs {
                all_sources,
                partitioning_source_idx: partitioning_idx,
                query: vec![],
                with_aggregates: false,
            });
        }
        let non_partitioning_segments = unsafe { (*pscan_state).non_partitioning_segment_ids() };
        state.custom_state_mut().parallel_state = Some(pscan_state);
        state.custom_state_mut().non_partitioning_segments = non_partitioning_segments;
        state.custom_state_mut().mpp_partitioning_source_idx = Some(partitioning_idx);

        // Init the MPP region.
        let mpp_coordinate =
            unsafe { (coordinate as *mut u8).add(mpp_offset) as *mut std::os::raw::c_void };
        let n_cache_sources = state
            .custom_state()
            .source_manifests
            .len()
            .saturating_sub(1) as u32;
        let n_peer_meshes: u32 = if crate::gucs::enable_mpp_postagg_shuffle() {
            1
        } else {
            0
        };
        let leader = match unsafe {
            leader_setup(
                mpp_coordinate,
                seg,
                n_partitions,
                plan_bytes,
                n_cache_sources,
                n_peer_meshes,
            )
        } {
            Ok(l) => l,
            Err(e) => {
                pgrx::warning!("mpp: leader_setup failed: {e}; falling back to serial");
                return;
            }
        };
        let df_state = state
            .custom_state_mut()
            .datafusion_state
            .as_mut()
            .expect("datafusion_state must still be set");
        df_state.mpp = Some(scan_state::MppExecState::Leader(leader));
    }

    fn reinitialize_dsm_custom_scan(
        _state: &mut CustomScanStateWrapper<Self>,
        _pcxt: *mut pg_sys::ParallelContext,
        coordinate: *mut std::os::raw::c_void,
    ) {
        // Reset the ParallelScanState header so a re-execution re-claims segments.
        let pscan_offset = mpp_agg_pscan_offset();
        let pscan_state =
            unsafe { (coordinate as *mut u8).add(pscan_offset) as *mut ParallelScanState };
        if !pscan_state.is_null() {
            unsafe { (*pscan_state).reset() };
        }
    }

    fn initialize_worker_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        _toc: *mut pg_sys::shm_toc,
        coordinate: *mut std::os::raw::c_void,
    ) {
        if state.custom_state().datafusion_state.is_none() {
            return;
        }

        // Read the MPP-region offset + partitioning-source index from the
        // DSM header.
        let header = unsafe { mpp_agg_read_header(coordinate) };
        let mpp_offset = header.mpp_offset as usize;
        let pscan_offset = mpp_agg_pscan_offset();
        state.custom_state_mut().mpp_partitioning_source_idx =
            Some(header.partitioning_source_idx as usize);

        // Attach to the ParallelScanState and wait for the leader to
        // populate it before reading the canonical segment list.
        let pscan_state =
            unsafe { (coordinate as *mut u8).add(pscan_offset) as *mut ParallelScanState };
        assert!(!pscan_state.is_null(), "MPP DSM coordinate is null");
        unsafe { (*pscan_state).wait_for_initialization() };
        let non_partitioning_segments = unsafe { (*pscan_state).non_partitioning_segment_ids() };
        state.custom_state_mut().parallel_state = Some(pscan_state);
        state.custom_state_mut().non_partitioning_segments = non_partitioning_segments;

        // Attach to the MPP region.
        let mpp_coordinate =
            unsafe { (coordinate as *mut u8).add(mpp_offset) as *mut std::os::raw::c_void };
        let region_total = unsafe { (*mpp_coordinate.cast::<MppDsmHeader>()).region_total };
        let worker_number = unsafe { pg_sys::ParallelWorkerNumber };
        let worker = match unsafe {
            worker_setup(
                mpp_coordinate,
                region_total,
                worker_number,
                std::ptr::null_mut(),
            )
        } {
            Ok(w) => w,
            Err(e) => {
                pgrx::warning!("mpp: worker_setup failed: {e}; falling back to serial");
                return;
            }
        };
        let df_state = state
            .custom_state_mut()
            .datafusion_state
            .as_mut()
            .expect("checked above");
        df_state.mpp = Some(scan_state::MppExecState::Worker(worker));
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
        // Each producer writes `target_partitions` partitions per worker.
        // The DF-D fork's `_distribute_plan` is patched so that in in-process
        // mode (custom WorkerTransport registered) it clamps the Shuffle's
        // consumer_task_count to 1, which disables `NetworkShuffleExec`'s
        // per-task hash scaling. So producer output = target_partitions =
        // n_workers (matching `build_mpp_leader_session_context`).
        let n_workers = producer_worker_count();
        df_state.mpp_n_partitions = n_workers.max(1);
    }

    /// MPP leader exec helper: build a `SessionContext` that mirrors
    /// `create_aggregate_session_context`'s rules + the DF-D fork's
    /// `with_distributed_planner` + our `ShmMqWorkerTransport` wired to the
    /// runtime mesh. The resulting context, when used to
    /// `create_physical_plan` over the leader's logical plan, returns a
    /// `DistributedExec` whose `NetworkShuffleExec`s pull batches from the
    /// shm_mq mesh at execute time.
    fn build_mpp_leader_session_context(mesh: Arc<MppMesh>) -> datafusion::prelude::SessionContext {
        let serial = create_aggregate_session_context();
        let n_workers = mesh.n_workers as usize;
        // Four-knob unlock for actually inserting NetworkShuffleExec/etc.:
        //   1. target_partitions(N) — without this, EnforceDistribution skips
        //      every RepartitionExec, so the annotator never sees a Shuffle.
        //   2. distributed_task_estimator(N) — without this, leaves default to
        //      Maximum(1) and `_distribute_plan` elides every shuffle.
        //   3. distributed_broadcast_joins(true) — CollectLeft HashJoins
        //      otherwise cap their stage's task_count to Maximum(1) and
        //      propagate that cap upward, eliding shuffles above the join.
        //   4. distributed_user_codec — the DF-D fork's prepare_plan unconditionally
        //      encodes worker subplans for gRPC shipment; without a codec for
        //      our custom physical execs, encoding errors before execution.
        //      In our model the encoded bytes are never observed (workers
        //      re-plan from the logical plan in DSM), so the codec is a stub.
        let cfg = serial
            .copied_config()
            .with_target_partitions(n_workers.max(2));

        let state_builder = SessionStateBuilder::new()
            .with_default_features()
            .with_config(cfg)
            .with_distributed_worker_resolver(MppWorkerResolver::new(n_workers))
            .with_distributed_worker_transport(ShmMqWorkerTransport::new(mesh))
            .with_distributed_in_process_mode(true)
            .expect("with_distributed_in_process_mode")
            .with_distributed_task_estimator(n_workers)
            .with_distributed_broadcast_joins(true)
            .expect("with_distributed_broadcast_joins")
            .with_distributed_in_process_peer_shuffle(crate::gucs::enable_mpp_postagg_shuffle())
            .expect("with_distributed_in_process_peer_shuffle")
            .with_distributed_user_codec(crate::scan::codec::PgSearchPhysicalCodecStub)
            .with_distributed_planner();
        datafusion::prelude::SessionContext::new_with_state(state_builder.build())
    }

    /// MPP worker exec: deserialize the logical plan from DSM, build the
    /// distributed physical plan (matching what the leader produced), find
    /// the worker fragment (the `input_stage.plan` of the bottom
    /// `NetworkShuffleExec`), and run it via
    /// [`mpp::exec::run_producer_fragment`] which pushes every output batch
    /// to the leader's shm_mq queues. Workers emit zero rows back to PG;
    /// returning `null_mut()` signals end-of-stream.
    fn exec_mpp_worker(state: &mut CustomScanStateWrapper<Self>) -> *mut pg_sys::TupleTableSlot {
        // Pull worker-thread inputs from the outer state before we borrow
        // df_state mutably. parallel_state and non_partitioning_segments
        // are required to pin each worker's PgSearchTableProvider to the
        // right segment slice (the partitioning source) and the leader's
        // canonical replica (the non-partitioning sources). Without them,
        // every worker re-scans the full data and the leader-side hash
        // partitions get the same rows from every worker.
        let parallel_state = state.custom_state().parallel_state;
        let non_partitioning_segments = state.custom_state().non_partitioning_segments.clone();
        let partitioning_source_idx = state
            .custom_state()
            .mpp_partitioning_source_idx
            .unwrap_or(0);
        let plan_sources_count = state
            .custom_state()
            .source_manifests
            .len()
            .max(non_partitioning_segments.len() + 1);

        let df_state = state
            .custom_state_mut()
            .datafusion_state
            .as_mut()
            .expect("DataFusion state must be initialized");

        if df_state.runtime.is_some() {
            // Already drained on a prior call; just signal EOF.
            return std::ptr::null_mut();
        }
        let scan_state::MppExecState::Worker(worker) = df_state.mpp.as_ref().expect("checked")
        else {
            unreachable!("exec_mpp_worker called outside Worker state");
        };
        let plan_bytes = worker.plan_bytes.clone();
        // Worker count from the participant config (matches the leader's
        // mesh.n_workers). `outbound_senders.len()` gives partitions-per-
        // producer (= mpp_n_partitions), not the worker count — using it as
        // n_workers misconfigured the planner so NetworkShuffleExec scaled
        // its hash to mpp_n_partitions × mpp_n_partitions = 81 partitions.
        let n_workers = worker.participant_config.total_participants;
        let worker_idx_for_cache = worker.participant_config.participant_index;
        let outbound_senders: Vec<crate::postgres::customscan::mpp::transport::MppSender> =
            match df_state.mpp.as_mut() {
                Some(scan_state::MppExecState::Worker(w)) => {
                    std::mem::take(&mut w.outbound_senders)
                }
                _ => unreachable!(),
            };
        let peer_meshes: Vec<Arc<crate::postgres::customscan::mpp::mesh::MppPeerMesh>> =
            match df_state.mpp.as_ref() {
                Some(scan_state::MppExecState::Worker(w)) => w.peer_meshes.clone(),
                _ => Vec::new(),
            };

        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => pgrx::error!("mpp worker: tokio runtime build failed: {e}"),
        };
        df_state.runtime = Some(runtime);
        let runtime = df_state.runtime.as_ref().unwrap();
        // Use a bare SessionContext for plan deserialization; the workers
        // need the same `with_distributed_planner` config the leader had so
        // the resulting physical plan exposes the matching NetworkShuffleExec.
        let ctx = create_aggregate_session_context();

        // Build per-source canonical segment ID sets. For the partitioning
        // source, pull the full list out of the populated ParallelScanState
        // (workers will then claim individual segments via `checkout_segment`
        // inside their `PgSearchTableProvider`). For non-partitioning sources,
        // use the segment IDs the leader snapshotted into shared memory.
        let mut index_segment_ids: Vec<HashSet<tantivy::index::SegmentId>> =
            vec![HashSet::default(); plan_sources_count];
        if let Some(ps) = parallel_state {
            let mut np_counter = 0usize;
            for (i, slot) in index_segment_ids.iter_mut().enumerate() {
                if i == partitioning_source_idx {
                    *slot = unsafe { list_segment_ids(ps) };
                } else if let Some(ids) = non_partitioning_segments.get(np_counter) {
                    *slot = ids.clone();
                    np_counter += 1;
                }
            }
        }

        let mpp_build_cache = match df_state.mpp.as_ref() {
            Some(scan_state::MppExecState::Worker(w)) => w.build_cache.as_ref().map(Arc::clone),
            _ => None,
        };
        let logical = match deserialize_logical_plan_with_runtime(
            &plan_bytes,
            &ctx.task_ctx(),
            parallel_state,
            None, // expr_context: bm25 search predicates don't need runtime params
            None, // planstate: same
            non_partitioning_segments,
            index_segment_ids,
            mpp_build_cache,
            worker_idx_for_cache,
        ) {
            Ok(lp) => lp,
            Err(e) => pgrx::error!("mpp worker: deserialize_logical_plan failed: {e}"),
        };

        // Build state with the DF-D fork's distributed planner so create_physical_plan
        // produces a DistributedExec wrapping NetworkShuffleExec. The
        // target_partitions, task-estimator, broadcast-joins and codec
        // overrides must mirror the leader (see
        // `build_mpp_leader_session_context`); otherwise the worker re-plans
        // a different shape and `find_worker_fragment` returns None.
        let n_workers_us = n_workers as usize;
        // Stamp DistributedTaskContext { task_index, task_count } so the fork's
        // NetworkShuffleExec.execute computes per-task partition slices correctly.
        // Without this, every worker reads off=0 and produces all N global
        // partitions itself, so workers send duplicate data to the leader.
        let task_ctx_ext = std::sync::Arc::new(datafusion_distributed::DistributedTaskContext {
            task_index: worker_idx_for_cache as usize,
            task_count: n_workers_us,
        });
        let cfg = ctx
            .copied_config()
            .with_target_partitions(n_workers_us.max(2))
            .with_extension(task_ctx_ext);
        // Shared stage→peer-mesh routing map for the worker's
        // `CompositeWorkerTransport`. Empty until populated below.
        let shared_routes =
            crate::postgres::customscan::mpp::runtime::CompositeWorkerTransport::empty_routes();
        use datafusion::execution::SessionStateBuilder;
        use datafusion_distributed::SessionStateBuilderExt;
        let state_builder = SessionStateBuilder::new()
            .with_default_features()
            .with_config(cfg)
            .with_distributed_worker_resolver(
                crate::postgres::customscan::mpp::runtime::MppWorkerResolver::new(n_workers_us),
            )
            // Workers may encounter two kinds of nested network boundaries
            // inside the producer fragment:
            //   1. `NetworkBroadcastExec` on the build side of a HashJoin —
            //      input_stage.tasks.len() == 1, no peer mesh needed; the
            //      composite routes to `LocalExecWorkerTransport` which
            //      re-executes the build subtree locally.
            //   2. `NetworkShuffleExec` for the post-agg peer mesh (Track B,
            //      gated by `enable_mpp_postagg_shuffle`) — input_stage.tasks
            //      .len() == n_workers > 1; the composite routes to
            //      `ShmMqPeerWorkerTransport` which streams from the peer
            //      mesh column for this worker.
            // When the GUC is off (or the planner emits no nested
            // cross-worker shuffles) the shared map stays empty and every
            // `NetworkShuffleExec` falls through to `LocalExecWorkerTransport`.
            // The map is populated below, AFTER the physical plan is built,
            // by walking the worker fragment for nested `NetworkShuffleExec`
            // stage IDs and zipping with the worker's `peer_meshes` Vec.
            .with_distributed_worker_transport(
                crate::postgres::customscan::mpp::runtime::CompositeWorkerTransport::new(
                    Arc::clone(&shared_routes),
                ),
            )
            .with_distributed_in_process_mode(true)
            .expect("with_distributed_in_process_mode")
            .with_distributed_task_estimator(n_workers_us)
            .with_distributed_broadcast_joins(true)
            .expect("with_distributed_broadcast_joins")
            .with_distributed_in_process_peer_shuffle(crate::gucs::enable_mpp_postagg_shuffle())
            .expect("with_distributed_in_process_peer_shuffle")
            .with_distributed_user_codec(crate::scan::codec::PgSearchPhysicalCodecStub)
            .with_distributed_planner();
        let session_state = state_builder.build();
        let session = datafusion::prelude::SessionContext::new_with_state(session_state);

        let physical_plan =
            runtime.block_on(async { session.state().create_physical_plan(&logical).await });
        let physical_plan = match physical_plan {
            Ok(p) => p,
            Err(e) => pgrx::error!("mpp worker: create_physical_plan failed: {e}"),
        };
        if crate::gucs::mpp_debug() && worker_idx_for_cache == 0 {
            let dumped = datafusion::physical_plan::displayable(physical_plan.as_ref())
                .indent(true)
                .to_string();
            pgrx::warning!("mpp worker[0] physical plan:\n{dumped}");
        }
        // Find the bottom NetworkShuffleExec; its input_stage.plan (==
        // children()[0]) is the worker fragment. If the DF-D fork's planner
        // didn't insert one (some plan shapes don't benefit from a network
        // shuffle, or PartialReduce isn't enabled), the worker has no
        // fragment to run — emit zero rows and let the leader produce
        // results via its own copy. Logging at WARNING so production
        // monitors can spot the falls-back-to-no-op case.
        let fragment = match Self::find_worker_fragment(&physical_plan) {
            Some(f) => f,
            None => {
                pgrx::warning!(
                    "mpp worker: no NetworkShuffleExec found in distributed plan; \
                     skipping producer-fragment run (worker emits zero rows)"
                );
                return std::ptr::null_mut();
            }
        };

        // Populate the shared stage→peer-mesh routing map now that the
        // physical plan is in hand. Walk the worker fragment for nested
        // `NetworkShuffleExec` stage IDs (deterministic across leader and
        // worker), zip with the leader's allocated `peer_meshes` Vec.
        // Mismatch is a hard error since it indicates the leader and
        // worker disagree on plan shape.
        let inner_stage_ids =
            crate::postgres::customscan::mpp::runtime::collect_inner_shuffle_stage_ids(&fragment);
        match crate::postgres::customscan::mpp::runtime::build_stage_to_mesh_map(
            &inner_stage_ids,
            &peer_meshes,
        ) {
            Ok(map) => {
                *shared_routes.write() = map;
            }
            Err(e) => {
                pgrx::error!("mpp worker: failed to build stage→mesh routing: {e}");
            }
        }
        let task_ctx = build_task_context(
            &session,
            &fragment,
            unsafe { pg_sys::work_mem as usize * 1024 },
            unsafe { pg_sys::hash_mem_multiplier },
        );

        // Two-boundary mode (Track A + Track B): spawn the inner peer-mesh
        // producer fragment alongside the outer worker→leader fragment.
        // The inner fragment pushes hash-partitioned partial-agg rows into
        // peer_outbound (tagged by partition_id); the outer fragment runs
        // FinalPartitioned by pulling from the peer mesh and pushes the
        // final-aggregated rows into the leader-bound mesh.
        //
        // For now (with the fork's `!has_shuffle_ancestor` guard) at most
        // one peer-mesh shuffle is in play. G3 will replace this with K
        // concurrent inner-fragment runners.
        let inner_fragment =
            if !peer_meshes.is_empty() && Self::count_network_shuffles(&physical_plan) >= 2 {
                Self::find_inner_producer_fragment(&physical_plan)
            } else {
                None
            };

        let result = runtime.block_on(async {
            match (inner_fragment, peer_meshes.first()) {
                (Some(inner), Some(mesh)) => {
                    let peer_outbound = mesh.take_outbound().ok_or_else(|| {
                        datafusion::common::DataFusionError::Internal(
                            "mpp worker: peer mesh outbound senders already taken".into(),
                        )
                    })?;
                    let n_consumers = mesh.n_workers as usize;
                    let inner_task_ctx = build_task_context(
                        &session,
                        &inner,
                        unsafe { pg_sys::work_mem as usize * 1024 },
                        unsafe { pg_sys::hash_mem_multiplier },
                    );
                    // Run both fragments concurrently on the worker's
                    // current_thread Tokio runtime. The cooperative drains
                    // interleave correctly: when the outer fragment awaits
                    // peer-mesh data, the runtime polls the inner fragment
                    // which pushes a batch, which then unblocks the outer
                    // fragment's pull.
                    let inner_fut =
                        crate::postgres::customscan::mpp::exec::run_inner_producer_fragment(
                            inner,
                            peer_outbound,
                            n_consumers,
                            inner_task_ctx,
                        );
                    let outer_fut = crate::postgres::customscan::mpp::exec::run_producer_fragment(
                        fragment,
                        outbound_senders,
                        task_ctx,
                    );
                    let (inner_res, outer_res) = futures::join!(inner_fut, outer_fut);
                    inner_res?;
                    outer_res
                }
                _ => {
                    crate::postgres::customscan::mpp::exec::run_producer_fragment(
                        fragment,
                        outbound_senders,
                        task_ctx,
                    )
                    .await
                }
            }
        });
        if let Err(e) = result {
            pgrx::error!("mpp worker: producer fragment(s) failed: {e}");
        }
        std::ptr::null_mut()
    }

    /// Walk a DistributedExec-rooted plan and return the input subtree of
    /// the **topmost** `NetworkShuffleExec` (the worker producer fragment).
    /// Returns `None` if no `NetworkShuffleExec` is reachable from the root.
    ///
    /// Pre-order traversal returns on the first match, which is the
    /// outermost shuffle — the one that straddles the worker→leader split.
    fn find_worker_fragment(
        plan: &Arc<dyn datafusion::physical_plan::ExecutionPlan>,
    ) -> Option<Arc<dyn datafusion::physical_plan::ExecutionPlan>> {
        if plan.name() == "NetworkShuffleExec" {
            return plan.children().first().map(|c| Arc::clone(c));
        }
        for child in plan.children() {
            if let Some(found) = Self::find_worker_fragment(child) {
                return Some(found);
            }
        }
        None
    }

    /// Walk a DistributedExec-rooted plan and return the input subtree of
    /// the **bottommost** `NetworkShuffleExec` (the inner peer-mesh
    /// producer fragment for two-boundary plans).
    ///
    /// In single-boundary mode (one shuffle) this returns the same subtree
    /// as `find_worker_fragment`. In two-boundary mode (Track A's gather +
    /// peer-mesh shuffle) this returns the input of the inner shuffle —
    /// `RepartitionExec(Hash, N²) → AggregateExec(Partial) → ...` — which
    /// is the data each worker pushes to the peer mesh.
    fn find_inner_producer_fragment(
        plan: &Arc<dyn datafusion::physical_plan::ExecutionPlan>,
    ) -> Option<Arc<dyn datafusion::physical_plan::ExecutionPlan>> {
        // Recurse first so the deepest `NetworkShuffleExec` wins (post-order).
        for child in plan.children() {
            if let Some(found) = Self::find_inner_producer_fragment(child) {
                return Some(found);
            }
        }
        if plan.name() == "NetworkShuffleExec" {
            return plan.children().first().map(|c| Arc::clone(c));
        }
        None
    }

    /// Identifier for the boundary kinds currently emitted by the fork's
    /// `_distribute_plan`. Used to disambiguate single-boundary vs
    /// two-boundary plans when deciding whether to spawn the inner
    /// peer-mesh producer fragment.
    fn count_network_shuffles(plan: &Arc<dyn datafusion::physical_plan::ExecutionPlan>) -> usize {
        let mut count = if plan.name() == "NetworkShuffleExec" {
            1
        } else {
            0
        };
        for child in plan.children() {
            count += Self::count_network_shuffles(child);
        }
        count
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

        // Activate MPP at planning time if the GUC is on. Must be set on
        // the builder *before* `.build()` — PG's path-builder freezes the
        // parallel flags at build time, and setting them after produces a
        // `Single Copy: true` Gather where the customscan never actually
        // runs in multiple workers.
        let builder = if mpp_is_active() {
            let n_workers = mpp_worker_count();
            let workers_to_launch = (n_workers.saturating_sub(1) as usize).max(1);
            builder.set_parallel(workers_to_launch)
        } else {
            builder
        };

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
        // Grab the scan_slot pointer before entering the mutable borrow
        let scan_slot = state
            .custom_state()
            .scan_slot
            .expect("scan_slot must be initialized in begin_custom_scan");

        let df_state = state
            .custom_state_mut()
            .datafusion_state
            .as_mut()
            .expect("DataFusion state must be initialized");

        // MPP worker fast-path: run the producer fragment to exhaustion
        // (pushing every batch into the leader's shm_mq mesh) on the first
        // call, then return null forever. Workers emit zero rows back to
        // PG; the leader assembles the result via the consumer plan.
        if let Some(scan_state::MppExecState::Worker(_)) = &df_state.mpp {
            return Self::exec_mpp_worker(state);
        }

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
                Some(scan_state::MppExecState::Leader(leader)) => {
                    Self::build_mpp_leader_session_context(Arc::clone(&leader.mesh))
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
            let stream = {
                let _guard = runtime.enter();
                match physical_plan.execute(0, task_ctx) {
                    Ok(s) => s,
                    Err(e) => pgrx::error!("Failed to execute DataFusion aggregate plan: {}", e),
                }
            };

            df_state.runtime = Some(runtime);
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
            let runtime = df_state.runtime.as_ref().unwrap();
            let stream = df_state.stream.as_mut().unwrap();

            let next = runtime.block_on(async { stream.next().await });

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
