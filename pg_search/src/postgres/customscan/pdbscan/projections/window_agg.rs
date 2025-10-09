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
/// Extracts ALL window functions with their complete specifications:
/// - Standard aggregates: COUNT, SUM, AVG, MIN, MAX
/// - Window specification: PARTITION BY, ORDER BY, frame clauses
/// - FILTER clauses
/// - Result types
///
/// we extract everything even if we can't execute it yet.
/// This ensures all information is available for future execution implementations.
///
/// Parameters:
/// - target_list: The Query's targetList
/// - window_clause_list: The Query's windowClause list (for PARTITION BY/ORDER BY/frame details)
/// - _rti: Range table index (currently unused, but kept for future use)
pub unsafe fn extract_window_aggregates(
    target_list: *mut pg_sys::List,
    window_clause_list: *mut pg_sys::List,
    _rti: pg_sys::Index,
) -> Vec<WindowAggregateInfo> {
    let mut window_aggs = Vec::new();
    let tlist = PgList::<pg_sys::TargetEntry>::from_pg(target_list);

    for (idx, te) in tlist.iter_ptr().enumerate() {
        if let Some(window_func) = nodecast!(WindowFunc, T_WindowFunc, (*te).expr) {
            // Extract complete window specification (PARTITION BY, ORDER BY, frame, etc.)
            let window_spec = extract_window_specification(window_func, window_clause_list);

            // Extract the aggregate function and its details
            if let Some((agg_type, result_oid)) =
                extract_standard_aggregate(window_func, &window_spec)
            {
                // Check if we can currently execute this window function
                let can_execute = can_handle_window_spec(&window_spec);

                if !can_execute {
                    pgrx::warning!(
                        "Window function at index {} extracted but cannot be executed yet: {:?}",
                        idx,
                        window_spec
                    );
                    // Still extract it! Execution capability check happens later
                }

                let window_agg_info = WindowAggregateInfo {
                    agg_type,
                    target_entry_index: idx,
                    result_type_oid: result_oid,
                    window_spec,
                };

                // Print comprehensive extraction details
                print_window_aggregate_info(&window_agg_info, idx, can_execute);

                window_aggs.push(window_agg_info);
            } else {
                pgrx::warning!(
                    "Window function at index {} is not a supported aggregate type",
                    idx
                );
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
    window_spec: &WindowSpecification,
) -> Option<(AggregateType, pg_sys::Oid)> {
    // Note: We don't check if the window spec is empty here anymore
    // We extract ALL aggregates regardless of whether we can execute them
    // The execution capability check happens separately via can_handle_window_spec()

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

/// Extract complete window specification from a WindowFunc node
/// Note: FILTER clause is extracted separately and stored in AggregateType
///
/// This function extracts:
/// - PARTITION BY columns
/// - ORDER BY specification
/// - Frame clause (ROWS/RANGE/GROUPS BETWEEN...)
unsafe fn extract_window_specification(
    window_func: *mut pg_sys::WindowFunc,
    window_clause_list: *mut pg_sys::List,
) -> WindowSpecification {
    // Get the WindowClause from winref (if it exists)
    // winref is an index (1-based) into the query's windowClause list
    let winref = (*window_func).winref;

    if winref == 0 {
        // No window clause - means empty OVER ()
        return WindowSpecification {
            partition_by: Vec::new(),
            order_by: None,
            frame_clause: None,
        };
    }

    // Access the WindowClause from the list
    if window_clause_list.is_null() {
        pgrx::warning!("Window clause list is null but winref={}", winref);
        return WindowSpecification {
            partition_by: Vec::new(),
            order_by: None,
            frame_clause: None,
        };
    }

    let window_clauses = PgList::<pg_sys::WindowClause>::from_pg(window_clause_list);

    // winref is 1-based, but list is 0-indexed
    let window_clause_idx = (winref - 1) as usize;

    if window_clause_idx >= window_clauses.len() {
        pgrx::warning!(
            "winref {} out of bounds (window_clauses.len={})",
            winref,
            window_clauses.len()
        );
        return WindowSpecification {
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

    pgrx::warning!(
        "Extracted window spec: partition_by={:?}, order_by={:?}, frame_clause={:?}",
        partition_by,
        order_by,
        frame_clause
    );

    WindowSpecification {
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

/// Check if window clause is empty (no PARTITION BY, no ORDER BY)
/// Note: This is used during extraction before we have the window_clause_list
/// For full validation, use can_handle_window_spec() on the extracted WindowSpecification
unsafe fn is_empty_window_clause(window_func: *mut pg_sys::WindowFunc) -> bool {
    // Quick check: if winref is 0, it's definitely empty
    (*window_func).winref == 0
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

/// Print comprehensive window aggregate information for debugging
fn print_window_aggregate_info(info: &WindowAggregateInfo, idx: usize, can_execute: bool) {
    pgrx::warning!("═══════════════════════════════════════════════════════════");
    pgrx::warning!("EXTRACTED WINDOW FUNCTION #{}", idx);
    pgrx::warning!("═══════════════════════════════════════════════════════════");

    // Print aggregate type
    pgrx::warning!("Aggregate Type: {}", format_aggregate_type(&info.agg_type));

    // Print FILTER clause if present
    if let Some(filter_info) = get_filter_info(&info.agg_type) {
        pgrx::warning!("  FILTER: {}", filter_info);
    }

    // Print PARTITION BY
    if !info.window_spec.partition_by.is_empty() {
        pgrx::warning!("PARTITION BY:");
        for (i, col) in info.window_spec.partition_by.iter().enumerate() {
            pgrx::warning!("  [{}] {}", i, col);
        }
    } else {
        pgrx::warning!("PARTITION BY: (none - aggregate over entire result set)");
    }

    // Print ORDER BY
    if let Some(ref order_by) = info.window_spec.order_by {
        pgrx::warning!("ORDER BY:");
        for (i, order_info) in order_by.iter().enumerate() {
            pgrx::warning!(
                "  [{}] {:?} {:?}",
                i,
                order_info.feature,
                order_info.direction
            );
        }
    } else {
        pgrx::warning!("ORDER BY: (none)");
    }

    // Print frame clause
    if let Some(ref frame) = info.window_spec.frame_clause {
        pgrx::warning!(
            "FRAME: {:?} BETWEEN {:?} AND {:?}",
            frame.frame_type,
            frame.start_bound,
            frame.end_bound.as_ref().unwrap_or(&FrameBound::CurrentRow)
        );
    } else {
        pgrx::warning!("FRAME: (default)");
    }

    // Print result type
    pgrx::warning!("Result Type OID: {}", info.result_type_oid);
    pgrx::warning!("Target Entry Index: {}", info.target_entry_index);

    // Print execution capability
    if can_execute {
        pgrx::warning!("✅ CAN EXECUTE: Yes - will compute at execution time");
    } else {
        pgrx::warning!("⚠️  CAN EXECUTE: No - extracted but not yet supported");
        pgrx::warning!("   Will fall back to PostgreSQL's WindowAgg if not replaced");
    }

    pgrx::warning!("═══════════════════════════════════════════════════════════");
}

/// Format aggregate type for display
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
