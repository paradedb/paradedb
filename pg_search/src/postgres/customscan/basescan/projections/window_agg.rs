// Copyright (c) 2023-2026 ParadeDB, Inc.
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

//! Window Function Support for TopN Queries (Faceting)
//!
//! This module implements window function support for TopN queries, enabling
//! Elasticsearch-style faceting patterns where result sets and aggregates are
//! computed in a single pass.
//!
//! # Architecture
//!
//! The implementation uses an early replacement strategy at the `planner_hook` stage:
//!
//! 1. **Detection**: Identify queries with window functions and `@@@` operator or `pdb.agg()`
//! 2. **Extraction**: Parse `WindowFunc` AST nodes and extract aggregate definitions into `TargetList`
//! 3. **Replacement**: Replace `WindowFunc` nodes with `paradedb.window_agg(json)` placeholders
//! 4. **Execution**: Execute aggregations via Tantivy's `MultiCollector` in TopN scan
//! 5. **Injection**: Inject aggregate results as constant values in each output row
//!
//! # Why Early Replacement?
//!
//! We replace window functions at `planner_hook` (before path generation) rather than at
//! `create_upper_paths_hook` with `UPPERREL_WINDOW` because:
//!
//! - **Simpler Integration**: Reuses existing `BaseScan` infrastructure without nested scans
//! - **Avoids Complexity**: No need for `WindowCustomScan` wrapping `BaseScan`
//! - **Single Scan**: Allows TopN + aggregation in one scan pass
//!
//! Note: `AggregateCustomScan` uses `create_upper_paths_hook` with `UPPERREL_GROUP_AGG`,
//! not `planner_hook`. The approaches differ because:
//! - GROUP BY aggregates: Need to replace the entire Aggregate node (upper path)
//! - Window functions: Can replace individual function calls in target list (planner hook)
//!
//! **Tradeoff**: This approach requires duplicating some detection logic between planner_hook
//! and create_custom_path (see comments in hook.rs about duplication with extract_quals).
//! See GitHub issue #3455 for potential unification.
//!
//! # Example Usage
//!
//! ```sql
//! -- Simple count facet
//! SELECT *, COUNT(*) OVER () AS total_count
//! FROM products
//! WHERE description @@@ 'laptop'
//! ORDER BY rating DESC
//! LIMIT 10;
//!
//! -- Custom Tantivy aggregation
//! SELECT *, pdb.agg('{"terms": {"field": "brand"}}') OVER () AS brand_facets
//! FROM products
//! WHERE description @@@ 'smartphone'
//! ORDER BY rating DESC
//! LIMIT 10;
//! ```

use crate::api::operator::anyelement_query_input_opoid;
use crate::api::window_aggregate::window_agg_oid;
use crate::api::FieldName;
use crate::api::{
    agg_funcoid, agg_with_solve_mvcc_funcoid, extract_solve_mvcc_from_const, MvccVisibility,
};
use crate::nodecast;
use crate::postgres::customscan::aggregatescan::aggregate_type::{
    create_aggregate_from_oid, parse_coalesce_expression, AggregateType,
};
use crate::postgres::customscan::aggregatescan::targetlist::TargetList;
use crate::postgres::customscan::builders::custom_path::RestrictInfoType;
use crate::postgres::customscan::qual_inspect::{extract_quals, PlannerContext, QualExtractState};
use crate::postgres::var::{fieldname_from_var, VarContext};
use crate::postgres::PgSearchRelation;
use crate::query::{PostgresExpression, SearchQueryInput};
use pgrx::{pg_sys, PgList};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ptr::addr_of_mut;

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
    ///
    /// Important: If we're rejecting a window function due to unsupported features (e.g., FILTER),
    /// and it uses pdb.agg() (Custom aggregate), we must error out because PostgreSQL
    /// cannot handle pdb.agg() - it's a placeholder that only we can execute.
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
                // If we're rejecting due to FILTER not being supported, check if this is
                // a Custom aggregate (pdb.agg()). If so, we must error out because
                // PostgreSQL cannot handle it.
                if matches!(agg_type, AggregateType::Custom { .. }) {
                    pgrx::error!(
                        "pdb.agg() with FILTER clause is not currently supported. \
                         FILTER with window functions requires the '{}' feature flag to be enabled. \
                         Try removing the FILTER clause or use a standard aggregate function instead. \
                         See https://github.com/paradedb/paradedb/issues for more information.",
                        "WINDOW_AGG_FILTER_CLAUSE"
                    );
                }
                return false;
            }
        }

        // Note: PARTITION BY and ORDER BY in OVER clauses are not supported in our use case,
        // because we compute facets over the entire result set, not partitioned subsets.
        // If grouping_columns is non-empty, we reject the query.
        if !tlist.grouping_columns().is_empty() {
            // Check if any aggregate is Custom (pdb.agg()) - if so, error out
            for agg_type in tlist.aggregates() {
                if matches!(agg_type, AggregateType::Custom { .. }) {
                    pgrx::error!(
                        "pdb.agg() with PARTITION BY or ORDER BY in OVER clause is not currently supported. \
                         These features require the '{}' feature flag to be enabled. \
                         Try removing PARTITION BY/ORDER BY or use a standard aggregate function instead. \
                         See https://github.com/paradedb/paradedb/issues for more information.",
                        "WINDOW_AGG_PARTITION_BY / WINDOW_AGG_ORDER_BY"
                    );
                }
            }
            return false;
        }

        // All required features are supported
        true
    }
}

/// Extract window functions from a query and convert them to our internal TargetList representation
///
/// This is the main entry point for window function processing at planner hook time.
/// It scans the query's target list for WindowFunc nodes, validates they use supported features,
/// converts them to our internal AggregateType/TargetList format, and returns a map of
/// target entry index → TargetList for later replacement with placeholder functions.
///
/// Returns: HashMap mapping target entry indices to their corresponding TargetList
///          Empty HashMap if query has unsupported features or no window functions
pub unsafe fn extract_and_convert_window_functions(
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

    let mut window_aggs = HashMap::new();
    let tlist = PgList::<pg_sys::TargetEntry>::from_pg((*parse).targetList);

    // Extract all window functions and check if they're supported
    // Use expression_tree_walker to find WindowFunc nodes even when wrapped in other functions
    for (idx, te) in tlist.iter_ptr().enumerate() {
        // Helper to find WindowFunc nodes in the expression tree
        struct WindowFuncFinder {
            window_funcs: Vec<*mut pg_sys::WindowFunc>,
        }

        unsafe extern "C-unwind" fn find_window_func(
            node: *mut pg_sys::Node,
            context: *mut core::ffi::c_void,
        ) -> bool {
            if node.is_null() {
                return false;
            }

            let ctx = context.cast::<WindowFuncFinder>();

            // Check if this node is a WindowFunc
            if let Some(window_func) = nodecast!(WindowFunc, T_WindowFunc, node) {
                (*ctx).window_funcs.push(window_func);
            }

            // Continue walking the tree
            pg_sys::expression_tree_walker(node, Some(find_window_func), context)
        }

        let mut finder = WindowFuncFinder {
            window_funcs: Vec::new(),
        };

        // Walk the expression tree to find all WindowFunc nodes
        find_window_func(
            (*te).expr as *mut pg_sys::Node,
            addr_of_mut!(finder) as *mut core::ffi::c_void,
        );

        // Process each WindowFunc found in this target entry
        // Note: We only support one WindowFunc per target entry for now
        if !finder.window_funcs.is_empty() {
            if finder.window_funcs.len() > 1 {
                // Multiple window functions in one target entry - not supported
                return HashMap::new();
            }

            let window_agg = finder.window_funcs[0];

            // Extract the aggregate function and its details first
            if let Some(agg_type) = convert_window_func_to_aggregate_type(parse, window_agg) {
                // Build TargetList and validate OVER clause
                let agg_tl = validate_and_build_target_list(parse, agg_type, window_agg);

                // Only include supported window functions
                if WindowAggregateInfo::is_supported(&agg_tl) {
                    window_aggs.insert(idx, agg_tl.unwrap());
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

/// Convert a PostgreSQL WindowFunc node to our internal AggregateType representation
/// Uses OID-based approach (same as aggregatescan) to identify the aggregate function
/// and extract its field name, missing value, and FILTER clause
///
/// Returns: AggregateType (COUNT, SUM, AVG, MIN, MAX, or Custom for pdb.agg)
unsafe fn convert_window_func_to_aggregate_type(
    parse: *mut pg_sys::Query,
    window_agg: *mut pg_sys::WindowFunc,
) -> Option<AggregateType> {
    use pg_sys::*;

    let aggfnoid = (*window_agg).winfnoid.to_u32();
    let args = PgList::<pg_sys::Node>::from_pg((*window_agg).args);

    // Extract FILTER clause if present
    let filter = if !(*window_agg).aggfilter.is_null() {
        convert_filter_expr_to_search_query((*window_agg).aggfilter)
    } else {
        None
    };

    // Handle custom agg function pdb.agg() (both overloads)
    let custom_agg_oid = agg_funcoid().to_u32();
    let custom_agg_with_mvcc_oid = agg_with_solve_mvcc_funcoid().to_u32();

    if aggfnoid == custom_agg_oid || aggfnoid == custom_agg_with_mvcc_oid {
        if args.is_empty() {
            return None;
        }

        // Extract the jsonb argument (first arg)
        let first_arg = args.get_ptr(0)?;
        let json_value = if let Some(const_node) = nodecast!(Const, T_Const, first_arg) {
            if (*const_node).constisnull {
                return None;
            }
            let jsonb_datum = (*const_node).constvalue;
            let jsonb = <pgrx::JsonB as pgrx::FromDatum>::from_datum(jsonb_datum, false)?;
            jsonb.0
        } else {
            return None;
        };

        // Extract solve_mvcc bool argument (second arg) if using the two-arg overload
        let solve_mvcc = if aggfnoid == custom_agg_with_mvcc_oid {
            args.get_ptr(1)
                .and_then(|mvcc_arg| nodecast!(Const, T_Const, mvcc_arg))
                .map(|const_node| extract_solve_mvcc_from_const(const_node))
                .unwrap_or(true)
        } else {
            true // Single-arg overload: default to solve_mvcc = true
        };

        let mvcc_visibility = if solve_mvcc {
            MvccVisibility::Enabled
        } else {
            MvccVisibility::Disabled
        };

        // Validate that the JSON is a valid Tantivy aggregation
        // It should be a single aggregation definition (e.g., {"terms": {...}}, {"avg": {...}})
        // NOT wrapped in a "buckets" key (that's for the old pdb.aggregate function)
        if json_value.get("buckets").is_some() {
            pgrx::error!(
                "pdb.agg() received JSON with 'buckets' key. \
                 Remove the 'buckets' wrapper - pdb.agg() expects a single aggregation definition. \
                 Example: {{\"terms\": {{\"field\": \"country\"}}}} instead of {{\"buckets\": {{\"terms\": {{\"field\": \"country\"}}}}}}"
            );
        }

        // Validate it's an object
        if !json_value.is_object() {
            pgrx::error!(
                "pdb.agg() expects a JSON object representing a Tantivy aggregation. \
                 Example: {{\"terms\": {{\"field\": \"country\"}}}}"
            );
        }

        return Some(AggregateType::Custom {
            agg_json: json_value,
            filter,
            indexrelid: pg_sys::InvalidOid, // Will be filled in during planning
            mvcc_visibility,
            numeric_field_scales: HashMap::default(), // Will be filled in during planning
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
    let (field, missing) = extract_field_name_from_aggregate_arg(parse, first_arg)?;

    let agg_type = create_aggregate_from_oid(
        aggfnoid,
        field.into_inner(),
        missing,
        filter,
        pg_sys::InvalidOid, // Will be filled in during planning
        None,               // numeric_scale: filled in by resolve_window_aggregate_numeric_scales
    )?;
    Some(agg_type)
}

/// Extract the field name and missing value from an aggregate function's argument node
/// Handles Var nodes (column references) and COALESCE expressions (for missing values)
/// Returns: (field_name, optional_missing_value)
unsafe fn extract_field_name_from_aggregate_arg(
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

/// Convert a FILTER clause expression to SearchQueryInput by serializing it for later conversion
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
unsafe fn convert_filter_expr_to_search_query(
    filter_expr: *mut pg_sys::Expr,
) -> Option<SearchQueryInput> {
    if filter_expr.is_null() {
        return None;
    }
    // Serialize the filter expression - nodeToString will be called during JSON serialization
    let filter_node = filter_expr as *mut pg_sys::Node;
    Some(SearchQueryInput::PostgresExpression {
        expr: PostgresExpression::new(filter_node, "Window Agg Filter".to_string()),
    })
}

/// Build a TargetList for the window function and validate its OVER clause
///
/// Validates that the window function only uses supported features:
/// - Empty OVER () is supported
/// - PARTITION BY is NOT supported (returns None)
/// - Custom frame clauses are NOT supported (returns None)
/// - ORDER BY within OVER() is NOT supported (returns None)
///
/// Returns: Some(TargetList) if supported, None if unsupported features detected
unsafe fn validate_and_build_target_list(
    parse: *mut pg_sys::Query,
    agg_type: AggregateType,
    window_agg: *mut pg_sys::WindowFunc,
) -> Option<TargetList> {
    // Get the WindowClause from winref (if it exists)
    // winref is an index (1-based) into the query's windowClause list
    let winref = (*window_agg).winref;

    if winref == 0 {
        // No window clause - means empty OVER ()
        return Some(TargetList::new(agg_type));
    }

    // Access the WindowClause from the list
    if (*parse).windowClause.is_null() {
        return Some(TargetList::new(agg_type));
    }

    let window_clauses = PgList::<pg_sys::WindowClause>::from_pg((*parse).windowClause);

    // winref is 1-based, but list is 0-indexed
    let window_clause_idx = (winref - 1) as usize;

    if window_clause_idx >= window_clauses.len() {
        return Some(TargetList::new(agg_type));
    }

    let window_clause = window_clauses.get_ptr(window_clause_idx).unwrap();

    let has_partition_by = window_has_partition_by(parse, (*window_clause).partitionClause);
    let has_frame_clause = window_has_custom_frame_clause(
        (*window_clause).frameOptions,
        (*window_clause).startOffset,
        (*window_clause).endOffset,
    );
    let has_order_by = window_has_order_by(parse, (*window_clause).orderClause);

    // Reject if PARTITION BY, frame clause, or ORDER BY is present
    if has_partition_by || has_frame_clause || has_order_by {
        return None;
    }

    Some(TargetList::new(agg_type)) // PARTITION BY is not supported for window functions
}

/// Check if the window function has a custom frame clause
/// (not the default RANGE UNBOUNDED PRECEDING AND CURRENT ROW)
unsafe fn window_has_custom_frame_clause(
    frame_options: i32, // frameOptions is a bitmask containing frame type and bounds
    start_offset: *mut pg_sys::Node,
    end_offset: *mut pg_sys::Node,
) -> bool {
    const FRAMEOPTION_NONDEFAULT: i32 = 0x00001;
    // Check if there's a non-default frame clause
    frame_options & FRAMEOPTION_NONDEFAULT != 0
}

/// Check if the window function has a PARTITION BY clause
unsafe fn window_has_partition_by(
    parse: *mut pg_sys::Query,
    partition_clause: *mut pg_sys::List,
) -> bool {
    if partition_clause.is_null() || parse.is_null() || (*parse).targetList.is_null() {
        return false;
    }

    let partition_list = PgList::<pg_sys::Node>::from_pg(partition_clause);
    !partition_list.is_empty()
}

/// Check if the window function has an ORDER BY clause within the OVER()
unsafe fn window_has_order_by(parse: *mut pg_sys::Query, order_clause: *mut pg_sys::List) -> bool {
    if order_clause.is_null() || parse.is_null() || (*parse).targetList.is_null() {
        return false;
    }

    let order_list = PgList::<pg_sys::Node>::from_pg(order_clause);
    !order_list.is_empty()
}

/// Resolve numeric field scales for window aggregates at custom plan creation time
///
/// For Custom aggregates (pdb.agg), extracts field names from the aggregate JSON and
/// looks up their scales in the schema. This is necessary for descaling Numeric64 fields
/// in aggregate results.
///
/// For standard aggregates (SUM, AVG, etc.), checks if the field is Numeric64 and sets
/// the scale.
///
/// Returns `Err` with field name if any aggregate uses NumericBytes (unbounded NUMERIC),
/// which cannot be aggregated by the search index.
///
/// This is called at plan_custom_path time when we have access to the schema.
pub fn resolve_window_aggregate_numeric_scales(
    window_aggregates: &mut [WindowAggregateInfo],
    bm25_index: &PgSearchRelation,
) -> Result<(), String> {
    use crate::postgres::customscan::aggregatescan::descale::build_numeric_field_scales;
    use crate::postgres::customscan::aggregatescan::extract_agg_name_to_field;
    use crate::schema::SearchFieldType;

    let schema = match bm25_index.schema() {
        Ok(s) => s,
        Err(_) => return Ok(()),
    };

    for window_agg in window_aggregates.iter_mut() {
        for agg_type in window_agg.targetlist.aggregates_mut() {
            match agg_type {
                AggregateType::Custom {
                    agg_json,
                    numeric_field_scales,
                    ..
                } => {
                    // Build scales for Numeric64 fields to descale aggregate results
                    let agg_name_to_field = extract_agg_name_to_field(agg_json);
                    *numeric_field_scales = build_numeric_field_scales(&schema, &agg_name_to_field);
                }
                AggregateType::Sum {
                    field,
                    numeric_scale,
                    ..
                }
                | AggregateType::Avg {
                    field,
                    numeric_scale,
                    ..
                }
                | AggregateType::Min {
                    field,
                    numeric_scale,
                    ..
                }
                | AggregateType::Max {
                    field,
                    numeric_scale,
                    ..
                } => {
                    // Check field type
                    if let Some(search_field) = schema.search_field(field.as_str()) {
                        match search_field.field_type() {
                            SearchFieldType::NumericBytes(_) => {
                                // NumericBytes cannot be aggregated - reject the plan
                                return Err(field.clone());
                            }
                            SearchFieldType::Numeric64(_, scale) => {
                                *numeric_scale = Some(scale);
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}

/// Resolve window aggregate FILTER clauses at custom plan creation time
///
/// Converts PostgresExpression filters (serialized at planner hook time) to SearchQueryInput
/// (executable at runtime). This is called at plan_custom_path time when we have access to
/// root (PlannerInfo), allowing us to use extract_quals to properly convert FILTER expressions
/// (same logic as aggregatescan).
///
/// This two-phase approach is necessary because:
/// 1. At planner_hook time: We serialize filters as PostgresExpression (no PlannerInfo yet)
/// 2. At plan_custom_path time: We convert to SearchQueryInput (PlannerInfo available)
pub unsafe fn resolve_window_aggregate_filters_at_plan_time(
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
            let filter_opt = agg_type.filter_expr_mut();
            if let Some(filter) = filter_opt {
                // Check if it's a PostgresExpression that needs conversion
                if let SearchQueryInput::PostgresExpression { expr } = filter {
                    let filter_node = expr.node();
                    if !filter_node.is_null() {
                        // Use the same logic as aggregatescan to convert the filter
                        let mut filter_qual_state = QualExtractState::default();
                        if let Some(qual) = extract_quals(
                            &PlannerContext::from_planner(root),
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

/// Deserialize window aggregate placeholders from the target list at custom plan creation time
///
/// After window functions are replaced with `paradedb.window_agg(json)` placeholders at
/// planner hook time, this function extracts them from the processed target list and
/// deserializes the JSON back into WindowAggregateInfo structures.
///
/// This function:
/// 1. Iterates through target entries in the provided target list (usually processed_tlist)
/// 2. Finds `paradedb.window_agg(json)` placeholder calls
/// 3. Deserializes the JSON to recover the TargetList
/// 4. Creates `WindowAggregateInfo` with the current position as target_entry_index
///
/// Returns: Vec of WindowAggregateInfo, one for each window aggregate in the query
pub unsafe fn deserialize_window_agg_placeholders(
    tlist: *mut pg_sys::List,
) -> Vec<WindowAggregateInfo> {
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

                            // Deserialize TargetList and create WindowAggregateInfo
                            // with the correct target_entry_index from the current position
                            match serde_json::from_str::<TargetList>(json_str) {
                                Ok(targetlist) => {
                                    let info = WindowAggregateInfo {
                                        target_entry_index: (*context).current_te_index,
                                        targetlist,
                                    };
                                    (*context).window_aggs.push(info);
                                }
                                Err(e) => {
                                    pgrx::error!(
                                        "Failed to deserialize window aggregate specification: {}. \
                                         This is an internal error - the window function replacement may have failed.",
                                        e
                                    );
                                }
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
