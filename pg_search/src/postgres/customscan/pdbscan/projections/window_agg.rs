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

use crate::api::OrderByInfo;
use crate::nodecast;
use crate::postgres::customscan::aggregatescan::AggregateType;
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
    /// Result type OID for the aggregate
    pub result_type_oid: pg_sys::Oid,
    /// Window specification (aggregate type (with optional FILTER), PARTITION BY, ORDER BY, frame clause)
    pub window_spec: WindowSpecification,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FrameType {
    Rows,
    Range,
    Groups,
}

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
    ///
    /// We extract ALL window functions with complete specifications:
    /// - ✅ PARTITION BY (extracted but not yet executable)
    /// - ✅ ORDER BY (extracted but not yet executable)
    /// - ✅ FILTER clause (extracted but not yet executable)
    /// - ✅ Custom frame clauses (ROWS/RANGE/GROUPS) (extracted but not yet executable)
    ///
    /// Execution capability is determined by feature flags defined in this module.
    pub fn is_supported(&self) -> bool {
        // First check if the aggregate function itself is supported
        if !self.is_aggregate_supported() {
            return false;
        }

        let has_filter = self.agg_type.has_filter();
        let has_partition_by = !self.partition_by.is_empty();
        let has_order_by = self.order_by.is_some();
        let has_frame = self.frame_clause.is_some();

        // Check each feature against its flag
        if has_filter && !window_functions::WINDOW_AGG_FILTER_CLAUSE {
            return false;
        }
        if has_partition_by && !window_functions::WINDOW_AGG_PARTITION_BY {
            return false;
        }
        if has_order_by && !window_functions::WINDOW_AGG_ORDER_BY {
            return false;
        }
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

/// Extract window aggregates from the target list with full context
///
/// Uses an all-or-nothing approach: either ALL window functions in the query are supported
/// and get replaced with window_func placeholders, or NONE of them are replaced and
/// PostgreSQL handles all window functions with standard execution.
///
/// This ensures consistent execution - we don't mix our custom window function execution
/// with PostgreSQL's standard window function execution in the same query.
///
/// Parameters:
/// - parse: The Query object containing all query information
/// - heap_rti: Range table index for the base relation
/// - bm25_index: The BM25 index for the relation (needed for FILTER extraction)
/// - root: PlannerInfo (can be null - will store FILTER as PostgresExpression for later extraction)
pub unsafe fn extract_window_aggregates_with_context(
    parse: *mut pg_sys::Query,
    heap_rti: pg_sys::Index,
    bm25_index: &crate::postgres::PgSearchRelation,
    root: *mut pg_sys::PlannerInfo,
) -> Vec<WindowAggregateInfo> {
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
            if let Some((agg_type, result_oid)) =
                extract_standard_aggregate(window_func, bm25_index, root, heap_rti)
            {
                // Extract complete window specification (aggregate type, PARTITION BY, ORDER BY, frame, etc.)
                let window_spec =
                    extract_window_specification(window_func, (*parse).windowClause, agg_type);

                let window_agg_info = WindowAggregateInfo {
                    target_entry_index: idx,
                    result_type_oid: result_oid,
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

/// Check if an AggregateType has a FILTER clause
fn agg_type_has_filter(agg_type: &AggregateType) -> bool {
    match agg_type {
        AggregateType::CountAny { filter } => filter.is_some(),
        AggregateType::Count { filter, .. } => filter.is_some(),
        AggregateType::Sum { filter, .. } => filter.is_some(),
        AggregateType::Avg { filter, .. } => filter.is_some(),
        AggregateType::Min { filter, .. } => filter.is_some(),
        AggregateType::Max { filter, .. } => filter.is_some(),
    }
}

/// Extract FILTER expression by serializing it for later conversion
///
/// ## Why we can't convert now:
/// We can't use extract_quals here because root (PlannerInfo) doesn't exist yet
/// in the planner_hook (it's created by standard_planner which runs after).
///
/// ## How we preserve the FILTER:
/// We wrap the filter expression in a PostgresExpression, which:
/// 1. **At planning time (now)**: Calls nodeToString() during JSON serialization,
///    converting the node tree to a string representation
/// 2. **At execution time (later)**: Calls stringToNode() during JSON deserialization,
///    recreating the node tree in the execution memory context
///
/// This is safe because:
/// - nodeToString creates a new string copy (not a pointer to planning memory)
/// - stringToNode allocates new nodes in current memory context
/// - The deserialized nodes live as long as needed for execution
///
/// ## Future work:
/// At execution time, we need to:
/// 1. Deserialize the PostgresExpression (stringToNode)
/// 2. Call extract_quals with full context (root, bm25_index, heap_rti)
/// 3. Convert to SearchQueryInput
/// 4. Apply as a filter during aggregation
unsafe fn extract_filter_expression_with_context(
    filter_expr: *mut pg_sys::Expr,
    _bm25_index: &crate::postgres::PgSearchRelation,
    _root: *mut pg_sys::PlannerInfo,
    _heap_rti: pg_sys::Index,
) -> Option<crate::query::SearchQueryInput> {
    if filter_expr.is_null() {
        return None;
    }

    // Serialize the filter expression - nodeToString will be called during JSON serialization
    let filter_node = filter_expr as *mut pg_sys::Node;

    Some(crate::query::SearchQueryInput::PostgresExpression {
        expr: crate::query::PostgresExpression::new(filter_node),
    })
}

/// Extract standard aggregate function (COUNT, SUM, AVG, MIN, MAX)
///
/// Returns: (AggregateType, result_type_oid)
unsafe fn extract_standard_aggregate(
    window_func: *mut pg_sys::WindowFunc,
    bm25_index: &crate::postgres::PgSearchRelation,
    root: *mut pg_sys::PlannerInfo,
    heap_rti: pg_sys::Index,
) -> Option<(AggregateType, pg_sys::Oid)> {
    // We extract standard aggregates and let the caller decide if they're supported
    // The execution capability check happens in the caller via is_supported()
    let funcoid = (*window_func).winfnoid;
    let args = PgList::<pg_sys::Node>::from_pg((*window_func).args);

    // Extract FILTER clause if present
    let filter = if !(*window_func).aggfilter.is_null() {
        // Extract the filter expression using the same method as aggregatescan
        extract_filter_expression_with_context(
            (*window_func).aggfilter,
            bm25_index,
            root, // Can be null - will store as PostgresExpression
            heap_rti,
        )
    } else {
        None
    };

    // Get function name to identify the aggregate
    let funcname = get_function_name(funcoid)?;

    match funcname.as_str() {
        "count" => {
            if args.is_empty() || is_count_star_arg(args.get_ptr(0)?) {
                // COUNT(*) - count all documents
                Some((AggregateType::CountAny { filter }, pg_sys::INT8OID))
            } else {
                // COUNT(field) - count non-null values
                let field = extract_field_name(args.get_ptr(0)?)?;
                Some((
                    AggregateType::Count {
                        field,
                        missing: None,
                        filter,
                    },
                    pg_sys::INT8OID,
                ))
            }
        }
        "sum" => {
            let field = extract_field_name(args.get_ptr(0)?)?;
            Some((
                AggregateType::Sum {
                    field,
                    missing: None,
                    filter,
                },
                pg_sys::FLOAT8OID,
            ))
        }
        "avg" => {
            let field = extract_field_name(args.get_ptr(0)?)?;
            Some((
                AggregateType::Avg {
                    field,
                    missing: None,
                    filter,
                },
                pg_sys::FLOAT8OID,
            ))
        }
        "min" => {
            let field = extract_field_name(args.get_ptr(0)?)?;
            Some((
                AggregateType::Min {
                    field,
                    missing: None,
                    filter,
                },
                pg_sys::FLOAT8OID,
            ))
        }
        "max" => {
            let field = extract_field_name(args.get_ptr(0)?)?;
            Some((
                AggregateType::Max {
                    field,
                    missing: None,
                    filter,
                },
                pg_sys::FLOAT8OID,
            ))
        }
        _ => None,
    }
}

/// Extract complete window specification from a WindowFunc node
///
/// This function extracts:
/// - Aggregate type (with FILTER clause)
/// - PARTITION BY columns
/// - ORDER BY specification
/// - Frame clause (ROWS/RANGE/GROUPS BETWEEN...)
unsafe fn extract_window_specification(
    window_func: *mut pg_sys::WindowFunc,
    window_clause_list: *mut pg_sys::List,
    agg_type: AggregateType,
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
    if window_clause_list.is_null() {
        return WindowSpecification {
            agg_type,
            partition_by: Vec::new(),
            order_by: None,
            frame_clause: None,
        };
    }

    let window_clauses = PgList::<pg_sys::WindowClause>::from_pg(window_clause_list);

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

    // Extract PARTITION BY columns
    let partition_by = extract_partition_by((*window_clause).partitionClause);

    // Extract ORDER BY specification
    let order_by = extract_order_by((*window_clause).orderClause);

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
unsafe fn extract_partition_by(partition_clause: *mut pg_sys::List) -> Vec<String> {
    if partition_clause.is_null() {
        return Vec::new();
    }

    let mut columns = Vec::new();
    let partition_list = PgList::<pg_sys::Node>::from_pg(partition_clause);

    for node in partition_list.iter_ptr() {
        // Each node is a SortGroupClause or similar
        // For now, we'll extract a placeholder name
        // TODO: Properly extract column names from Var nodes
        columns.push(format!("partition_col_{}", columns.len()));
    }

    columns
}

/// Extract ORDER BY specification from orderClause
unsafe fn extract_order_by(order_clause: *mut pg_sys::List) -> Option<Vec<OrderByInfo>> {
    if order_clause.is_null() {
        return None;
    }

    let order_list = PgList::<pg_sys::Node>::from_pg(order_clause);
    if order_list.is_empty() {
        return None;
    }

    let mut order_by_infos = Vec::new();

    for node in order_list.iter_ptr() {
        // Each node is a SortGroupClause
        // For now, create a placeholder OrderByInfo
        // TODO: Properly extract from SortGroupClause and map to OrderByFeature
        use crate::api::{FieldName, OrderByFeature, SortDirection};

        let field_name: FieldName = format!("order_col_{}", order_by_infos.len()).into();

        order_by_infos.push(OrderByInfo {
            feature: OrderByFeature::Field(field_name),
            direction: SortDirection::Asc,
        });
    }

    Some(order_by_infos)
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

/// Check if this is a COUNT(*) argument (NULL constant)
unsafe fn is_count_star_arg(arg: *mut pg_sys::Node) -> bool {
    if let Some(const_node) = nodecast!(Const, T_Const, arg) {
        (*const_node).constisnull
    } else {
        false
    }
}

/// Extract field name from a Var or expression
unsafe fn extract_field_name(node: *mut pg_sys::Node) -> Option<String> {
    // Try to extract from Var
    if let Some(var) = nodecast!(Var, T_Var, node) {
        // Get the attribute name from the relation
        // For now, return a placeholder - we'll need to look this up properly
        return Some(format!("field_{}", (*var).varattno));
    }

    // TODO: Handle more complex expressions (JSON operators, etc.)
    None
}

/// Get function name from OID
unsafe fn get_function_name(funcoid: pg_sys::Oid) -> Option<String> {
    let func_tuple =
        pg_sys::SearchSysCache1(pg_sys::SysCacheIdentifier::PROCOID as _, funcoid.into());

    if func_tuple.is_null() {
        return None;
    }

    let func_form = pg_sys::GETSTRUCT(func_tuple) as *mut pg_sys::FormData_pg_proc;
    let name_data = &(*func_form).proname;
    let name = std::ffi::CStr::from_ptr(name_data.data.as_ptr())
        .to_str()
        .ok()?
        .to_string();

    pg_sys::ReleaseSysCache(func_tuple);

    Some(name)
}

/// Format aggregate type for display
#[allow(dead_code)]
fn format_aggregate_type(agg_type: &AggregateType) -> String {
    match agg_type {
        AggregateType::CountAny { .. } => "COUNT(*)".to_string(),
        AggregateType::Count { field, .. } => format!("COUNT({})", field),
        AggregateType::Sum { field, .. } => format!("SUM({})", field),
        AggregateType::Avg { field, .. } => format!("AVG({})", field),
        AggregateType::Min { field, .. } => format!("MIN({})", field),
        AggregateType::Max { field, .. } => format!("MAX({})", field),
    }
}

/// Get filter info from aggregate type
#[allow(dead_code)]
fn get_filter_info(agg_type: &AggregateType) -> Option<String> {
    let filter = match agg_type {
        AggregateType::CountAny { filter } => filter,
        AggregateType::Count { filter, .. } => filter,
        AggregateType::Sum { filter, .. } => filter,
        AggregateType::Avg { filter, .. } => filter,
        AggregateType::Min { filter, .. } => filter,
        AggregateType::Max { filter, .. } => filter,
    };

    filter
        .as_ref()
        .map(|_| "WHERE (filter expression)".to_string())
}

/// Extract window_func(json) calls from the processed target list at planning time
/// Convert PostgresExpression filters to SearchQueryInput
///
/// This is called at plan_custom_path time when we have access to root (PlannerInfo),
/// allowing us to use extract_quals to properly convert FILTER expressions.
pub unsafe fn convert_window_aggregate_filters(
    window_aggregates: &mut [WindowAggregateInfo],
    bm25_index: &crate::postgres::PgSearchRelation,
    root: *mut pg_sys::PlannerInfo,
    heap_rti: pg_sys::Index,
) {
    use crate::api::operator::anyelement_query_input_opoid;
    use crate::postgres::customscan::builders::custom_path::RestrictInfoType;
    use crate::postgres::customscan::qual_inspect::{extract_quals, QualExtractState};

    for window_agg in window_aggregates.iter_mut() {
        // Check if this aggregate has a FILTER
        if !agg_type_has_filter(&window_agg.window_spec.agg_type) {
            continue;
        }

        // Try to get the filter
        let filter_opt = get_aggregate_filter_mut(&mut window_agg.window_spec.agg_type);
        if let Some(filter) = filter_opt {
            // Check if it's a PostgresExpression that needs conversion
            if let crate::query::SearchQueryInput::PostgresExpression { expr } = filter {
                let filter_node = expr.node();
                if !filter_node.is_null() {
                    // Use extract_quals to convert the expression
                    let mut filter_qual_state = QualExtractState::default();
                    let result = extract_quals(
                        root,
                        heap_rti,
                        filter_node,
                        anyelement_query_input_opoid(),
                        RestrictInfoType::BaseRelation,
                        bm25_index,
                        false,
                        &mut filter_qual_state,
                        true, // attempt_pushdown
                    );

                    // Replace the PostgresExpression with the converted SearchQueryInput
                    if let Some(qual) = result {
                        let converted = crate::query::SearchQueryInput::from(&qual);
                        *filter = converted;
                    }
                }
            }
        }
    }
}

/// Helper function to get mutable filter from an aggregate type
fn get_aggregate_filter_mut(
    agg_type: &mut AggregateType,
) -> Option<&mut crate::query::SearchQueryInput> {
    match agg_type {
        AggregateType::CountAny { filter } => filter.as_mut(),
        AggregateType::Count { filter, .. } => filter.as_mut(),
        AggregateType::Sum { filter, .. } => filter.as_mut(),
        AggregateType::Avg { filter, .. } => filter.as_mut(),
        AggregateType::Min { filter, .. } => filter.as_mut(),
        AggregateType::Max { filter, .. } => filter.as_mut(),
    }
}

/// Similar to uses_scores/uses_snippets, this walks the expression tree to find our placeholders
pub unsafe fn extract_window_func_calls(
    node: *mut pg_sys::Node,
    window_func_procid: pg_sys::Oid,
) -> Vec<WindowAggregateInfo> {
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
        window_func_procid,
        window_aggs: Vec::new(),
    };

    walker(node, addr_of_mut!(context).cast());
    context.window_aggs
}
