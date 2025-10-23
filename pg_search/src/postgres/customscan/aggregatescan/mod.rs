// Copyright (c) 2023-2025 ParadeDB, Inc.
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
pub mod exec;
pub mod filterquery;
pub mod groupby;
pub mod limit_offset;
pub mod orderby;
pub mod privdat;
pub mod scan_state;
pub mod searchquery;
pub mod targetlist;

<<<<<<< HEAD
pub use privdat::AggregateType;

use std::ffi::CStr;

use crate::aggregate::{build_aggregation_json_for_explain, execute_aggregation, AggQueryParams};
use crate::api::operator::anyelement_query_input_opoid;
use crate::api::{HashMap, HashSet, OrderByFeature};
use crate::gucs;
use crate::index::mvcc::MvccSatisfies;
use crate::nodecast;
use crate::postgres::customscan::agg::AggregationSpec;
use crate::postgres::customscan::aggregatescan::privdat::{
    AggregateValue, GroupingColumn, PrivateData, TargetListEntry,
};
use crate::postgres::customscan::aggregatescan::scan_state::{
    AggregateScanState, ExecutionState, GroupedAggregateRow,
};
use crate::postgres::customscan::builders::custom_path::{
    restrict_info, CustomPathBuilder, OrderByStyle, RestrictInfoType,
};
=======
use crate::nodecast;

use crate::customscan::aggregatescan::build::AggregateCSClause;
use crate::postgres::customscan::aggregatescan::exec::aggregation_results_iter;
use crate::postgres::customscan::aggregatescan::groupby::{GroupByClause, GroupingColumn};
use crate::postgres::customscan::aggregatescan::privdat::PrivateData;
use crate::postgres::customscan::aggregatescan::scan_state::{AggregateScanState, ExecutionState};
use crate::postgres::customscan::aggregatescan::targetlist::TargetListEntry;
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
>>>>>>> main
use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::builders::custom_state::{
    CustomScanStateBuilder, CustomScanStateWrapper,
};
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::solve_expr::SolvePostgresExpressions;
use crate::postgres::customscan::{
    range_table, CreateUpperPathsHookArgs, CustomScan, ExecMethod, PlainExecCapable,
};
use crate::postgres::rel_get_bm25_index;
use crate::postgres::types::TantivyValue;
use crate::postgres::PgSearchRelation;

use pgrx::{pg_sys, IntoDatum, PgList, PgTupleDesc};
use std::ffi::CStr;
use tantivy::schema::OwnedValue;

#[derive(Default)]
pub struct AggregateScan;

impl CustomScan for AggregateScan {
    const NAME: &'static CStr = c"ParadeDB Aggregate Scan";
    type Args = CreateUpperPathsHookArgs;
    type State = AggregateScanState;
    type PrivateData = PrivateData;

    fn create_custom_path(builder: CustomPathBuilder<Self>) -> Option<pg_sys::CustomPath> {
        // We can only handle single base relations as input
        if builder.args().input_rel().reloptkind != pg_sys::RelOptKind::RELOPT_BASEREL {
            return None;
        }

        let parent_relids = builder.args().input_rel().relids;
        let heap_rti = unsafe { range_table::bms_exactly_one_member(parent_relids)? };
        let heap_rte = unsafe {
            // NOTE: The docs indicate that `simple_rte_array` is always the same length
            // as `simple_rel_array`.
            range_table::get_rte(
                builder.args().root().simple_rel_array_size as usize,
                builder.args().root().simple_rte_array,
                heap_rti,
            )?
        };
        let (table, index) = rel_get_bm25_index(unsafe { (*heap_rte).relid })?;
        let (builder, aggregate_clause) = AggregateCSClause::build(builder, heap_rti, &index)?;

        Some(builder.build(PrivateData {
<<<<<<< HEAD
            agg_spec: AggregationSpec {
                agg_types: aggregate_types,
                grouping_columns,
            },
            orderby_info,
            indexrelid: bm25_index.oid(),
            heap_rti,
            query,
            target_list_mapping,
            has_order_by,
            limit,
            offset,
            maybe_truncated,
            filter_groups,
=======
            heap_rti,
            indexrelid: index.oid(),
            aggregate_clause,
>>>>>>> main
        }))
    }

    fn plan_custom_path(mut builder: CustomScanBuilder<Self>) -> pg_sys::CustomScan {
        builder.set_scanrelid(builder.custom_private().heap_rti);

        if builder
            .custom_private()
<<<<<<< HEAD
            .agg_spec
            .grouping_columns
            .is_empty()
            && builder.custom_private().orderby_info.is_empty()
            && !builder.custom_private().has_order_by
=======
            .aggregate_clause
            .planner_should_replace_aggrefs()
>>>>>>> main
        {
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

    fn create_custom_scan_state(
        mut builder: CustomScanStateBuilder<Self, Self::PrivateData>,
    ) -> *mut CustomScanStateWrapper<Self> {
        // EXECUTION-TIME REPLACEMENT: Replace T_Aggref if we have GROUP BY or ORDER BY
        // For simple aggregations without GROUP BY or ORDER BY, replacement should have happened at planning time
        // Now we have the complete reverse logic: replace at execution time if we have any of these conditions
        if !builder
            .custom_private()
<<<<<<< HEAD
            .agg_spec
            .grouping_columns
            .is_empty()
            || !builder.custom_private().orderby_info.is_empty()
            || builder.custom_private().has_order_by
=======
            .aggregate_clause
            .planner_should_replace_aggrefs()
>>>>>>> main
        {
            unsafe {
                let cscan = builder.args().cscan;
                let plan = &mut (*cscan).scan.plan;
                replace_aggrefs_in_target_list(plan);
            }
        }

<<<<<<< HEAD
        builder.custom_state().agg_spec = builder.custom_private().agg_spec.clone();
        builder.custom_state().orderby_info = builder.custom_private().orderby_info.clone();
        builder.custom_state().target_list_mapping =
            builder.custom_private().target_list_mapping.clone();
=======
>>>>>>> main
        builder.custom_state().indexrelid = builder.custom_private().indexrelid;
        builder.custom_state().execution_rti =
            unsafe { (*builder.args().cscan).scan.scanrelid as pg_sys::Index };
        builder.custom_state().aggregate_clause = builder.custom_private().aggregate_clause.clone();
        builder.build()
    }

    fn explain_custom_scan(
        state: &CustomScanStateWrapper<Self>,
        ancestors: *mut pg_sys::List,
        explainer: &mut Explainer,
    ) {
        explainer.add_text("Index", state.custom_state().indexrel().name());
        explainer.add_query(state.custom_state().aggregate_clause.query());
        state
            .custom_state()
            .aggregate_clause
            .add_to_explainer(explainer);
    }

    fn begin_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        estate: *mut pg_sys::EState,
        eflags: i32,
    ) {
        unsafe {
            let rte = pg_sys::exec_rt_fetch(state.custom_state().execution_rti, estate);
            assert!(!rte.is_null());
            let lockmode = (*rte).rellockmode as pg_sys::LOCKMODE;
            let planstate = state.planstate();
            // TODO: Opening of the index could be deduped between custom scans: see
            // `PdbScanState::open_relations`.
            state.custom_state_mut().open_relations(lockmode);

            state
                .custom_state_mut()
                .init_expr_context(estate, planstate);
            state.runtime_context = state.csstate.ss.ps.ps_ExprContext;
        }
    }

    fn rescan_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        state.custom_state_mut().state = ExecutionState::NotStarted;
    }

    fn exec_custom_scan(state: &mut CustomScanStateWrapper<Self>) -> *mut pg_sys::TupleTableSlot {
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
            let slot = pg_sys::MakeTupleTableSlot(
                (*state.planstate()).ps_ResultTupleDesc,
                &pg_sys::TTSOpsVirtual,
            );
            pg_sys::ExecClearTuple(slot);

            let natts = (*(*slot).tts_tupleDescriptor).natts as usize;
            let datums = std::slice::from_raw_parts_mut((*slot).tts_values, natts);
            let isnull = std::slice::from_raw_parts_mut((*slot).tts_isnull, natts);

            let mut aggregates = row.aggregates.clone().into_iter();
            let mut natts_processed = 0;

            // Fill in values according to the target list mapping
            for (i, entry) in state.custom_state().aggregate_clause.entries().enumerate() {
                let attr = tupdesc.get(i).expect("missing attribute");
                let expected_typoid = attr.type_oid().value();

                let datum = match (entry, row.is_empty()) {
                    (TargetListEntry::GroupingColumn(gc_idx), false) => row.group_keys[*gc_idx]
                        .clone()
                        .try_into_datum(pgrx::PgOid::from(expected_typoid))
                        .expect("should be able to convert to datum"),
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
                            aggregates
                                .next()
                                .and_then(|v| v)
                                .unwrap_or_else(|| agg_type.nullish())
                                .value
                                .and_then(|value| {
                                    TantivyValue(OwnedValue::F64(value))
                                        .try_into_datum(expected_typoid.into())
                                        .unwrap()
                                })
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
            (*slot).tts_flags &= !(pg_sys::TTS_FLAG_EMPTY as u16);
            (*slot).tts_flags |= pg_sys::TTS_FLAG_SHOULDFREE as u16;
            (*slot).tts_nvalid = natts as i16;
            slot
        }
    }

    fn shutdown_custom_scan(state: &mut CustomScanStateWrapper<Self>) {}

    fn end_custom_scan(state: &mut CustomScanStateWrapper<Self>) {}
}

impl ExecMethod for AggregateScan {
    fn exec_methods() -> *const pg_sys::CustomExecMethods {
        <AggregateScan as PlainExecCapable>::exec_methods()
    }
}

<<<<<<< HEAD
/// Convert an AggregateValue to a PostgreSQL Datum using TantivyValue's conversion infrastructure
fn convert_aggregate_value_to_datum(
    agg_value: &AggregateValue,
    expected_typoid: pg_sys::Oid,
) -> (pg_sys::Datum, bool) {
    // Convert AggregateValue to OwnedValue
    let owned_value = match agg_value {
        AggregateValue::Null => OwnedValue::Null,
        AggregateValue::Int(val) => OwnedValue::I64(*val),
        AggregateValue::Float(val) => OwnedValue::F64(*val),
        AggregateValue::Json(val) => {
            // For JSON values, serialize to string and convert to OwnedValue
            let json_str = val.to_string();
            OwnedValue::Str(json_str)
        }
    };
=======
impl PlainExecCapable for AggregateScan {}
>>>>>>> main

pub trait CustomScanClause<CS: CustomScan> {
    type Args;

    fn from_pg(args: &CS::Args, heap_rti: pg_sys::Index, index: &PgSearchRelation) -> Option<Self>
    where
        Self: Sized;

    fn add_to_custom_path(&self, builder: CustomPathBuilder<CS>) -> CustomPathBuilder<CS>;

    fn explain_output(&self) -> Box<dyn Iterator<Item = (String, String)>> {
        Box::new(std::iter::empty())
    }

<<<<<<< HEAD
fn explain_execution_strategy(
    state: &CustomScanStateWrapper<AggregateScan>,
    filter_groups: &[(Option<SearchQueryInput>, Vec<usize>)],
    explainer: &mut Explainer,
) {
    // Helper to add GROUP BY information
    let add_group_by = |explainer: &mut Explainer| {
        if !state.custom_state().agg_spec.grouping_columns.is_empty() {
            let group_by_fields: String = state
                .custom_state()
                .agg_spec
                .grouping_columns
                .iter()
                .map(|col| col.field_name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            explainer.add_text("  Group By", group_by_fields);
        }
    };

    // Helper to add LIMIT/OFFSET information
    let add_limit_offset = |explainer: &mut Explainer| {
        if let Some(limit) = state.custom_state().limit {
            let offset = state.custom_state().offset.unwrap_or(0);
            if offset > 0 {
                explainer.add_text("  Limit", limit.to_string());
                explainer.add_text("  Offset", offset.to_string());
            } else {
                explainer.add_text("  Limit", limit.to_string());
            }
        }
    };

    // Helper to build aggregation definition JSON (for no-filter cases)
    // Uses the shared function from aggregate module to avoid duplication
    let build_aggregate_json = || -> Option<String> {
        let qparams = AggQueryParams {
            base_query: &state.custom_state().query,
            aggregate_types: &state.custom_state().agg_spec.agg_types,
            grouping_columns: &state.custom_state().agg_spec.grouping_columns,
            orderby_info: &state.custom_state().orderby_info,
            limit: &state.custom_state().limit,
            offset: &state.custom_state().offset,
        };
        build_aggregation_json_for_explain(&qparams).ok()
    };

    // Helper to show base query + all aggregates (no filters case)
    let explain_no_filters = |explainer: &mut Explainer| {
        explainer.add_query(&state.custom_state().query);
        let all_indices: Vec<usize> = (0..state.custom_state().agg_spec.agg_types.len()).collect();
        explainer.add_text(
            "  Applies to Aggregates",
            AggregateType::format_aggregates(
                &state.custom_state().agg_spec.agg_types,
                &all_indices,
            ),
        );
        add_group_by(explainer);
        add_limit_offset(explainer);

        // Add aggregate definition for no-filter cases (can be built without QueryContext)
        if let Some(agg_def) = build_aggregate_json() {
            explainer.add_text("  Aggregate Definition", agg_def);
        }
    };

    if filter_groups.is_empty() {
        explain_no_filters(explainer);
    } else if filter_groups.len() == 1 {
        // Single query
        let (filter_expr, aggregate_indices) = &filter_groups[0];
        if filter_expr.is_none() {
            explain_no_filters(explainer);
        } else {
            // Show the combined query
            let combined_query =
                combine_query_with_filter(&state.custom_state().query, filter_expr);
            explainer.add_text("  Combined Query", combined_query.explain_format());
            add_group_by(explainer);
            add_limit_offset(explainer);
            explainer.add_text(
                "  Applies to Aggregates",
                AggregateType::format_aggregates(
                    &state.custom_state().agg_spec.agg_types,
                    aggregate_indices,
                ),
            );
        }
    } else {
        // Multi-group
        explainer.add_text(
            "Execution Strategy",
            format!("Multi-Query ({} Filter Groups)", filter_groups.len()),
        );
        add_group_by(explainer);
        add_limit_offset(explainer);

        for (group_idx, (filter_expr, aggregate_indices)) in filter_groups.iter().enumerate() {
            let combined_query =
                combine_query_with_filter(&state.custom_state().query, filter_expr);

            let query_label = if filter_expr.is_some() {
                format!("  Group {} Query", group_idx + 1)
            } else {
                format!("  Group {} Query (No Filter)", group_idx + 1)
            };
            explainer.add_text(&query_label, combined_query.explain_format());
            explainer.add_text(
                &format!("  Group {} Aggregates", group_idx + 1),
                AggregateType::format_aggregates(
                    &state.custom_state().agg_spec.agg_types,
                    aggregate_indices,
                ),
            );
        }
    }
}

fn combine_query_with_filter(
    query: &SearchQueryInput,
    filter_expr: &Option<SearchQueryInput>,
) -> SearchQueryInput {
    match filter_expr {
        Some(filter) => match query {
            SearchQueryInput::All => filter.clone(),
            _ => SearchQueryInput::Boolean {
                must: vec![query.clone(), filter.clone()],
                should: vec![],
                must_not: vec![],
            },
        },
        None => query.clone(),
    }
}

/// Extract grouping columns from pathkeys and validate they are fast fields
fn extract_grouping_columns(
    pathkeys: &PgList<pg_sys::PathKey>,
    root: *mut pg_sys::PlannerInfo,
    heap_rti: pg_sys::Index,
    schema: &SearchIndexSchema,
) -> Option<Vec<GroupingColumn>> {
    let mut grouping_columns = Vec::new();

    for pathkey in pathkeys.iter_ptr() {
        unsafe {
            let equivclass = (*pathkey).pk_eclass;
            let members = PgList::<pg_sys::EquivalenceMember>::from_pg((*equivclass).ec_members);

            let mut found_valid_column = false;
            for member in members.iter_ptr() {
                let expr = (*member).em_expr;

                // Create VarContext for field extraction
                let var_context = VarContext::from_planner(root);

                // Try to extract field name and variable info
                let (field_name, attno) = if let Some((var, field_name)) =
                    find_one_var_and_fieldname(var_context, expr as *mut pg_sys::Node)
                {
                    // JSON operator expression or complex field access
                    let (heaprelid, attno, _) = find_var_relation(var, root);
                    if heaprelid == pg_sys::InvalidOid {
                        continue;
                    }
                    (field_name.to_string(), attno)
                } else {
                    continue;
                };

                // Check if this field exists in the index schema as a fast field
                if let Some(search_field) = schema.search_field(&field_name) {
                    if search_field.is_fast() {
                        grouping_columns.push(GroupingColumn { field_name, attno });
                        found_valid_column = true;
                        break; // Found a valid grouping column for this pathkey
                    }
                }
            }

            if !found_valid_column {
                return None;
            }
=======
    fn add_to_explainer(&self, explainer: &mut Explainer) {
        for (key, value) in self.explain_output() {
            explainer.add_text(&format!("  {}", key), &value);
>>>>>>> main
        }
    }

    fn build(
        builder: CustomPathBuilder<CS>,
        heap_rti: pg_sys::Index,
        index: &PgSearchRelation,
    ) -> Option<(CustomPathBuilder<CS>, Self)>
    where
        Self: Sized,
    {
        let clause = Self::from_pg(builder.args(), heap_rti, index)?;
        let builder = clause.add_to_custom_path(builder);
        Some((builder, clause))
    }
}

/// Replace any T_Aggref expressions in the target list with T_FuncExpr placeholders
/// This is called at execution time to avoid "Aggref found in non-Agg plan node" errors
unsafe fn replace_aggrefs_in_target_list(plan: *mut pg_sys::Plan) {
    if (*plan).targetlist.is_null() {
        return;
    }

    let targetlist = (*plan).targetlist;
    let original_tlist = PgList::<pg_sys::TargetEntry>::from_pg((*plan).targetlist);
    let mut new_targetlist = PgList::<pg_sys::TargetEntry>::new();

    for (te_idx, te) in original_tlist.iter_ptr().enumerate() {
        if let Some(aggref) = nodecast!(Aggref, T_Aggref, (*te).expr) {
            // Create a flat copy of the target entry
            let new_te = pg_sys::flatCopyTargetEntry(te);
            // Replace the T_Aggref with a T_FuncExpr placeholder
            let funcexpr = make_placeholder_func_expr(aggref);
            (*new_te).expr = funcexpr as *mut pg_sys::Expr;
            new_targetlist.push(new_te);
        } else {
            // For non-Aggref entries, just make a flat copy
            let copied_te = pg_sys::flatCopyTargetEntry(te);
            new_targetlist.push(copied_te);
        }
    }

    (*plan).targetlist = new_targetlist.into_pg();
}

unsafe fn make_placeholder_func_expr(aggref: *mut pg_sys::Aggref) -> *mut pg_sys::FuncExpr {
    let paradedb_funcexpr: *mut pg_sys::FuncExpr =
        pg_sys::palloc0(size_of::<pg_sys::FuncExpr>()).cast();
    (*paradedb_funcexpr).xpr.type_ = pg_sys::NodeTag::T_FuncExpr;
    (*paradedb_funcexpr).funcid = placeholder_procid();
    (*paradedb_funcexpr).funcresulttype = (*aggref).aggtype;
    (*paradedb_funcexpr).funcretset = false;
    (*paradedb_funcexpr).funcvariadic = false;
    (*paradedb_funcexpr).funcformat = pg_sys::CoercionForm::COERCE_EXPLICIT_CALL;
    (*paradedb_funcexpr).funccollid = pg_sys::InvalidOid;
    (*paradedb_funcexpr).inputcollid = (*aggref).inputcollid;
    (*paradedb_funcexpr).location = (*aggref).location;
    (*paradedb_funcexpr).args = PgList::<pg_sys::Node>::new().into_pg();

    paradedb_funcexpr
}

/// Get the Oid of a placeholder function to use in the target list of aggregate custom scans.
unsafe fn placeholder_procid() -> pg_sys::Oid {
    pgrx::direct_function_call::<pg_sys::Oid>(pg_sys::regprocedurein, &[c"now()".into_datum()])
        .expect("the `now()` function should exist")
}
<<<<<<< HEAD

fn execute(
    state: &mut CustomScanStateWrapper<AggregateScan>,
) -> std::vec::IntoIter<GroupedAggregateRow> {
    let planstate = state.planstate();
    let expr_context = state.runtime_context;

    state
        .custom_state_mut()
        .prepare_query_for_execution(planstate, expr_context);

    let qparams = AggQueryParams {
        base_query: &state.custom_state().query, // WHERE clause or AllQuery if no WHERE clause
        aggregate_types: &state.custom_state().agg_spec.agg_types,
        grouping_columns: &state.custom_state().agg_spec.grouping_columns,
        orderby_info: &state.custom_state().orderby_info,
        limit: &state.custom_state().limit,
        offset: &state.custom_state().offset,
    };

    let result = execute_aggregation(
        state.custom_state().indexrel(),
        &qparams,
        true,                                              // solve_mvcc
        gucs::adjust_work_mem().get().try_into().unwrap(), // memory_limit
        DEFAULT_BUCKET_LIMIT,                              // bucket_limit
    )
    .unwrap_or_else(|e| pgrx::error!("Failed to execute filter aggregation: {}", e));
    // Process results using unified result processing
    let aggregate_results = state.custom_state().process_aggregation_results(result);

    aggregate_results.into_iter()
}

impl ExecMethod for AggregateScan {
    fn exec_methods() -> *const pg_sys::CustomExecMethods {
        <AggregateScan as PlainExecCapable>::exec_methods()
    }
}

impl PlainExecCapable for AggregateScan {}

impl SolvePostgresExpressions for AggregateScanState {
    fn has_heap_filters(&mut self) -> bool {
        self.query.has_heap_filters()
            || self
                .agg_spec
                .agg_types
                .iter_mut()
                .any(|agg| agg.has_heap_filters())
    }

    fn has_postgres_expressions(&mut self) -> bool {
        self.query.has_postgres_expressions()
            || self
                .agg_spec
                .agg_types
                .iter_mut()
                .any(|agg| agg.has_postgres_expressions())
    }

    fn init_postgres_expressions(&mut self, planstate: *mut pg_sys::PlanState) {
        self.query.init_postgres_expressions(planstate);
        self.agg_spec
            .agg_types
            .iter_mut()
            .for_each(|agg| agg.init_postgres_expressions(planstate));
    }

    fn solve_postgres_expressions(&mut self, expr_context: *mut pg_sys::ExprContext) {
        self.query.solve_postgres_expressions(expr_context);
        self.agg_spec
            .agg_types
            .iter_mut()
            .for_each(|agg| agg.solve_postgres_expressions(expr_context));
    }
}

/// Extract pathkeys from ORDER BY clauses to inform PostgreSQL about sorted output
fn extract_order_by_pathkeys(
    root: *mut pg_sys::PlannerInfo,
    heap_rti: pg_sys::Index,
    schema: &SearchIndexSchema,
) -> PathKeyInfo {
    unsafe {
        extract_pathkey_styles_with_sortability_check(
            root,
            heap_rti,
            schema,
            |search_field| search_field.is_fast(), // Use is_fast() for regular vars
            |_search_field| false,                 // Don't accept lower functions in aggregatescan
        )
    }
}
=======
>>>>>>> main
