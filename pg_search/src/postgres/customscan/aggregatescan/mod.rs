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
    AggregateType, AggregateValue, GroupingColumn, PrivateData, TargetListEntry,
};
use crate::postgres::customscan::aggregatescan::scan_state::{
    AggregateScanState, ExecutionState, GroupedAggregateRow,
};
use crate::postgres::customscan::builders::custom_path::{
    restrict_info, CustomPathBuilder, OrderByStyle, RestrictInfoType,
};
use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::builders::custom_state::{
    CustomScanStateBuilder, CustomScanStateWrapper,
};
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::pdbscan::{
    extract_pathkey_styles_with_sortability_check, PathKeyInfo,
};
use crate::postgres::customscan::qual_inspect::{extract_quals, QualExtractState};
use crate::postgres::customscan::{
    range_table, CreateUpperPathsHookArgs, CustomScan, ExecMethod, PlainExecCapable,
};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::rel_get_bm25_index;
use crate::postgres::types::TantivyValue;
use crate::postgres::var::{find_one_var_and_fieldname, find_var_relation, VarContext};
use crate::query::SearchQueryInput;
use crate::schema::SearchIndexSchema;
use pgrx::{pg_sys, IntoDatum, PgList, PgTupleDesc};
use tantivy::schema::OwnedValue;
use tantivy::Index;

#[derive(Default)]
pub struct AggregateScan;

impl CustomScan for AggregateScan {
    const NAME: &'static CStr = c"ParadeDB Aggregate Scan";
    type Args = CreateUpperPathsHookArgs;
    type State = AggregateScanState;
    type PrivateData = PrivateData;

    fn create_custom_path(mut builder: CustomPathBuilder<Self>) -> Option<pg_sys::CustomPath> {
        let args = builder.args();

        // We can only handle single base relations as input
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
        // We can't handle HAVING yet
        if args.root().hasHavingQual {
            // We can't handle HAVING yet
            return None;
        }

        // Check for DISTINCT - we can't handle DISTINCT queries
        unsafe {
            let parse = args.root().parse;
            if !parse.is_null() && (!(*parse).distinctClause.is_null() || (*parse).hasDistinctOn) {
                return None;
            }
        }

        // Extract grouping columns if present
        let group_pathkeys = if args.root().group_pathkeys.is_null() {
            None
        } else {
            Some(unsafe { PgList::<pg_sys::PathKey>::from_pg(args.root().group_pathkeys) })
        };

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
        let grouping_columns = if let Some(ref pathkeys) = group_pathkeys {
            // This will return None if any grouping column is not a fast field
            extract_grouping_columns(pathkeys, args.root, heap_rti, &schema)?
        } else {
            vec![]
        };

        // Extract and validate aggregates - must have schema for field validation
        let aggregate_types = extract_and_validate_aggregates(args, &schema, &grouping_columns)?;

        // Extract ORDER BY pathkeys if present
        let order_pathkey_info = extract_order_by_pathkeys(args.root, heap_rti, &schema);
        let orderby_info = OrderByStyle::extract_orderby_info(order_pathkey_info.pathkeys());

        // Can we handle all of the quals?
        let query = unsafe {
            let result = extract_quals(
                args.root,
                heap_rti,
                restrict_info.as_ptr().cast(),
                anyelement_query_input_opoid(),
                ri_type,
                &bm25_index,
                false, // Base relation quals should not convert external to all
                &mut QualExtractState::default(),
            );
            SearchQueryInput::from(&result?)
        };

        // Check if any GROUP BY field is also being searched (conflicts with Tantivy aggregation)
        // Tantivy cannot handle having aggregate function columns in the GROUP BY clause (e.g.,
        // 'SELECT AVG(rating) FROM products GROUP BY rating').
        if has_search_field_conflicts(&grouping_columns, &query) {
            return None;
        }

        // If we're handling ORDER BY, we need to inform PostgreSQL that our output is sorted.
        // To do this, we set pathkeys for ORDER BY if present.
        if let Some(pathkeys) = order_pathkey_info.pathkeys() {
            for pathkey_style in pathkeys {
                builder = builder.add_path_key(pathkey_style);
            }
        };

        Some(builder.build(PrivateData {
            aggregate_types,
            indexrelid: bm25_index.oid(),
            heap_rti,
            query,
            grouping_columns,
            orderby_info,
            target_list_mapping: vec![], // Will be filled in plan_custom_path
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
        let mut target_list_mapping = Vec::new();
        let mut agg_idx = 0;

        for (te_idx, input_te) in builder.args().tlist.iter_ptr().enumerate() {
            let te = unsafe {
                if let Some(var) = nodecast!(Var, T_Var, (*input_te).expr) {
                    // This is a Var - it should be a grouping column
                    // Find which grouping column this is
                    let mut found = false;
                    for (i, gc) in grouping_columns.iter().enumerate() {
                        if (*var).varattno == gc.attno {
                            target_list_mapping.push(TargetListEntry::GroupingColumn(i));
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        panic!("Var in target list not found in grouping columns");
                    }
                    // Keep it as-is
                    pg_sys::flatCopyTargetEntry(input_te)
                } else if let Some(aggref) = nodecast!(Aggref, T_Aggref, (*input_te).expr) {
                    // This is an aggregate - replace with placeholder FuncExpr
                    target_list_mapping.push(TargetListEntry::Aggregate(agg_idx));
                    agg_idx += 1;

                    let te = pg_sys::flatCopyTargetEntry(input_te);
                    (*te).expr = make_placeholder_func_expr(aggref) as *mut pg_sys::Expr;
                    te
                } else if let Some(_opexpr) = nodecast!(OpExpr, T_OpExpr, (*input_te).expr) {
                    // This might be a JSON operator expression - verify and find matching grouping column
                    let var_context = VarContext::from_planner(builder.args().root);
                    let (var, field_name) = find_one_var_and_fieldname(
                        var_context,
                        (*input_te).expr as *mut pg_sys::Node,
                    )
                    .expect("OpExpr in target list is not a recognized JSON operator expression");

                    // Find which grouping column this expression matches
                    let mut found_idx = None;
                    for (i, gc) in grouping_columns.iter().enumerate() {
                        if (*var).varattno == gc.attno && gc.field_name == field_name.to_string() {
                            found_idx = Some(i);
                            break;
                        }
                    }

                    let idx = found_idx.expect(
                        "OpExpr in target list does not match any detected grouping column",
                    );
                    target_list_mapping.push(TargetListEntry::GroupingColumn(idx));
                    // Keep it as-is
                    pg_sys::flatCopyTargetEntry(input_te)
                } else {
                    // Other expression types we don't support yet
                    panic!(
                        "Unsupported target list entry type: node tag {:?}",
                        (*(*input_te).expr).type_
                    );
                }
            };

            targetlist.push(te);
        }

        // Update the private data with the target list mapping
        builder.custom_private_mut().target_list_mapping = target_list_mapping;

        builder.set_targetlist(targetlist);
        builder.set_scanrelid(builder.custom_private().heap_rti);

        builder.build()
    }

    fn create_custom_scan_state(
        mut builder: CustomScanStateBuilder<Self, Self::PrivateData>,
    ) -> *mut CustomScanStateWrapper<Self> {
        builder.custom_state().aggregate_types = builder.custom_private().aggregate_types.clone();
        builder.custom_state().grouping_columns = builder.custom_private().grouping_columns.clone();
        builder.custom_state().orderby_info = builder.custom_private().orderby_info.clone();
        builder.custom_state().target_list_mapping =
            builder.custom_private().target_list_mapping.clone();
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

            let natts = (*(*slot).tts_tupleDescriptor).natts as usize;
            let target_list_mapping = &state.custom_state().target_list_mapping;

            assert_eq!(
                natts,
                target_list_mapping.len(),
                "Target list mapping length mismatch"
            );

            // Simple slot setup
            pg_sys::ExecClearTuple(slot);

            let datums = std::slice::from_raw_parts_mut((*slot).tts_values, natts);
            let isnull = std::slice::from_raw_parts_mut((*slot).tts_isnull, natts);

            // Fill in values according to the target list mapping
            for (i, entry) in target_list_mapping.iter().enumerate() {
                match entry {
                    &TargetListEntry::GroupingColumn(gc_idx) => {
                        let group_val = row.group_keys[gc_idx].clone();
                        let attr = tupdesc.get(i).expect("missing attribute");
                        let typoid = attr.type_oid().value();

                        let (datum, is_null) = convert_group_value_to_datum(group_val, typoid);
                        datums[i] = datum;
                        isnull[i] = is_null;
                    }
                    TargetListEntry::Aggregate(agg_idx) => {
                        let agg_value = &row.aggregate_values[*agg_idx];
                        let attr = tupdesc.get(i).expect("missing attribute");
                        let expected_typoid = attr.type_oid().value();

                        let (datum, is_null) =
                            convert_aggregate_value_to_datum(agg_value, expected_typoid);
                        datums[i] = datum;
                        isnull[i] = is_null;
                    }
                }
            }

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

/// Convert a group value (OwnedValue) to a PostgreSQL Datum
unsafe fn convert_group_value_to_datum(
    group_val: OwnedValue,
    typoid: pg_sys::Oid,
) -> (pg_sys::Datum, bool) {
    let oid = pgrx::PgOid::from(typoid);
    let tantivy_value = TantivyValue(group_val);
    match tantivy_value.try_into_datum(oid) {
        Ok(Some(datum)) => (datum, false),
        Ok(None) => (pg_sys::Datum::from(0), true),
        Err(e) => {
            panic!("Failed to convert TantivyValue to datum: {e:?}");
        }
    }
}

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
    };

    // Determine the best target type for conversion
    // For numeric compatibility, prefer wider types when converting floats to integer types
    let target_oid = match (&owned_value, expected_typoid) {
        // For null values, use the expected type
        (OwnedValue::Null, _) => expected_typoid,

        // For integer values, use the expected type directly
        (OwnedValue::I64(_), _) => expected_typoid,

        // For float values, be more lenient with integer target types
        (OwnedValue::F64(_), pg_sys::INT2OID) => pg_sys::INT8OID, // Use BIGINT instead of SMALLINT
        (OwnedValue::F64(_), pg_sys::INT4OID) => pg_sys::INT8OID, // Use BIGINT instead of INTEGER
        (OwnedValue::F64(_), _) => expected_typoid,               // Keep other types as-is

        // Default case
        _ => expected_typoid,
    };

    let tantivy_value = TantivyValue(owned_value);
    unsafe {
        match tantivy_value.try_into_datum(pgrx::PgOid::from(target_oid)) {
            Ok(Some(datum)) => (datum, false),
            Ok(None) => (pg_sys::Datum::null(), true),
            Err(e) => (pg_sys::Datum::null(), true),
        }
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
                    if heaprelid == pg_sys::Oid::INVALID {
                        continue;
                    }
                    (field_name.to_string(), attno)
                } else if let Some(var) = nodecast!(Var, T_Var, expr) {
                    // Simple Var - extract field name from attribute
                    let (heaprelid, attno, _) = find_var_relation(var, root);
                    if heaprelid == pg_sys::Oid::INVALID {
                        continue;
                    }

                    let heaprel =
                        PgSearchRelation::with_lock(heaprelid, pg_sys::AccessShareLock as _);
                    let tupdesc = heaprel.tuple_desc();
                    if let Some(att) = tupdesc.get(attno as usize - 1) {
                        (att.name().to_string(), attno)
                    } else {
                        continue;
                    }
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
        }
    }

    Some(grouping_columns)
}

/// Extract and validate aggregates, ensuring all aggregate fields are compatible fast fields
/// and don't conflict with GROUP BY columns
fn extract_and_validate_aggregates(
    args: &CreateUpperPathsHookArgs,
    schema: &SearchIndexSchema,
    grouping_columns: &[GroupingColumn],
) -> Option<Vec<AggregateType>> {
    let aggregate_types = extract_aggregates(args)?;

    // Create a set of grouping column field names for quick lookup
    let grouping_field_names: crate::api::HashSet<&String> =
        grouping_columns.iter().map(|gc| &gc.field_name).collect();

    // Validate that all aggregate fields are fast fields and don't conflict with GROUP BY
    for aggregate in &aggregate_types {
        if let Some(field_name) = aggregate.field_name() {
            // Check for conflict with GROUP BY columns
            if grouping_field_names.contains(&field_name) {
                // Aggregate field conflicts with GROUP BY column - causes incompatible fruit types in Tantivy
                return None;
            }

            // Check if field exists in schema and is a fast field
            if let Some(search_field) = schema.search_field(&field_name) {
                if !search_field.is_fast() {
                    // Aggregate field is not a fast field
                    return None;
                }
            } else {
                // Aggregate field not found in schema
                return None;
            }
        }
    }

    Some(aggregate_types)
}

/// If the given args consist only of AggregateTypes that we can handle, return them.
fn extract_aggregates(args: &CreateUpperPathsHookArgs) -> Option<Vec<AggregateType>> {
    // The PathTarget `exprs` are the closest that we have to a target list at this point.
    let target_list =
        unsafe { PgList::<pg_sys::Expr>::from_pg((*args.output_rel().reltarget).exprs) };
    if target_list.is_empty() {
        return None;
    }

    // Get the relation OID for field name lookup
    let parent_relids = args.input_rel().relids;
    let heap_rti = unsafe { range_table::bms_exactly_one_member(parent_relids)? };
    let heap_rte = unsafe {
        let rt = PgList::<pg_sys::RangeTblEntry>::from_pg((*args.root().parse).rtable);
        rt.get_ptr((heap_rti - 1) as usize)?
    };
    let relation_oid = unsafe { (*heap_rte).relid };

    // We must recognize all target list entries as either grouping columns (Vars) or supported aggregates.
    let mut aggregate_types = Vec::new();
    for expr in target_list.iter_ptr() {
        unsafe {
            let node_tag = (*expr).type_;

            if let Some(_var) = nodecast!(Var, T_Var, expr) {
                // This is a Var - it should be a grouping column, skip it
                continue;
            } else if let Some(_opexpr) = nodecast!(OpExpr, T_OpExpr, expr) {
                // This might be a JSON operator expression - verify it's recognized
                let var_context = VarContext::from_planner(args.root() as *const _ as *mut _);
                if let Some((_var, _field_name)) =
                    find_one_var_and_fieldname(var_context, expr as *mut pg_sys::Node)
                {
                    // This is a recognized JSON operator expression used in GROUP BY - skip it
                    continue;
                } else {
                    // This is an unrecognized OpExpr, we can't support it
                    return None;
                }
            } else if let Some(aggref) = nodecast!(Aggref, T_Aggref, expr) {
                // Check for DISTINCT in aggregate functions
                if !(*aggref).aggdistinct.is_null() {
                    // TODO: Support DISTINCT in aggregate custom scans if Tantivy supports it.
                    return None;
                }

                if (*aggref).aggstar {
                    // COUNT(*) (aggstar)
                    aggregate_types.push(AggregateType::Count);
                } else {
                    // Check for other aggregate functions with arguments
                    let agg_type = identify_aggregate_function(aggref, relation_oid)?;
                    aggregate_types.push(agg_type);
                }
            } else {
                // Unsupported expression type
                return None;
            }
        }
    }

    // It's valid to have zero aggregates when the query is only a GROUP BY on fast fields
    // (e.g., SELECT category FROM .. GROUP BY category). In that case, we can still build
    // a ParadeDB Aggregate Scan that only returns the grouping keys. Therefore we return
    // an empty vector instead of rejecting the plan.

    Some(aggregate_types)
}

/// Identify an aggregate function by its OID and extract field name from its arguments
unsafe fn identify_aggregate_function(
    aggref: *mut pg_sys::Aggref,
    relation_oid: pg_sys::Oid,
) -> Option<AggregateType> {
    let aggfnoid = (*aggref).aggfnoid;

    // Get the function name to identify the aggregate
    let func_name = get_aggregate_function_name(aggfnoid)?;

    // Extract the field name from the first argument
    let field_name = extract_field_name_from_aggref(aggref, relation_oid);

    match func_name {
        "count" => Some(AggregateType::Count),
        "sum" => Some(AggregateType::Sum { field: field_name? }),
        "avg" => Some(AggregateType::Avg { field: field_name? }),
        "min" => Some(AggregateType::Min { field: field_name? }),
        "max" => Some(AggregateType::Max { field: field_name? }),
        _ => {
            pgrx::debug1!("Unsupported aggregate function: {func_name}");
            None
        }
    }
}

/// Get the name of an aggregate function from its OID
unsafe fn get_aggregate_function_name(aggfnoid: pg_sys::Oid) -> Option<&'static str> {
    use pgrx::pg_sys::{
        F_AVG_FLOAT4, F_AVG_FLOAT8, F_AVG_INT2, F_AVG_INT4, F_AVG_INT8, F_AVG_NUMERIC, F_COUNT_ANY,
        F_MAX_DATE, F_MAX_FLOAT4, F_MAX_FLOAT8, F_MAX_INT2, F_MAX_INT4, F_MAX_INT8, F_MAX_NUMERIC,
        F_MAX_TIME, F_MAX_TIMESTAMP, F_MAX_TIMESTAMPTZ, F_MAX_TIMETZ, F_MIN_DATE, F_MIN_FLOAT4,
        F_MIN_FLOAT8, F_MIN_INT2, F_MIN_INT4, F_MIN_INT8, F_MIN_MONEY, F_MIN_NUMERIC, F_MIN_TIME,
        F_MIN_TIMESTAMP, F_MIN_TIMESTAMPTZ, F_MIN_TIMETZ, F_SUM_FLOAT4, F_SUM_FLOAT8, F_SUM_INT2,
        F_SUM_INT4, F_SUM_INT8, F_SUM_NUMERIC,
    };
    // Use well-known PostgreSQL function OIDs for standard aggregates
    // These are consistent across PostgreSQL versions
    match aggfnoid.to_u32() {
        F_AVG_INT8 | F_AVG_INT4 | F_AVG_INT2 | F_AVG_NUMERIC | F_AVG_FLOAT4 | F_AVG_FLOAT8 => {
            Some("avg")
        }
        F_SUM_INT8 | F_SUM_INT4 | F_SUM_INT2 | F_SUM_FLOAT4 | F_SUM_FLOAT8 | F_SUM_NUMERIC => {
            Some("sum")
        }
        F_MAX_INT8 | F_MAX_INT4 | F_MAX_INT2 | F_MAX_FLOAT4 | F_MAX_FLOAT8 | F_MAX_DATE
        | F_MAX_TIME | F_MAX_TIMETZ | F_MAX_TIMESTAMP | F_MAX_TIMESTAMPTZ | F_MAX_NUMERIC => {
            Some("max")
        }
        F_MIN_INT8 | F_MIN_INT4 | F_MIN_INT2 | F_MIN_FLOAT4 | F_MIN_FLOAT8 | F_MIN_DATE
        | F_MIN_TIME | F_MIN_TIMETZ | F_MIN_MONEY | F_MIN_TIMESTAMP | F_MIN_TIMESTAMPTZ
        | F_MIN_NUMERIC => Some("min"),
        F_COUNT_ANY => Some("count"),
        _ => {
            // For unknown function OIDs, we'll reject them for now
            pgrx::debug1!("Unknown aggregate function OID: {}", aggfnoid.to_u32());
            None
        }
    }
}

/// Extract field name from the first argument of an aggregate function
unsafe fn extract_field_name_from_aggref(
    aggref: *mut pg_sys::Aggref,
    relation_oid: pg_sys::Oid,
) -> Option<String> {
    let args = PgList::<pg_sys::TargetEntry>::from_pg((*aggref).args);
    if args.is_empty() {
        return None;
    }

    let first_arg = args.get_ptr(0)?;
    if let Some(var) = nodecast!(Var, T_Var, (*first_arg).expr) {
        return get_var_field_name(var, relation_oid);
    }

    None
}

/// Get the field name from a Var node
unsafe fn get_var_field_name(var: *mut pg_sys::Var, relation_oid: pg_sys::Oid) -> Option<String> {
    let varattno = (*var).varattno;

    // Get the actual column name from the relation
    let attname = pg_sys::get_attname(relation_oid, varattno, false);
    if !attname.is_null() {
        let name = std::ffi::CStr::from_ptr(attname).to_str().ok()?;
        return Some(name.to_string());
    }

    None
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

/// Check if any GROUP BY field is also being searched in the WHERE clause
/// This causes "incompatible fruit types" errors in Tantivy aggregation
fn has_search_field_conflicts(
    grouping_columns: &[GroupingColumn],
    query: &SearchQueryInput,
) -> bool {
    if grouping_columns.is_empty() {
        return false;
    }

    let grouping_field_names: crate::api::HashSet<String> = grouping_columns
        .iter()
        .map(|gc| gc.field_name.clone())
        .collect();

    let mut search_field_names = crate::api::HashSet::default();
    query.extract_field_names(&mut search_field_names);

    // Check for conflicts
    !search_field_names.is_disjoint(&grouping_field_names)
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
