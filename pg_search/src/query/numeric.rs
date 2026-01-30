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
//! Numeric conversion utilities for NUMERIC column pushdown.
//!
//! This module provides utilities for converting numeric values between different
//! representations used in query processing:
//!
//! - **Numeric64**: I64 fixed-point storage for NUMERIC(p,s) where p <= 18
//! - **NumericBytes**: Lexicographically sortable bytes for unlimited precision
//! - **JSON numeric types**: I64, U64, F64 for JSON field comparisons
//!
//! The module consolidates all numeric conversion logic to avoid duplication
//! and provide consistent error handling.

use std::ops::Bound;
use std::str::FromStr;

use super::value_to_term;
use crate::api::FieldName;
use crate::schema::SearchField;
use crate::schema::SearchFieldType;
use anyhow::Result;
use decimal_bytes::Decimal;
use tantivy::query::{Query as TantivyQuery, TermQuery};
use tantivy::schema::IndexRecordOption;
use tantivy::schema::OwnedValue;

// ============================================================================
// Core String Extraction
// ============================================================================

/// Extract a numeric string representation from an OwnedValue.
///
/// This is the foundation for all numeric conversions, ensuring consistent
/// handling of different input types.
pub fn extract_numeric_string(value: &OwnedValue) -> Option<String> {
    match value {
        OwnedValue::Str(s) => Some(s.clone()),
        OwnedValue::F64(f) => Some(f.to_string()),
        OwnedValue::I64(i) => Some(i.to_string()),
        OwnedValue::U64(u) => Some(u.to_string()),
        _ => None,
    }
}

// ============================================================================
// Numeric64 (I64 Fixed-Point) Conversions
// ============================================================================

// Re-export scale_owned_value from the centralized descale module.
// This provides the symmetric counterpart to descale_owned_value.
pub use crate::postgres::customscan::aggregatescan::descale::scale_owned_value;

// ============================================================================
// NumericBytes Conversions
// ============================================================================

/// Convert a numeric value to lexicographically sortable bytes.
///
/// Used for NumericBytes storage where precision exceeds 18 digits.
/// The byte representation maintains sort order for range queries.
pub fn numeric_value_to_bytes(value: OwnedValue) -> Result<OwnedValue> {
    let numeric_str = extract_numeric_string(&value)
        .ok_or_else(|| anyhow::anyhow!("Cannot convert non-numeric value to bytes: {:?}", value))?;

    let decimal = Decimal::from_str(&numeric_str).map_err(|e| {
        anyhow::anyhow!(
            "Failed to convert numeric value '{}' to bytes: {:?}",
            numeric_str,
            e
        )
    })?;

    Ok(OwnedValue::Bytes(decimal.as_bytes().to_vec()))
}

/// Convert a numeric string to hex-encoded sortable bytes.
///
/// Used for NUMRANGE fields where values are stored as hex strings.
pub fn numeric_to_hex_bytes(numeric_str: &str) -> Result<String> {
    let decimal = Decimal::from_str(numeric_str)
        .map_err(|e| anyhow::anyhow!("Failed to parse numeric '{}': {:?}", numeric_str, e))?;

    Ok(decimal
        .as_bytes()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect())
}

// ============================================================================
// JSON Numeric Type Detection and Conversion
// ============================================================================

/// Check if a string represents a plain integer (no decimal point or scientific notation).
fn is_plain_integer(s: &str) -> bool {
    !s.contains('.') && !s.contains('e') && !s.contains('E')
}

/// Convert a string-encoded numeric value to the appropriate JSON type.
///
/// JSON distinguishes between integers (I64/U64) and floats (F64).
/// Detection is based on whether the value contains a decimal point or scientific notation.
pub fn string_to_json_numeric(value: OwnedValue) -> OwnedValue {
    let s = match &value {
        OwnedValue::Str(s) => s,
        _ => return value,
    };

    let trimmed = s.trim();

    if is_plain_integer(trimmed) {
        // Try i64 first (handles negative and small positive integers)
        if let Ok(i) = trimmed.parse::<i64>() {
            return OwnedValue::I64(i);
        }
        // Try u64 for large positive integers beyond i64::MAX
        if let Ok(u) = trimmed.parse::<u64>() {
            return OwnedValue::U64(u);
        }
    }

    // For decimal values, scientific notation, or fallback, use F64
    if let Ok(f) = trimmed.parse::<f64>() {
        return OwnedValue::F64(f);
    }

    value
}

/// Convert a string-encoded numeric value to I64.
///
/// Parses directly as i64 to preserve precision (f64 loses precision for large integers).
pub fn string_to_i64(value: OwnedValue) -> OwnedValue {
    let s = match &value {
        OwnedValue::Str(s) => s,
        _ => return value,
    };

    let trimmed = s.trim();

    // Try to parse directly as i64 first to preserve precision
    if let Ok(i) = trimmed.parse::<i64>() {
        return OwnedValue::I64(i);
    }
    // Fall back to f64 parsing for decimal values, then truncate
    if let Ok(f) = trimmed.parse::<f64>() {
        return OwnedValue::I64(f as i64);
    }

    value
}

/// Convert a string-encoded numeric value to U64.
///
/// Parses directly as u64 to preserve precision (f64 loses precision for large integers).
pub fn string_to_u64(value: OwnedValue) -> OwnedValue {
    let s = match &value {
        OwnedValue::Str(s) => s,
        _ => return value,
    };

    let trimmed = s.trim();

    // Try to parse directly as u64 first to preserve precision
    if let Ok(u) = trimmed.parse::<u64>() {
        return OwnedValue::U64(u);
    }
    // Fall back to f64 parsing for decimal values, then truncate
    if let Ok(f) = trimmed.parse::<f64>() {
        if f >= 0.0 {
            return OwnedValue::U64(f as u64);
        }
    }

    value
}

/// Convert a string-encoded numeric value to F64.
pub fn string_to_f64(value: OwnedValue) -> OwnedValue {
    if let OwnedValue::Str(s) = &value {
        if let Ok(f) = s.parse::<f64>() {
            return OwnedValue::F64(f);
        }
    }
    value
}

// ============================================================================
// Generic Bound Conversion
// ============================================================================

/// Convert a bound using a fallible conversion function.
///
/// This generic helper eliminates the need for separate `convert_bound_to_*` functions.
pub fn convert_bound<F>(bound: Bound<OwnedValue>, converter: F) -> Result<Bound<OwnedValue>>
where
    F: Fn(OwnedValue) -> Result<OwnedValue>,
{
    Ok(match bound {
        Bound::Included(v) => Bound::Included(converter(v)?),
        Bound::Excluded(v) => Bound::Excluded(converter(v)?),
        Bound::Unbounded => Bound::Unbounded,
    })
}

/// Convert a bound using an infallible conversion function.
///
/// Used for JSON type conversions that always succeed (returning input on failure).
pub fn map_bound<F>(bound: Bound<OwnedValue>, converter: F) -> Bound<OwnedValue>
where
    F: Fn(OwnedValue) -> OwnedValue,
{
    match bound {
        Bound::Included(v) => Bound::Included(converter(v)),
        Bound::Excluded(v) => Bound::Excluded(converter(v)),
        Bound::Unbounded => Bound::Unbounded,
    }
}

/// Scale a numeric bound value for Numeric64 storage.
pub fn scale_numeric_bound(bound: Bound<OwnedValue>, scale: i16) -> Result<Bound<OwnedValue>> {
    convert_bound(bound, |v| scale_owned_value(v, scale))
}

/// Convert a numeric bound to lexicographically sortable bytes.
pub fn numeric_bound_to_bytes(bound: Bound<OwnedValue>) -> Result<Bound<OwnedValue>> {
    convert_bound(bound, numeric_value_to_bytes)
}

// ============================================================================
// Range Field Conversion
// ============================================================================

/// Convert a value for range field queries based on the range element type.
///
/// Range fields are indexed with specific element types:
/// - INT4RANGEOID, INT8RANGEOID: indexed as i32/i64 → convert to I64
/// - NUMRANGEOID: indexed as hex-encoded sortable bytes → convert to hex string
/// - Date/time ranges: use datetime conversion (handled elsewhere)
pub fn convert_value_for_range_field(
    value: OwnedValue,
    field_type: &SearchFieldType,
) -> OwnedValue {
    use pgrx::pg_sys::BuiltinOid;

    // Get the OID to determine the range element type
    let oid = match field_type {
        SearchFieldType::Range(oid) => *oid,
        _ => return value, // Not a range field, pass through
    };

    // Get string representation for conversion
    let numeric_str = match extract_numeric_string(&value) {
        Some(s) => s,
        None => return value, // Non-numeric types pass through unchanged
    };

    // Convert based on the range's element type
    match oid.try_into() {
        Ok(BuiltinOid::INT4RANGEOID) | Ok(BuiltinOid::INT8RANGEOID) => {
            // Integer ranges: parse directly to i64 to preserve precision
            if let Ok(i) = numeric_str.parse::<i64>() {
                return OwnedValue::I64(i);
            }
            // Fallback: try parsing as decimal for values with decimal points
            if let Ok(f) = numeric_str.parse::<f64>() {
                return OwnedValue::I64(f as i64);
            }
            value
        }
        Ok(BuiltinOid::NUMRANGEOID) => {
            // Numeric ranges are indexed as hex-encoded sortable bytes
            if let Ok(hex) = numeric_to_hex_bytes(&numeric_str) {
                return OwnedValue::Str(hex);
            }
            value
        }
        // Date/time ranges are handled by the is_datetime flag
        _ => value,
    }
}

// ============================================================================
// Generic Field Type Conversion
// ============================================================================

/// Convert a value to the appropriate format for a search field type.
///
/// This consolidates the conversion logic used across term queries, term_set queries,
/// and other places where values need to be converted based on field type.
///
/// # Arguments
/// * `value` - The input value to convert
/// * `field_type` - The target field type determining the conversion
///
/// # Returns
/// The converted value, or an error if conversion fails for Numeric64/NumericBytes.
pub fn convert_value_for_field(
    value: OwnedValue,
    field_type: &SearchFieldType,
) -> Result<OwnedValue> {
    match field_type {
        SearchFieldType::Numeric64(_, scale) => scale_owned_value(value, *scale),
        SearchFieldType::NumericBytes(_) => numeric_value_to_bytes(value),
        SearchFieldType::Json(_) => Ok(string_to_json_numeric(value)),
        SearchFieldType::I64(_) => Ok(string_to_i64(value)),
        SearchFieldType::U64(_) => Ok(string_to_u64(value)),
        SearchFieldType::F64(_) => Ok(string_to_f64(value)),
        _ => Ok(value),
    }
}

/// Convert a value for a field, returning a default on error.
///
/// This is useful for bulk conversions (like term_set) where individual
/// failures shouldn't abort the entire operation.
pub fn convert_value_for_field_or_default(
    value: OwnedValue,
    field_type: &SearchFieldType,
    default: OwnedValue,
) -> OwnedValue {
    convert_value_for_field(value, field_type).unwrap_or(default)
}

/// Try to create a term query for a numeric field from a query string.
///
/// For Numeric64 and NumericBytes fields, Tantivy's QueryParser can't parse
/// decimal strings directly. This function handles the conversion:
/// - Numeric64: Scales the string value to I64 fixed-point
/// - NumericBytes: Converts the string to lexicographically sortable bytes
///
/// Returns `Some(query)` if successful, `None` if the field type doesn't need
/// special handling or the string isn't a valid numeric value.
pub fn try_numeric_term_query(
    search_field: &SearchField,
    field: &FieldName,
    query_string: &str,
) -> Option<Result<Box<dyn TantivyQuery>>> {
    use crate::postgres::customscan::aggregatescan::descale::scale_owned_value;

    let trimmed = query_string.trim();

    match search_field.field_type() {
        SearchFieldType::Numeric64(_, scale) => {
            // Scale the string value directly for I64 storage (preserves precision)
            match scale_owned_value(OwnedValue::Str(trimmed.to_string()), scale) {
                Ok(scaled_value) => {
                    let field_type = search_field.field_entry().field_type();
                    match value_to_term(
                        search_field.field(),
                        &scaled_value,
                        field_type,
                        field.path().as_deref(),
                        false,
                    ) {
                        Ok(term) => Some(Ok(Box::new(TermQuery::new(
                            term,
                            IndexRecordOption::WithFreqsAndPositions,
                        )))),
                        Err(e) => Some(Err(e)),
                    }
                }
                Err(_) => None, // Not a valid decimal, fall through to regular parsing
            }
        }
        SearchFieldType::NumericBytes(_) => {
            // Convert the string value directly to bytes (preserves precision)
            match numeric_value_to_bytes(OwnedValue::Str(trimmed.to_string())) {
                Ok(bytes_value) => {
                    let field_type = search_field.field_entry().field_type();
                    match value_to_term(
                        search_field.field(),
                        &bytes_value,
                        field_type,
                        field.path().as_deref(),
                        false,
                    ) {
                        Ok(term) => Some(Ok(Box::new(TermQuery::new(
                            term,
                            IndexRecordOption::WithFreqsAndPositions,
                        )))),
                        Err(e) => Some(Err(e)),
                    }
                }
                Err(_) => None, // Not a valid decimal, fall through to regular parsing
            }
        }
        _ => None, // Not a numeric field, use regular parsing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_numeric_string() {
        assert_eq!(
            extract_numeric_string(&OwnedValue::Str("123.45".to_string())),
            Some("123.45".to_string())
        );
        assert_eq!(
            extract_numeric_string(&OwnedValue::I64(42)),
            Some("42".to_string())
        );
        assert_eq!(
            extract_numeric_string(&OwnedValue::F64(2.5)),
            Some("2.5".to_string())
        );
        assert_eq!(extract_numeric_string(&OwnedValue::Null), None);
    }

    #[test]
    fn test_string_to_json_numeric() {
        // Plain integers
        assert_eq!(
            string_to_json_numeric(OwnedValue::Str("42".to_string())),
            OwnedValue::I64(42)
        );
        assert_eq!(
            string_to_json_numeric(OwnedValue::Str("-42".to_string())),
            OwnedValue::I64(-42)
        );

        // Decimal values
        assert_eq!(
            string_to_json_numeric(OwnedValue::Str("2.5".to_string())),
            OwnedValue::F64(2.5)
        );

        // Scientific notation
        assert_eq!(
            string_to_json_numeric(OwnedValue::Str("1e10".to_string())),
            OwnedValue::F64(1e10)
        );
    }

    #[test]
    fn test_map_bound() {
        let bound = Bound::Included(OwnedValue::Str("42".to_string()));
        let result = map_bound(bound, string_to_i64);
        assert_eq!(result, Bound::Included(OwnedValue::I64(42)));

        let unbounded: Bound<OwnedValue> = Bound::Unbounded;
        let result = map_bound(unbounded, string_to_i64);
        assert_eq!(result, Bound::Unbounded);
    }
}
