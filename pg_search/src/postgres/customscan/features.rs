/// Feature flags for ParadeDB custom scan capabilities
///
/// This module defines what features are currently supported in execution vs. what's planned.
/// As we implement more execution capabilities, these flags can be updated to enable new features.
///
/// The planning phase extracts ALL features regardless of execution support, but execution
/// is gated by these feature flags to ensure we only attempt what we can actually handle.

/// Window function execution capabilities
pub mod window_functions {
    /// Basic window aggregates over entire result set (no PARTITION BY, ORDER BY, FILTER, or frame)
    /// Currently supported: COUNT(*) OVER (), SUM(field) OVER (), etc.
    pub const SIMPLE_AGGREGATES: bool = true;

    /// Window functions with PARTITION BY clause
    /// Status: Extracted during planning, not yet executable
    /// TODO: Implement partitioned window execution
    pub const PARTITION_BY: bool = false;

    /// Window functions with ORDER BY clause  
    /// Status: Extracted during planning, not yet executable
    /// TODO: Implement ordered window execution
    pub const ORDER_BY: bool = false;

    /// Window functions with FILTER clause (WHERE condition per aggregate)
    /// Status: Extracted during planning, not yet executable
    /// TODO: Implement filter execution in window context
    pub const FILTER_CLAUSE: bool = false;

    /// Custom frame clauses (ROWS/RANGE/GROUPS BETWEEN ...)
    /// Status: Extracted during planning, not yet executable
    /// TODO: Implement frame-based window execution
    pub const FRAME_CLAUSES: bool = false;

    /// Window functions with complex expressions (not just simple aggregates)
    /// Status: Not yet planned or executable
    /// TODO: Support ROW_NUMBER(), RANK(), DENSE_RANK(), LAG(), LEAD(), etc.
    pub const COMPLEX_WINDOW_FUNCTIONS: bool = false;

    /// Check if a window specification can be executed with current capabilities
    pub fn can_execute_window_spec(
        has_partition_by: bool,
        has_order_by: bool,
        has_filter: bool,
        has_frame: bool,
    ) -> bool {
        // Currently only simple aggregates over entire result set
        !has_partition_by && !has_order_by && !has_filter && !has_frame && SIMPLE_AGGREGATES
    }
}

/// Aggregate function execution capabilities
pub mod aggregates {
    /// Basic aggregates (COUNT, SUM, AVG, MIN, MAX) without GROUP BY
    pub const SIMPLE_AGGREGATES: bool = true;

    /// Aggregates with GROUP BY clause
    pub const GROUP_BY: bool = true;

    /// Aggregates with FILTER clause (per-aggregate WHERE conditions)
    pub const FILTER_CLAUSE: bool = true;

    /// Aggregates with HAVING clause
    /// Status: Detected during planning, execution rejected
    /// TODO: Implement HAVING clause execution
    pub const HAVING_CLAUSE: bool = false;

    /// Aggregates with DISTINCT (COUNT(DISTINCT field), etc.)
    /// Status: Detected during planning, execution rejected  
    /// TODO: Implement DISTINCT aggregates if Tantivy supports it
    pub const DISTINCT_AGGREGATES: bool = false;

    /// Aggregates with ORDER BY in the aggregate function (array_agg(field ORDER BY other_field))
    /// Status: Not yet planned or executable
    /// TODO: Support ordered aggregates
    pub const ORDERED_AGGREGATES: bool = false;

    /// Multiple grouping sets (GROUP BY GROUPING SETS, ROLLUP, CUBE)
    /// Status: Not yet planned or executable  
    /// TODO: Support advanced grouping
    pub const GROUPING_SETS: bool = false;
}

/// Query execution capabilities
pub mod queries {
    /// Basic SELECT queries with WHERE clause
    pub const BASIC_SELECT: bool = true;

    /// Queries with LIMIT and OFFSET
    pub const LIMIT_OFFSET: bool = true;

    /// Queries with ORDER BY clause
    pub const ORDER_BY: bool = true;

    /// Subqueries in FROM clause
    /// Status: Detected during planning, limited execution support
    /// TODO: Full subquery execution support
    pub const SUBQUERIES: bool = false;

    /// Common Table Expressions (WITH clause)
    /// Status: Not yet planned or executable
    /// TODO: Support CTEs
    pub const COMMON_TABLE_EXPRESSIONS: bool = false;

    /// UNION/INTERSECT/EXCEPT operations
    /// Status: Not yet planned or executable
    /// TODO: Support set operations
    pub const SET_OPERATIONS: bool = false;
}

/// Index and search capabilities
pub mod search {
    /// Basic full-text search with @@@ operator
    pub const FULL_TEXT_SEARCH: bool = true;

    /// Hybrid search (combining full-text and similarity)
    /// Status: Partially implemented
    pub const HYBRID_SEARCH: bool = true;

    /// Vector similarity search
    /// Status: Implemented in separate modules
    pub const VECTOR_SEARCH: bool = true;

    /// Complex search expressions with AND/OR/NOT
    pub const COMPLEX_SEARCH_EXPRESSIONS: bool = true;

    /// Search with custom scoring/ranking
    /// Status: Basic support, could be enhanced
    pub const CUSTOM_SCORING: bool = true;

    /// Faceted search and aggregations
    /// Status: Implemented via GROUP BY aggregates
    pub const FACETED_SEARCH: bool = true;
}

/// Data type support
pub mod data_types {
    /// Text fields (full-text searchable)
    pub const TEXT_FIELDS: bool = true;

    /// Numeric fields (integers, floats)
    pub const NUMERIC_FIELDS: bool = true;

    /// Boolean fields
    pub const BOOLEAN_FIELDS: bool = true;

    /// Date/timestamp fields
    /// Status: Basic support, could be enhanced
    pub const DATE_FIELDS: bool = true;

    /// JSON fields with path queries
    /// Status: Implemented but could be enhanced
    pub const JSON_FIELDS: bool = true;

    /// Array fields
    /// Status: Limited support
    pub const ARRAY_FIELDS: bool = false;

    /// Geographic/spatial fields
    /// Status: Not yet implemented
    pub const SPATIAL_FIELDS: bool = false;
}

/// Performance and optimization features
pub mod performance {
    /// Parallel query execution
    pub const PARALLEL_EXECUTION: bool = true;

    /// Query result caching
    /// Status: Not implemented
    pub const RESULT_CACHING: bool = false;

    /// Index-only scans (no heap access needed)
    /// Status: Partially implemented
    pub const INDEX_ONLY_SCANS: bool = true;

    /// Bitmap heap scans
    /// Status: Not implemented
    pub const BITMAP_SCANS: bool = false;

    /// Adaptive query optimization
    /// Status: Not implemented
    pub const ADAPTIVE_OPTIMIZATION: bool = false;
}

/// Integration features
pub mod integration {
    /// PostgreSQL EXPLAIN integration
    pub const EXPLAIN_INTEGRATION: bool = true;

    /// PostgreSQL statistics integration
    /// Status: Basic support
    pub const STATISTICS_INTEGRATION: bool = true;

    /// Foreign data wrapper support
    /// Status: Not implemented
    pub const FOREIGN_DATA_WRAPPER: bool = false;

    /// Logical replication support
    /// Status: Not implemented  
    pub const LOGICAL_REPLICATION: bool = false;

    /// Backup/restore integration
    /// Status: Basic support via PostgreSQL mechanisms
    pub const BACKUP_RESTORE: bool = true;
}

/// Development and debugging features
pub mod development {
    /// Query plan visualization
    /// Status: Basic support via EXPLAIN
    pub const QUERY_PLAN_VISUALIZATION: bool = true;

    /// Performance profiling and metrics
    /// Status: Basic support
    pub const PERFORMANCE_PROFILING: bool = true;

    /// Debug logging and tracing
    /// Status: Implemented via pgrx logging
    pub const DEBUG_LOGGING: bool = true;

    /// Query execution statistics
    /// Status: Basic support
    pub const EXECUTION_STATISTICS: bool = true;
}

/// Utility functions for feature checking
pub mod utils {
    use super::*;

    /// Check if all required features are available for a given query plan
    pub fn validate_query_features(
        has_window_functions: bool,
        has_partition_by: bool,
        has_window_order_by: bool,
        has_window_filter: bool,
        has_frame_clause: bool,
        has_group_by: bool,
        has_having: bool,
        has_distinct: bool,
        has_subqueries: bool,
    ) -> Result<(), String> {
        // Check window function support
        if has_window_functions {
            if !window_functions::can_execute_window_spec(
                has_partition_by,
                has_window_order_by,
                has_window_filter,
                has_frame_clause,
            ) {
                return Err("Window function features not yet supported".to_string());
            }
        }

        // Check aggregate support
        if has_having && !aggregates::HAVING_CLAUSE {
            return Err("HAVING clause not yet supported".to_string());
        }

        if has_distinct && !aggregates::DISTINCT_AGGREGATES {
            return Err("DISTINCT aggregates not yet supported".to_string());
        }

        // Check query support
        if has_subqueries && !queries::SUBQUERIES {
            return Err("Subqueries not yet fully supported".to_string());
        }

        Ok(())
    }

    /// Get a summary of currently supported features
    pub fn supported_features_summary() -> String {
        format!(
            "ParadeDB Supported Features:\n\
             • Window Functions: Simple aggregates ({})\n\
             • Aggregates: GROUP BY ({}), FILTER ({})\n\
             • Search: Full-text ({}), Hybrid ({}), Vector ({})\n\
             • Performance: Parallel execution ({}), Index-only scans ({})\n\
             • Integration: EXPLAIN ({}), Statistics ({})",
            window_functions::SIMPLE_AGGREGATES,
            aggregates::GROUP_BY,
            aggregates::FILTER_CLAUSE,
            search::FULL_TEXT_SEARCH,
            search::HYBRID_SEARCH,
            search::VECTOR_SEARCH,
            performance::PARALLEL_EXECUTION,
            performance::INDEX_ONLY_SCANS,
            integration::EXPLAIN_INTEGRATION,
            integration::STATISTICS_INTEGRATION,
        )
    }
}
