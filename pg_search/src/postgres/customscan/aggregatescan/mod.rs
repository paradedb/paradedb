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

use crate::aggregate::execute_aggregate;
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
use crate::postgres::customscan::qual_inspect::{extract_quals, QualExtractState};
use crate::postgres::customscan::{
    range_table, CreateUpperPathsHookArgs, CustomScan, CustomScanState, ExecMethod,
    PlainExecCapable,
};
use crate::postgres::{rel_get_bm25_index, PgSearchRelation};
use crate::query::SearchQueryInput;

use pgrx::{pg_sys, IntoDatum, PgList, PgTupleDesc};
use tantivy::Index;
use tinyvec::TinyVec;

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
        let index =
            Index::open(directory).expect("aggregate_custom_scan: should be able to open index");
        let schema = bm25_index
            .schema()
            .expect("aggregate_custom_scan: should have a schema");

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
                &mut QualExtractState::default(),
            )?
        };

        Some(builder.build(PrivateData {
            aggregate_types,
            indexrelid: bm25_index.oid(),
            query: SearchQueryInput::from(&quals),
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
        builder.custom_state().indexrelid = builder.custom_private().indexrelid;
        builder.custom_state().query = builder.custom_private().query.clone();
        builder.build()
    }

    fn explain_custom_scan(
        state: &CustomScanStateWrapper<Self>,
        ancestors: *mut pg_sys::List,
        explainer: &mut Explainer,
    ) {
        // TODO: Expand with additional information about the scan.
    }

    fn begin_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        estate: *mut pg_sys::EState,
        eflags: i32,
    ) {
    }

    fn rescan_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        state.custom_state_mut().state = ExecutionState::NotStarted;
    }

    fn exec_custom_scan(state: &mut CustomScanStateWrapper<Self>) -> *mut pg_sys::TupleTableSlot {
        let next = match &mut state.custom_state_mut().state {
            ExecutionState::Completed => return std::ptr::null_mut(),
            ExecutionState::NotStarted => {
                // Execute the aggregate, and change the state to Emitting.
                let mut row_iter = execute(state);
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
            let natts = (*(*slot).tts_tupleDescriptor).natts as usize;

            assert_eq!(natts, row.len());

            (*slot).tts_flags &= !pg_sys::TTS_FLAG_EMPTY as u16;
            (*slot).tts_flags |= pg_sys::TTS_FLAG_SHOULDFREE as u16;
            (*slot).tts_nvalid = natts as _;

            let datums = std::slice::from_raw_parts_mut((*slot).tts_values, natts);
            let isnull = std::slice::from_raw_parts_mut((*slot).tts_isnull, natts);

            for (i, col_val) in row.into_iter().enumerate() {
                datums[i] = col_val.into_datum().unwrap();
                isnull[i] = false;
            }

            slot
        }
    }

    fn shutdown_custom_scan(state: &mut CustomScanStateWrapper<Self>) {}

    fn end_custom_scan(state: &mut CustomScanStateWrapper<Self>) {}
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

/// Get the Oid of a placeholder function to use in the target list of aggregate custom scans.
unsafe fn placeholder_procid() -> pg_sys::Oid {
    pgrx::direct_function_call::<pg_sys::Oid>(pg_sys::regprocedurein, &[c"now()".into_datum()])
        .expect("the `now()` function should exist")
}

fn execute(state: &CustomScanStateWrapper<AggregateScan>) -> std::vec::IntoIter<AggregateRow> {
    // TODO: Opening of the index could be deduped between custom scans: see
    // `PdbScanState::open_relations`.
    let indexrel = PgSearchRelation::with_lock(
        state.custom_state().indexrelid,
        pg_sys::AccessShareLock as _,
    );
    let relation = unsafe { pgrx::PgRelation::from_pg(indexrel.as_ptr()) };

    let result = execute_aggregate(
        relation,
        state.custom_state().query.clone(),
        state.custom_state().aggregates_to_json(),
        // TODO: Consider adding a GUC to control whether we solve MVCC.
        true,      // solve_mvcc
        500000000, // memory_limit
        65000,     // bucket_limit
    )
    .expect("failed to execute aggregate");

    // Note: Since we don't support GROUP BY, we only ever have one result row.
    state
        .custom_state()
        .json_to_aggregate_results(result)
        .into_iter()
}

impl ExecMethod for AggregateScan {
    fn exec_methods() -> *const pg_sys::CustomExecMethods {
        <AggregateScan as PlainExecCapable>::exec_methods()
    }
}

impl PlainExecCapable for AggregateScan {}

// TODO: This should match the output types of the extracted aggregate functions. For now we only
// support COUNT.
type AggregateRow = TinyVec<[i64; 4]>;

#[derive(Default)]
enum ExecutionState {
    #[default]
    NotStarted,
    Emitting(std::vec::IntoIter<AggregateRow>),
    Completed,
}

#[derive(Default)]
pub struct AggregateScanState {
    // The state of this scan.
    state: ExecutionState,
    // The aggregate types that we are executing for.
    aggregate_types: Vec<AggregateType>,
    // The index that will be scanned.
    indexrelid: pg_sys::Oid,
    // The query that will be executed.
    query: SearchQueryInput,
}

impl AggregateScanState {
    fn aggregates_to_json(&self) -> serde_json::Value {
        serde_json::Value::Object(
            self.aggregate_types
                .iter()
                .enumerate()
                .map(|(idx, aggregate)| (idx.to_string(), aggregate.to_json()))
                .collect(),
        )
    }

    fn json_to_aggregate_results(&self, result: serde_json::Value) -> Vec<AggregateRow> {
        let result_map = result
            .as_object()
            .expect("unexpected aggregate result collection type");

        let row = self
            .aggregate_types
            .iter()
            .enumerate()
            .map(move |(idx, aggregate)| {
                let aggregate_val = result_map
                    .get(&idx.to_string())
                    .expect("missing aggregate result")
                    .as_object()
                    .expect("unexpected aggregate structure")
                    .get("value")
                    .expect("missing aggregate result value")
                    .as_number()
                    .expect("unexpected aggregate result type");

                aggregate.result_from_json(aggregate_val)
            })
            .collect::<AggregateRow>();

        vec![row]
    }
}

impl CustomScanState for AggregateScanState {
    fn init_exec_method(&mut self, cstate: *mut pg_sys::CustomScanState) {
        // TODO: Unused currently. See the comment on `trait CustomScanState` regarding making this
        // more useful.
    }
}
