//! Descaling utilities for NUMERIC aggregate results.
//!
//! This module provides utilities for descaling numeric values that were stored
//! as scaled integers (Numeric64 format). When PostgreSQL NUMERIC values are indexed
//! using Numeric64 storage, they are multiplied by 10^scale to preserve precision
//! as integers. After aggregation, the results need to be divided by 10^scale to
//! restore the original decimal representation.
//!
//! ## Descaling Patterns
//!
//! 1. **Simple aggregates (SUM, AVG, MIN, MAX)**: The `numeric_scale` field in
//!    `AggregateType` stores the scale, and results are descaled in `aggregate_result_to_datum`.
//!
//! 2. **Custom aggregates (pdb.agg)**: The `numeric_field_scales` map stores scales
//!    for each aggregate name, allowing selective descaling within JSON results.
//!
//! 3. **GROUP BY values**: When grouping by a Numeric64 column, the group key values
//!    are also scaled and need descaling for display.

use crate::schema::{SearchFieldType, SearchIndexSchema};
use std::collections::HashMap;

/// Compute the divisor for a given scale: 10^scale
#[inline]
pub fn scale_divisor(scale: i16) -> f64 {
    10f64.powi(scale as i32)
}

/// Descale a f64 value by dividing by 10^scale.
///
/// This is the core operation for converting scaled integer results back
/// to their original decimal representation.
#[inline]
pub fn descale_f64(value: f64, scale: i16) -> f64 {
    value / scale_divisor(scale)
}

/// Extract numeric scales from a schema for a list of field names.
///
/// Returns a map of field name -> scale for all fields that use Numeric64 storage.
/// Fields that don't exist or don't use Numeric64 are excluded from the result.
#[allow(dead_code)]
pub fn extract_numeric_scales_from_schema(
    schema: &SearchIndexSchema,
    field_names: &[&str],
) -> HashMap<String, i16> {
    let mut scales = HashMap::new();
    for &field_name in field_names {
        if let Some(search_field) = schema.search_field(field_name) {
            if let SearchFieldType::Numeric64(_, scale) = search_field.field_type() {
                scales.insert(field_name.to_string(), scale);
            }
        }
    }
    scales
}

/// Get the numeric scale for a single field if it uses Numeric64 storage.
#[allow(dead_code)]
pub fn get_numeric_scale_for_field(schema: &SearchIndexSchema, field_name: &str) -> Option<i16> {
    schema.search_field(field_name).and_then(|search_field| {
        if let SearchFieldType::Numeric64(_, scale) = search_field.field_type() {
            Some(scale)
        } else {
            None
        }
    })
}

/// Check if a field uses NumericBytes storage (unlimited precision).
///
/// Fields with NumericBytes storage cannot be aggregated by Tantivy,
/// so aggregate pushdown must be disabled for them.
#[allow(dead_code)]
pub fn field_uses_numeric_bytes(schema: &SearchIndexSchema, field_name: &str) -> bool {
    schema
        .search_field(field_name)
        .map(|search_field| matches!(search_field.field_type(), SearchFieldType::NumericBytes(_)))
        .unwrap_or(false)
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
///
/// Only descales values that are nested under an aggregate name that has a scale.
/// Also handles stats aggregate fields (avg, max, min, sum) which need descaling.
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
    fn test_scale_divisor() {
        assert_eq!(scale_divisor(0), 1.0);
        assert_eq!(scale_divisor(1), 10.0);
        assert_eq!(scale_divisor(2), 100.0);
        assert_eq!(scale_divisor(3), 1000.0);
    }

    #[test]
    fn test_descale_f64() {
        assert_eq!(descale_f64(12345.0, 2), 123.45);
        assert_eq!(descale_f64(999.0, 3), 0.999);
        assert_eq!(descale_f64(42.0, 0), 42.0);
    }

    #[test]
    fn test_descale_json_empty_scales() {
        let json = json!({"value": 123.45});
        let scales = HashMap::new();
        let result = descale_numeric_values_in_json(json.clone(), &scales);
        assert_eq!(result, json);
    }

    #[test]
    fn test_descale_json_top_level() {
        let json = json!({"value": 12345.0});
        let mut scales = HashMap::new();
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
        let mut scales = HashMap::new();
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
