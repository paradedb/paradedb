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

/// Information about a window aggregate to compute during TopN execution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WindowAggregateInfo {
    /// The aggregate type (reuses existing AggregateType from aggregatescan)
    /// This includes COUNT, SUM, AVG, MIN, MAX with optional filter support
    pub agg_type: AggregateType,
    /// Target entry index where this aggregate should be projected
    pub target_entry_index: usize,
    /// Result type OID for the aggregate
    pub result_type_oid: pg_sys::Oid,
    /// Window specification (PARTITION BY, ORDER BY, frame clause)
    pub window_spec: WindowSpecification,
}

/// Window specification from the OVER clause
/// Note: FILTER clause is stored in AggregateType.filter, not here
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WindowSpecification {
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

/// Extract window aggregates from the target list
///
/// Supports standard aggregate window functions (COUNT, SUM, AVG, MIN, MAX)
/// Only extracts window functions that we can handle (simple aggregates over entire result set)
/// Returns single scalar values (INT8, FLOAT8)
pub unsafe fn extract_window_aggregates(
    target_list: *mut pg_sys::List,
    _rti: pg_sys::Index,
) -> Vec<WindowAggregateInfo> {
    let mut window_aggs = Vec::new();
    let tlist = PgList::<pg_sys::TargetEntry>::from_pg(target_list);

    for (idx, te) in tlist.iter_ptr().enumerate() {
        if let Some(window_func) = nodecast!(WindowFunc, T_WindowFunc, (*te).expr) {
            // Extract window specification
            let window_spec = extract_window_specification(window_func);

            // Check if we can handle this window function
            if !can_handle_window_spec(&window_spec) {
                pgrx::warning!(
                    "Cannot handle window function at index {}: {:?}",
                    idx,
                    window_spec
                );
                continue;
            }

            // Check for standard aggregate functions
            if let Some((agg_type, result_oid)) =
                extract_standard_aggregate(window_func, &window_spec)
            {
                pgrx::warning!(
                    "Extracted window aggregate at index {}: {:?}",
                    idx,
                    agg_type
                );
                window_aggs.push(WindowAggregateInfo {
                    agg_type,
                    target_entry_index: idx,
                    result_type_oid: result_oid,
                    window_spec,
                });
            }
        }
    }

    window_aggs
}

/// Check if we can handle this window specification
/// For now, we only support simple aggregates over the entire result set:
/// - No PARTITION BY
/// - No ORDER BY  
/// - No frame clause
///
///   Note: FILTER clause is handled separately in AggregateType
unsafe fn can_handle_window_spec(spec: &WindowSpecification) -> bool {
    spec.partition_by.is_empty() && spec.order_by.is_none() && spec.frame_clause.is_none()
}

/// Extract standard aggregate function (COUNT, SUM, AVG, MIN, MAX)
///
/// Returns: (AggregateType, result_type_oid)
unsafe fn extract_standard_aggregate(
    window_func: *mut pg_sys::WindowFunc,
    _window_spec: &WindowSpecification,
) -> Option<(AggregateType, pg_sys::Oid)> {
    // Verify empty window specification (no PARTITION BY, no ORDER BY)
    if !is_empty_window_clause(window_func) {
        return None;
    }

    let funcoid = (*window_func).winfnoid;
    let args = PgList::<pg_sys::Node>::from_pg((*window_func).args);

    // Extract FILTER clause if present
    let filter = if !(*window_func).aggfilter.is_null() {
        // TODO: Convert filter expression to SearchQueryInput
        // For now, we don't support FILTER clauses
        return None;
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

/// Extract window specification from a WindowFunc node
/// Note: FILTER clause is extracted separately and stored in AggregateType
unsafe fn extract_window_specification(
    window_func: *mut pg_sys::WindowFunc,
) -> WindowSpecification {
    // Get the WindowClause from winref (if it exists)
    // winref is an index into the query's windowClause list
    let winref = (*window_func).winref;

    if winref == 0 {
        // No window clause - means empty OVER ()
        return WindowSpecification {
            partition_by: Vec::new(),
            order_by: None,
            frame_clause: None,
        };
    }

    // TODO: To fully extract PARTITION BY, ORDER BY, and frame clauses,
    // we would need access to the Query's windowClause list via the PlannerInfo
    // For now, we'll return a minimal specification
    // This will need to be enhanced when we pass the root pointer through

    WindowSpecification {
        partition_by: Vec::new(),
        order_by: None,
        frame_clause: None,
    }
}

/// Check if window clause is empty (no PARTITION BY, no ORDER BY)
unsafe fn is_empty_window_clause(window_func: *mut pg_sys::WindowFunc) -> bool {
    let spec = extract_window_specification(window_func);
    can_handle_window_spec(&spec)
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
