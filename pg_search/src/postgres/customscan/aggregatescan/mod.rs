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

use crate::nodecast;

use crate::customscan::aggregatescan::build::AggregateCSClause;
use crate::postgres::customscan::aggregatescan::exec::aggregation_results_iter;
use crate::postgres::customscan::aggregatescan::groupby::{GroupByClause, GroupingColumn};
use crate::postgres::customscan::aggregatescan::privdat::PrivateData;
use crate::postgres::customscan::aggregatescan::scan_state::{AggregateScanState, ExecutionState};
use crate::postgres::customscan::aggregatescan::targetlist::TargetListEntry;
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
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
        let stage = builder.args().stage;
        pgrx::warning!(
            "AggregateScan::create_custom_path called at stage {:?}",
            stage
        );

        // At UPPERREL_WINDOW, we have an input_rel with a subplan
        // At UPPERREL_GROUP_AGG, input_rel should be a base relation
        unsafe {
            let input_rel = builder.args().input_rel();
            pgrx::warning!("  input_rel.reloptkind = {:?}", (*input_rel).reloptkind);

            if stage == pg_sys::UpperRelationKind::UPPERREL_WINDOW {
                // At UPPERREL_WINDOW, check what's available
                pgrx::warning!("  UPPERREL_WINDOW: checking available target lists");

                // Check input_rel.reltarget (base columns from subplan)
                if !(*input_rel).reltarget.is_null() {
                    let input_exprs =
                        PgList::<pg_sys::Expr>::from_pg((*(*input_rel).reltarget).exprs);
                    pgrx::warning!("  input_rel.reltarget has {} exprs", input_exprs.len());
                }

                // Check output_rel.reltarget (should have window functions)
                let output_rel = builder.args().output_rel();
                if !(*output_rel).reltarget.is_null() {
                    let output_exprs =
                        PgList::<pg_sys::Expr>::from_pg((*(*output_rel).reltarget).exprs);
                    pgrx::warning!("  output_rel.reltarget has {} exprs", output_exprs.len());
                    for (i, expr) in output_exprs.iter_ptr().enumerate() {
                        pgrx::warning!("    output_rel expr {}: type={:?}", i, (*expr).type_);
                    }
                }

                // Check if output_rel has existing paths we can inspect
                if !(*output_rel).pathlist.is_null() {
                    let pathlist = PgList::<pg_sys::Path>::from_pg((*output_rel).pathlist);
                    pgrx::warning!("  output_rel.pathlist has {} paths", pathlist.len());
                }

                // Check input_rel paths - these are the subplans we'll be wrapping
                if !(*input_rel).pathlist.is_null() {
                    let input_pathlist = PgList::<pg_sys::Path>::from_pg((*input_rel).pathlist);
                    pgrx::warning!("  input_rel.pathlist has {} paths", input_pathlist.len());

                    // Check the cheapest path's target
                    if !(*input_rel).cheapest_total_path.is_null() {
                        let cheapest_path = (*input_rel).cheapest_total_path;
                        if !(*cheapest_path).pathtarget.is_null() {
                            let target_exprs = PgList::<pg_sys::Expr>::from_pg(
                                (*(*cheapest_path).pathtarget).exprs,
                            );
                            pgrx::warning!(
                                "  input_rel.cheapest_total_path.pathtarget has {} exprs",
                                target_exprs.len()
                            );
                            for (i, expr) in target_exprs.iter_ptr().enumerate() {
                                pgrx::warning!(
                                    "    input path expr {}: type={:?}",
                                    i,
                                    (*expr).type_
                                );
                            }
                        }
                    }
                }

                // Check parse tree for window functions
                let root = builder.args().root();
                let parse = (*root).parse;
                if !parse.is_null() {
                    if !(*parse).windowClause.is_null() {
                        let window_clause =
                            PgList::<pg_sys::WindowClause>::from_pg((*parse).windowClause);
                        pgrx::warning!("  parse.windowClause has {} entries", window_clause.len());
                    }

                    // Check targetList for WindowFunc nodes
                    if !(*parse).targetList.is_null() {
                        let target_entries =
                            PgList::<pg_sys::TargetEntry>::from_pg((*parse).targetList);
                        let mut window_func_count = 0;
                        for te in target_entries.iter_ptr() {
                            if !(*te).expr.is_null()
                                && (*(*te).expr).type_ == pg_sys::NodeTag::T_WindowFunc
                            {
                                window_func_count += 1;
                            }
                        }
                        pgrx::warning!(
                            "  parse.targetList has {} WindowFunc nodes",
                            window_func_count
                        );
                    }

                    // Check root->processed_tlist (this is what PostgreSQL uses for window functions)
                    if !(*root).processed_tlist.is_null() {
                        let processed_tlist =
                            PgList::<pg_sys::TargetEntry>::from_pg((*root).processed_tlist);
                        pgrx::warning!(
                            "  root.processed_tlist has {} entries",
                            processed_tlist.len()
                        );
                        let mut window_func_count = 0;
                        for te in processed_tlist.iter_ptr() {
                            if !(*te).expr.is_null()
                                && (*(*te).expr).type_ == pg_sys::NodeTag::T_WindowFunc
                            {
                                window_func_count += 1;
                            }
                        }
                        pgrx::warning!(
                            "  root.processed_tlist has {} WindowFunc nodes",
                            window_func_count
                        );
                    }
                }
            }
        }

        // We can only handle single base relations as input
        if builder.args().input_rel().reloptkind != pg_sys::RelOptKind::RELOPT_BASEREL {
            pgrx::warning!("  input_rel is not a base relation, returning None");
            return None;
        }

        // At UPPERREL_WINDOW stage, handle window functions
        // At UPPERREL_GROUP_AGG stage, handle regular aggregates
        // Both use the same infrastructure (TargetList, AggregateType, etc.)

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
        pgrx::warning!("AggregateScan::plan_custom_path called");

        // Both UPPERREL_WINDOW and UPPERREL_GROUP_AGG scan the base relation directly
        // The difference is only in what they compute (window aggregates vs regular aggregates)
        let heap_rti = builder.custom_private().heap_rti;
        pgrx::warning!("  Setting scanrelid = {} (base scan)", heap_rti);

        // Build target list if needed (at UPPERREL_WINDOW, PostgreSQL doesn't provide one)
        let new_tlist_opt = unsafe {
            let rel = builder.args().rel;
            pgrx::warning!("  rel.relid = {}", (*rel).relid);

            // Check what's in the tlist parameter passed by PostgreSQL
            let tlist = &builder.args().tlist;
            pgrx::warning!("  tlist has {} entries", tlist.len());

            // At UPPERREL_WINDOW, PostgreSQL doesn't build a target list for us (tlist is empty)
            // We need to build it ourselves from input_rel.reltarget + window functions
            let has_window_aggs = builder
                .custom_private()
                .aggregate_clause
                .targetlist()
                .has_window_aggregates();
            if tlist.is_empty() && has_window_aggs {
                pgrx::warning!("  tlist is empty at UPPERREL_WINDOW, building our own");

                // Build target list from input_rel.reltarget (base columns) + window functions
                let input_rel = (*builder.args().best_path).path.parent;
                let root = builder.args().root;

                if !(*root).processed_tlist.is_null() {
                    let processed_tlist =
                        PgList::<pg_sys::TargetEntry>::from_pg((*root).processed_tlist);

                    pgrx::warning!("    processed_tlist has {} entries", processed_tlist.len());

                    // Build target list with placeholders for WindowFunc nodes from the start
                    // Don't include WindowFunc nodes at all - PostgreSQL validates immediately
                    let mut new_tlist = PgList::<pg_sys::TargetEntry>::new();
                    let mut resno = 1;

                    for te in processed_tlist.iter_ptr() {
                        if !(*te).expr.is_null() {
                            let expr = (*te).expr;

                            // Replace WindowFunc with placeholder immediately
                            let final_expr = if (*expr).type_ == pg_sys::NodeTag::T_WindowFunc {
                                pgrx::warning!(
                                    "      Replacing WindowFunc at resno={} with placeholder",
                                    resno
                                );
                                let funcexpr =
                                    make_window_func_placeholder_without_accessing_windowfunc();
                                funcexpr as *mut pg_sys::Expr
                            } else {
                                expr
                            };

                            let new_te = pg_sys::makeTargetEntry(
                                final_expr,
                                resno,
                                (*te).resname,
                                (*te).resjunk,
                            );
                            new_tlist.push(new_te);
                            resno += 1;
                        }
                    }

                    pgrx::warning!(
                        "  Built new tlist with {} entries (WindowFunc replaced with placeholders)",
                        new_tlist.len()
                    );
                    pgrx::warning!("  About to call new_tlist.into_pg()");
                    let result = new_tlist.into_pg();
                    pgrx::warning!("  Called new_tlist.into_pg() successfully");

                    // CRITICAL: Also replace WindowFunc nodes in root.processed_tlist
                    // PostgreSQL validates the plan and checks root.processed_tlist
                    pgrx::warning!("  Replacing WindowFunc in root.processed_tlist");
                    let mut new_processed_tlist = PgList::<pg_sys::TargetEntry>::new();
                    for te in processed_tlist.iter_ptr() {
                        if !(*te).expr.is_null() {
                            let expr = (*te).expr;
                            if (*expr).type_ == pg_sys::NodeTag::T_WindowFunc {
                                let funcexpr =
                                    make_window_func_placeholder_without_accessing_windowfunc();
                                let new_te = pg_sys::makeTargetEntry(
                                    funcexpr as *mut pg_sys::Expr,
                                    (*te).resno,
                                    (*te).resname,
                                    (*te).resjunk,
                                );
                                new_processed_tlist.push(new_te);
                            } else {
                                new_processed_tlist.push(pg_sys::flatCopyTargetEntry(te));
                            }
                        }
                    }
                    (*root).processed_tlist = new_processed_tlist.into_pg();
                    pgrx::warning!("  Replaced WindowFunc in root.processed_tlist");

                    // Also check if there are WindowFunc nodes in windowClause
                    let parse = (*root).parse;
                    if !parse.is_null() && !(*parse).windowClause.is_null() {
                        pgrx::warning!("  WARNING: parse.windowClause is not null - this might contain WindowFunc references");
                        // Clear the windowClause since we're handling window functions ourselves
                        (*parse).windowClause = std::ptr::null_mut();
                        pgrx::warning!("  Cleared parse.windowClause");
                    }

                    // Also replace in parse.targetList
                    if !parse.is_null() && !(*parse).targetList.is_null() {
                        pgrx::warning!("  Replacing WindowFunc in parse.targetList");
                        let parse_tlist =
                            PgList::<pg_sys::TargetEntry>::from_pg((*parse).targetList);
                        let mut new_parse_tlist = PgList::<pg_sys::TargetEntry>::new();
                        for te in parse_tlist.iter_ptr() {
                            if !(*te).expr.is_null() {
                                let expr = (*te).expr;
                                if (*expr).type_ == pg_sys::NodeTag::T_WindowFunc {
                                    let funcexpr =
                                        make_window_func_placeholder_without_accessing_windowfunc();
                                    let new_te = pg_sys::makeTargetEntry(
                                        funcexpr as *mut pg_sys::Expr,
                                        (*te).resno,
                                        (*te).resname,
                                        (*te).resjunk,
                                    );
                                    new_parse_tlist.push(new_te);
                                } else {
                                    new_parse_tlist.push(pg_sys::flatCopyTargetEntry(te));
                                }
                            }
                        }
                        (*parse).targetList = new_parse_tlist.into_pg();
                        pgrx::warning!("  Replaced WindowFunc in parse.targetList");
                    }

                    Some(result)
                } else {
                    None
                }
            } else {
                None
            }
        };

        builder.set_scanrelid(heap_rti);

        let should_replace_aggrefs = builder
            .custom_private()
            .aggregate_clause
            .planner_should_replace_aggrefs();

        let has_window_aggregates = builder
            .custom_private()
            .aggregate_clause
            .targetlist()
            .has_window_aggregates();

        if should_replace_aggrefs || has_window_aggregates || new_tlist_opt.is_some() {
            unsafe {
                let mut cscan = builder.build();
                let plan = &mut cscan.scan.plan;

                // Set the new target list if we built one
                // Note: WindowFunc nodes are already replaced with placeholders at this point
                if let Some(new_tlist) = new_tlist_opt {
                    pgrx::warning!(
                        "  Setting new target list on plan (WindowFunc already replaced)"
                    );
                    plan.targetlist = new_tlist;
                }

                if should_replace_aggrefs {
                    replace_aggrefs_in_target_list(plan);
                }

                // WindowFunc replacement already done above if we built a new target list
                if has_window_aggregates && new_tlist_opt.is_none() {
                    replace_windowfuncs_in_target_list(plan);
                }

                pgrx::warning!("AggregateScan::plan_custom_path returning CustomScan");
                cscan
            }
        } else {
            pgrx::warning!(
                "AggregateScan::plan_custom_path returning CustomScan (no replacements)"
            );
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
    }

    fn begin_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        estate: *mut pg_sys::EState,
        eflags: i32,
    ) {
        unsafe {
            pgrx::warning!("  execution_rti = {}", state.custom_state().execution_rti);
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
                    (TargetListEntry::PassThrough { .. }, _) => {
                        // PassThrough columns should be handled by PdbScan-style execution
                        // For now, return NULL as a placeholder
                        pgrx::error!(
                            "PassThrough columns not yet implemented in AggregateScan execution"
                        )
                    }
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
                    // Window aggregates are not handled by AggregateScan execution
                    // They should be handled by PdbScan when window functions are present
                    (TargetListEntry::WindowAggregate(_), _) => {
                        pgrx::error!("WindowAggregate should not be present in AggregateScan execution - this is a planning bug")
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

/// Replace WindowFunc nodes in the target list with window_func() placeholders.
/// This is similar to replace_aggrefs_in_target_list but for window functions.
unsafe fn replace_windowfuncs_in_target_list(plan: *mut pg_sys::Plan) {
    pgrx::warning!("replace_windowfuncs_in_target_list called");
    if (*plan).targetlist.is_null() {
        pgrx::warning!("  targetlist is NULL, returning");
        return;
    }

    let original_tlist = PgList::<pg_sys::TargetEntry>::from_pg((*plan).targetlist);
    pgrx::warning!("  original_tlist has {} entries", original_tlist.len());
    let mut new_targetlist = PgList::<pg_sys::TargetEntry>::new();

    for (i, te) in original_tlist.iter_ptr().enumerate() {
        pgrx::warning!("    Processing entry {}: type={:?}", i, (*(*te).expr).type_);

        // Check if it's a WindowFunc without using nodecast (which might trigger validation)
        let expr = (*te).expr;
        if !expr.is_null() && (*expr).type_ == pg_sys::NodeTag::T_WindowFunc {
            pgrx::warning!("    Found WindowFunc at entry {}, replacing", i);
            // Don't cast to WindowFunc - just create a placeholder with dummy type
            pgrx::warning!(
                "      About to call make_window_func_placeholder_without_accessing_windowfunc"
            );
            let funcexpr = make_window_func_placeholder_without_accessing_windowfunc();
            pgrx::warning!("      Got funcexpr, about to call makeTargetEntry");
            // Create a new TargetEntry with the placeholder
            let new_te = pg_sys::makeTargetEntry(
                funcexpr as *mut pg_sys::Expr,
                (*te).resno,
                (*te).resname,
                (*te).resjunk,
            );
            pgrx::warning!("      Created new_te, about to push");
            new_targetlist.push(new_te);
            pgrx::warning!("      Pushed new_te");
        } else {
            // For non-WindowFunc entries, just make a flat copy
            let copied_te = pg_sys::flatCopyTargetEntry(te);
            new_targetlist.push(copied_te);
        }
    }

    pgrx::warning!("  Finished processing all entries, about to set plan.targetlist");
    (*plan).targetlist = new_targetlist.into_pg();
    pgrx::warning!("  Set plan.targetlist successfully");
}

unsafe fn make_window_func_placeholder_without_accessing_windowfunc() -> *mut pg_sys::FuncExpr {
    pgrx::warning!("      make_window_func_placeholder_without_accessing_windowfunc called");
    let funcexpr: *mut pg_sys::FuncExpr = pg_sys::palloc0(size_of::<pg_sys::FuncExpr>()).cast();
    (*funcexpr).xpr.type_ = pg_sys::NodeTag::T_FuncExpr;
    (*funcexpr).funcid = crate::api::window_function::window_func_oid();
    // Don't access window_func fields - they cause "WindowFunc found in non-WindowAgg plan node" error
    // Just use a dummy type for now (INT8) - the actual type will be determined at execution
    (*funcexpr).funcresulttype = pg_sys::INT8OID;
    (*funcexpr).funcretset = false;
    (*funcexpr).funcvariadic = false;
    (*funcexpr).funcformat = pg_sys::CoercionForm::COERCE_EXPLICIT_CALL;
    (*funcexpr).funccollid = pg_sys::InvalidOid;
    (*funcexpr).inputcollid = pg_sys::InvalidOid;
    (*funcexpr).location = -1;
    (*funcexpr).args = PgList::<pg_sys::Node>::new().into_pg();

    funcexpr
}

unsafe fn make_window_func_placeholder(
    window_func: *mut pg_sys::WindowFunc,
) -> *mut pg_sys::FuncExpr {
    // This function is kept for compatibility but shouldn't be used
    // because accessing window_func fields causes errors
    make_window_func_placeholder_without_accessing_windowfunc()
}

/// Get the Oid of a placeholder function to use in the target list of aggregate custom scans.
unsafe fn placeholder_procid() -> pg_sys::Oid {
    pgrx::direct_function_call::<pg_sys::Oid>(pg_sys::regprocedurein, &[c"now()".into_datum()])
        .expect("the `now()` function should exist")
}
