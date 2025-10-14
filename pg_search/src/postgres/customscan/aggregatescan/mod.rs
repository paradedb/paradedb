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
use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::builders::custom_state::{
    CustomScanStateBuilder, CustomScanStateWrapper,
};
use crate::postgres::customscan::explain::ExplainFormat;
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::pdbscan::{
    extract_pathkey_styles_with_sortability_check, PathKeyInfo,
};
use crate::postgres::customscan::qual_inspect::{extract_quals, QualExtractState};
use crate::postgres::customscan::solve_expr::SolvePostgresExpressions;
use crate::postgres::customscan::{
    range_table, CreateUpperPathsHookArgs, CustomScan, ExecMethod, PlainExecCapable,
};
use crate::postgres::rel_get_bm25_index;
use crate::postgres::types::TantivyValue;
use crate::postgres::var::{find_one_var_and_fieldname, find_var_relation, VarContext};
use crate::postgres::PgSearchRelation;
use crate::query::SearchQueryInput;
use crate::schema::SearchIndexSchema;
use pgrx::{pg_sys, FromDatum, IntoDatum, PgList, PgTupleDesc};
use tantivy::aggregation::DEFAULT_BUCKET_LIMIT;
use tantivy::schema::OwnedValue;
use tantivy::Index;

/// Sentinel key for aggregates without FILTER clauses
/// Used to group non-filtered aggregates together during query optimization
const NO_FILTER_KEY: &str = "NO_FILTER";

/// Result type for aggregate extraction, containing:
/// - Vec<AggregateType>: The extracted aggregate types
/// - Vec<FilterGroup>: Groups of aggregates with the same filter
/// - bool: Whether any filter uses the @@@ search operator
type AggregateExtractionResult = (Vec<AggregateType>, Vec<FilterGroup>, bool);

/// A group of aggregate indices that share the same filter condition
type FilterGroup = (Option<SearchQueryInput>, Vec<usize>);

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
        if matches!(ri_type, RestrictInfoType::Join) {
            // This relation is a join, or has no restrictions (WHERE clause predicates), so there's no need
            // for us to do anything.
            return None;
        }
        let has_where_clause = matches!(ri_type, RestrictInfoType::BaseRelation);

        // Are there any group (/distinct/order-by) or having clauses?
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
        let (aggregate_types, filter_groups, filter_uses_search_operator) =
            extract_and_validate_aggregates(
                args,
                &schema,
                &grouping_columns,
                &bm25_index,
                heap_rti,
            )?;

        let has_filters = aggregate_types.iter().any(|agg| agg.has_filter());
        let handle_query_without_op = !gucs::enable_custom_scan_without_operator();
        // If we don't have a WHERE clause and we don't have FILTER clauses,
        // we'd only handle the query if the GUC is enabled
        if handle_query_without_op && !has_where_clause && !has_filters {
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
        if unsafe { !(*parse).groupClause.is_null() }
            && (limit.unwrap_or(0) + offset.unwrap_or(0) > max_term_agg_buckets
                || orderby_info.len() != sort_clause.len())
        {
            return None;
        }

        // Extract the WHERE clause query if present and track @@@ operator usage
        let mut where_qual_state = QualExtractState::default();
        let query = if has_where_clause {
            unsafe {
                let result = extract_quals(
                    args.root,
                    heap_rti,
                    restrict_info.as_ptr().cast(),
                    anyelement_query_input_opoid(),
                    ri_type,
                    &bm25_index,
                    false, // Base relation quals should not convert external to all
                    &mut where_qual_state,
                    true,
                );
                SearchQueryInput::from(&result?)
            }
        } else {
            // No WHERE clause - use an "All" query that matches everything
            SearchQueryInput::All
        };
        let where_uses_search_operator = where_qual_state.uses_our_operator;
        let has_search_operator = where_uses_search_operator || filter_uses_search_operator;
        if handle_query_without_op && !has_search_operator {
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
            agg_spec: AggregationSpec {
                agg_types: aggregate_types,
                grouping_columns,
                orderby_info,
            },
            indexrelid: bm25_index.oid(),
            heap_rti,
            query,
            target_list_mapping,
            has_order_by,
            limit,
            offset,
            maybe_truncated,
            filter_groups,
        }))
    }

    fn plan_custom_path(mut builder: CustomScanBuilder<Self>) -> pg_sys::CustomScan {
        builder.set_scanrelid(builder.custom_private().heap_rti);

        if builder
            .custom_private()
            .agg_spec
            .grouping_columns
            .is_empty()
            && builder.custom_private().agg_spec.orderby_info.is_empty()
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
        if !builder
            .custom_private()
            .agg_spec
            .grouping_columns
            .is_empty()
            || !builder.custom_private().agg_spec.orderby_info.is_empty()
            || builder.custom_private().has_order_by
        {
            unsafe {
                let cscan = builder.args().cscan;
                let plan = &mut (*cscan).scan.plan;
                replace_aggrefs_in_target_list(plan);
            }
        }

        builder.custom_state().agg_spec = builder.custom_private().agg_spec.clone();
        builder.custom_state().target_list_mapping =
            builder.custom_private().target_list_mapping.clone();
        builder.custom_state().indexrelid = builder.custom_private().indexrelid;
        builder.custom_state().query = builder.custom_private().query.clone();
        builder.custom_state().execution_rti =
            unsafe { (*builder.args().cscan).scan.scanrelid as pg_sys::Index };
        builder.custom_state().limit = builder.custom_private().limit;
        builder.custom_state().offset = builder.custom_private().offset;
        builder.custom_state().maybe_truncated = builder.custom_private().maybe_truncated;
        builder.custom_state().filter_groups = builder.custom_private().filter_groups.clone();
        builder.build()
    }

    fn explain_custom_scan(
        state: &CustomScanStateWrapper<Self>,
        ancestors: *mut pg_sys::List,
        explainer: &mut Explainer,
    ) {
        explainer.add_text("Index", state.custom_state().indexrel().name());

        // Use pre-computed filter groups from the scan state
        let filter_groups = &state.custom_state().filter_groups;
        explain_execution_strategy(state, filter_groups, explainer);
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
            orderby_info: &state.custom_state().agg_spec.orderby_info,
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
) -> Option<AggregateExtractionResult> {
    let (aggregate_types, filter_groups, filter_uses_search_operator) =
        extract_aggregates(args, bm25_index, heap_rti)?;

    // Create a set of grouping column field names for quick lookup
    let grouping_field_names: crate::api::HashSet<&String> =
        grouping_columns.iter().map(|gc| &gc.field_name).collect();

    // Validate that all aggregate fields are fast fields and don't conflict with GROUP BY
    for aggregate in &aggregate_types {
        if let Some(field_name) = aggregate.field_name() {
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

    Some((aggregate_types, filter_groups, filter_uses_search_operator))
}

/// If the given args consist only of AggregateTypes that we can handle, return them.
fn extract_aggregates(
    args: &CreateUpperPathsHookArgs,
    bm25_index: &PgSearchRelation,
    heap_rti: pg_sys::Index,
) -> Option<AggregateExtractionResult> {
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
    let mut filter_uses_search_operator = false;
    let mut filter_groups: HashMap<String, Vec<usize>> = HashMap::default();

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

                // Extract the aggregate with filter support
                let agg_idx = aggregate_types.len(); // Current index before adding
                let (agg_type, uses_search_op) =
                    AggregateType::try_from(aggref, relation_oid, bm25_index, args.root, heap_rti)?;
                filter_uses_search_operator = filter_uses_search_operator || uses_search_op;

                // Group aggregates by their filter expression during extraction
                let filter_key = if let Some(filter_expr) = agg_type.filter_expr() {
                    // This is the most reliable way to get a deterministic filter key
                    filter_expr.explain_format()
                } else {
                    NO_FILTER_KEY.to_string()
                };
                filter_groups.entry(filter_key).or_default().push(agg_idx);

                aggregate_types.push(agg_type);
            } else {
                // Unsupported expression type
                return None;
            }
        }
    }

    // Convert filter groups to the expected format and sort for deterministic output
    let mut grouped_aggregates: Vec<(Option<SearchQueryInput>, Vec<usize>, String)> = filter_groups
        .into_iter()
        .map(|(filter_key, mut indices)| {
            // Sort indices within each group for deterministic ordering
            indices.sort();
            let filter_expr = if filter_key == NO_FILTER_KEY {
                None
            } else {
                // Get the actual filter expression from the first aggregate in this group
                aggregate_types[indices[0]].filter_expr().clone()
            };
            (filter_expr, indices, filter_key)
        })
        .collect();

    // Sort by filter key to ensure deterministic ordering
    // NO_FILTER groups come first, then sorted by filter expression string
    grouped_aggregates.sort_by(|a, b| {
        match (a.2.as_str(), b.2.as_str()) {
            (NO_FILTER_KEY, NO_FILTER_KEY) => std::cmp::Ordering::Equal,
            (NO_FILTER_KEY, _) => std::cmp::Ordering::Less, // NO_FILTER comes first
            (_, NO_FILTER_KEY) => std::cmp::Ordering::Greater,
            (a_key, b_key) => a_key.cmp(b_key), // Sort other filters alphabetically
        }
    });

    // Remove the filter_key from the result tuple
    let filter_groups = grouped_aggregates
        .into_iter()
        .map(|(filter_expr, indices, _)| (filter_expr, indices))
        .collect();

    // It's valid to have zero aggregates when the query is only a GROUP BY on fast fields
    // (e.g., SELECT category FROM .. GROUP BY category). In that case, we can still build
    // a ParadeDB Aggregate Scan that only returns the grouping keys. Therefore we return
    // an empty vector instead of rejecting the plan.

    Some((aggregate_types, filter_groups, filter_uses_search_operator))
}

/// Extract filter expression from a FILTER clause and track @@@ operator usage
pub unsafe fn extract_filter_clause(
    filter_expr: *mut pg_sys::Expr,
    bm25_index: &PgSearchRelation,
    root: *mut pg_sys::PlannerInfo,
    heap_rti: pg_sys::Index,
    qual_state: &mut QualExtractState,
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
        qual_state, // Pass the state to track @@@ operator usage
        true,       // attempt_pushdown
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
        orderby_info: &state.custom_state().agg_spec.orderby_info,
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
