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
//! Numeric scaling utilities for NUMERIC column pushdown.
//!
//! This module provides an abstraction for scaling and descaling numeric values
//! in the Numeric64 (I64 fixed-point) storage format. The core idea is simple:
//!
//! - **Scaling** (at index/query time): Multiply by 10^scale to convert decimals to integers
//! - **Descaling** (at result time): Divide by 10^scale to restore original decimals
//!
//! ## Storage Format
//!
//! NUMERIC(p, s) values with precision p <= 18 are stored as I64 using fixed-point:
//! - Original value: 123.45 with scale=2
//! - Stored value: 12345 (123.45 * 10^2)
//!
//! ## Usage Patterns
//!
//! 1. **Indexing**: `scale_i64()` converts NUMERIC to scaled I64 for storage
//! 2. **Queries**: `scale_i64()` converts query constants to match indexed format
//! 3. **Aggregates**: `descale_f64()` converts aggregate results back to decimals
//! 4. **GROUP BY**: `descale_owned_value()` restores group key values

use crate::api::HashMap;
use crate::postgres::PgSearchRelation;
use crate::schema::{SearchFieldType, SearchIndexSchema};
use std::sync::LazyLock;
use tantivy::schema::OwnedValue;

// ============================================================================
// Core Scaling/Descaling Operations
// ============================================================================

/// Pre-computed scale factors for common scales (0-18).
/// Avoids repeated calls to 10f64.powi() for frequently used scale values.
/// Scale 18 is the maximum for Numeric64 storage (i64 precision limit).
static SCALE_FACTORS: LazyLock<[f64; 19]> = LazyLock::new(|| {
    let mut factors = [0.0; 19];
    for (i, factor) in factors.iter_mut().enumerate() {
        *factor = 10f64.powi(i as i32);
    }
    factors
});

/// Compute the scale factor: 10^scale
///
/// Uses pre-computed values for common scales (0-18) for performance.
/// Falls back to computation for other scales (negative or >18).
#[inline]
pub fn scale_factor(scale: i16) -> f64 {
    if scale >= 0 && (scale as usize) < SCALE_FACTORS.len() {
        SCALE_FACTORS[scale as usize]
    } else {
        10f64.powi(scale as i32)
    }
}

/// Scale a numeric string to I64 fixed-point representation.
///
/// Multiplies the value by 10^scale to convert to integer.
/// Uses `decimal_bytes::Decimal64NoScale` for precise conversion.
///
/// # Example
/// ```ignore
/// scale_i64("123.45", 2) // Returns Ok(12345)
/// ```
pub fn scale_i64(numeric_str: &str, scale: i16) -> anyhow::Result<i64> {
    use decimal_bytes::Decimal64NoScale;

    let decimal = Decimal64NoScale::new(numeric_str, scale as i32).map_err(|e| {
        anyhow::anyhow!(
            "Failed to scale '{}' with scale {}: {:?}. This may occur if the value exceeds i64 range after scaling.",
            numeric_str,
            scale,
            e
        )
    })?;

    Ok(decimal.value())
}

/// Descale an I64 value back to f64.
///
/// Divides by 10^scale to restore the original decimal representation.
///
/// # Precision Warning
///
/// For aggregate results (SUM, AVG) on large datasets, values exceeding
/// 2^53 (~9 quadrillion) may lose precision due to f64 representation limits.
/// This is inherent to IEEE 754 double-precision floating-point format.
/// For most use cases, this precision is sufficient, but users working with
/// very large sums of high-precision NUMERIC values should be aware of this limit.
#[inline]
pub fn descale_i64(value: i64, scale: i16) -> f64 {
    value as f64 / scale_factor(scale)
}

/// Descale a U64 value back to f64.
#[inline]
pub fn descale_u64(value: u64, scale: i16) -> f64 {
    value as f64 / scale_factor(scale)
}

/// Descale an f64 value by dividing by 10^scale.
#[inline]
pub fn descale_f64(value: f64, scale: i16) -> f64 {
    value / scale_factor(scale)
}

/// Descale an OwnedValue based on its type.
///
/// Handles I64, U64, and F64 values, returning the descaled result as F64.
/// Used for GROUP BY key descaling where the value type may vary.
pub fn descale_owned_value(value: &OwnedValue, scale: i16) -> OwnedValue {
    match value {
        OwnedValue::I64(v) => OwnedValue::F64(descale_i64(*v, scale)),
        OwnedValue::U64(v) => OwnedValue::F64(descale_u64(*v, scale)),
        OwnedValue::F64(v) => OwnedValue::F64(descale_f64(*v, scale)),
        _ => value.clone(),
    }
}

/// Scale an OwnedValue to I64 fixed-point representation.
///
/// Handles Str, I64, U64, and F64 values, converting to scaled I64.
/// Used for query value scaling where the input type may vary.
///
/// # Example
/// ```ignore
/// scale_owned_value(OwnedValue::Str("123.45"), 2) // Returns Ok(OwnedValue::I64(12345))
/// ```
pub fn scale_owned_value(value: OwnedValue, scale: i16) -> anyhow::Result<OwnedValue> {
    let numeric_str = match &value {
        OwnedValue::Str(s) => s.clone(),
        OwnedValue::F64(f) => f.to_string(),
        OwnedValue::I64(i) => i.to_string(),
        OwnedValue::U64(u) => u.to_string(),
        _ => anyhow::bail!("Cannot scale non-numeric value: {:?}", value),
    };

    let scaled = scale_i64(&numeric_str, scale)?;
    Ok(OwnedValue::I64(scaled))
}

/// Build a mapping of aggregate names to their numeric scales from a schema.
///
/// Given a mapping of aggregate names to field names (from `extract_agg_name_to_field`),
/// looks up each field in the schema and extracts the scale for Numeric64 fields.
///
/// # Arguments
///
/// * `schema` - The search schema containing field definitions
/// * `agg_name_to_field` - Map of aggregate names to field names
///
/// # Returns
///
/// A HashMap where keys are aggregate names and values are the scale for Numeric64 fields.
/// Fields that are not Numeric64 are not included in the result.
pub fn build_numeric_field_scales(
    schema: &SearchIndexSchema,
    agg_name_to_field: &HashMap<String, String>,
) -> HashMap<String, i16> {
    let mut scales = HashMap::default();
    for (agg_name, field_name) in agg_name_to_field {
        if let Some(search_field) = schema.search_field(field_name) {
            if let SearchFieldType::Numeric64(_, scale) = search_field.field_type() {
                scales.insert(agg_name.clone(), scale);
            }
        }
    }
    scales
}

/// Check if aggregate pushdown is supported for a field and return its numeric scale.
///
/// Returns:
/// - `Some(Some(scale))` for Numeric64 fields (I64 fixed-point storage)
/// - `Some(None)` for other aggregatable field types
/// - `None` if the field uses NumericBytes storage (aggregate pushdown not supported)
pub fn get_numeric_scale_for_field(
    bm25_index: &PgSearchRelation,
    field: &str,
) -> Option<Option<i16>> {
    let schema = bm25_index.schema().ok()?;
    let search_field = schema.search_field(field)?;

    match search_field.field_type() {
        SearchFieldType::NumericBytes(_) => {
            pgrx::notice!(
                "Aggregate pushdown disabled for field '{}': \
                 NUMERIC columns without precision (or precision > 18) use byte storage \
                 which cannot be aggregated by the search index. \
                 Consider using NUMERIC(p,s) where p <= 18 for aggregate pushdown support.",
                field
            );
            None
        }
        SearchFieldType::Numeric64(_, scale) => Some(Some(scale)),
        _ => Some(None),
    }
}

// ============================================================================
// JSON Descaling for Custom Aggregates
// ============================================================================

/// Descale aggregate JSON results for Numeric64 fields.
///
/// This is the high-level API that handles the entire descaling workflow:
/// 1. Extracts aggregate name to field mappings from the aggregate definition
/// 2. Looks up Numeric64 scales from the schema
/// 3. Applies descaling to the result JSON
///
/// # Arguments
///
/// * `agg_definition` - The original aggregate JSON definition (used to extract field mappings)
/// * `schema` - The search schema containing field type information
/// * `result` - The aggregate result JSON to descale
///
/// # Returns
///
/// The descaled JSON result. If no Numeric64 fields are found, returns the original result unchanged.
pub fn descale_aggregate_result(
    agg_definition: &serde_json::Value,
    schema: &SearchIndexSchema,
    result: serde_json::Value,
) -> serde_json::Value {
    let agg_name_to_field = super::extract_agg_name_to_field(agg_definition);
    let numeric_field_scales = build_numeric_field_scales(schema, &agg_name_to_field);

    if numeric_field_scales.is_empty() {
        result
    } else {
        descale_numeric_values_in_json(result, &numeric_field_scales)
    }
}

/// Descale numeric values in custom aggregate JSON results.
///
/// This function traverses the JSON and divides "value" fields by 10^scale
/// only for aggregates that reference Numeric64 fields.
///
/// # Arguments
///
/// * `json` - The JSON value to process
/// * `scales` - Map of aggregate names to their numeric scales
///
/// # Special Keys
///
/// * `"__top_level__"` - Indicates a simple metric aggregate where the
///   top-level "value" field should be descaled.
pub fn descale_numeric_values_in_json(
    json: serde_json::Value,
    scales: &HashMap<String, i16>,
) -> serde_json::Value {
    if scales.is_empty() {
        return json;
    }

    // Check for top-level metric aggregate (special marker)
    let top_level_scale = scales.get("__top_level__").copied();
    descale_json_values_by_agg_name(json, scales, top_level_scale)
}

/// Stats aggregate field names that contain values needing descaling.
const STATS_FIELDS: &[&str] = &["avg", "max", "min", "sum", "value"];

/// Recursively descale numeric "value" fields in JSON based on aggregate name.
fn descale_json_values_by_agg_name(
    json: serde_json::Value,
    scales: &HashMap<String, i16>,
    current_scale: Option<i16>,
) -> serde_json::Value {
    match json {
        serde_json::Value::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (key, value) in map {
                // Check if this key is an aggregate name that needs descaling
                let scale_for_key = scales.get(&key).copied().or(current_scale);

                // Check if this is a stats field or value field that needs descaling
                if STATS_FIELDS.contains(&key.as_str()) {
                    // Descale the value if we have a scale for the current aggregate
                    if let Some(scale) = current_scale {
                        if let serde_json::Value::Number(ref n) = value {
                            if let Some(f) = n.as_f64() {
                                let descaled = descale_f64(f, scale);
                                new_map.insert(
                                    key,
                                    serde_json::Value::Number(
                                        serde_json::Number::from_f64(descaled)
                                            .unwrap_or_else(|| serde_json::Number::from(0)),
                                    ),
                                );
                                continue;
                            }
                        }
                    }
                    // No descaling needed or not a number
                    new_map.insert(
                        key,
                        descale_json_values_by_agg_name(value, scales, current_scale),
                    );
                } else {
                    // Recurse with the scale for this aggregate name (if any)
                    new_map.insert(
                        key,
                        descale_json_values_by_agg_name(value, scales, scale_for_key),
                    );
                }
            }
            serde_json::Value::Object(new_map)
        }
        serde_json::Value::Array(arr) => serde_json::Value::Array(
            arr.into_iter()
                .map(|v| descale_json_values_by_agg_name(v, scales, current_scale))
                .collect(),
        ),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_scale_factor() {
        assert_eq!(scale_factor(0), 1.0);
        assert_eq!(scale_factor(1), 10.0);
        assert_eq!(scale_factor(2), 100.0);
        assert_eq!(scale_factor(3), 1000.0);
    }

    #[test]
    fn test_scale_i64() {
        assert_eq!(scale_i64("123.45", 2).unwrap(), 12345);
        assert_eq!(scale_i64("0.999", 3).unwrap(), 999);
        assert_eq!(scale_i64("-50.5", 1).unwrap(), -505);
    }

    #[test]
    fn test_descale_i64() {
        assert_eq!(descale_i64(12345, 2), 123.45);
        assert_eq!(descale_i64(999, 3), 0.999);
        assert_eq!(descale_i64(-505, 1), -50.5);
    }

    #[test]
    fn test_descale_f64() {
        assert_eq!(descale_f64(12345.0, 2), 123.45);
        assert_eq!(descale_f64(999.0, 3), 0.999);
        assert_eq!(descale_f64(42.0, 0), 42.0);
    }

    #[test]
    fn test_descale_owned_value() {
        assert_eq!(
            descale_owned_value(&OwnedValue::I64(12345), 2),
            OwnedValue::F64(123.45)
        );
        assert_eq!(
            descale_owned_value(&OwnedValue::U64(999), 3),
            OwnedValue::F64(0.999)
        );
        assert_eq!(
            descale_owned_value(&OwnedValue::F64(100.0), 1),
            OwnedValue::F64(10.0)
        );
        // Non-numeric values pass through unchanged
        assert_eq!(
            descale_owned_value(&OwnedValue::Str("test".to_string()), 2),
            OwnedValue::Str("test".to_string())
        );
    }

    #[test]
    fn test_scale_owned_value() {
        // String input
        assert_eq!(
            scale_owned_value(OwnedValue::Str("123.45".to_string()), 2).unwrap(),
            OwnedValue::I64(12345)
        );
        // F64 input
        assert_eq!(
            scale_owned_value(OwnedValue::F64(0.999), 3).unwrap(),
            OwnedValue::I64(999)
        );
        // I64 input (already integer, but scales)
        assert_eq!(
            scale_owned_value(OwnedValue::I64(50), 1).unwrap(),
            OwnedValue::I64(500)
        );
        // Negative value
        assert_eq!(
            scale_owned_value(OwnedValue::Str("-50.5".to_string()), 1).unwrap(),
            OwnedValue::I64(-505)
        );
    }

    #[test]
    fn test_scale_descale_roundtrip() {
        // Verify that scale followed by descale returns the original value
        let original = 123.45f64;
        let scale = 2i16;
        let scaled = scale_owned_value(OwnedValue::F64(original), scale).unwrap();
        let descaled = descale_owned_value(&scaled, scale);
        assert_eq!(descaled, OwnedValue::F64(original));
    }

    #[test]
    fn test_descale_json_empty_scales() {
        let json = json!({"value": 123.45});
        let scales = HashMap::default();
        let result = descale_numeric_values_in_json(json.clone(), &scales);
        assert_eq!(result, json);
    }

    #[test]
    fn test_descale_json_top_level() {
        let json = json!({"value": 12345.0});
        let mut scales = HashMap::default();
        scales.insert("__top_level__".to_string(), 2i16);
        let result = descale_numeric_values_in_json(json, &scales);
        assert_eq!(result, json!({"value": 123.45}));
    }

    #[test]
    fn test_descale_json_nested() {
        let json = json!({
            "avg_price": {
                "value": 9999.0
            }
        });
        let mut scales = HashMap::default();
        scales.insert("avg_price".to_string(), 2i16);
        let result = descale_numeric_values_in_json(json, &scales);
        assert_eq!(
            result,
            json!({
                "avg_price": {
                    "value": 99.99
                }
            })
        );
    }
}
