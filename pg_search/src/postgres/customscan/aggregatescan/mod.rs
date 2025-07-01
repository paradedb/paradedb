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

use crate::api::operator::anyelement_query_input_opoid;
use crate::index::mvcc::MvccSatisfies;
use crate::nodecast;
use crate::postgres::customscan::aggregatescan::privdat::{AggregateType, PrivateData};
use crate::postgres::customscan::builders::custom_path::{
    restrict_info, CustomPathBuilder, RestrictInfoType,
};
use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::builders::custom_state::{
    CustomScanStateBuilder, CustomScanStateWrapper,
};
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::qual_inspect::extract_quals;
use crate::postgres::customscan::{
    range_table, CreateUpperPathsHookArgs, CustomScan, CustomScanState, ExecMethod,
    PlainExecCapable,
};
use crate::postgres::rel_get_bm25_index;
use crate::query::SearchQueryInput;
use crate::schema::SearchIndexSchema;

use pgrx::{pg_sys, IntoDatum, PgList, PgTupleDesc};
use tantivy::Index;

#[derive(Default)]
pub struct AggregateScan;

impl CustomScan for AggregateScan {
    const NAME: &'static CStr = c"ParadeDB Aggregate Scan";
    type Args = CreateUpperPathsHookArgs;
    type State = AggregateScanState;
    type PrivateData = PrivateData;

    fn create_custom_path(builder: CustomPathBuilder<Self>) -> Option<pg_sys::CustomPath> {
        let args = builder.args();

        // We can only handle single relations.
        if args.input_rel().reloptkind != pg_sys::RelOptKind::RELOPT_BASEREL {
            return None;
        }
        let (restrict_info, ri_type) = restrict_info(builder.args().input_rel());
        if !matches!(ri_type, RestrictInfoType::BaseRelation) {
            // this relation is a join, or has no restrictions (WHERE clause predicates), so there's no need
            // for us to do anything
            return None;
        }

        // Are there any group (/distinct/order-by) or having clauses?
        if !args.root().group_pathkeys.is_null() || args.root().hasHavingQual {
            // We can't handle GROUP BY or HAVING
            return None;
        }

        // Is the target list entirely aggregates?
        let aggregate_types = extract_aggregates(args)?;

        // Does that relation have a bm25 index?
        let parent_relids = args.input_rel().relids;
        let rti = unsafe { range_table::bms_exactly_one_member(parent_relids)? };
        let rte = unsafe {
            // NOTE: The docs indicate that `simple_rte_array` is always the same length
            // as `simple_rel_array`.
            range_table::get_rte(
                args.root().simple_rel_array_size as usize,
                args.root().simple_rte_array,
                rti,
            )?
        };
        let (table, bm25_index) = rel_get_bm25_index(unsafe { (*rte).relid })?;
        let directory = MvccSatisfies::LargestSegment.directory(&bm25_index);
        let index = Index::open(directory).expect("custom_scan: should be able to open index");
        let schema = SearchIndexSchema::from_index(&bm25_index, &index);

        // Can we handle all of the quals?
        let quals = unsafe {
            extract_quals(
                args.root,
                rti,
                restrict_info.as_ptr().cast(),
                anyelement_query_input_opoid(),
                ri_type,
                &schema,
                false, // Base relation quals should not convert external to all
                &mut false,
            )?
        };

        Some(builder.build(PrivateData {
            query: SearchQueryInput::from(&quals),
            aggregate_types,
        }))
    }

    fn plan_custom_path(mut builder: CustomScanBuilder<Self>) -> pg_sys::CustomScan {
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
        mut builder: CustomScanStateBuilder<Self, Self::PrivateData>,
    ) -> *mut CustomScanStateWrapper<Self> {
        builder.custom_state().aggregate_types = builder.custom_private().aggregate_types.clone();
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

            assert_eq!(natts, state.custom_state().aggregate_types.len());

            (*slot).tts_flags &= !pg_sys::TTS_FLAG_EMPTY as u16;
            (*slot).tts_flags |= pg_sys::TTS_FLAG_SHOULDFREE as u16;
            (*slot).tts_nvalid = natts as _;

            let datums = std::slice::from_raw_parts_mut((*slot).tts_values, natts);
            let isnull = std::slice::from_raw_parts_mut((*slot).tts_isnull, natts);

            // TODO: Actually execute using the `AggregateType` on the `PrivateData`.
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

/// If the given args consist only of AggregateTypes that we can handle, return them.
fn extract_aggregates(args: &CreateUpperPathsHookArgs) -> Option<Vec<AggregateType>> {
    // The PathTarget `exprs` are the closest that we have to a target list at this point.
    let target_list =
        unsafe { PgList::<pg_sys::Expr>::from_pg((*args.output_rel().reltarget).exprs) };
    if target_list.is_empty() {
        return None;
    }

    // We must recognize all target list entries as supported aggregates.
    let mut aggregate_types = Vec::new();
    for expr in target_list.iter_ptr() {
        unsafe {
            let aggref = nodecast!(Aggref, T_Aggref, expr)?;
            if (*aggref).aggstar {
                // Only `count(*)` (aggstar) is supported.
                aggregate_types.push(AggregateType::Count);
            } else {
                return None;
            }
        }
    }
    Some(aggregate_types)
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
    // The aggregate types that we are executing for.
    aggregate_types: Vec<AggregateType>,
}

impl CustomScanState for AggregateScanState {
    fn init_exec_method(&mut self, cstate: *mut pg_sys::CustomScanState) {
        todo!("TODO: init_exec_method")
    }
}
