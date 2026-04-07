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
};
use crate::postgres::customscan::aggregatescan::datafusion_project::project_aggregate_row_to_slot;
use crate::postgres::customscan::aggregatescan::exec::aggregation_results_iter;
use crate::postgres::customscan::aggregatescan::groupby::GroupByClause;
use crate::postgres::customscan::aggregatescan::join_targetlist::extract_aggregate_targetlist;
use crate::postgres::customscan::aggregatescan::privdat::PrivateData;
use crate::postgres::customscan::aggregatescan::scan_state::{AggregateScanState, ExecutionState};
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::builders::custom_state::{
    CustomScanStateBuilder, CustomScanStateWrapper,
};
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::joinscan::memory::create_memory_pool;
use crate::postgres::customscan::joinscan::scan_state::build_physical_plan;
use crate::postgres::customscan::limit_offset::LimitOffset;
use crate::postgres::customscan::projections::{create_placeholder_targetlist, placeholder_procid};
use crate::postgres::customscan::solve_expr::SolvePostgresExpressions;
use crate::postgres::customscan::{range_table, CreateUpperPathsHookArgs, CustomScan};
use crate::postgres::rel_get_bm25_index;
use crate::postgres::types::{is_datetime_type, TantivyValue};
use crate::postgres::utils::{is_unnest_func, make_text_const};
use crate::postgres::PgSearchRelation;
use chrono::{DateTime as ChronoDateTime, Utc};
use datafusion::execution::runtime_env::RuntimeEnvBuilder;
use datafusion::execution::TaskContext;
use pgrx::{pg_sys, PgList, PgMemoryContexts, PgTupleDesc};
use std::ffi::CStr;
use std::sync::Arc;
use tantivy::schema::OwnedValue;

#[derive(Default)]
pub struct AggregateScan;

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
            EstimateDSMCustomScan: None,
            InitializeDSMCustomScan: None,
            ReInitializeDSMCustomScan: None,
            InitializeWorkerCustomScan: None,
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
        match builder.custom_private() {
            PrivateData::Tantivy {
                heap_rti,
                aggregate_clause,
                ..
            } => {
                let heap_rti = *heap_rti;
                let should_replace = aggregate_clause.planner_should_replace_aggrefs();
                builder.set_scanrelid(heap_rti);
                if should_replace {
                    unsafe {
                        let mut cscan = builder.build();
                        let plan = &mut cscan.scan.plan;
                        replace_aggrefs_in_target_list(plan);
                        cscan
                    }
                } else {
                    builder.build()
                }
            }
            PrivateData::DataFusion { .. } => {
                // For join aggregates, scanrelid=0 (no single base relation)
                builder.set_scanrelid(0);

                // Check if the query has pathkeys (ORDER BY) before consuming builder.
                let root = builder.args().root;
                let has_pathkeys = unsafe {
                    !(*root).query_pathkeys.is_null()
                        && pg_sys::list_length((*root).query_pathkeys) > 0
                };

                unsafe {
                    let mut cscan = builder.build();

                    // Set custom_scan_tlist so Postgres can resolve variable references
                    // when Sort/Limit nodes are placed above this scanrelid=0 CustomScan.
                    // This is a copy of the original targetlist (with Aggrefs intact) —
                    // setrefs.c uses it to create INDEX_VAR references in parent nodes.
                    let original_tlist = cscan.scan.plan.targetlist;
                    cscan.custom_scan_tlist =
                        pg_sys::copyObjectImpl(original_tlist.cast()).cast::<pg_sys::List>();

                    if !has_pathkeys {
                        // No ORDER BY: safe to replace Aggrefs at plan time.
                        let plan = &mut cscan.scan.plan;
                        replace_aggrefs_in_target_list(plan);
                    }
                    // When has_pathkeys: aggrefs stay in plan.targetlist so Postgres's
                    // make_sort_from_pathkeys can find them. Replacement is deferred to
                    // create_custom_scan_state (execution time).
                    cscan
                }
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
            } => {
                // Replace Aggrefs for DataFusion path too
                unsafe {
                    let cscan = builder.args().cscan;
                    let pg_plan = &mut (*cscan).scan.plan;
                    replace_aggrefs_in_target_list(pg_plan);
                }
                builder.custom_state().datafusion_state = Some(scan_state::DataFusionAggState {
                    plan,
                    targetlist,
                    topk,
                    join_level_predicates,
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

                // Show aggregates
                let aggs: Vec<String> = df_state
                    .targetlist
                    .aggregates
                    .iter()
                    .map(|a| {
                        if a.field_refs.is_empty() {
                            format!("{}(*)", a.agg_kind)
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
                state.custom_state_mut().placeholder_targetlist = Some(placeholder_tlist);
                state.custom_state_mut().const_agg_nodes = const_nodes;
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
        // DataFusion backend: consume Arrow RecordBatches
        if state.custom_state().is_datafusion_backend() {
            return Self::exec_datafusion_aggregate(state);
        }

        // Tantivy backend: existing path
        let next = match &mut state.custom_state_mut().state {
            ExecutionState::Completed => {
                return std::ptr::null_mut();
            }
            ExecutionState::NotStarted => {
                // Execute the aggregate, and change the state to Emitting.
                let mut row_iter = aggregation_results_iter(state);
                let next = row_iter.next();
                state.custom_state_mut().state = ExecutionState::Emitting(row_iter);
                next
            }
            ExecutionState::Emitting(row_iter) => {
                // Emit the next row.
                row_iter.next()
            }
        };

        let Some(row) = next else {
            state.custom_state_mut().state = ExecutionState::Completed;
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

            let natts = (*(*slot).tts_tupleDescriptor).natts as usize;
            let datums = std::slice::from_raw_parts_mut((*slot).tts_values, natts);
            let isnull = std::slice::from_raw_parts_mut((*slot).tts_isnull, natts);

            let mut aggregates = row.aggregates.clone().into_iter();
            let mut natts_processed = 0;

            // Fill in values according to the target list
            for (i, entry) in state.custom_state().aggregate_clause.entries().enumerate() {
                let attr = tupdesc.get(i).expect("missing attribute");
                let expected_typoid = attr.type_oid().value();

                let datum = match (entry, row.is_empty()) {
                    (TargetListEntry::GroupingColumn(gc_idx), false) => {
                        let key = row.group_keys[*gc_idx].clone();
                        // Check if this is a NULL sentinel (handles both MIN and MAX sentinels).
                        // U64 uses string sentinel for MIN (since 0 is valid); u64::MAX for MAX.
                        // Bool uses string sentinels for both MIN and MAX.
                        // DateTime columns don't have a missing sentinel (NULLs are excluded).
                        let is_datetime = is_datetime_type(expected_typoid);
                        let is_null_sentinel = match &key.0 {
                            OwnedValue::Str(s) => s == NULL_SENTINEL_MIN || s == NULL_SENTINEL_MAX,
                            OwnedValue::I64(v) => *v == i64::MAX || *v == i64::MIN,
                            OwnedValue::U64(v) => *v == u64::MAX,
                            OwnedValue::F64(v) => *v == f64::MAX || *v == f64::MIN,
                            _ => false,
                        };
                        if is_null_sentinel {
                            None
                        } else if is_datetime {
                            // For datetime types, Tantivy's terms aggregation returns the date as
                            // an ISO 8601 string (e.g., "2025-12-26T00:00:00Z"). We need to parse
                            // this string and convert it to the appropriate PostgreSQL date type.
                            match &key.0 {
                                OwnedValue::Str(date_str) => {
                                    // Parse ISO 8601 datetime string using chrono
                                    match date_str.parse::<ChronoDateTime<Utc>>() {
                                        Ok(chrono_dt) => {
                                            // Convert to nanoseconds since epoch for Tantivy DateTime
                                            let nanos =
                                                chrono_dt.timestamp_nanos_opt().unwrap_or(0);
                                            let datetime =
                                                tantivy::DateTime::from_timestamp_nanos(nanos);
                                            TantivyValue(OwnedValue::Date(datetime))
                                                .try_into_datum(pgrx::PgOid::from(expected_typoid))
                                                .expect(
                                                    "should be able to convert datetime to datum",
                                                )
                                        }
                                        Err(e) => {
                                            pgrx::error!(
                                                "Failed to parse datetime string '{}': {}",
                                                date_str,
                                                e
                                            );
                                        }
                                    }
                                }
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
                        } else {
                            key.try_into_datum(pgrx::PgOid::from(expected_typoid))
                                .expect("should be able to convert to datum")
                        }
                    }
                    (TargetListEntry::GroupingColumn(_), true) => None,
                    (TargetListEntry::Aggregate(agg_type), false) => {
                        if agg_type.can_use_doc_count()
                            && !state.custom_state().aggregate_clause.has_filter()
                            && state.custom_state().aggregate_clause.has_groupby()
                        {
                            row.doc_count()
                                .try_into_datum(pgrx::PgOid::from(expected_typoid))
                                .expect("should be able to convert to datum")
                        } else {
                            exec::aggregate_result_to_datum(
                                aggregates.next().and_then(|v| v),
                                agg_type,
                                expected_typoid,
                            )
                        }
                    }
                    (TargetListEntry::Aggregate(agg_type), true) => {
                        agg_type.nullish().value.and_then(|value| {
                            TantivyValue(OwnedValue::F64(value))
                                .try_into_datum(expected_typoid.into())
                                .unwrap()
                        })
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

            // Simple finalization - just set the flags and return the slot (no ExecStoreVirtualTuple needed)
            // Note: We don't set TTS_FLAG_SHOULDFREE since we're reusing this slot across rows
            (*slot).tts_flags &= !(pg_sys::TTS_FLAG_EMPTY as u16);
            (*slot).tts_nvalid = natts as i16;

            // If we have wrapped aggregates, project the expressions using basescan pattern:
            // 1. Mutate Const nodes with actual aggregate values (directly, not from slot)
            // 2. Build projection in per-tuple memory context (bakes Const values in)
            // 3. ExecProject
            if let Some(placeholder_tlist) = state.custom_state().placeholder_targetlist {
                let planstate = state.planstate();
                let expr_context = (*planstate).ps_ExprContext;

                // Switch to per-tuple memory context and reset it to avoid memory leaks
                // from ExecBuildProjectionInfo allocations and wrapper functions
                let mut per_tuple_context =
                    PgMemoryContexts::For((*expr_context).ecxt_per_tuple_memory);
                per_tuple_context.reset();

                // Mutate Const nodes with values directly from the row results.
                // We DON'T use the slot's datums for aggregates because those were converted
                // using the output tuple descriptor's types (e.g., TEXT for jsonb_pretty output),
                // but we need the native aggregate type (e.g., JSONB for pdb.agg).
                // This matches basescan's approach of setting Const values directly.
                let mut agg_iter = row.aggregates.iter();
                for (i, entry) in state.custom_state().aggregate_clause.entries().enumerate() {
                    let Some(const_node) = state
                        .custom_state()
                        .const_agg_nodes
                        .get(i)
                        .copied()
                        .flatten()
                    else {
                        // No Const node for this entry, skip the aggregate iterator if it's an aggregate
                        if matches!(entry, TargetListEntry::Aggregate(_)) {
                            agg_iter.next();
                        }
                        continue;
                    };

                    let (datum, is_null) = match entry {
                        TargetListEntry::Aggregate(agg_type) => {
                            // Get the next aggregate result
                            let agg_result = agg_iter.next().and_then(|v| v.clone());

                            // Convert to datum using the Const node's type (native aggregate type)
                            // not the output tuple descriptor's type
                            if row.is_empty() {
                                // Empty result - use nullish value
                                let nullish_datum = agg_type.nullish().value.and_then(|value| {
                                    TantivyValue(OwnedValue::F64(value))
                                        .try_into_datum((*const_node).consttype.into())
                                        .unwrap()
                                });
                                (
                                    nullish_datum.unwrap_or(pg_sys::Datum::null()),
                                    nullish_datum.is_none(),
                                )
                            } else if agg_type.can_use_doc_count()
                                && !state.custom_state().aggregate_clause.has_filter()
                                && state.custom_state().aggregate_clause.has_groupby()
                            {
                                let d = row
                                    .doc_count()
                                    .try_into_datum(pgrx::PgOid::from((*const_node).consttype));
                                match d {
                                    Ok(Some(datum)) => (datum, false),
                                    _ => (pg_sys::Datum::null(), true),
                                }
                            } else {
                                // Use the native aggregate result type (from the Const node)
                                let d = exec::aggregate_result_to_datum(
                                    agg_result,
                                    agg_type,
                                    (*const_node).consttype, // Use Const's type, not output type
                                );
                                match d {
                                    Some(datum) => (datum, false),
                                    None => (pg_sys::Datum::null(), true),
                                }
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
                return per_tuple_context.switch_to(|_| {
                    let proj_info = pg_sys::ExecBuildProjectionInfo(
                        placeholder_tlist,
                        expr_context,
                        (*planstate).ps_ResultTupleSlot,
                        planstate,
                        (*slot).tts_tupleDescriptor,
                    );
                    pg_sys::ExecProject(proj_info)
                });
            }

            slot
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
                let alias = unsafe {
                    if !(*heap_rte).eref.is_null() && !(*(*heap_rte).eref).aliasname.is_null() {
                        std::ffi::CStr::from_ptr((*(*heap_rte).eref).aliasname)
                            .to_string_lossy()
                            .into_owned()
                    } else {
                        "unknown".to_string()
                    }
                };
                Self::add_planner_warning(
                    "Aggregate Scan not used: table must have a BM25 index",
                    alias,
                );
            }
            return Vec::new();
        };

        match AggregateCSClause::build(builder, heap_rti, &index) {
            Ok((builder, aggregate_clause)) => {
                let alias = unsafe {
                    if !(*heap_rte).eref.is_null() && !(*(*heap_rte).eref).aliasname.is_null() {
                        std::ffi::CStr::from_ptr((*(*heap_rte).eref).aliasname)
                            .to_string_lossy()
                            .into_owned()
                    } else {
                        "unknown".to_string()
                    }
                };
                Self::mark_contexts_successful(alias);

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
        let root = builder.args().root;
        let input_rel = builder.args().input_rel();

        // Collect all tables in the join and their BM25 indexes
        let sources = unsafe { collect_join_agg_sources(root, input_rel) };

        if sources.is_empty() {
            return Vec::new();
        }

        // Only 2-table joins are supported; 3+ table joins produce
        // unreliable plans in DataFusion (empty batches, join errors).
        if sources.len() > 2 {
            Self::add_planner_warning(
                "Aggregate Scan (DataFusion) not used: only 2-table joins are currently supported",
                "join".to_string(),
            );
            return Vec::new();
        }

        // At least one table must have a BM25 index
        if !has_any_bm25_index(&sources) {
            return Vec::new();
        }

        // For M1, all tables must have BM25 indexes (DataFusion scans all via PgSearchTableProvider)
        if !all_have_bm25_index(&sources) {
            Self::add_planner_warning(
                "Aggregate Scan (DataFusion) not used: all tables in the join must have BM25 indexes",
                "join".to_string(),
            );
            return Vec::new();
        }

        // Reject joins with non-equi quals (OR across tables, cross-table
        // filters, non-@@@ conditions). Check both the cheapest path's
        // joinrestrictinfo AND the parse tree's WHERE quals for cross-table
        // references that our DataFusion backend can't apply.
        if unsafe { datafusion_build::has_non_equi_join_quals(input_rel, &sources) } {
            Self::add_planner_warning(
                "Aggregate Scan (DataFusion) not used: join has non-equi quals that cannot be pushed to individual table scans",
                "join".to_string(),
            );
            return Vec::new();
        }

        // Extract the join tree from the parse tree
        let (mut plan, join_level_predicates) = match unsafe {
            extract_join_tree_from_parse(root, &sources, builder.args().input_rel())
        } {
            Ok(result) => result,
            Err(e) => {
                Self::add_planner_warning(
                    format!("Aggregate Scan (DataFusion) not used: {}", e),
                    "join".to_string(),
                );
                return Vec::new();
            }
        };

        // Extract aggregate target list (GROUP BY + aggregates)
        let targetlist = match unsafe { extract_aggregate_targetlist(builder.args(), &sources) } {
            Ok(tl) => tl,
            Err(e) => {
                Self::add_planner_warning(
                    format!("Aggregate Scan (DataFusion) not used: {}", e),
                    "join".to_string(),
                );
                return Vec::new();
            }
        };

        // Reject CROSS JOINs (no equi-join keys). Without join keys the
        // second table's PgSearchTableProvider has no Named fields, producing
        // empty RecordBatches. Single-table scans (sources.len() == 1) have
        // no join keys by definition and are allowed — they reach this path
        // when routed from RELOPT_BASEREL (e.g., max_buckets overflow or
        // ORDER BY aggregate + LIMIT).
        if sources.len() > 1 && plan.join_keys().is_empty() {
            Self::add_planner_warning(
                "Aggregate Scan (DataFusion) not used: CROSS JOINs are not supported (no equi-join keys)",
                "join".to_string(),
            );
            return Vec::new();
        }

        // Populate the fast fields on each source so PgSearchTableProvider exposes them.
        // This fails if join key fields aren't indexed as fast fields.
        if let Err(e) =
            unsafe { datafusion_build::populate_required_fields(&mut plan, &targetlist) }
        {
            Self::add_planner_warning(
                format!("Aggregate Scan (DataFusion) not used: {}", e),
                "join".to_string(),
            );
            return Vec::new();
        }

        // Detect ORDER BY on aggregate + LIMIT for TopK pushdown into DataFusion.
        // DataFusion's SortExec(fetch=K) uses a bounded TopK heap internally.
        // We do NOT declare pathkeys to Postgres because scanrelid=0 CustomScans
        // cannot resolve pathkey items through setrefs.c. Postgres may add a
        // redundant Sort above us, which is correct (just wasteful on K rows).
        let topk = unsafe { detect_join_aggregate_topk(builder.args(), &targetlist) };

        // Build the custom path with DataFusion private data
        vec![builder.build(PrivateData::DataFusion {
            plan,
            targetlist,
            topk,
            join_level_predicates,
        })]
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

        // First call: build and execute the DataFusion plan
        if df_state.runtime.is_none() {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap_or_else(|e| pgrx::error!("Failed to create tokio runtime: {}", e));

            let ctx = create_aggregate_session_context();

            let physical_plan = runtime.block_on(async {
                let logical = build_join_aggregate_plan(
                    &df_state.plan,
                    &df_state.targetlist,
                    df_state.topk.as_ref(),
                    &df_state.join_level_predicates,
                    &ctx,
                )
                .await?;
                build_physical_plan(&ctx, logical).await
            });

            let physical_plan = match physical_plan {
                Ok(p) => p,
                Err(e) => pgrx::error!("Failed to build DataFusion aggregate plan: {}", e),
            };

            let memory_pool = create_memory_pool(
                &physical_plan,
                unsafe { pg_sys::work_mem as usize * 1024 },
                unsafe { pg_sys::hash_mem_multiplier },
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

/// Detects ORDER BY on aggregate + LIMIT for join aggregate queries.
/// Returns `Some(DataFusionTopK)` when the sort clause targets a single aggregate
/// that can be pushed down into the DataFusion plan as sort + limit.
///
/// NOTE: shares structural pattern with `build::detect_aggregate_orderby` (single-table
/// variant). Both parse sort clause → aggref identity → direction → reltarget match →
/// LIMIT. They diverge in target list type (`TargetList` vs `JoinAggregateTargetList`)
/// which makes a generic extraction non-trivial without trait machinery.
unsafe fn detect_join_aggregate_topk(
    args: &CreateUpperPathsHookArgs,
    targetlist: &join_targetlist::JoinAggregateTargetList,
) -> Option<privdat::DataFusionTopK> {
    let parse = args.root().parse;
    if parse.is_null() || (*parse).sortClause.is_null() {
        return None;
    }

    // Only single sort clause for TopK
    let sort_clauses = PgList::<pg_sys::SortGroupClause>::from_pg((*parse).sortClause);
    if sort_clauses.len() != 1 {
        return None;
    }

    let sort_clause_ptr = sort_clauses.get_ptr(0)?;
    let sort_expr = pg_sys::get_sortgroupclause_expr(sort_clause_ptr, (*parse).targetList);

    // The sort expression must BE an aggregate, not merely contain one.
    // e.g. ORDER BY ABS(SUM(score)) wraps the aggregate — ABS breaks
    // monotonicity so Tantivy's ordering wouldn't match Postgres.
    let aggref = targetlist::find_single_aggref_in_expr(sort_expr)?;
    if aggref as *mut pg_sys::Node != sort_expr {
        return None;
    }

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

    // Check if this output position corresponds to an aggregate in the join targetlist
    let agg_idx = targetlist
        .aggregates
        .iter()
        .position(|a| a.output_index == pos)?;

    // Extract LIMIT
    let limit_offset = LimitOffset::from_parse(parse);
    let limit = limit_offset.limit()? as usize;
    let offset = limit_offset.offset().unwrap_or(0) as usize;
    let k = limit + offset;

    Some(privdat::DataFusionTopK {
        sort_agg_idx: agg_idx,
        direction,
        k,
    })
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
