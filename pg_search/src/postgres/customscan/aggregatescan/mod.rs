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
use crate::api::{HashMap, HashSet, OrderByFeature};
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
use crate::postgres::rel_get_bm25_index;
use crate::postgres::types::TantivyValue;
use crate::postgres::var::{find_one_var_and_fieldname, find_var_relation, VarContext};
use crate::postgres::PgSearchRelation;
use crate::query::pdb_query::pdb::Query;
use crate::query::SearchQueryInput;
use crate::schema::SearchIndexSchema;
use pgrx::{pg_sys, FromDatum, IntoDatum, PgList, PgTupleDesc};
use tantivy::schema::OwnedValue;
use tantivy::Index;

// Constants for better maintainability
const DEFAULT_BUCKET_LIMIT: u32 = 65000;
const NO_FILTER_KEY: &str = "NO_FILTER";
const FAILED_TO_EXECUTE_AGGREGATE: &str = "failed to execute aggregate";

#[derive(Default)]
pub struct AggregateScan;

impl CustomScan for AggregateScan {
    const NAME: &'static CStr = c"ParadeDB Aggregate Scan";
    type Args = CreateUpperPathsHookArgs;
    type State = AggregateScanState;
    type PrivateData = PrivateData;

    fn create_custom_path(mut builder: CustomPathBuilder<Self>) -> Option<pg_sys::CustomPath> {
        let args = builder.args();
        let parse = args.root().parse;

        // We can only handle single base relations as input
        if args.input_rel().reloptkind != pg_sys::RelOptKind::RELOPT_BASEREL {
            return None;
        }

        // Check if there are restrictions (WHERE clause)
        let (restrict_info, ri_type) = restrict_info(builder.args().input_rel());
        let has_where_clause = matches!(ri_type, RestrictInfoType::BaseRelation);

        // Are there any group (/distinct/order-by) or having clauses?
        // We can't handle HAVING yet
        if args.root().hasHavingQual {
            // We can't handle HAVING yet
            return None;
        }

        // Check for DISTINCT - we can't handle DISTINCT queries
        unsafe {
            if !parse.is_null() && (!(*parse).distinctClause.is_null() || (*parse).hasDistinctOn) {
                return None;
            }
        }

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
        let group_pathkeys = if args.root().group_pathkeys.is_null() {
            PgList::<pg_sys::PathKey>::new()
        } else {
            unsafe { PgList::<pg_sys::PathKey>::from_pg(args.root().group_pathkeys) }
        };
        let grouping_columns =
            extract_grouping_columns(&group_pathkeys, args.root, heap_rti, &schema)?;

        // Extract and validate aggregates - must have schema for field validation
        let aggregate_types = extract_and_validate_aggregates(
            args,
            &schema,
            &grouping_columns,
            &bm25_index,
            heap_rti,
        )?;

        // Check if any aggregates have filters
        let has_filters = aggregate_types.iter().any(|agg| agg.has_filter());
        if has_filters {
            // Check if any filters are HeapFilters (non-search predicates)
            let has_heap_filters = aggregate_types.iter().any(|agg| {
                if let Some(mut filter_expr) = agg.filter_expr() {
                    filter_expr.has_heap_filters()
                } else {
                    false
                }
            });

            if has_heap_filters {
                return None;
            }
        }

        // If we don't have a WHERE clause and we don't have FILTER clauses,
        // there's no benefit to using AggregateScan
        if !has_where_clause && !has_filters {
            return None;
        }

        // Extract ORDER BY pathkeys if present
        let sort_clause =
            unsafe { PgList::<pg_sys::SortGroupClause>::from_pg((*parse).sortClause) };
        let sort_fields = unsafe {
            sort_clause
                .iter_ptr()
                .filter_map(|sort_clause| {
                    let expr = pg_sys::get_sortgroupclause_expr(sort_clause, (*parse).targetList);
                    let var_context = VarContext::from_planner(builder.args().root);
                    if let Some((_, field_name)) = find_one_var_and_fieldname(var_context, expr) {
                        Some(field_name)
                    } else {
                        None
                    }
                })
                .collect::<HashSet<_>>()
        };
        let order_pathkey_info = extract_order_by_pathkeys(args.root, heap_rti, &schema);
        let orderby_info = OrderByStyle::extract_orderby_info(order_pathkey_info.pathkeys())
            .into_iter()
            .filter(|info| {
                if let OrderByFeature::Field(field_name) = &info.feature {
                    sort_fields.contains(field_name)
                } else {
                    false
                }
            })
            .collect::<Vec<_>>();

        // Extract LIMIT/OFFSET if it's a GROUP BY...ORDER BY...LIMIT query
        let max_term_agg_buckets = gucs::max_term_agg_buckets() as u32;

        let (limit, offset) = unsafe {
            let limit_count = (*parse).limitCount;
            let offset_count = (*parse).limitOffset;

            let extract_const = |node: *mut pg_sys::Node| -> Option<u32> {
                let const_node = nodecast!(Const, T_Const, node);
                if let Some(const_node) = const_node {
                    u32::from_datum((*const_node).constvalue, (*const_node).constisnull)
                } else {
                    None
                }
            };

            (extract_const(limit_count), extract_const(offset_count))
        };

        // We cannot push down a GROUP BY if the user asks for more than `max_term_agg_buckets`
        // or if it orders by columns that we cannot push down
        if unsafe { !(*parse).groupClause.is_null() } {
            let total_limit = limit.unwrap_or(0) + offset.unwrap_or(0);

            if total_limit > max_term_agg_buckets || orderby_info.len() != sort_clause.len() {
                return None;
            }
        }

        // Extract the WHERE clause query if present
        let mut query = if has_where_clause {
            unsafe {
                let result = extract_quals(
                    args.root,
                    heap_rti,
                    restrict_info.as_ptr().cast(),
                    anyelement_query_input_opoid(),
                    ri_type,
                    &bm25_index,
                    false, // Base relation quals should not convert external to all
                    &mut QualExtractState::default(),
                    true,
                );
                SearchQueryInput::from(&result?)
            }
        } else {
            // No WHERE clause - use an "All" query that matches everything
            SearchQueryInput::All
        };

        // Check if the WHERE clause query contains HeapFilter conditions that Tantivy cannot handle
        if query.has_heap_filters() {
            return None;
        }

        // Create a new target list which includes grouping columns and replaces aggregates
        // with FuncExprs which will be produced by our CustomScan.
        //
        // We don't use Vars here, because there doesn't seem to be a reasonable RTE to associate
        // them with.
        let target_list = unsafe { PgList::<pg_sys::TargetEntry>::from_pg((*parse).targetList) };
        let mut target_list_mapping = Vec::new();
        let mut agg_idx = 0;

        for (te_idx, input_te) in target_list.iter_ptr().enumerate() {
            unsafe {
                let var_context = VarContext::from_planner(args.root() as *const _ as *mut _);

                if let Some((var, field_name)) =
                    find_one_var_and_fieldname(var_context, (*input_te).expr as *mut pg_sys::Node)
                {
                    // This is a Var - it should be a grouping column
                    // Find which grouping column this is
                    let mut found = false;
                    for (i, gc) in grouping_columns.iter().enumerate() {
                        if (*var).varattno == gc.attno
                            && gc.field_name == field_name.clone().into_inner()
                        {
                            target_list_mapping.push(TargetListEntry::GroupingColumn(i));
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        return None;
                    }
                } else if let Some(aggref) = nodecast!(Aggref, T_Aggref, (*input_te).expr) {
                    target_list_mapping.push(TargetListEntry::Aggregate(agg_idx));
                    agg_idx += 1;
                } else {
                    return None;
                }
            };
        }

        // Replace T_Aggref for simple aggregations without GROUP BY or ORDER BY
        // For queries with GROUP BY or ORDER BY, we keep T_Aggref during planning for pathkey matching
        // TODO(mdashti): remove the planning time replacement once we figured the reason behind
        // the aggregate_custom_scan/test_count test failure
        let has_order_by = unsafe { !parse.is_null() && !(*parse).sortClause.is_null() };

        // If we're handling ORDER BY, we need to inform PostgreSQL that our output is sorted.
        // To do this, we set pathkeys for ORDER BY if present.
        if let Some(pathkeys) = order_pathkey_info.pathkeys() {
            for pathkey_style in pathkeys {
                builder = builder.add_path_key(pathkey_style);
            }
        };

        // A GROUP BY...ORDER BY query could have some results truncated
        let maybe_truncated = !parse.is_null()
            && unsafe { !(*parse).groupClause.is_null() }
            && unsafe { !(*parse).sortClause.is_null() }
            && limit.is_none();

        Some(builder.build(PrivateData {
            aggregate_types,
            indexrelid: bm25_index.oid(),
            heap_rti,
            query,
            grouping_columns,
            orderby_info,
            target_list_mapping,
            has_order_by,
            limit,
            offset,
            maybe_truncated,
        }))
    }

    fn plan_custom_path(mut builder: CustomScanBuilder<Self>) -> pg_sys::CustomScan {
        builder.set_scanrelid(builder.custom_private().heap_rti);

        if builder.custom_private().grouping_columns.is_empty()
            && builder.custom_private().orderby_info.is_empty()
            && !builder.custom_private().has_order_by
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
        if !builder.custom_private().grouping_columns.is_empty()
            || !builder.custom_private().orderby_info.is_empty()
            || builder.custom_private().has_order_by
        {
            unsafe {
                let cscan = builder.args().cscan;
                let plan = &mut (*cscan).scan.plan;
                replace_aggrefs_in_target_list(plan);
            }
        }

        builder.custom_state().aggregate_types = builder.custom_private().aggregate_types.clone();
        builder.custom_state().grouping_columns = builder.custom_private().grouping_columns.clone();
        builder.custom_state().orderby_info = builder.custom_private().orderby_info.clone();
        builder.custom_state().target_list_mapping =
            builder.custom_private().target_list_mapping.clone();
        builder.custom_state().indexrelid = builder.custom_private().indexrelid;
        builder.custom_state().query = builder.custom_private().query.clone();
        builder.custom_state().execution_rti =
            unsafe { (*builder.args().cscan).scan.scanrelid as pg_sys::Index };
        builder.custom_state().limit = builder.custom_private().limit;
        builder.custom_state().offset = builder.custom_private().offset;
        builder.custom_state().maybe_truncated = builder.custom_private().maybe_truncated;
        builder.build()
    }

    fn explain_custom_scan(
        state: &CustomScanStateWrapper<Self>,
        ancestors: *mut pg_sys::List,
        explainer: &mut Explainer,
    ) {
        explainer.add_text("Index", state.custom_state().indexrel().name());

        // Analyze filter patterns for optimization (same logic as execution)
        let filter_groups = analyze_filter_patterns(&state.custom_state().aggregate_types);

        // Show execution strategy and queries based on filter patterns
        explain_filter_execution_strategy(state, &filter_groups, explainer);

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

/// Helper function to get human-readable aggregate descriptions
fn get_aggregate_descriptions(aggregate_types: &[AggregateType], indices: &[usize]) -> String {
    let descriptions: Vec<String> = indices
        .iter()
        .map(|&idx| {
            let agg_type = &aggregate_types[idx];
            match agg_type {
                AggregateType::CountAny => "COUNT(*)".to_string(),
                AggregateType::CountAnyWithFilter { filter_expr } => {
                    format!(
                        "COUNT(*) FILTER (WHERE {})",
                        format_filter_condition(filter_expr)
                    )
                }
                AggregateType::Sum { field, .. } => format!("SUM({field})"),
                AggregateType::SumWithFilter {
                    field, filter_expr, ..
                } => {
                    format!(
                        "SUM({}) FILTER (WHERE {})",
                        field,
                        format_filter_condition(filter_expr)
                    )
                }
                AggregateType::Avg { field, .. } => format!("AVG({field})"),
                AggregateType::AvgWithFilter {
                    field, filter_expr, ..
                } => {
                    format!(
                        "AVG({}) FILTER (WHERE {})",
                        field,
                        format_filter_condition(filter_expr)
                    )
                }
                AggregateType::Min { field, .. } => format!("MIN({field})"),
                AggregateType::MinWithFilter {
                    field, filter_expr, ..
                } => {
                    format!(
                        "MIN({}) FILTER (WHERE {})",
                        field,
                        format_filter_condition(filter_expr)
                    )
                }
                AggregateType::Max { field, .. } => format!("MAX({field})"),
                AggregateType::MaxWithFilter {
                    field, filter_expr, ..
                } => {
                    format!(
                        "MAX({}) FILTER (WHERE {})",
                        field,
                        format_filter_condition(filter_expr)
                    )
                }
            }
        })
        .collect();
    descriptions.join(", ")
}

/// Helper function to explain filter execution strategy
fn explain_filter_execution_strategy(
    state: &CustomScanStateWrapper<AggregateScan>,
    filter_groups: &[(Option<SearchQueryInput>, Vec<usize>)],
    explainer: &mut Explainer,
) {
    if filter_groups.is_empty() {
        // No filters - just show the base query without mentioning execution strategy
        explainer.add_query(&state.custom_state().query);
    } else if filter_groups.len() == 1 {
        // Single query optimization
        let (filter_expr, aggregate_indices) = &filter_groups[0];

        if filter_expr.is_none() {
            // No filters - just show the base query without mentioning execution strategy
            explainer.add_query(&state.custom_state().query);
        } else {
            // Show the combined query
            let combined_query = state
                .custom_state()
                .query
                .combine_query_with_filter(filter_expr.as_ref());

            explainer.add_text(
                "  Combined Query",
                combined_query.serialize_and_clean_query(),
            );

            explainer.add_text(
                "  Applies to Aggregates",
                get_aggregate_descriptions(
                    &state.custom_state().aggregate_types,
                    aggregate_indices,
                ),
            );
        }
    } else {
        // Multi-group optimization
        explainer.add_text(
            "Execution Strategy",
            format!(
                "Optimized Multi-Query ({} Filter Groups)",
                filter_groups.len()
            ),
        );

        for (group_idx, (filter_expr, aggregate_indices)) in filter_groups.iter().enumerate() {
            let combined_query = state
                .custom_state()
                .query
                .combine_query_with_filter(filter_expr.as_ref());

            let query_label = if filter_expr.is_some() {
                format!("  Group {} Query", group_idx + 1)
            } else {
                format!("  Group {} Query (No Filter)", group_idx + 1)
            };

            explainer.add_text(&query_label, combined_query.serialize_and_clean_query());

            explainer.add_text(
                &format!("  Group {} Aggregates", group_idx + 1),
                get_aggregate_descriptions(
                    &state.custom_state().aggregate_types,
                    aggregate_indices,
                ),
            );
        }

        explainer.add_unsigned_integer("Total Groups", filter_groups.len() as u64, None);
    }
}

/// Helper function to convert filtered aggregates to unfiltered ones
fn convert_to_unfiltered_aggregates(aggregate_types: &[AggregateType]) -> Vec<AggregateType> {
    aggregate_types
        .iter()
        .map(|agg| agg.convert_filtered_aggregate_to_unfiltered())
        .collect()
}

/// Helper function to create temporary scan state for query execution
fn create_temp_scan_state(
    base_state: &CustomScanStateWrapper<AggregateScan>,
    aggregate_types: Vec<AggregateType>,
    query: SearchQueryInput,
    target_list_mapping: Option<Vec<TargetListEntry>>,
) -> crate::postgres::customscan::aggregatescan::scan_state::AggregateScanState {
    crate::postgres::customscan::aggregatescan::scan_state::AggregateScanState {
        state: crate::postgres::customscan::aggregatescan::scan_state::ExecutionState::NotStarted,
        aggregate_types,
        grouping_columns: base_state.custom_state().grouping_columns.clone(),
        orderby_info: base_state.custom_state().orderby_info.clone(),
        target_list_mapping: target_list_mapping
            .unwrap_or_else(|| base_state.custom_state().target_list_mapping.clone()),
        query,
        indexrelid: base_state.custom_state().indexrelid,
        indexrel: base_state.custom_state().indexrel.clone(),
        execution_rti: base_state.custom_state().execution_rti,
        limit: base_state.custom_state().limit,
        offset: base_state.custom_state().offset,
        maybe_truncated: false,
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
            for (i, member) in members.iter_ptr().enumerate() {
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
    bm25_index: &PgSearchRelation,
    heap_rti: pg_sys::Index,
) -> Option<Vec<AggregateType>> {
    let aggregate_types = extract_aggregates(args, bm25_index, heap_rti)?;

    // Create a set of grouping column field names for quick lookup
    let grouping_field_names: crate::api::HashSet<&String> =
        grouping_columns.iter().map(|gc| &gc.field_name).collect();

    // Validate that all aggregate fields are fast fields and don't conflict with GROUP BY
    for (i, aggregate) in aggregate_types.iter().enumerate() {
        if let Some(field_name) = aggregate.field_name() {
            // Check if field exists in schema and is a fast field
            if let Some(search_field) = schema.search_field(&field_name) {
                if !search_field.is_fast() {
                    return None;
                }
            } else {
                return None;
            }
        }
    }

    Some(aggregate_types)
}

/// If the given args consist only of AggregateTypes that we can handle, return them.
fn extract_aggregates(
    args: &CreateUpperPathsHookArgs,
    bm25_index: &PgSearchRelation,
    heap_rti: pg_sys::Index,
) -> Option<Vec<AggregateType>> {
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
    for (i, expr) in target_list.iter_ptr().enumerate() {
        unsafe {
            let node_tag = (*expr).type_;

            if let Some(_var) = nodecast!(Var, T_Var, expr) {
                continue;
            } else if let Some(_opexpr) = nodecast!(OpExpr, T_OpExpr, expr) {
                // This might be a JSON operator expression - verify it's recognized
                let var_context = VarContext::from_planner(args.root() as *const _ as *mut _);
                if let Some((_var, _field_name)) =
                    find_one_var_and_fieldname(var_context, expr as *mut pg_sys::Node)
                {
                    continue;
                } else {
                    return None;
                }
            } else if let Some(_const_expr) = nodecast!(Const, T_Const, expr) {
                continue;
            } else if let Some(aggref) = nodecast!(Aggref, T_Aggref, expr) {
                // Check for DISTINCT in aggregate functions
                if !(*aggref).aggdistinct.is_null() {
                    // TODO: Support DISTINCT in aggregate custom scans if Tantivy supports it.
                    return None;
                }

                // Check for FILTER clause in aggregate functions
                // We now support FILTER clauses via multi-query execution!

                if (*aggref).aggstar {
                    // COUNT(*) (aggstar) - but check for FILTER clause
                    if !(*aggref).aggfilter.is_null() {
                        let filter_expr = extract_filter_clause(
                            (*aggref).aggfilter,
                            bm25_index,
                            args.root,
                            heap_rti,
                        );
                        if let Some(filter) = filter_expr {
                            aggregate_types.push(AggregateType::CountAnyWithFilter {
                                filter_expr: filter,
                            });
                        } else {
                            aggregate_types.push(AggregateType::CountAny);
                        }
                    } else {
                        aggregate_types.push(AggregateType::CountAny);
                    }
                } else {
                    // Check for other aggregate functions with arguments
                    let agg_type = identify_aggregate_function(
                        aggref,
                        relation_oid,
                        bm25_index,
                        args.root,
                        heap_rti,
                    )?;
                    aggregate_types.push(agg_type);
                }
            } else {
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
    bm25_index: &PgSearchRelation,
    root: *mut pg_sys::PlannerInfo,
    heap_rti: pg_sys::Index,
) -> Option<AggregateType> {
    // First try to create the base aggregate using the new try_from method
    let base_agg_type = AggregateType::try_from(aggref, relation_oid)?;

    // Check if there's a FILTER clause
    let filter_expr = if !(*aggref).aggfilter.is_null() {
        extract_filter_clause((*aggref).aggfilter, bm25_index, root, heap_rti)
    } else {
        None
    };

    // If there's a filter, convert the base aggregate to its filtered variant
    if let Some(filter) = filter_expr {
        match base_agg_type {
            AggregateType::CountAny => Some(AggregateType::CountAnyWithFilter {
                filter_expr: filter,
            }),
            AggregateType::Sum { field, missing } => Some(AggregateType::SumWithFilter {
                field,
                missing,
                filter_expr: filter,
            }),
            AggregateType::Avg { field, missing } => Some(AggregateType::AvgWithFilter {
                field,
                missing,
                filter_expr: filter,
            }),
            AggregateType::Min { field, missing } => Some(AggregateType::MinWithFilter {
                field,
                missing,
                filter_expr: filter,
            }),
            AggregateType::Max { field, missing } => Some(AggregateType::MaxWithFilter {
                field,
                missing,
                filter_expr: filter,
            }),
            // Already filtered variants should not happen here
            _ => Some(base_agg_type),
        }
    } else {
        Some(base_agg_type)
    }
}

/// Extract filter expression from a FILTER clause
unsafe fn extract_filter_clause(
    filter_expr: *mut pg_sys::Expr,
    bm25_index: &PgSearchRelation,
    root: *mut pg_sys::PlannerInfo,
    heap_rti: pg_sys::Index,
) -> Option<SearchQueryInput> {
    // The filter expression is an Expr
    if filter_expr.is_null() {
        return None;
    }

    // Log the node type to understand what we're dealing with
    let node_type = (*filter_expr).type_;

    // Extract quals from the filter expression
    let filter_node = filter_expr as *mut pg_sys::Node;
    let result = extract_quals(
        root,
        heap_rti,
        filter_node,
        anyelement_query_input_opoid(),
        RestrictInfoType::BaseRelation,
        bm25_index,
        false,
        &mut QualExtractState::default(),
        true, // attempt_pushdown
    );

    // Convert Qual to SearchQueryInput
    result.map(|qual| SearchQueryInput::from(&qual))
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

fn execute(
    state: &CustomScanStateWrapper<AggregateScan>,
) -> std::vec::IntoIter<GroupedAggregateRow> {
    // Analyze filter patterns for optimization opportunities
    let filter_groups = analyze_filter_patterns(&state.custom_state().aggregate_types);

    if filter_groups.len() == 1 {
        // All aggregates have the same filter (or no filter) - use single query optimization
        let (filter_expr, aggregate_indices) = &filter_groups[0];
        execute_single_optimized_query(state, filter_expr.clone(), aggregate_indices.clone())
    } else {
        // Multiple distinct filters - use optimized multi-query approach
        execute_optimized_multi_filter_queries(state, filter_groups)
    }
}

/// Format a filter condition in human-readable form
fn format_filter_condition(filter_expr: &SearchQueryInput) -> String {
    match filter_expr {
        SearchQueryInput::FieldedQuery { field, query } => match query {
            Query::Term { value, .. } => match value {
                tantivy::schema::OwnedValue::Bool(b) => format!("{field} = {b}"),
                tantivy::schema::OwnedValue::I64(i) => format!("{field} = {i}"),
                tantivy::schema::OwnedValue::F64(f) => format!("{field} = {f}"),
                tantivy::schema::OwnedValue::Str(s) => format!("{field} = '{s}'"),
                _ => format!("{field} = <value>"),
            },
            Query::Range {
                lower_bound,
                upper_bound,
                ..
            } => {
                format!("{field} BETWEEN {lower_bound:?} AND {upper_bound:?}")
            }
            Query::Phrase { phrases, .. } => {
                format!("{field} @@@ '{}'", phrases.join(" "))
            }
            Query::ParseWithField { query_string, .. } => {
                format!("{field} @@@ '{query_string}'")
            }
            _ => format!("{field} <condition>"),
        },
        SearchQueryInput::Boolean { must, .. } => {
            // For Boolean queries, try to extract the simple case
            if must.len() == 1 {
                format_filter_condition(&must[0])
            } else {
                "<complex boolean condition>".to_string()
            }
        }
        _ => "<complex condition>".to_string(),
    }
}

/// Analyze filter patterns to identify optimization opportunities
/// Returns groups of (filter_expr, aggregate_indices) where aggregates with the same filter are grouped together
fn analyze_filter_patterns(
    aggregate_types: &[AggregateType],
) -> Vec<(Option<SearchQueryInput>, Vec<usize>)> {
    // Group aggregates by their filter expression
    let mut filter_groups: HashMap<String, Vec<usize>> = HashMap::default();

    for (idx, agg_type) in aggregate_types.iter().enumerate() {
        let filter_key = if let Some(filter_expr) = agg_type.filter_expr() {
            format!("{filter_expr:?}")
        } else {
            NO_FILTER_KEY.to_string()
        };

        filter_groups.entry(filter_key).or_default().push(idx);
    }

    // Convert to the expected format and sort for deterministic output
    let mut result = Vec::new();
    for (filter_key, indices) in filter_groups {
        let filter_expr = if filter_key == NO_FILTER_KEY {
            None
        } else {
            // Get the actual filter expression from the first aggregate in this group
            aggregate_types[indices[0]].filter_expr()
        };

        result.push((filter_expr, indices, filter_key));
    }

    // Sort by filter key to ensure deterministic ordering
    // NO_FILTER groups come first, then sorted by filter expression string
    result.sort_by(|a, b| {
        match (a.2.as_str(), b.2.as_str()) {
            (NO_FILTER_KEY, NO_FILTER_KEY) => std::cmp::Ordering::Equal,
            (NO_FILTER_KEY, _) => std::cmp::Ordering::Less, // NO_FILTER comes first
            (_, NO_FILTER_KEY) => std::cmp::Ordering::Greater,
            (a_key, b_key) => a_key.cmp(b_key), // Sort other filters alphabetically
        }
    });

    // Remove the filter_key from the result tuple
    result
        .into_iter()
        .map(|(filter_expr, indices, _)| (filter_expr, indices))
        .collect()
}

/// Execute a single optimized query when all aggregates have the same filter (or no filter)
fn execute_single_optimized_query(
    state: &CustomScanStateWrapper<AggregateScan>,
    common_filter: Option<SearchQueryInput>,
    _aggregate_indices: Vec<usize>,
) -> std::vec::IntoIter<GroupedAggregateRow> {
    // Combine base query with common filter (if any)
    let combined_query = state
        .custom_state()
        .query
        .combine_query_with_filter(common_filter.as_ref());

    // Create unfiltered aggregate types (since filter is moved to query level)
    let unfiltered_aggregates =
        convert_to_unfiltered_aggregates(&state.custom_state().aggregate_types);

    // Create temporary state for aggregation JSON generation
    let temp_scan_state =
        create_temp_scan_state(state, unfiltered_aggregates, combined_query.clone(), None);

    let agg_json = temp_scan_state.aggregates_to_json();

    // Execute the single query
    let result = execute_aggregate(
        state.custom_state().indexrel(),
        combined_query,
        agg_json,
        true,                                              // solve_mvcc
        gucs::adjust_work_mem().get().try_into().unwrap(), // memory_limit
        DEFAULT_BUCKET_LIMIT,                              // bucket_limit
    )
    .expect(FAILED_TO_EXECUTE_AGGREGATE);

    // Parse results using the temporary state
    temp_scan_state
        .json_to_aggregate_results(result)
        .into_iter()
}

/// Execute optimized multi-query approach, grouping aggregates by filter
fn execute_optimized_multi_filter_queries(
    state: &CustomScanStateWrapper<AggregateScan>,
    filter_groups: Vec<(Option<SearchQueryInput>, Vec<usize>)>,
) -> std::vec::IntoIter<GroupedAggregateRow> {
    let mut all_results = Vec::new();

    // Execute one query per filter group
    for (filter_expr, aggregate_indices) in filter_groups.iter() {
        // Combine base query with group filter
        let combined_query = state
            .custom_state()
            .query
            .combine_query_with_filter(filter_expr.as_ref());

        // Create aggregates for this group (unfiltered since filter is in query)
        let group_aggregates: Vec<AggregateType> = aggregate_indices
            .iter()
            .map(|&idx| {
                state.custom_state().aggregate_types[idx].convert_filtered_aggregate_to_unfiltered()
            })
            .collect();

        // Create target list mapping for this group
        let target_list_mapping = aggregate_indices
            .iter()
            .map(|&idx| {
                crate::postgres::customscan::aggregatescan::privdat::TargetListEntry::Aggregate(idx)
            })
            .collect();

        // Create temporary state for this group
        let temp_scan_state = create_temp_scan_state(
            state,
            group_aggregates,
            combined_query.clone(),
            Some(target_list_mapping),
        );

        let agg_json = temp_scan_state.aggregates_to_json();

        // Execute the query for this group
        let result = execute_aggregate(
            state.custom_state().indexrel(),
            combined_query,
            agg_json,
            true,                                              // solve_mvcc
            gucs::adjust_work_mem().get().try_into().unwrap(), // memory_limit
            DEFAULT_BUCKET_LIMIT,                              // bucket_limit
        )
        .expect(FAILED_TO_EXECUTE_AGGREGATE);

        all_results.push((result, aggregate_indices.clone()));
    }

    // Merge results from all groups
    state
        .custom_state()
        .merge_optimized_multi_group_results(all_results)
        .into_iter()
}

impl ExecMethod for AggregateScan {
    fn exec_methods() -> *const pg_sys::CustomExecMethods {
        <AggregateScan as PlainExecCapable>::exec_methods()
    }
}

impl PlainExecCapable for AggregateScan {}

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
