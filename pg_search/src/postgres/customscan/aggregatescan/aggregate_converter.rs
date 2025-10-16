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

//! Conversion logic for AggregateType to Tantivy representations

use super::privdat::AggregateType;
use crate::aggregate::errors::AggregationError;
use crate::aggregate::tantivy_keys::{AVG, CTID, FIELD, MAX, MIN, MISSING, SUM, VALUE_COUNT};
use tantivy::aggregation::agg_req::Aggregation;

/// Handles conversion of AggregateType to various representations
pub struct AggregateConverter;

impl AggregateConverter {
    /// Create Tantivy aggregation from AggregateType (without filter wrapper)
    pub fn to_tantivy_agg(agg: &AggregateType) -> Result<Aggregation, AggregationError> {
        let unfiltered = if agg.filter_expr().is_some() {
            Self::convert_filtered_to_unfiltered(agg)
        } else {
            agg.clone()
        };
        Ok(serde_json::from_value(Self::to_json(&unfiltered))?)
    }

    /// Convert AggregateType to JSON representation
    pub fn to_json(agg: &AggregateType) -> serde_json::Value {
        let (key, field) = match agg {
            AggregateType::CountAny { .. } => (VALUE_COUNT, CTID),
            AggregateType::Count { field, .. } => (VALUE_COUNT, field.as_str()),
            AggregateType::Sum { field, .. } => (SUM, field.as_str()),
            AggregateType::Avg { field, .. } => (AVG, field.as_str()),
            AggregateType::Min { field, .. } => (MIN, field.as_str()),
            AggregateType::Max { field, .. } => (MAX, field.as_str()),
        };

        if let Some(missing) = agg.missing() {
            serde_json::json!({
                key: {
                    FIELD: field,
                    MISSING: missing,
                }
            })
        } else {
            serde_json::json!({
                key: {
                    FIELD: field,
                }
            })
        }
    }

    /// Helper function to convert a filtered aggregate to unfiltered
    fn convert_filtered_to_unfiltered(agg: &AggregateType) -> AggregateType {
        match agg {
            AggregateType::CountAny { .. } => AggregateType::CountAny { filter: None },
            AggregateType::Count { field, missing, .. } => AggregateType::Count {
                field: field.clone(),
                missing: *missing,
                filter: None,
            },
            AggregateType::Sum { field, missing, .. } => AggregateType::Sum {
                field: field.clone(),
                missing: *missing,
                filter: None,
            },
            AggregateType::Avg { field, missing, .. } => AggregateType::Avg {
                field: field.clone(),
                missing: *missing,
                filter: None,
            },
            AggregateType::Min { field, missing, .. } => AggregateType::Min {
                field: field.clone(),
                missing: *missing,
                filter: None,
            },
            AggregateType::Max { field, missing, .. } => AggregateType::Max {
                field: field.clone(),
                missing: *missing,
                filter: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_any_to_json() {
        let agg = AggregateType::CountAny { filter: None };
        let json = AggregateConverter::to_json(&agg);
        assert_eq!(
            json,
            serde_json::json!({
                "value_count": {
                    "field": "ctid"
                }
            })
        );
    }

    #[test]
    fn test_sum_with_missing_to_json() {
        let agg = AggregateType::Sum {
            field: "price".to_string(),
            missing: Some(0.0),
            filter: None,
        };
        let json = AggregateConverter::to_json(&agg);
        assert_eq!(
            json,
            serde_json::json!({
                "sum": {
                    "field": "price",
                    "missing": 0.0
                }
            })
        );
    }
}
