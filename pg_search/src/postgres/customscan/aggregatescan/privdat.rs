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

use crate::api::{AsCStr, OrderByInfo};
use crate::query::SearchQueryInput;
use pgrx::pg_sys::AsPgCStr;
use pgrx::prelude::*;
use pgrx::PgList;
use serde::Deserialize;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AggregateType {
    Count,
    Sum { field: String },
    Avg { field: String },
    Min { field: String },
    Max { field: String },
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum AggregateValue {
    #[default]
    Null,
    Int(i64),
    Float(f64),
}

/// Enum to specify how numbers should be converted for different aggregate types
#[derive(Debug, Clone, Copy)]
enum NumberConversionMode {
    /// For COUNT: validate integer, check range, always return Int
    ToInt,
    /// For SUM/MIN/MAX: preserve original type (Int or Float)
    Preserve,
    /// For AVG: always convert to Float
    ToFloat,
}

/// Represents an aggregate result that can be either a direct value or wrapped in a "value" object
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum AggregateResult {
    /// Direct numeric value (e.g., `42`)
    DirectValue(serde_json::Number),
    /// Object with "value" field (e.g., `{"value": 42}`)
    ValueObject { value: Option<serde_json::Number> },
    /// Null value
    Null,
}

impl AggregateResult {
    /// Extract the numeric value, returning None if null
    pub fn extract_number(&self) -> Option<&serde_json::Number> {
        match self {
            AggregateResult::DirectValue(num) => Some(num),
            AggregateResult::ValueObject { value } => value.as_ref(),
            AggregateResult::Null => None,
        }
    }
}

// TODO: We should use Tantivy's native aggregate types (CountAggregation, SumAggregation, etc.)
// which implement serde, but the current fork/version produces incorrect JSON structure.
// The Tantivy types serialize to {"field": "name", "missing": null} instead of
// the expected {"aggregation_type": {"field": "name"}} format.
// https://docs.rs/tantivy/latest/tantivy/aggregation/metric/struct.CountAggregation.html
impl AggregateType {
    /// Get the field name for field-based aggregates (None for COUNT)
    pub fn field_name(&self) -> Option<String> {
        match self {
            AggregateType::Count => None,
            AggregateType::Sum { field } => Some(field.clone()),
            AggregateType::Avg { field } => Some(field.clone()),
            AggregateType::Min { field } => Some(field.clone()),
            AggregateType::Max { field } => Some(field.clone()),
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        match self {
            AggregateType::Count => {
                serde_json::json!({
                    "value_count": {
                        "field": "ctid"
                    }
                })
            }
            AggregateType::Sum { field } => {
                serde_json::json!({
                    "sum": {
                        "field": field
                    }
                })
            }
            AggregateType::Avg { field } => {
                serde_json::json!({
                    "avg": {
                        "field": field
                    }
                })
            }
            AggregateType::Min { field } => {
                serde_json::json!({
                    "min": {
                        "field": field
                    }
                })
            }
            AggregateType::Max { field } => {
                serde_json::json!({
                    "max": {
                        "field": field
                    }
                })
            }
        }
    }

    #[allow(unreachable_patterns)]
    pub fn to_json_for_group(&self, idx: usize) -> Option<(String, serde_json::Value)> {
        match self {
            AggregateType::Count => None, // 'terms' bucket already has a 'doc_count'
            _ => Some((format!("agg_{idx}"), self.to_json())),
        }
    }

    /// Convert AggregateResult to AggregateValue with empty result set handling.
    ///
    /// This method handles the interaction between aggregate types, document counts,
    /// and number processing to ensure correct behavior across all scenarios:
    ///
    /// ## Empty Result Set Handling
    /// When `doc_count` is provided and equals 0, this indicates an empty result set:
    /// - **COUNT**: Returns 0 (counting zero documents is valid and equals 0)
    /// - **SUM**: Returns NULL (sum of empty set is undefined/NULL in SQL standard)
    /// - **AVG/MIN/MAX**: Processed normally (will return NULL from empty AggregateResult)
    ///
    /// ## Document Count Usage by Aggregate Type
    /// - **SUM**: Uses `doc_count` to distinguish truly empty buckets (doc_count=0) from
    ///   buckets containing documents with zero/null field values
    /// - **Other aggregates**: Ignore `doc_count` as they can determine emptiness from
    ///   the aggregate result itself
    ///
    /// ## Number Processing and Type Conversion
    /// Once a valid numeric result is extracted, it undergoes type-specific processing:
    /// - **COUNT**: Validates integer values, checks range, always returns Int
    /// - **SUM/MIN/MAX**: Preserves original type (Int or Float) from the result
    /// - **AVG**: Always converts to Float (division result should be floating-point)
    ///
    /// ## Parameters
    /// - `result`: The raw aggregate result from the search engine (JSON format)
    /// - `doc_count`: Optional document count for the bucket/result set being processed
    ///
    /// ## Returns
    /// `AggregateValue` which can be Int, Float, or Null depending on the aggregate type
    /// and the input data.
    pub fn result_from_aggregate_with_doc_count(
        &self,
        result: AggregateResult,
        doc_count: Option<i64>,
    ) -> AggregateValue {
        // Handle empty result sets for SUM aggregates specifically
        // SUM needs doc_count to distinguish empty buckets from buckets with zero values
        if matches!(self, AggregateType::Sum { .. }) && doc_count == Some(0) {
            return AggregateValue::Null;
        }

        // Extract the numeric value from the aggregate result
        // This handles both direct values and {"value": ...} wrapped objects
        match result.extract_number() {
            None => AggregateValue::Null,
            Some(num) => {
                // Determine the appropriate number conversion mode based on aggregate type
                let processing_type = match self {
                    AggregateType::Count => NumberConversionMode::ToInt,
                    AggregateType::Sum { .. } => NumberConversionMode::Preserve,
                    AggregateType::Avg { .. } => NumberConversionMode::ToFloat,
                    AggregateType::Min { .. } => NumberConversionMode::Preserve,
                    AggregateType::Max { .. } => NumberConversionMode::Preserve,
                };

                // Process and convert the number according to the aggregate type requirements
                Self::process_number(num, processing_type)
            }
        }
    }

    /// Process a number value based on the aggregate type requirements
    fn process_number(
        num: &serde_json::Number,
        processing_type: NumberConversionMode,
    ) -> AggregateValue {
        match processing_type {
            NumberConversionMode::ToInt => {
                let f64_val = num.as_f64().expect("invalid COUNT result");
                if f64_val.fract() != 0.0 {
                    panic!("COUNT should not have a fractional result");
                }
                if f64_val < (i64::MIN as f64) || (i64::MAX as f64) < f64_val {
                    panic!("COUNT value was out of range");
                }
                AggregateValue::Int(f64_val as i64)
            }
            NumberConversionMode::Preserve => {
                if let Some(int_val) = num.as_i64() {
                    AggregateValue::Int(int_val)
                } else if let Some(f64_val) = num.as_f64() {
                    AggregateValue::Float(f64_val)
                } else {
                    panic!("Numeric result should be a valid number");
                }
            }
            NumberConversionMode::ToFloat => {
                let f64_val = num.as_f64().expect("invalid float result");
                AggregateValue::Float(f64_val)
            }
        }
    }
}

impl AggregateValue {
    pub fn into_datum(self) -> pg_sys::Datum {
        match self {
            AggregateValue::Int(val) => val.into_datum().unwrap(),
            AggregateValue::Float(val) => val.into_datum().unwrap(),
            AggregateValue::Null => pg_sys::Datum::null(),
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, AggregateValue::Null)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GroupingColumn {
    pub field_name: String,
    pub attno: pg_sys::AttrNumber,
    pub json_path: Option<String>, // JSON path if this is a JSON field access (e.g., "reservation_id" for metadata->>'reservation_id')
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TargetListEntry {
    GroupingColumn(usize), // Index into grouping_columns vec
    Aggregate(usize),      // Index into aggregate_types vec
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PrivateData {
    pub aggregate_types: Vec<AggregateType>,
    pub indexrelid: pg_sys::Oid,
    pub heap_rti: pg_sys::Index,
    pub query: SearchQueryInput,
    pub grouping_columns: Vec<GroupingColumn>,
    pub orderby_info: Vec<OrderByInfo>,
    pub target_list_mapping: Vec<TargetListEntry>, // Maps target list position to data type
}

impl From<*mut pg_sys::List> for PrivateData {
    fn from(list: *mut pg_sys::List) -> Self {
        unsafe {
            let list = PgList::<pg_sys::Node>::from_pg(list);
            let node = list.get_ptr(0).unwrap();
            let content = node
                .as_c_str()
                .unwrap()
                .to_str()
                .expect("string node should be valid utf8");
            serde_json::from_str(content).unwrap()
        }
    }
}

impl From<PrivateData> for *mut pg_sys::List {
    fn from(value: PrivateData) -> Self {
        let content = serde_json::to_string(&value).unwrap();
        unsafe {
            let mut ser = PgList::new();
            ser.push(pg_sys::makeString(content.as_pg_cstr()).cast::<pg_sys::Node>());
            ser.into_pg()
        }
    }
}
