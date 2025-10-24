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

use crate::postgres::customscan::aggregatescan::aggregate_type::AggregateType;
use crate::postgres::customscan::aggregatescan::groupby::GroupingColumn;
use pgrx::pg_sys;
use serde::{Deserialize, Serialize};
use tantivy::aggregation::agg_req::{Aggregation, Aggregations};

/// This is the **core struct** that captures aggregation parameters at the SQL level,
/// shared between:
/// - **pdbscan**: Window functions (`COUNT(*) OVER ()`)
/// - **aggregatescan**: GROUP BY queries (`SELECT ... GROUP BY ... HAVING ...`)
///
/// Note: ORDER BY is NOT part of this spec because:
/// - For window functions: We don't support ORDER BY in the OVER clause
/// - For GROUP BY queries: ORDER BY is a result ordering concern, not an aggregation concern
///   (it's handled separately in aggregatescan's PrivateData)
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AggregationSpec {
    /// Aggregate types (COUNT, SUM, AVG, MIN, MAX with optional FILTER support)
    /// For window functions, this typically contains one aggregate per window function
    /// For GROUP BY queries, this contains all aggregates in the query
    pub agg_types: Vec<AggregateType>,
    /// GROUP BY columns (empty if no grouping)
    /// Note: At planner hook time, attno will be 0 (invalid). It's filled in during
    /// custom scan planning when we have access to the relation descriptor.
    pub grouping_columns: Vec<GroupingColumn>,
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

    /// Convert AggregationSpec to Tantivy Aggregations for execution (simple, no GROUP BY)
    ///
    /// This is used for window functions which don't have GROUP BY.
    /// For GROUP BY queries, use AggregateCSClause instead.
    pub fn to_tantivy_aggregations(&self) -> Result<Aggregations, String> {
        let mut aggregations = Aggregations::new();

        for (idx, agg_type) in self.agg_types.iter().enumerate() {
            // Convert each AggregateType to AggregationVariants
            let agg_variant = agg_type.clone().into();
            aggregations.insert(
                idx.to_string(),
                Aggregation {
                    agg: agg_variant,
                    sub_aggregation: Aggregations::new(),
                },
            );
        }

        Ok(aggregations)
    }
}
