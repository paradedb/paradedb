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
use crate::postgres::customscan::aggregatescan::extract_filter_clause;
use crate::postgres::customscan::aggregatescan::privdat::parse_coalesce_expression;
use crate::postgres::customscan::aggregatescan::AggregateType;
use crate::postgres::customscan::qual_inspect::QualExtractState;
use crate::postgres::utils::{determine_sort_direction, resolve_tle_ref};
use crate::postgres::var::get_var_relation_oid;
use crate::postgres::var::{fieldname_from_var, resolve_var_with_parse};
use crate::postgres::PgSearchRelation;
use crate::query::{PostgresExpression, SearchQueryInput};
use pgrx::{pg_sys, PgList};
use serde::{Deserialize, Serialize};

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

    /// Enable support for custom frame clauses (e.g., `ROWS BETWEEN ...`, `RANGE BETWEEN ...`).
    pub const WINDOW_AGG_FRAME_CLAUSES: bool = false;

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
    /// Window specification (aggregate type (with optional FILTER), PARTITION BY, ORDER BY, frame clause)
    pub window_spec: WindowSpecification,
}

impl WindowAggregateInfo {
    pub fn result_type_oid(&self) -> pg_sys::Oid {
        match &self.window_spec.agg_type {
            AggregateType::CountAny { .. } | AggregateType::Count { .. } => pg_sys::INT8OID,
            AggregateType::Sum { .. }
            | AggregateType::Avg { .. }
            | AggregateType::Min { .. }
            | AggregateType::Max { .. } => pg_sys::FLOAT8OID,
        }
    }
}

/// Window specification from the OVER clause
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WindowSpecification {
    /// The aggregate type (COUNT, SUM, AVG, MIN, MAX with optional filter support)
    pub agg_type: AggregateType,
    /// PARTITION BY columns (empty if no partitioning)
    pub partition_by: Vec<String>,
    /// ORDER BY specification (None if no ordering)
    /// Reuses existing OrderByInfo structure from api module
    pub order_by: Option<Vec<OrderByInfo>>,
    /// Window frame clause (None if default)
    pub frame_clause: Option<FrameClause>,
}

/// Window frame clause
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrameClause {
    pub frame_type: FrameType,
    pub start_bound: FrameBound,
    pub end_bound: Option<FrameBound>,
}

/// Frame type (ROWS, RANGE, or GROUPS)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FrameType {
    Rows,
    Range,
    Groups,
}

/// Frame boundary specification
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FrameBound {
    UnboundedPreceding,
    Preceding(i64),
    CurrentRow,
    Following(i64),
    UnboundedFollowing,
}

impl WindowSpecification {
    /// Check if we can handle this window specification
    /// Execution capability is determined by feature flags.
    pub fn is_supported(&self) -> bool {
        // First check if the aggregate function itself is supported
        if !self.is_aggregate_supported() {
            return false;
        }

        // Check each feature against its flag
        let has_filter = self.agg_type.has_filter();
        if has_filter && !window_functions::WINDOW_AGG_FILTER_CLAUSE {
            return false;
        }
        let has_partition_by = !self.partition_by.is_empty();
        if has_partition_by && !window_functions::WINDOW_AGG_PARTITION_BY {
            return false;
        }
        let has_order_by = self.order_by.is_some();
        if has_order_by && !window_functions::WINDOW_AGG_ORDER_BY {
            return false;
        }
        let has_frame = self.frame_clause.is_some();
        if has_frame && !window_functions::WINDOW_AGG_FRAME_CLAUSES {
            return false;
        }

        // All required features are supported
        true
    }

    /// Check if the aggregate function type is supported
    fn is_aggregate_supported(&self) -> bool {
        match &self.agg_type {
            AggregateType::CountAny { .. } => window_functions::aggregates::COUNT_ANY,
            AggregateType::Count { .. } => window_functions::aggregates::COUNT,
            AggregateType::Sum { .. } => window_functions::aggregates::SUM,
            AggregateType::Avg { .. } => window_functions::aggregates::AVG,
            AggregateType::Min { .. } => window_functions::aggregates::MIN,
            AggregateType::Max { .. } => window_functions::aggregates::MAX,
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
pub unsafe fn extract_window_aggregates(parse: *mut pg_sys::Query) -> Vec<WindowAggregateInfo> {
    // Check TopN context requirement if enabled
    if window_functions::ONLY_ALLOW_TOP_N {
        let has_order_by = !(*parse).sortClause.is_null();
        let has_limit = !(*parse).limitCount.is_null();
        let is_top_n_query = has_order_by && has_limit;
        if !is_top_n_query {
            // Not a TopN query - return empty vec so PostgreSQL handles all window functions
            return Vec::new();
        }
    }

    // Check query context features
    // Check HAVING clause support
    if !window_functions::HAVING_SUPPORT && !(*parse).havingQual.is_null() {
        // Query has HAVING clause but we don't support it - return empty vec
        return Vec::new();
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
            return Vec::new();
        }
    }

    // Note: SUBQUERY_SUPPORT is checked at a higher level in the planner hook
    // since subqueries are processed recursively

    let mut potential_window_aggs = Vec::new();
    let tlist = PgList::<pg_sys::TargetEntry>::from_pg((*parse).targetList);

    // First pass: extract all window functions and check if they're supported
    for (idx, te) in tlist.iter_ptr().enumerate() {
        if let Some(window_func) = nodecast!(WindowFunc, T_WindowFunc, (*te).expr) {
            // Extract the aggregate function and its details first
            if let Some(agg_type) = extract_standard_aggregate(parse, window_func) {
                // Extract complete window specification (aggregate type, PARTITION BY, ORDER BY, frame, etc.)
                let window_spec = extract_window_specification(parse, agg_type, window_func);

                let window_agg_info = WindowAggregateInfo {
                    target_entry_index: idx,
                    window_spec,
                };

                potential_window_aggs.push(window_agg_info);
            }
        }
    }

    // Second pass: check if ALL window functions are supported
    let all_supported = potential_window_aggs
        .iter()
        .all(|agg| agg.window_spec.is_supported());

    if all_supported {
        // All window functions are supported - return them for custom execution
        potential_window_aggs
    } else {
        // Some window functions are not supported - return empty vec so PostgreSQL handles all
        Vec::new()
    }
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

/// Extract complete window specification from a WindowFunc node
///
/// This function extracts:
/// - Aggregate type (with FILTER clause)
/// - PARTITION BY columns
/// - ORDER BY specification
/// - Frame clause (ROWS/RANGE/GROUPS BETWEEN...)
unsafe fn extract_window_specification(
    parse: *mut pg_sys::Query,
    agg_type: AggregateType,
    window_func: *mut pg_sys::WindowFunc,
) -> WindowSpecification {
    // Get the WindowClause from winref (if it exists)
    // winref is an index (1-based) into the query's windowClause list
    let winref = (*window_func).winref;

    if winref == 0 {
        // No window clause - means empty OVER ()
        return WindowSpecification {
            agg_type,
            partition_by: Vec::new(),
            order_by: None,
            frame_clause: None,
        };
    }

    // Access the WindowClause from the list
    if (*parse).windowClause.is_null() {
        return WindowSpecification {
            agg_type,
            partition_by: Vec::new(),
            order_by: None,
            frame_clause: None,
        };
    }

    let window_clauses = PgList::<pg_sys::WindowClause>::from_pg((*parse).windowClause);

    // winref is 1-based, but list is 0-indexed
    let window_clause_idx = (winref - 1) as usize;

    if window_clause_idx >= window_clauses.len() {
        return WindowSpecification {
            agg_type,
            partition_by: Vec::new(),
            order_by: None,
            frame_clause: None,
        };
    }

    let window_clause = window_clauses.get_ptr(window_clause_idx).unwrap();

    let partition_by = extract_partition_by(parse, (*window_clause).partitionClause);
    let order_by = extract_order_by(parse, (*window_clause).orderClause);

    // Extract frame clause
    let frame_clause = extract_frame_clause(
        (*window_clause).frameOptions,
        (*window_clause).startOffset,
        (*window_clause).endOffset,
    );

    WindowSpecification {
        agg_type,
        partition_by,
        order_by,
        frame_clause,
    }
}

/// Extract PARTITION BY columns from partitionClause
unsafe fn extract_partition_by(
    parse: *mut pg_sys::Query,
    partition_clause: *mut pg_sys::List,
) -> Vec<String> {
    if partition_clause.is_null() || parse.is_null() || (*parse).targetList.is_null() {
        return Vec::new();
    }

    let partition_list = PgList::<pg_sys::Node>::from_pg(partition_clause);
    if partition_list.is_empty() {
        return Vec::new();
    }

    let mut column_names = Vec::new();
    for (idx, node) in partition_list.iter_ptr().enumerate() {
        // Each node should be a SortGroupClause
        if let Some(sort_clause) = nodecast!(SortGroupClause, T_SortGroupClause, node) {
            let tle_ref = (*sort_clause).tleSortGroupRef;

            // Resolve directly using target_list
            let column_name = resolve_tle_ref(tle_ref, (*parse).targetList)
                .unwrap_or(format!("unresolved_tle_{}", tle_ref));
            column_names.push(column_name);
        } else if let Some(var) = nodecast!(Var, T_Var, node) {
            let field_name = resolve_var_with_parse(parse, var)
                .unwrap_or(format!("unresolved_var_{}", (*var).varattno).into());
            column_names.push(field_name.into_inner());
        }
    }
    column_names
}

/// Extract ORDER BY specification from orderClause
unsafe fn extract_order_by(
    parse: *mut pg_sys::Query,
    order_clause: *mut pg_sys::List,
) -> Option<Vec<OrderByInfo>> {
    if order_clause.is_null() || parse.is_null() || (*parse).targetList.is_null() {
        return None;
    }

    let order_list = PgList::<pg_sys::Node>::from_pg(order_clause);
    if order_list.is_empty() {
        return None;
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

    if order_by_infos.is_empty() {
        None
    } else {
        Some(order_by_infos)
    }
}

/// Extract frame clause from frameOptions and offset expressions
unsafe fn extract_frame_clause(
    frame_options: i32,
    start_offset: *mut pg_sys::Node,
    end_offset: *mut pg_sys::Node,
) -> Option<FrameClause> {
    // frameOptions is a bitmask containing frame type and bounds
    // Defined in windowDefs.h

    const FRAMEOPTION_NONDEFAULT: i32 = 0x00001;
    #[allow(dead_code)]
    const FRAMEOPTION_RANGE: i32 = 0x00002;
    const FRAMEOPTION_ROWS: i32 = 0x00004;
    const FRAMEOPTION_GROUPS: i32 = 0x00008;
    const FRAMEOPTION_START_UNBOUNDED_PRECEDING: i32 = 0x00010;
    const FRAMEOPTION_END_UNBOUNDED_FOLLOWING: i32 = 0x00020;
    const FRAMEOPTION_START_CURRENT_ROW: i32 = 0x00040;
    const FRAMEOPTION_END_CURRENT_ROW: i32 = 0x00080;
    const FRAMEOPTION_START_OFFSET_PRECEDING: i32 = 0x00100;
    const FRAMEOPTION_END_OFFSET_PRECEDING: i32 = 0x00200;
    const FRAMEOPTION_START_OFFSET_FOLLOWING: i32 = 0x00400;
    const FRAMEOPTION_END_OFFSET_FOLLOWING: i32 = 0x00800;

    // Check if there's a non-default frame clause
    if frame_options & FRAMEOPTION_NONDEFAULT == 0 {
        return None;
    }

    // Determine frame type
    let frame_type = if frame_options & FRAMEOPTION_ROWS != 0 {
        FrameType::Rows
    } else if frame_options & FRAMEOPTION_GROUPS != 0 {
        FrameType::Groups
    } else {
        FrameType::Range
    };

    // Extract start bound
    let start_bound = if frame_options & FRAMEOPTION_START_UNBOUNDED_PRECEDING != 0 {
        FrameBound::UnboundedPreceding
    } else if frame_options & FRAMEOPTION_START_CURRENT_ROW != 0 {
        FrameBound::CurrentRow
    } else if frame_options & FRAMEOPTION_START_OFFSET_PRECEDING != 0 {
        // TODO: Extract offset value from start_offset node
        FrameBound::Preceding(1)
    } else if frame_options & FRAMEOPTION_START_OFFSET_FOLLOWING != 0 {
        // TODO: Extract offset value from start_offset node
        FrameBound::Following(1)
    } else {
        FrameBound::UnboundedPreceding
    };

    // Extract end bound
    let end_bound = if frame_options & FRAMEOPTION_END_UNBOUNDED_FOLLOWING != 0 {
        Some(FrameBound::UnboundedFollowing)
    } else if frame_options & FRAMEOPTION_END_CURRENT_ROW != 0 {
        Some(FrameBound::CurrentRow)
    } else if frame_options & FRAMEOPTION_END_OFFSET_PRECEDING != 0 {
        // TODO: Extract offset value from end_offset node
        Some(FrameBound::Preceding(1))
    } else if frame_options & FRAMEOPTION_END_OFFSET_FOLLOWING != 0 {
        // TODO: Extract offset value from end_offset node
        Some(FrameBound::Following(1))
    } else {
        None
    };

    Some(FrameClause {
        frame_type,
        start_bound,
        end_bound,
    })
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
        // Check if this aggregate has a FILTER
        if !window_agg.window_spec.agg_type.has_filter() {
            continue;
        }

        // Try to get the filter
        let filter_opt = window_agg.window_spec.agg_type.get_filter_mut();
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

/// Similar to uses_scores/uses_snippets, this walks the expression tree to find our placeholders
pub unsafe fn extract_window_func_calls(node: *mut pg_sys::Node) -> Vec<WindowAggregateInfo> {
    use pgrx::pg_guard;
    use pgrx::pg_sys::expression_tree_walker;
    use std::ffi::CStr;
    use std::ptr::addr_of_mut;

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

                            match serde_json::from_str::<WindowAggregateInfo>(json_str) {
                                Ok(info) => {
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
    }

    let mut context = Context {
        window_func_procid: window_func_oid(),
        window_aggs: Vec::new(),
    };

    walker(node, addr_of_mut!(context).cast());
    context.window_aggs
}
