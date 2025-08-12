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

use crate::api::AsCStr;
use crate::postgres::customscan::builders::custom_path::OrderByInfo;
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

// TODO: We should likely directly using tantivy's aggregate types, which all derive serde.
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

    /// Convert AggregateResult to AggregateValue with proper empty result set handling.
    ///
    /// This method handles the complex interaction between aggregate types and document counts
    /// to ensure correct behavior for empty result sets:
    ///
    /// ## Empty Result Set Handling (when doc_count is Some(0)):
    /// - **COUNT**: Returns 0 (counting zero documents)
    /// - **SUM**: Returns NULL (sum of empty set is undefined/NULL in SQL)
    /// - **AVG/MIN/MAX**: Returns NULL (operations on empty sets are undefined)
    ///
    /// ## Non-empty Result Sets:
    /// - **SUM**: Uses doc_count to detect truly empty buckets vs. buckets with all zero values
    /// - **Other aggregates**: Ignore doc_count and process the aggregate result directly
    ///
    /// ## Parameters:
    /// - `result`: The raw aggregate result from the search engine
    /// - `doc_count`: Optional document count for the bucket/result set being processed
    pub fn result_from_aggregate_with_doc_count(
        &self,
        result: AggregateResult,
        doc_count: Option<i64>,
    ) -> AggregateValue {
        match (self, doc_count) {
            (AggregateType::Sum { .. }, Some(0)) => AggregateValue::Null,
            _ => self.result_from_aggregate_internal(result),
        }
    }

    fn result_from_aggregate_internal(&self, agg_result: AggregateResult) -> AggregateValue {
        // Extract the number and process it based on the aggregate type
        match agg_result.extract_number() {
            None => AggregateValue::Null,
            Some(num) => {
                let processing_type = match self {
                    AggregateType::Count => NumberConversionMode::ToInt,
                    AggregateType::Sum { .. } => NumberConversionMode::Preserve,
                    AggregateType::Avg { .. } => NumberConversionMode::ToFloat,
                    AggregateType::Min { .. } => NumberConversionMode::Preserve,
                    AggregateType::Max { .. } => NumberConversionMode::Preserve,
                };
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
    pub order_by_info: Vec<OrderByInfo>,
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
