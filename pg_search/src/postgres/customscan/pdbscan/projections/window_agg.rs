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

use crate::api::agg_funcoid;
use crate::api::operator::anyelement_query_input_opoid;
use crate::api::window_aggregate::window_agg_oid;
use crate::api::FieldName;
use crate::nodecast;
use crate::postgres::customscan::aggregatescan::aggregate_type::{
    parse_coalesce_expression, AggregateType,
};
use crate::postgres::customscan::aggregatescan::targetlist::TargetList;
use crate::postgres::customscan::builders::custom_path::RestrictInfoType;
use crate::postgres::customscan::qual_inspect::{extract_quals, QualExtractState};
use crate::postgres::var::{fieldname_from_var, VarContext};
use crate::postgres::PgSearchRelation;
use crate::query::{PostgresExpression, SearchQueryInput};
use pgrx::{pg_sys, PgList};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Feature flags for window functions.
///
/// These constants control the enablement of experimental or incomplete window function features.
/// They are used during the planning phase to determine if a custom scan can handle
/// a particular window function construct.
///
/// When a feature is fully implemented and stable, its flag should be set to `true`.
pub mod window_aggregates {
    /// Only allow window function replacement in TopN queries (with ORDER BY and LIMIT).
    /// When true, window functions are only replaced with window_agg in TopN execution context.
    /// When false, window functions can be replaced in any query context.
    pub const ONLY_ALLOW_TOP_N: bool = true;

    /// Enable support for window functions in subqueries.
    pub const SUBQUERY_SUPPORT: bool = false;

    /// Enable support for window functions in queries with HAVING clauses.
    pub const HAVING_SUPPORT: bool = false;

    /// Enable support for window functions in queries with JOINs.
    pub const JOIN_SUPPORT: bool = false;

    /// Enable support for `FILTER` clause in window functions.
    pub const WINDOW_AGG_FILTER_CLAUSE: bool = false;
}

/// Information about a window aggregate to compute during TopN execution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WindowAggregateInfo {
    /// Target entry index where this aggregate should be projected
    pub target_entry_index: usize,
    /// Target list containing the aggregate (shared structure with aggregatescan)
    pub targetlist: TargetList,
}

impl WindowAggregateInfo {
    pub fn result_type_oid(&self) -> pg_sys::Oid {
        self.targetlist.singleton_result_type_oid()
    }

    /// Check if we can handle this aggregation specification
    ///
    /// This is primarily used by window functions to check feature flag support.
    /// Execution capability is determined by feature flags.
    pub fn is_supported(targetlist: &Option<TargetList>) -> bool {
        if targetlist.is_none() {
            return false;
        }
        let tlist = targetlist.as_ref().unwrap();

        // Check if all aggregate functions are supported
        for agg_type in tlist.aggregates() {
            // Check if this aggregate has a filter
            let has_filter = agg_type.has_filter();
            if has_filter && !window_aggregates::WINDOW_AGG_FILTER_CLAUSE {
                return false;
            }
        }

        // Note: PARTITION BY and ORDER BY in OVER clauses are not supported in our use case,
        // because we compute facets over the entire result set, not partitioned subsets.
        // If grouping_columns is non-empty, we reject the query.
        if !tlist.grouping_columns().is_empty() {
            return false;
        }

        // All required features are supported
        true
    }
}

/// Extract window aggregates from a query with all-or-nothing support
///
/// This function implements an all-or-nothing approach: either ALL window functions
/// in the query are supported (and get replaced with window_agg placeholders),
/// or NONE of them are replaced and PostgreSQL handles all window functions with
/// standard execution.
///
/// This ensures consistent execution - we don't mix our custom window function execution
/// with PostgreSQL's standard window function execution in the same query.
///
/// Parameters:
/// - parse: The Query object containing all query information
///
/// Returns a HashMap mapping target_entry_index -> TargetList
pub unsafe fn extract_window_specifications(
    parse: *mut pg_sys::Query,
) -> HashMap<usize, TargetList> {
    // Check TopN context requirement if enabled
    if window_aggregates::ONLY_ALLOW_TOP_N {
        let has_order_by = !(*parse).sortClause.is_null();
        let has_limit = !(*parse).limitCount.is_null();
        let is_top_n_query = has_order_by && has_limit;
        if !is_top_n_query {
            // Not a TopN query - return empty map so PostgreSQL handles all window functions
            return HashMap::new();
        }
    }

    // Check query context features
    // Check HAVING clause support
    if !window_aggregates::HAVING_SUPPORT && !(*parse).havingQual.is_null() {
        // Query has HAVING clause but we don't support it - return empty map
        return HashMap::new();
    }

    // Check JOIN support
    if !window_aggregates::JOIN_SUPPORT && !(*parse).rtable.is_null() {
        let rtable = PgList::<pg_sys::RangeTblEntry>::from_pg((*parse).rtable);
        let relation_count = rtable
            .iter_ptr()
            .filter(|rte| (**rte).rtekind == pg_sys::RTEKind::RTE_RELATION)
            .count();

        if relation_count > 1 {
            // Query has multiple relations (likely JOINs) but we don't support it
            return HashMap::new();
        }
    }

    // Note: SUBQUERY_SUPPORT is checked at a higher level in the planner hook
    // since subqueries are processed recursively

    let mut window_aggs = HashMap::new();
    let tlist = PgList::<pg_sys::TargetEntry>::from_pg((*parse).targetList);

    // Extract all window functions and check if they're supported
    for (idx, te) in tlist.iter_ptr().enumerate() {
        if let Some(window_agg) = nodecast!(WindowFunc, T_WindowFunc, (*te).expr) {
            // Extract the aggregate function and its details first
            if let Some(agg_type) = extract_standard_aggregate(parse, window_agg) {
                // Extract complete aggregation specification (aggregate type and ORDER BY)
                let agg_spec = extract_aggregation_spec(parse, agg_type, window_agg);

                // Only include supported window functions
                if WindowAggregateInfo::is_supported(&agg_spec) {
                    window_aggs.insert(idx, agg_spec.unwrap());
                } else {
                    // Found an unsupported window function - abort and return empty map
                    // so PostgreSQL handles ALL window functions in this query
                    return HashMap::new();
                }
            }
        }
    }

    window_aggs
}

/// Extract window aggregate function using OID-based approach (same as aggregatescan)
///
/// Returns: AggregateType
unsafe fn extract_standard_aggregate(
    parse: *mut pg_sys::Query,
    window_agg: *mut pg_sys::WindowFunc,
) -> Option<AggregateType> {
    use pg_sys::*;
    use pgrx::{FromDatum, JsonB};

    let aggfnoid = (*window_agg).winfnoid.to_u32();
    let args = PgList::<pg_sys::Node>::from_pg((*window_agg).args);

    // Extract FILTER clause if present
    let filter = if !(*window_agg).aggfilter.is_null() {
        extract_filter_expression((*window_agg).aggfilter)
    } else {
        None
    };

    // Handle custom agg function
    let custom_agg_oid = agg_funcoid().to_u32();
    if aggfnoid == custom_agg_oid {
        if args.is_empty() {
            return None;
        }

        // Extract the jsonb argument
        let first_arg = args.get_ptr(0)?;
        let json_value = if let Some(const_node) = nodecast!(Const, T_Const, first_arg) {
            if (*const_node).constisnull {
                return None;
            }
            let jsonb_datum = (*const_node).constvalue;
            let jsonb = JsonB::from_datum(jsonb_datum, false)?;
            jsonb.0
        } else {
            return None;
        };

        return Some(AggregateType::Custom {
            agg_json: json_value,
            filter,
            indexrelid: pg_sys::InvalidOid, // Will be filled in during planning
        });
    }

    // Handle COUNT(*) special case - same logic as aggregatescan
    if aggfnoid == F_COUNT_ && args.is_empty() {
        return Some(AggregateType::CountAny {
            filter,
            indexrelid: pg_sys::InvalidOid, // Will be filled in during planning
        });
    }

    // For other aggregates, we need a field name
    if args.is_empty() {
        return None;
    }

    let first_arg = args.get_ptr(0)?;

    // Extract field name and missing value using the same logic as aggregatescan
    let (field, missing) = parse_aggregate_field_from_node(parse, first_arg)?;

    let agg_type = AggregateType::create_aggregate_from_oid(
        aggfnoid,
        field.into_inner(),
        missing,
        filter,
        pg_sys::InvalidOid, // Will be filled in during planning
    )?;
    Some(agg_type)
}

/// Parse field name and missing value from a Node argument (for window functions)
/// This is similar to aggregatescan's parse_aggregate_field but works with Node instead of TargetEntry
unsafe fn parse_aggregate_field_from_node(
    parse: *mut pg_sys::Query,
    arg_node: *mut pg_sys::Node,
) -> Option<(FieldName, Option<f64>)> {
    let (var, missing) =
        if let Some(coalesce_node) = nodecast!(CoalesceExpr, T_CoalesceExpr, arg_node) {
            parse_coalesce_expression(coalesce_node)?
        } else if let Some(var) = nodecast!(Var, T_Var, arg_node) {
            (var, None)
        } else {
            return None;
        };

    // Get heaprelid from the rtable using VarContext
    let var_context = VarContext::from_query(parse);
    let (heaprelid, varattno) = var_context.var_relation(var);

    if heaprelid == pg_sys::InvalidOid {
        return None;
    }

    let field = fieldname_from_var(heaprelid, var, varattno)?;
    Some((field, missing))
}

/// Extract FILTER expression by serializing it for later conversion
///
/// ## Why we can't convert now:
/// We can't use extract_quals here because root (PlannerInfo) doesn't exist yet
/// in the planner_hook (it's created by standard_planner which runs after).
///
/// ## How we preserve the FILTER:
/// We wrap the filter expression in a PostgresExpression, which:
/// 1. **At planner hook time (now)**: Calls nodeToString() during JSON serialization,
///    converting the node tree to a string representation
/// 2. **At planning time (later)**: Calls stringToNode() during JSON deserialization,
///    recreating the node tree in the planning memory context
///
/// This is safe because:
/// - nodeToString creates a new string copy (not a pointer to planner hook memory)
/// - stringToNode allocates new nodes in current memory context
/// - The deserialized nodes live as long as needed for planning and execution
unsafe fn extract_filter_expression(filter_expr: *mut pg_sys::Expr) -> Option<SearchQueryInput> {
    if filter_expr.is_null() {
        return None;
    }
    // Serialize the filter expression - nodeToString will be called during JSON serialization
    let filter_node = filter_expr as *mut pg_sys::Node;
    Some(SearchQueryInput::PostgresExpression {
        expr: PostgresExpression::new(filter_node),
    })
}

/// Extract complete aggregation specification from a WindowFunc node
///
/// This function extracts:
/// - Aggregate type (with FILTER clause)
/// - ORDER BY specification
unsafe fn extract_aggregation_spec(
    parse: *mut pg_sys::Query,
    agg_type: AggregateType,
    window_agg: *mut pg_sys::WindowFunc,
) -> Option<TargetList> {
    // Get the WindowClause from winref (if it exists)
    // winref is an index (1-based) into the query's windowClause list
    let winref = (*window_agg).winref;

    if winref == 0 {
        // No window clause - means empty OVER ()
        return Some(TargetList::new_for_window_function(agg_type));
    }

    // Access the WindowClause from the list
    if (*parse).windowClause.is_null() {
        return Some(TargetList::new_for_window_function(agg_type));
    }

    let window_clauses = PgList::<pg_sys::WindowClause>::from_pg((*parse).windowClause);

    // winref is 1-based, but list is 0-indexed
    let window_clause_idx = (winref - 1) as usize;

    if window_clause_idx >= window_clauses.len() {
        return Some(TargetList::new_for_window_function(agg_type));
    }

    let window_clause = window_clauses.get_ptr(window_clause_idx).unwrap();

    let has_partition_by = has_partition_by(parse, (*window_clause).partitionClause);
    let has_frame_clause = has_frame_clause(
        (*window_clause).frameOptions,
        (*window_clause).startOffset,
        (*window_clause).endOffset,
    );
    let has_order_by = has_order_by(parse, (*window_clause).orderClause);

    // Reject if PARTITION BY, frame clause, or ORDER BY is present
    if has_partition_by || has_frame_clause || has_order_by {
        return None;
    }

    Some(TargetList::new_for_window_function(agg_type)) // PARTITION BY is not supported for window functions
}

/// Check if there's a frame clause
unsafe fn has_frame_clause(
    frame_options: i32, // frameOptions is a bitmask containing frame type and bounds
    start_offset: *mut pg_sys::Node,
    end_offset: *mut pg_sys::Node,
) -> bool {
    const FRAMEOPTION_NONDEFAULT: i32 = 0x00001;
    // Check if there's a non-default frame clause
    frame_options & FRAMEOPTION_NONDEFAULT != 0
}

/// Check if there's a PARTITION BY clause
unsafe fn has_partition_by(parse: *mut pg_sys::Query, partition_clause: *mut pg_sys::List) -> bool {
    if partition_clause.is_null() || parse.is_null() || (*parse).targetList.is_null() {
        return false;
    }

    let partition_list = PgList::<pg_sys::Node>::from_pg(partition_clause);
    !partition_list.is_empty()
}

/// Check if there's an ORDER BY clause
unsafe fn has_order_by(parse: *mut pg_sys::Query, order_clause: *mut pg_sys::List) -> bool {
    if order_clause.is_null() || parse.is_null() || (*parse).targetList.is_null() {
        return false;
    }

    let order_list = PgList::<pg_sys::Node>::from_pg(order_clause);
    !order_list.is_empty()
}

/// Extract window_agg(json) calls from the processed target list at planning time
/// Convert PostgresExpression filters to SearchQueryInput
///
/// This is called at plan_custom_path time when we have access to root (PlannerInfo),
/// allowing us to use extract_filter_clause to properly convert FILTER expressions
/// (same logic as aggregatescan).
pub unsafe fn convert_window_aggregate_filters(
    window_aggregates: &mut [WindowAggregateInfo],
    bm25_index: &PgSearchRelation,
    root: *mut pg_sys::PlannerInfo,
    heap_rti: pg_sys::Index,
) {
    for window_agg in window_aggregates.iter_mut() {
        // Convert filters for all aggregates in this targetlist
        for agg_type in window_agg.targetlist.aggregates_mut() {
            // Check if this aggregate has a FILTER
            if !agg_type.has_filter() {
                continue;
            }

            // Try to get the filter
            let filter_opt = agg_type.get_filter_mut();
            if let Some(filter) = filter_opt {
                // Check if it's a PostgresExpression that needs conversion
                if let SearchQueryInput::PostgresExpression { expr } = filter {
                    let filter_node = expr.node();
                    if !filter_node.is_null() {
                        // Use the same logic as aggregatescan to convert the filter
                        let mut filter_qual_state = QualExtractState::default();
                        if let Some(qual) = extract_quals(
                            root,
                            heap_rti,
                            filter_node,
                            anyelement_query_input_opoid(),
                            RestrictInfoType::BaseRelation,
                            bm25_index,
                            false, // convert_external_to_special_qual
                            &mut filter_qual_state,
                            true, // attempt_pushdown
                        ) {
                            // Replace the PostgresExpression with the converted SearchQueryInput
                            *filter = SearchQueryInput::from(&qual);
                        }
                    }
                }
            }
        }
    }
}

/// Extract window_agg(json) calls from a target list and create WindowAggregateInfo
///
/// This function:
/// 1. Iterates through target entries in the PROVIDED target list (usually processed_tlist)
/// 2. Finds `paradedb.window_agg(json)` calls
/// 3. Deserializes the JSON to get `WindowSpecification`
/// 4. Creates `WindowAggregateInfo` with the CURRENT position as target_entry_index
pub unsafe fn extract_window_agg_calls(tlist: *mut pg_sys::List) -> Vec<WindowAggregateInfo> {
    use pgrx::pg_guard;
    use pgrx::pg_sys::expression_tree_walker;
    use std::ffi::CStr;
    use std::ptr::addr_of_mut;

    if tlist.is_null() {
        return Vec::new();
    }

    #[pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        data: *mut core::ffi::c_void,
    ) -> bool {
        if node.is_null() {
            return false;
        }

        if let Some(funcexpr) = nodecast!(FuncExpr, T_FuncExpr, node) {
            let context = data.cast::<Context>();
            if (*funcexpr).funcid == (*context).window_agg_procid {
                // Found a window_agg(json) call - deserialize it
                let args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
                if let Some(json_arg) = args.get_ptr(0) {
                    if let Some(const_node) = nodecast!(Const, T_Const, json_arg) {
                        if !(*const_node).constisnull {
                            let json_datum = (*const_node).constvalue;
                            let json_varlena = json_datum.cast_mut_ptr::<pg_sys::varlena>();
                            let json_varlena_detoasted =
                                pg_sys::pg_detoast_datum(json_varlena.cast());
                            let json_text = pg_sys::text_to_cstring(json_varlena_detoasted.cast());
                            let json_str =
                                CStr::from_ptr(json_text).to_str().expect("invalid UTF-8");

                            // Deserialize AggregationSpec and create WindowAggregateInfo
                            // with the correct target_entry_index from the current position
                            match serde_json::from_str::<TargetList>(json_str) {
                                Ok(targetlist) => {
                                    let info = WindowAggregateInfo {
                                        target_entry_index: (*context).current_te_index,
                                        targetlist,
                                    };
                                    (*context).window_aggs.push(info);
                                }
                                Err(e) => {}
                            }
                        }
                    }
                }
            }
        }

        expression_tree_walker(node, Some(walker), data)
    }

    struct Context {
        window_agg_procid: pg_sys::Oid,
        window_aggs: Vec<WindowAggregateInfo>,
        current_te_index: usize,
    }

    let window_agg_procid = window_agg_oid();

    // If window_agg function doesn't exist yet (e.g., during extension creation), return empty list
    if window_agg_procid == pg_sys::InvalidOid {
        return Vec::new();
    }

    let mut context = Context {
        window_agg_procid,
        window_aggs: Vec::new(),
        current_te_index: 0,
    };

    // Iterate through target entries explicitly to track their indices
    let target_entries = PgList::<pg_sys::TargetEntry>::from_pg(tlist);
    for (idx, te) in target_entries.iter_ptr().enumerate() {
        context.current_te_index = idx;
        walker((*te).expr.cast(), addr_of_mut!(context).cast());
    }

    context.window_aggs
}
