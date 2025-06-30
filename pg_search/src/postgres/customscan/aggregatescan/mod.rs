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

pub mod privdat;

use std::ffi::CStr;

use crate::nodecast;
use crate::postgres::customscan::aggregatescan::privdat::PrivateData;
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::builders::custom_state::{
    CustomScanStateBuilder, CustomScanStateWrapper,
};
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::{
    range_table, CreateUpperPathsHookArgs, CustomScan, CustomScanState, ExecMethod,
    PlainExecCapable,
};
use crate::postgres::rel_get_bm25_index;

use pgrx::{pg_sys, IntoDatum, PgList, PgTupleDesc};

#[derive(Default)]
pub struct AggregateScan;

impl CustomScan for AggregateScan {
    const NAME: &'static CStr = c"ParadeDB Aggregate Scan";
    type Args = CreateUpperPathsHookArgs;
    type State = AggregateScanState;
    type PrivateData = PrivateData;

    fn create_custom_path(builder: CustomPathBuilder<Self>) -> Option<pg_sys::CustomPath> {
        let args = builder.args();

        // We can only handle single-relation aggregates.
        if args.input_rel().reloptkind != pg_sys::RelOptKind::RELOPT_BASEREL {
            return None;
        }

        // Are there any group (/distinct/order-by) or having clauses?
        if !args.root().group_pathkeys.is_null() || args.root().hasHavingQual {
            // we can't handle GROUP BY or HAVING
            return None;
        }

        // Is it an aggregate on a single relation?
        if !is_single_relation_agg(args) {
            return None;
        }

        // Does that relation have a bm25 index?
        let parent_relids = args.input_rel().relids;
        let rte_idx = unsafe { range_table::bms_exactly_one_member(parent_relids)? };
        let rte = unsafe {
            // NOTE: The docs indicate that `simple_rte_array` is always the same length as `simple_rel_array`.
            range_table::get_rte(
                args.root().simple_rel_array_size as usize,
                args.root().simple_rte_array,
                rte_idx,
            )?
        };
        rel_get_bm25_index(unsafe { (*rte).relid })?;

        Some(builder.build())
    }

    fn plan_custom_path(mut builder: CustomScanBuilder<Self::PrivateData>) -> pg_sys::CustomScan {
        // Create a new target list which replaces aggregates with FuncExprs which will be produced
        // by our CustomScan.
        //
        // We don't use Vars here, because there doesn't seem to be a reasonable RTE to associate
        // them with.
        let mut targetlist = PgList::<pg_sys::TargetEntry>::new();
        for (te_idx, input_te) in builder.args().tlist.iter_ptr().enumerate() {
            let te = unsafe {
                if let Some(aggref) = nodecast!(Aggref, T_Aggref, (*input_te).expr) {
                    // Create a Var to replace the Aggref, with the same output type.
                    let te = pg_sys::flatCopyTargetEntry(input_te);
                    (*te).expr = make_placeholder_func_expr(aggref) as *mut pg_sys::Expr;
                    te
                } else {
                    todo!("Support non-aggregate target list entries.");
                }
            };

            targetlist.push(te);
        }
        builder.set_target_list(targetlist);

        builder.build()
    }

    fn create_custom_scan_state(
        builder: CustomScanStateBuilder<Self, Self::PrivateData>,
    ) -> *mut CustomScanStateWrapper<Self> {
        builder.build()
    }

    fn explain_custom_scan(
        state: &CustomScanStateWrapper<Self>,
        ancestors: *mut pg_sys::List,
        explainer: &mut Explainer,
    ) {
        println!("TODO: explain_custom_scan")
    }

    fn begin_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        estate: *mut pg_sys::EState,
        eflags: i32,
    ) {
        println!("TODO: begin_custom_scan")
    }

    fn rescan_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        state.custom_state_mut().has_emitted = false;
    }

    fn exec_custom_scan(state: &mut CustomScanStateWrapper<Self>) -> *mut pg_sys::TupleTableSlot {
        if state.custom_state().has_emitted {
            return std::ptr::null_mut();
        }
        state.custom_state_mut().has_emitted = true;

        unsafe {
            let tupdesc = PgTupleDesc::from_pg_unchecked((*state.planstate()).ps_ResultTupleDesc);
            let slot = pg_sys::MakeTupleTableSlot(
                (*state.planstate()).ps_ResultTupleDesc,
                &pg_sys::TTSOpsVirtual,
            );
            let natts = (*(*slot).tts_tupleDescriptor).natts as usize;

            (*slot).tts_flags &= !pg_sys::TTS_FLAG_EMPTY as u16;
            (*slot).tts_flags |= pg_sys::TTS_FLAG_SHOULDFREE as u16;
            (*slot).tts_nvalid = natts as _;

            let datums = std::slice::from_raw_parts_mut((*slot).tts_values, natts);
            let isnull = std::slice::from_raw_parts_mut((*slot).tts_isnull, natts);

            // TODO: Actually execute.
            for (i, att) in tupdesc.iter().enumerate() {
                datums[i] = 1337.into_datum().unwrap();
                isnull[i] = false;
            }

            slot
        }
    }

    fn shutdown_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        println!("TODO: shutdown_custom_scan")
    }

    fn end_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        println!("TODO: end_custom_scan")
    }
}

fn is_single_relation_agg(args: &CreateUpperPathsHookArgs) -> bool {
    // The PathTarget `exprs` are the closest that we have to a target list at this point.
    let target_list =
        unsafe { PgList::<pg_sys::Expr>::from_pg((*args.output_rel().reltarget).exprs) };
    if target_list.len() != 1 {
        return false;
    }

    // The single target list entry must be a `count(*)` (aggstar).
    unsafe {
        let expr = target_list
            .iter_ptr()
            .next()
            .expect("target list is non-empty");
        let Some(aggref) = nodecast!(Aggref, T_Aggref, expr) else {
            return false;
        };
        (*aggref).aggstar
    }
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

// TODO: Obviously not the one we actually want.
unsafe fn placeholder_procid() -> pg_sys::Oid {
    pgrx::direct_function_call::<pg_sys::Oid>(
            pg_sys::regprocedurein,
            // NB:  the SQL signature here needs to match our Rust implementation
            &[c"paradedb.term_with_operator(paradedb.fieldname, text, anyelement)".into_datum()],
        )
            .expect("the `paradedb.term_with_operator(paradedb.fieldname, text, anyelement)` function should exist")
}

impl ExecMethod for AggregateScan {
    fn exec_methods() -> *const pg_sys::CustomExecMethods {
        <AggregateScan as PlainExecCapable>::exec_methods()
    }
}

impl PlainExecCapable for AggregateScan {}

#[derive(Default)]
pub struct AggregateScanState {
    // True if we have already emitted a tuple.
    has_emitted: bool,
}

impl CustomScanState for AggregateScanState {
    fn init_exec_method(&mut self, cstate: *mut pg_sys::CustomScanState) {
        todo!("TODO: init_exec_method")
    }
}
