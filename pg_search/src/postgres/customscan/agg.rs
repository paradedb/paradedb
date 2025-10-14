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

//! Unified aggregation specification shared by both pdbscan (window functions) and aggregatescan (GROUP BY)

use crate::aggregate::AggQueryParams;
use crate::api::OrderByInfo;
use crate::postgres::customscan::aggregatescan::privdat::GroupingColumn;
use crate::postgres::customscan::aggregatescan::AggregateType;
use crate::query::SearchQueryInput;
use pgrx::pg_sys;
use serde::{Deserialize, Serialize};

/// This is the **core struct** that captures aggregation parameters at the SQL level,
/// shared between:
/// - **pdbscan**: Window functions (`COUNT(*) OVER (PARTITION BY ...)`)
/// - **aggregatescan**: GROUP BY queries (`SELECT ... GROUP BY ... HAVING ...`)
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AggregationSpec {
    /// Aggregate types (COUNT, SUM, AVG, MIN, MAX with optional FILTER support)
    /// For window functions, this typically contains one aggregate per window function
    /// For GROUP BY queries, this contains all aggregates in the query
    pub agg_types: Vec<AggregateType>,
    /// PARTITION BY / GROUP BY columns (empty if no grouping)
    /// Note: At planner hook time, attno will be 0 (invalid). It's filled in during
    /// custom scan planning when we have access to the relation descriptor.
    pub grouping_columns: Vec<GroupingColumn>,
    /// ORDER BY specification (empty if no ordering)
    /// Reuses existing OrderByInfo structure from api module
    pub orderby_info: Vec<OrderByInfo>,
}

impl AggregationSpec {
    /// Get the result type OID for the first aggregate
    ///
    /// For window functions, we typically have one aggregate.
    /// For GROUP BY queries, this is used for validation but all aggregates are processed.
    pub fn result_type_oid(&self) -> pg_sys::Oid {
        self.agg_types
            .first()
            .map(|agg| agg.result_type_oid())
            .unwrap_or(pg_sys::INT8OID)
    }

    /// Convert AggregationSpec to AggQueryParams for execution
    ///
    /// This function allows both window functions and GROUP BY queries to reuse the
    /// existing Tantivy aggregation infrastructure (build_aggregation_query_from_search_input).
    ///
    /// # Arguments
    /// * `base_query` - The search query from the WHERE clause
    /// * `limit` - Optional LIMIT (for GROUP BY queries, None for window functions)
    /// * `offset` - Optional OFFSET (for GROUP BY queries, None for window functions)
    ///
    /// # Note
    /// At execution time, the `attno` field in `grouping_columns` should already be filled
    /// by `fill_partition_by_attnos()` during custom scan planning. If attno is still 0,
    /// that indicates a bug in the planning phase.
    pub fn to_agg_params<'a>(
        &'a self,
        base_query: &'a SearchQueryInput,
        limit: &'a Option<u32>,
        offset: &'a Option<u32>,
    ) -> AggQueryParams<'a> {
        AggQueryParams {
            base_query,
            aggregate_types: &self.agg_types,
            grouping_columns: &self.grouping_columns,
            orderby_info: &self.orderby_info,
            limit,
            offset,
        }
    }
}
