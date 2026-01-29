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
pub mod descale;
pub mod exec;
pub mod filterquery;
pub mod groupby;
pub mod limit_offset;
pub mod orderby;
pub mod privdat;
pub mod scan_state;
pub mod searchquery;
pub mod targetlist;

// Re-export commonly used types for easier access
pub use aggregate_type::extract_agg_name_to_field;
pub use aggregate_type::AggregateType;
pub use descale::descale_numeric_values_in_json;
pub use groupby::GroupingColumn;
pub use targetlist::TargetListEntry;

use crate::api::agg_funcoid;
use crate::gucs;

use crate::aggregate::{NULL_SENTINEL_MAX, NULL_SENTINEL_MIN};
use crate::customscan::aggregatescan::build::AggregateCSClause;
use crate::postgres::customscan::aggregatescan::exec::aggregation_results_iter;
use crate::postgres::customscan::aggregatescan::groupby::GroupByClause;
use crate::postgres::customscan::aggregatescan::privdat::PrivateData;
use crate::postgres::customscan::aggregatescan::scan_state::{AggregateScanState, ExecutionState};
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::builders::custom_state::{
    CustomScanStateBuilder, CustomScanStateWrapper,
};
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::projections::{create_placeholder_targetlist, placeholder_procid};
use crate::postgres::customscan::solve_expr::SolvePostgresExpressions;
use crate::postgres::customscan::{
    range_table, CreateUpperPathsHookArgs, CustomScan, ExecMethod, PlainExecCapable,
};
use crate::postgres::rel_get_bm25_index;
use crate::postgres::types::TantivyValue;
use crate::postgres::PgSearchRelation;

use pgrx::{pg_sys, IntoDatum, PgList, PgMemoryContexts, PgTupleDesc};
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
            heap_rti,
            indexrelid: index.oid(),
            aggregate_clause,
        }))
    }

    fn plan_custom_path(mut builder: CustomScanBuilder<Self>) -> pg_sys::CustomScan {
        builder.set_scanrelid(builder.custom_private().heap_rti);

        if builder
            .custom_private()
            .aggregate_clause
            .planner_should_replace_aggrefs()
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
            .aggregate_clause
            .planner_should_replace_aggrefs()
        {
            unsafe {
                let cscan = builder.args().cscan;
                let plan = &mut (*cscan).scan.plan;
                replace_aggrefs_in_target_list(plan);
            }
        }

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
                        // Check if this is a NULL sentinel (handles both MIN and MAX sentinels)
                        // Note: U64/Bool use string sentinel for MIN (since 0 is valid).
                        // Bool uses 2 as MAX sentinel (0=false, 1=true, 2=null).
                        let is_bool_type = expected_typoid == pg_sys::BOOLOID;
                        let is_null_sentinel = match &key.0 {
                            OwnedValue::Str(s) => s == NULL_SENTINEL_MIN || s == NULL_SENTINEL_MAX,
                            OwnedValue::I64(v) => *v == i64::MAX || *v == i64::MIN,
                            OwnedValue::U64(v) => *v == u64::MAX || (is_bool_type && *v == 2),
                            OwnedValue::F64(v) => *v == f64::MAX || *v == f64::MIN,
                            _ => false,
                        };
                        if is_null_sentinel {
                            None
                        } else {
                            // For Numeric64 fields, descale the group key value
                            let grouping_columns =
                                state.custom_state().aggregate_clause.grouping_columns();
                            let descaled_key =
                                if let Some(scale) = grouping_columns[*gc_idx].numeric_scale {
                                    // Descale by dividing by 10^scale
                                    let divisor = 10f64.powi(scale as i32);
                                    match &key.0 {
                                        OwnedValue::I64(v) => {
                                            TantivyValue(OwnedValue::F64(*v as f64 / divisor))
                                        }
                                        OwnedValue::U64(v) => {
                                            TantivyValue(OwnedValue::F64(*v as f64 / divisor))
                                        }
                                        OwnedValue::F64(v) => {
                                            TantivyValue(OwnedValue::F64(*v / divisor))
                                        }
                                        _ => key,
                                    }
                                } else {
                                    key
                                };
                            descaled_key
                                .try_into_datum(pgrx::PgOid::from(expected_typoid))
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

                // Mutate Const nodes with aggregate values directly from the row results.
                // We DON'T use the slot's datums because those were converted using the
                // output tuple descriptor's types (e.g., TEXT for jsonb_pretty output),
                // but we need the native aggregate type (e.g., JSONB for pdb.agg).
                // This matches basescan's approach of setting Const values directly.
                let mut agg_iter = row.aggregates.iter();
                for (i, entry) in state.custom_state().aggregate_clause.entries().enumerate() {
                    let TargetListEntry::Aggregate(agg_type) = entry else {
                        continue;
                    };

                    let Some(const_node) = state
                        .custom_state()
                        .const_agg_nodes
                        .get(i)
                        .copied()
                        .flatten()
                    else {
                        // No Const node for this aggregate, skip the iterator
                        agg_iter.next();
                        continue;
                    };

                    // Get the next aggregate result
                    let agg_result = agg_iter.next().and_then(|v| v.clone());

                    // Convert to datum using the Const node's type (native aggregate type)
                    // not the output tuple descriptor's type
                    let (datum, is_null) = if row.is_empty() {
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

    fn shutdown_custom_scan(state: &mut CustomScanStateWrapper<Self>) {}

    fn end_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        // Clean up the reusable scan slot
        if let Some(slot) = state.custom_state().scan_slot {
            unsafe {
                pg_sys::ExecDropSingleTupleTableSlot(slot);
            }
        }
    }
}

impl ExecMethod for AggregateScan {
    fn exec_methods() -> *const pg_sys::CustomExecMethods {
        <AggregateScan as PlainExecCapable>::exec_methods()
    }
}

impl PlainExecCapable for AggregateScan {}

pub trait CustomScanClause<CS: CustomScan> {
    type Args;

    fn from_pg(args: &CS::Args, heap_rti: pg_sys::Index, index: &PgSearchRelation) -> Option<Self>
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
        _context: *mut core::ffi::c_void,
    ) -> *mut pg_sys::Node {
        if node.is_null() {
            return std::ptr::null_mut();
        }

        // If this is an Aggref, replace it with a placeholder FuncExpr
        if (*node).type_ == pg_sys::NodeTag::T_Aggref {
            let aggref = node as *mut pg_sys::Aggref;
            return make_placeholder_func_expr(aggref) as *mut pg_sys::Node;
        }

        // For all other nodes, use the standard mutator to walk children
        #[cfg(not(any(feature = "pg16", feature = "pg17", feature = "pg18")))]
        {
            let fnptr = aggref_mutator as usize as *const ();
            let mutator: unsafe extern "C-unwind" fn() -> *mut pg_sys::Node =
                std::mem::transmute(fnptr);
            pg_sys::expression_tree_mutator(node, Some(mutator), std::ptr::null_mut())
        }

        #[cfg(any(feature = "pg16", feature = "pg17", feature = "pg18"))]
        {
            pg_sys::expression_tree_mutator_impl(node, Some(aggref_mutator), std::ptr::null_mut())
        }
    }

    let targetlist = PgList::<pg_sys::TargetEntry>::from_pg((*plan).targetlist);

    // Check if there are any Aggref nodes anywhere in the target list
    let has_aggref = targetlist.iter_ptr().any(|te| {
        !te.is_null()
            && !(*te).expr.is_null()
            && expr_contains_aggref((*te).expr as *mut pg_sys::Node)
    });

    if !has_aggref {
        return;
    }

    // Build a new target list with Aggrefs replaced by placeholders
    let mut new_targetlist: *mut pg_sys::List = std::ptr::null_mut();
    for te in targetlist.iter_ptr() {
        let new_te = pg_sys::flatCopyTargetEntry(te);

        // Use the mutator to replace any Aggref nodes in the expression
        let new_expr = aggref_mutator((*te).expr as *mut pg_sys::Node, std::ptr::null_mut());
        (*new_te).expr = new_expr as *mut pg_sys::Expr;

        new_targetlist = pg_sys::lappend(new_targetlist, new_te.cast());
    }

    (*plan).targetlist = new_targetlist;
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

    // Create a string argument with the aggregate function name for better EXPLAIN output
    let agg_name = get_aggregate_name(aggref);
    let agg_name_const = make_text_const(&agg_name);
    let mut args = PgList::<pg_sys::Node>::new();
    args.push(agg_name_const.cast());
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

/// Create a text Const node from a string
///
/// # Safety
/// This function must be called within a PostgreSQL memory context that will persist
/// for the lifetime of the plan tree. The returned Const node will be allocated in the
/// current memory context and should not be freed manually.
unsafe fn make_text_const(text: &str) -> *mut pg_sys::Const {
    let text_datum = text
        .into_datum()
        .expect("failed to convert string to datum");

    pg_sys::makeConst(
        pg_sys::TEXTOID,
        -1,
        pg_sys::DEFAULT_COLLATION_OID,
        -1,
        text_datum,
        false, // constisnull
        false, // constbyval (text is not passed by value)
    )
}
