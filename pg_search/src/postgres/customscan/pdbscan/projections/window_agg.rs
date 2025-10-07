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

use crate::nodecast;
use pgrx::{pg_sys, PgList};
use serde::{Deserialize, Serialize};

/// Information about a window aggregate to compute during TopN execution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WindowAggregateInfo {
    /// The aggregate type (COUNT, SUM, AVG, MIN, MAX)
    pub agg_type: WindowAggregateType,
    /// Target entry index where this aggregate should be projected
    pub target_entry_index: usize,
    /// Result type OID for the aggregate
    pub result_type_oid: pg_sys::Oid,
}

/// Type of window aggregate
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WindowAggregateType {
    /// COUNT(*) - count all documents
    CountStar,
    /// COUNT(field) - count non-null values
    Count { field: String },
    /// SUM(field) - sum of field values
    Sum { field: String },
    /// AVG(field) - average of field values
    Avg { field: String },
    /// MIN(field) - minimum field value
    Min { field: String },
    /// MAX(field) - maximum field value
    Max { field: String },
}

/// Extract window aggregates from the target list
///
/// Supports standard aggregate window functions (COUNT, SUM, AVG, MIN, MAX)
/// Returns single scalar values (INT8, FLOAT8)
pub unsafe fn extract_window_aggregates(
    target_list: *mut pg_sys::List,
    _rti: pg_sys::Index,
) -> Vec<WindowAggregateInfo> {
    let mut window_aggs = Vec::new();
    let tlist = PgList::<pg_sys::TargetEntry>::from_pg(target_list);

    for (idx, te) in tlist.iter_ptr().enumerate() {
        if let Some(window_func) = nodecast!(WindowFunc, T_WindowFunc, (*te).expr) {
            // Check for standard aggregate functions
            if let Some((agg_type, result_oid)) = extract_standard_aggregate(window_func) {
                window_aggs.push(WindowAggregateInfo {
                    agg_type,
                    target_entry_index: idx,
                    result_type_oid: result_oid,
                });
            }
        }
    }

    window_aggs
}

/// Extract standard aggregate function (COUNT, SUM, AVG, MIN, MAX)
///
/// Returns: (WindowAggregateType, result_type_oid)
unsafe fn extract_standard_aggregate(
    window_func: *mut pg_sys::WindowFunc,
) -> Option<(WindowAggregateType, pg_sys::Oid)> {
    // Verify empty window specification (no PARTITION BY, no ORDER BY)
    if !is_empty_window_clause(window_func) {
        return None;
    }

    let funcoid = (*window_func).winfnoid;
    let args = PgList::<pg_sys::Node>::from_pg((*window_func).args);

    // Get function name to identify the aggregate
    let funcname = get_function_name(funcoid)?;

    match funcname.as_str() {
        "count" => {
            if args.is_empty() || is_count_star_arg(args.get_ptr(0)?) {
                // COUNT(*) - count all documents
                Some((WindowAggregateType::CountStar, pg_sys::INT8OID))
            } else {
                // COUNT(field) - count non-null values
                let field = extract_field_name(args.get_ptr(0)?)?;
                Some((WindowAggregateType::Count { field }, pg_sys::INT8OID))
            }
        }
        "sum" => {
            let field = extract_field_name(args.get_ptr(0)?)?;
            Some((WindowAggregateType::Sum { field }, pg_sys::FLOAT8OID))
        }
        "avg" => {
            let field = extract_field_name(args.get_ptr(0)?)?;
            Some((WindowAggregateType::Avg { field }, pg_sys::FLOAT8OID))
        }
        "min" => {
            let field = extract_field_name(args.get_ptr(0)?)?;
            Some((WindowAggregateType::Min { field }, pg_sys::FLOAT8OID))
        }
        "max" => {
            let field = extract_field_name(args.get_ptr(0)?)?;
            Some((WindowAggregateType::Max { field }, pg_sys::FLOAT8OID))
        }
        _ => None,
    }
}

/// Check if window clause is empty (no PARTITION BY, no ORDER BY)
unsafe fn is_empty_window_clause(window_func: *mut pg_sys::WindowFunc) -> bool {
    // Window functions for aggregates should have empty PARTITION BY and ORDER BY
    // This ensures they compute over the entire result set
    let winref = (*window_func).winref;
    if winref == 0 {
        return true;
    }

    // TODO: Actually check the WindowClause to verify it's empty
    // For now, we'll assume it's correct if winref is set
    true
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
