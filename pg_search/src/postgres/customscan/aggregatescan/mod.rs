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
pub mod scan_state;

use std::ffi::CStr;

use crate::aggregate::execute_aggregate;
use crate::api::operator::anyelement_query_input_opoid;
use crate::gucs;
use crate::index::mvcc::MvccSatisfies;
use crate::nodecast;
use crate::postgres::customscan::aggregatescan::privdat::{
    AggregateType, GroupingColumn, PrivateData,
};
use crate::postgres::customscan::aggregatescan::scan_state::{
    AggregateScanState, ExecutionState, GroupedAggregateRow,
};
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
    range_table, CreateUpperPathsHookArgs, CustomScan, ExecMethod, PlainExecCapable,
};
use crate::postgres::rel_get_bm25_index;
use crate::postgres::var::find_var_relation;
use crate::query::SearchQueryInput;
use crate::schema::SearchIndexSchema;

use pgrx::{pg_sys, IntoDatum, PgList, PgRelation, PgTupleDesc};
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

        // Check if there are restrictions (WHERE clause)
        let (restrict_info, ri_type) = restrict_info(builder.args().input_rel());
        if !matches!(ri_type, RestrictInfoType::BaseRelation) {
            // This relation is a join, or has no restrictions (WHERE clause predicates), so there's no need
            // for us to do anything.
            return None;
        }

        // Are there any group (/distinct/order-by) or having clauses?
        // We now support GROUP BY, but not HAVING yet
        if args.root().hasHavingQual {
            // We can't handle HAVING yet
            return None;
        }

        // Extract grouping columns if present
        let group_pathkeys = if args.root().group_pathkeys.is_null() {
            None
        } else {
            Some(unsafe { PgList::<pg_sys::PathKey>::from_pg(args.root().group_pathkeys) })
        };

        // Is the target list entirely aggregates?
        let aggregate_types = extract_aggregates(args)?;

        // Is there a single relation with a bm25 index?
        let parent_relids = args.input_rel().relids;
        let heap_rti = unsafe { range_table::bms_exactly_one_member(parent_relids)? };
        let heap_rte = unsafe {
            // NOTE: The docs indicate that `simple_rte_array` is always the same length
            // as `simple_rel_array`.
            range_table::get_rte(
                args.root().simple_rel_array_size as usize,
                args.root().simple_rte_array,
                heap_rti,
            )?
        };
        let (table, bm25_index) = rel_get_bm25_index(unsafe { (*heap_rte).relid })?;
        let directory = MvccSatisfies::LargestSegment.directory(&bm25_index);
        let index =
            Index::open(directory).expect("aggregate_custom_scan: should be able to open index");
        let schema = bm25_index
            .schema()
            .expect("aggregate_custom_scan: should have a schema");

        // Extract grouping columns and validate they are fast fields
        let grouping_columns = if let Some(pathkeys) = group_pathkeys {
            extract_grouping_columns(&pathkeys, args.root, heap_rti, &schema)?
        } else {
            vec![]
        };

        // Can we handle all of the quals?
        let query = unsafe {
            let result = extract_quals(
                args.root,
                heap_rti,
                restrict_info.as_ptr().cast(),
                anyelement_query_input_opoid(),
                ri_type,
                &schema,
                false, // Base relation quals should not convert external to all
                &mut QualExtractState::default(),
            );
            SearchQueryInput::from(&result?)
        };

        Some(builder.build(PrivateData {
            aggregate_types,
            indexrelid: bm25_index.oid(),
            heap_rti,
            query,
            grouping_columns,
        }))
    }

    fn plan_custom_path(mut builder: CustomScanBuilder<Self>) -> pg_sys::CustomScan {
        // Create a new target list which includes grouping columns and replaces aggregates
        // with FuncExprs which will be produced by our CustomScan.
        //
        // We don't use Vars here, because there doesn't seem to be a reasonable RTE to associate
        // them with.
        let mut targetlist = PgList::<pg_sys::TargetEntry>::new();
        let grouping_columns = &builder.custom_private().grouping_columns;

        // For GROUP BY queries, we need to ensure grouping columns appear in the target list
        // The target list should be: grouping_cols + aggregate_cols

        for (te_idx, input_te) in builder.args().tlist.iter_ptr().enumerate() {
            let te = unsafe {
                if let Some(var) = nodecast!(Var, T_Var, (*input_te).expr) {
                    // This is a Var - it should be a grouping column
                    // Keep it as-is
                    pg_sys::flatCopyTargetEntry(input_te)
                } else if let Some(aggref) = nodecast!(Aggref, T_Aggref, (*input_te).expr) {
                    // This is an aggregate - replace with placeholder FuncExpr
                    let te = pg_sys::flatCopyTargetEntry(input_te);
                    (*te).expr = make_placeholder_func_expr(aggref) as *mut pg_sys::Expr;
                    te
                } else {
                    // For now, we only support Vars (grouping cols) and Aggrefs
                    todo!(
                        "Support other target list entry types: {:?}",
                        (*input_te).expr
                    );
                }
            };

            targetlist.push(te);
        }
        builder.set_targetlist(targetlist);
        builder.set_scanrelid(builder.custom_private().heap_rti);

        builder.build()
    }

    fn create_custom_scan_state(
        mut builder: CustomScanStateBuilder<Self, Self::PrivateData>,
    ) -> *mut CustomScanStateWrapper<Self> {
        builder.custom_state().aggregate_types = builder.custom_private().aggregate_types.clone();
        builder.custom_state().grouping_columns = builder.custom_private().grouping_columns.clone();
        builder.custom_state().indexrelid = builder.custom_private().indexrelid;
        builder.custom_state().query = builder.custom_private().query.clone();
        builder.custom_state().execution_rti =
            unsafe { (*builder.args().cscan).scan.scanrelid as pg_sys::Index };
        builder.build()
    }

    fn explain_custom_scan(
        state: &CustomScanStateWrapper<Self>,
        ancestors: *mut pg_sys::List,
        explainer: &mut Explainer,
    ) {
        explainer.add_text("Index", state.custom_state().indexrel().name());
        explainer.add_query(&state.custom_state().query);
        explainer.add_text(
            "Aggregate Definition",
            serde_json::to_string(&state.custom_state().aggregates_to_json())
                .expect("Failed to serialize aggregate definition."),
        );
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
            // TODO: Opening of the index could be deduped between custom scans: see
            // `PdbScanState::open_relations`.
            state.custom_state_mut().open_relations(lockmode);
        }
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

            // Calculate expected number of attributes (grouping cols + aggregate cols)
            let n_grouping_cols = state.custom_state().grouping_columns.len();
            let n_agg_cols = row.aggregate_values.len();
            let expected_natts = n_grouping_cols + n_agg_cols;
            let natts = (*(*slot).tts_tupleDescriptor).natts as usize;

            assert_eq!(
                natts, expected_natts,
                "Mismatch between expected and actual tuple attributes"
            );

            (*slot).tts_flags &= !pg_sys::TTS_FLAG_EMPTY as u16;
            (*slot).tts_flags |= pg_sys::TTS_FLAG_SHOULDFREE as u16;
            (*slot).tts_nvalid = natts as _;

            let datums = std::slice::from_raw_parts_mut((*slot).tts_values, natts);
            let isnull = std::slice::from_raw_parts_mut((*slot).tts_isnull, natts);

            // Fill in grouping column values
            for (i, group_val) in row.group_keys.iter().enumerate() {
                pgrx::warning!("group_val: {:?}", group_val);
                // Get the type of this grouping column from the tuple descriptor
                let attr = tupdesc.get(i).expect("missing attribute");
                let typoid = attr.type_oid().value();

                // Convert the string value to the appropriate type
                datums[i] = match typoid {
                    t if t == pg_sys::TEXTOID
                        || t == pg_sys::VARCHAROID
                        || t == pg_sys::BPCHAROID =>
                    {
                        // Text types - use the string as-is
                        group_val.clone().into_datum().unwrap()
                    }
                    t if t == pg_sys::INT2OID => {
                        // smallint
                        let val: i16 = group_val.parse().expect("invalid int2 value");
                        val.into_datum().unwrap()
                    }
                    t if t == pg_sys::INT4OID => {
                        // integer
                        let val: i32 = group_val.parse().expect("invalid int4 value");
                        val.into_datum().unwrap()
                    }
                    t if t == pg_sys::INT8OID => {
                        // bigint
                        let val: i64 = group_val.parse().expect("invalid int8 value");
                        val.into_datum().unwrap()
                    }
                    t if t == pg_sys::FLOAT4OID => {
                        // real
                        let val: f32 = group_val.parse().expect("invalid float4 value");
                        val.into_datum().unwrap()
                    }
                    t if t == pg_sys::FLOAT8OID => {
                        // double precision
                        let val: f64 = group_val.parse().expect("invalid float8 value");
                        val.into_datum().unwrap()
                    }
                    t if t == pg_sys::BOOLOID => {
                        // boolean
                        let val: bool = group_val.parse().expect("invalid bool value");
                        val.into_datum().unwrap()
                    }
                    _ => {
                        panic!("Unsupported grouping column type: OID {typoid}");
                    }
                };
                isnull[i] = false;
            }

            // Fill in aggregate values
            for (i, agg_val) in row.aggregate_values.into_iter().enumerate() {
                datums[n_grouping_cols + i] = agg_val.into_datum().unwrap();
                isnull[n_grouping_cols + i] = false;
            }

            slot
        }
    }

    fn shutdown_custom_scan(state: &mut CustomScanStateWrapper<Self>) {}

    fn end_custom_scan(state: &mut CustomScanStateWrapper<Self>) {}
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

                // We only support simple Var expressions for now
                if let Some(var) = nodecast!(Var, T_Var, expr) {
                    let (heaprelid, attno, _) = find_var_relation(var, root);
                    if heaprelid == pg_sys::Oid::INVALID {
                        continue;
                    }

                    let heaprel = PgRelation::with_lock(heaprelid, pg_sys::AccessShareLock as _);
                    let tupdesc = heaprel.tuple_desc();
                    if let Some(att) = tupdesc.get(attno as usize - 1) {
                        let field_name = att.name();

                        // Check if this field exists in the index schema as a fast field
                        if let Some(search_field) = schema.search_field(field_name) {
                            let is_fast = search_field.is_fast();
                            if is_fast {
                                grouping_columns.push(GroupingColumn {
                                    field_name: field_name.to_string(),
                                    attno,
                                });
                                found_valid_column = true;
                                break; // Found a valid grouping column for this pathkey
                            }
                        }
                    }
                }
            }

            if !found_valid_column {
                return None;
            }
        }
    }

    // We must be able to handle all grouping columns
    let success = grouping_columns.len() == pathkeys.len();
    if success {
        Some(grouping_columns)
    } else {
        None
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

    // We must recognize all target list entries as either grouping columns (Vars) or supported aggregates.
    let mut aggregate_types = Vec::new();
    for expr in target_list.iter_ptr() {
        unsafe {
            if let Some(_var) = nodecast!(Var, T_Var, expr) {
                // This is a Var - it should be a grouping column, skip it
                continue;
            } else if let Some(aggref) = nodecast!(Aggref, T_Aggref, expr) {
                if (*aggref).aggstar {
                    // Only `count(*)` (aggstar) is supported.
                    aggregate_types.push(AggregateType::Count);
                } else {
                    return None;
                }
            } else {
                // Unsupported expression type
                return None;
            }
        }
    }

    // We need at least one aggregate
    if aggregate_types.is_empty() {
        None
    } else {
        Some(aggregate_types)
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

/// Get the Oid of a placeholder function to use in the target list of aggregate custom scans.
unsafe fn placeholder_procid() -> pg_sys::Oid {
    pgrx::direct_function_call::<pg_sys::Oid>(pg_sys::regprocedurein, &[c"now()".into_datum()])
        .expect("the `now()` function should exist")
}

fn execute(
    state: &CustomScanStateWrapper<AggregateScan>,
) -> std::vec::IntoIter<GroupedAggregateRow> {
    let result = execute_aggregate(
        state.custom_state().indexrel(),
        state.custom_state().query.clone(),
        state.custom_state().aggregates_to_json(),
        // TODO: Consider adding a GUC to control whether we solve MVCC.
        true,                                              // solve_mvcc
        gucs::adjust_work_mem().get().try_into().unwrap(), // memory_limit
        65000,                                             // bucket_limit
    )
    .expect("failed to execute aggregate");

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
