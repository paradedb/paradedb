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

use crate::api::window_function::window_func_oid;
use crate::api::{FieldName, OrderByFeature, OrderByInfo};
use crate::nodecast;
use crate::postgres::customscan::agg::AggregationSpec;
use crate::postgres::customscan::aggregatescan::extract_filter_clause;
use crate::postgres::customscan::aggregatescan::privdat::{
    parse_coalesce_expression, GroupingColumn,
};
use crate::postgres::customscan::aggregatescan::AggregateType;
use crate::postgres::customscan::qual_inspect::QualExtractState;
use crate::postgres::utils::{determine_sort_direction, resolve_tle_ref};
use crate::postgres::var::get_var_relation_oid;
use crate::postgres::var::{fieldname_from_var, resolve_var_with_parse};
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
pub mod window_functions {
    /// Only allow window function replacement in TopN queries (with ORDER BY and LIMIT).
    /// When true, window functions are only replaced with window_func in TopN execution context.
    /// When false, window functions can be replaced in any query context.
    pub const ONLY_ALLOW_TOP_N: bool = true;

    /// Enable support for window functions in subqueries.
    pub const SUBQUERY_SUPPORT: bool = false;

    /// Enable support for window functions in queries with HAVING clauses.
    pub const HAVING_SUPPORT: bool = false;

    /// Enable support for window functions in queries with JOINs.
    pub const JOIN_SUPPORT: bool = false;

    /// Enable support for `PARTITION BY` clause in window functions.
    pub const WINDOW_AGG_PARTITION_BY: bool = false;

    /// Enable support for `ORDER BY` clause in window functions.
    pub const WINDOW_AGG_ORDER_BY: bool = false;

    /// Enable support for `FILTER` clause in window functions.
    pub const WINDOW_AGG_FILTER_CLAUSE: bool = false;

    /// Supported aggregate functions in window functions
    pub mod aggregates {
        /// Enable support for `COUNT(*)` in window functions.
        pub const COUNT_ANY: bool = true;

        /// Enable support for `COUNT(field)` in window functions.
        pub const COUNT: bool = false;

        /// Enable support for `SUM(field)` in window functions.
        pub const SUM: bool = false;

        /// Enable support for `AVG(field)` in window functions.
        pub const AVG: bool = false;

        /// Enable support for `MIN(field)` in window functions.
        pub const MIN: bool = false;

        /// Enable support for `MAX(field)` in window functions.
        pub const MAX: bool = false;
    }
}

/// Information about a window aggregate to compute during TopN execution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WindowAggregateInfo {
    /// Target entry index where this aggregate should be projected
    pub target_entry_index: usize,
    /// Aggregation specification (shared with aggregatescan)
    pub agg_spec: AggregationSpec,
}

impl WindowAggregateInfo {
    pub fn result_type_oid(&self) -> pg_sys::Oid {
        self.agg_spec.result_type_oid()
    }

    /// Check if we can handle this aggregation specification
    ///
    /// This is primarily used by window functions to check feature flag support.
    /// Execution capability is determined by feature flags.
    pub fn is_supported(agg_spec: &AggregationSpec) -> bool {
        // Check if all aggregate functions are supported
        for agg_type in &agg_spec.agg_types {
            if !Self::is_window_agg_supported(agg_type) {
                return false;
            }
            // Check if this aggregate has a filter
            let has_filter = agg_type.has_filter();
            if has_filter && !window_functions::WINDOW_AGG_FILTER_CLAUSE {
                return false;
            }
        }

        // Check grouping/partitioning support
        let has_grouping = !agg_spec.grouping_columns.is_empty();
        if has_grouping && !window_functions::WINDOW_AGG_PARTITION_BY {
            return false;
        }

        // Check ordering support
        let has_order_by = !agg_spec.orderby_info.is_empty();
        if has_order_by && !window_functions::WINDOW_AGG_ORDER_BY {
            return false;
        }

        // All required features are supported
        true
    }

    /// Check if a specific aggregate type is supported (for window functions)
    fn is_window_agg_supported(agg_type: &AggregateType) -> bool {
        use crate::postgres::customscan::pdbscan::projections::window_agg::window_functions;

        match agg_type {
            AggregateType::CountAny { .. } => window_functions::aggregates::COUNT_ANY,
            AggregateType::Count { .. } => window_functions::aggregates::COUNT,
            AggregateType::Sum { .. } => window_functions::aggregates::SUM,
            AggregateType::Avg { .. } => window_functions::aggregates::AVG,
            AggregateType::Min { .. } => window_functions::aggregates::MIN,
            AggregateType::Max { .. } => window_functions::aggregates::MAX,
        }
    }

    /// Fill in the attno field for GroupingColumns
    ///
    /// During planner hook time, GroupingColumns have attno=0 (placeholder).
    /// This function fills in the real attno values using the relation descriptor.
    ///
    /// # Safety
    /// Must be called with a valid relation descriptor.
    pub unsafe fn fill_partition_by_attnos(&mut self, heaprel: &PgSearchRelation) {
        let heap_relation = heaprel.heap_relation().unwrap();
        let tuple_desc = heap_relation.tuple_desc();

        for grouping_col in &mut self.agg_spec.grouping_columns {
            if grouping_col.attno == 0 {
                // Find the attribute number for this field name
                for i in 0..tuple_desc.len() {
                    if let Some(attr) = tuple_desc.get(i) {
                        if attr.name() == grouping_col.field_name {
                            grouping_col.attno = (i + 1) as pg_sys::AttrNumber;
                            break;
                        }
                    }
                }
            }
        }
    }
}

/// Extract window aggregates from a query with all-or-nothing support
///
/// This function implements an all-or-nothing approach: either ALL window functions
/// in the query are supported (and get replaced with window_func placeholders),
/// or NONE of them are replaced and PostgreSQL handles all window functions with
/// standard execution.
///
/// This ensures consistent execution - we don't mix our custom window function execution
/// with PostgreSQL's standard window function execution in the same query.
///
/// Parameters:
/// - parse: The Query object containing all query information
///
/// Returns a HashMap mapping target_entry_index -> WindowSpecification
pub unsafe fn extract_window_specifications(
    parse: *mut pg_sys::Query,
) -> HashMap<usize, AggregationSpec> {
    // Check TopN context requirement if enabled
    if window_functions::ONLY_ALLOW_TOP_N {
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
    if !window_functions::HAVING_SUPPORT && !(*parse).havingQual.is_null() {
        // Query has HAVING clause but we don't support it - return empty map
        return HashMap::new();
    }

    // Check JOIN support
    if !window_functions::JOIN_SUPPORT && !(*parse).rtable.is_null() {
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
        if let Some(window_func) = nodecast!(WindowFunc, T_WindowFunc, (*te).expr) {
            // Extract the aggregate function and its details first
            if let Some(agg_type) = extract_standard_aggregate(parse, window_func) {
                // Extract complete aggregation specification (aggregate type, PARTITION BY, ORDER BY)
                let agg_spec = extract_aggregation_spec(parse, agg_type, window_func);

                // Only include supported window functions
                if WindowAggregateInfo::is_supported(&agg_spec) {
                    window_aggs.insert(idx, agg_spec);
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
    window_func: *mut pg_sys::WindowFunc,
) -> Option<AggregateType> {
    use pg_sys::*;

    let aggfnoid = (*window_func).winfnoid.to_u32();
    let args = PgList::<pg_sys::Node>::from_pg((*window_func).args);

    // Extract FILTER clause if present
    let filter = if !(*window_func).aggfilter.is_null() {
        extract_filter_expression((*window_func).aggfilter)
    } else {
        None
    };

    // Handle COUNT(*) special case - same logic as aggregatescan
    if aggfnoid == F_COUNT_ && args.is_empty() {
        return Some(AggregateType::CountAny { filter });
    }

    // For other aggregates, we need a field name
    if args.is_empty() {
        return None;
    }

    let first_arg = args.get_ptr(0)?;

    // Extract field name and missing value using the same logic as aggregatescan
    let (field, missing) = parse_aggregate_field_from_node(parse, first_arg)?;

    let agg_type =
        AggregateType::create_aggregate_from_oid(aggfnoid, field.into_inner(), missing, filter)?;
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

    // Get heaprelid from the rtable using the helper function
    let heaprelid = get_var_relation_oid(parse, var)?;
    let field = fieldname_from_var(heaprelid, var, (*var).varattno as pg_sys::AttrNumber)?;
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
/// - PARTITION BY columns (grouping_columns)
/// - ORDER BY specification
unsafe fn extract_aggregation_spec(
    parse: *mut pg_sys::Query,
    agg_type: AggregateType,
    window_func: *mut pg_sys::WindowFunc,
) -> AggregationSpec {
    // Get the WindowClause from winref (if it exists)
    // winref is an index (1-based) into the query's windowClause list
    let winref = (*window_func).winref;

    if winref == 0 {
        // No window clause - means empty OVER ()
        return AggregationSpec {
            agg_types: vec![agg_type],
            grouping_columns: Vec::new(),
            orderby_info: Vec::new(),
        };
    }

    // Access the WindowClause from the list
    if (*parse).windowClause.is_null() {
        return AggregationSpec {
            agg_types: vec![agg_type],
            grouping_columns: Vec::new(),
            orderby_info: Vec::new(),
        };
    }

    let window_clauses = PgList::<pg_sys::WindowClause>::from_pg((*parse).windowClause);

    // winref is 1-based, but list is 0-indexed
    let window_clause_idx = (winref - 1) as usize;

    if window_clause_idx >= window_clauses.len() {
        return AggregationSpec {
            agg_types: vec![agg_type],
            grouping_columns: Vec::new(),
            orderby_info: Vec::new(),
        };
    }

    let window_clause = window_clauses.get_ptr(window_clause_idx).unwrap();

    let grouping_columns = extract_partition_by(parse, (*window_clause).partitionClause);
    let orderby_info = extract_order_by(parse, (*window_clause).orderClause);

    AggregationSpec {
        agg_types: vec![agg_type],
        grouping_columns,
        orderby_info,
    }
}

/// Extract PARTITION BY columns from partitionClause
///
/// Returns GroupingColumn with field_name set and attno=0 (placeholder).
/// The attno will be filled in later during custom scan planning when we have
/// access to the relation descriptor.
unsafe fn extract_partition_by(
    parse: *mut pg_sys::Query,
    partition_clause: *mut pg_sys::List,
) -> Vec<GroupingColumn> {
    if partition_clause.is_null() || parse.is_null() || (*parse).targetList.is_null() {
        return Vec::new();
    }

    let partition_list = PgList::<pg_sys::Node>::from_pg(partition_clause);
    if partition_list.is_empty() {
        return Vec::new();
    }

    let mut grouping_columns = Vec::new();
    for (idx, node) in partition_list.iter_ptr().enumerate() {
        // Each node should be a SortGroupClause
        if let Some(sort_clause) = nodecast!(SortGroupClause, T_SortGroupClause, node) {
            let tle_ref = (*sort_clause).tleSortGroupRef;

            // Resolve directly using target_list
            let column_name = resolve_tle_ref(tle_ref, (*parse).targetList)
                .unwrap_or(format!("unresolved_tle_{}", tle_ref));
            grouping_columns.push(GroupingColumn {
                field_name: column_name,
                attno: 0, // Placeholder - will be filled in during custom scan planning
            });
        } else if let Some(var) = nodecast!(Var, T_Var, node) {
            let field_name = resolve_var_with_parse(parse, var)
                .unwrap_or(format!("unresolved_var_{}", (*var).varattno).into());
            grouping_columns.push(GroupingColumn {
                field_name: field_name.into_inner(),
                attno: 0, // Placeholder - will be filled in during custom scan planning
            });
        }
    }
    grouping_columns
}

/// Extract ORDER BY specification from orderClause
/// Returns empty Vec if no ORDER BY
unsafe fn extract_order_by(
    parse: *mut pg_sys::Query,
    order_clause: *mut pg_sys::List,
) -> Vec<OrderByInfo> {
    if order_clause.is_null() || parse.is_null() || (*parse).targetList.is_null() {
        return Vec::new();
    }

    let order_list = PgList::<pg_sys::Node>::from_pg(order_clause);
    if order_list.is_empty() {
        return Vec::new();
    }

    let mut order_by_infos = Vec::new();

    for (idx, node) in order_list.iter_ptr().enumerate() {
        // Each node should be a SortGroupClause
        if let Some(sort_clause) = nodecast!(SortGroupClause, T_SortGroupClause, node) {
            let tle_ref = (*sort_clause).tleSortGroupRef;
            let sort_op = (*sort_clause).sortop;

            // Resolve column name directly using target_list
            let column_name = resolve_tle_ref(tle_ref, (*parse).targetList)
                .unwrap_or(format!("unresolved_tle_{}", tle_ref));
            // Determine sort direction from sort operator
            let direction = determine_sort_direction(sort_op);

            let field_name = FieldName::from(column_name.as_str());
            order_by_infos.push(OrderByInfo {
                feature: OrderByFeature::Field(field_name),
                direction,
            });
        }
    }

    order_by_infos
}

/// Extract window_func(json) calls from the processed target list at planning time
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
        // Fill in attno values for GROUP BY/PARTITION BY columns
        // During planner hook time, these were set to 0 (placeholder)
        window_agg.fill_partition_by_attnos(bm25_index);

        // Convert filters for all aggregates in this spec
        for agg_type in &mut window_agg.agg_spec.agg_types {
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
                        // Cast Node back to Expr for extract_filter_clause
                        let filter_expr = filter_node as *mut pg_sys::Expr;

                        // Use the same logic as aggregatescan to convert the filter
                        let mut filter_qual_state = QualExtractState::default();
                        let converted = extract_filter_clause(
                            filter_expr,
                            bm25_index,
                            root,
                            heap_rti,
                            &mut filter_qual_state,
                        );

                        // Replace the PostgresExpression with the converted SearchQueryInput
                        if let Some(search_query) = converted {
                            *filter = search_query;
                        }
                    }
                }
            }
        }
    }
}

/// Extract window_func(json) calls from a target list and create WindowAggregateInfo
///
/// This function:
/// 1. Iterates through target entries in the PROVIDED target list (usually processed_tlist)
/// 2. Finds `paradedb.window_func(json)` calls
/// 3. Deserializes the JSON to get `WindowSpecification`
/// 4. Creates `WindowAggregateInfo` with the CURRENT position as target_entry_index
pub unsafe fn extract_window_func_calls(tlist: *mut pg_sys::List) -> Vec<WindowAggregateInfo> {
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
            if (*funcexpr).funcid == (*context).window_func_procid {
                // Found a window_func(json) call - deserialize it
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
                            match serde_json::from_str::<AggregationSpec>(json_str) {
                                Ok(agg_spec) => {
                                    let info = WindowAggregateInfo {
                                        target_entry_index: (*context).current_te_index,
                                        agg_spec,
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
        window_func_procid: pg_sys::Oid,
        window_aggs: Vec<WindowAggregateInfo>,
        current_te_index: usize,
    }

    let mut context = Context {
        window_func_procid: window_func_oid(),
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
