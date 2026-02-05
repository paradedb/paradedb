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

use crate::schema::SearchFieldType;
use anyhow::Result;
use decimal_bytes::Decimal;
use tantivy::schema::OwnedValue;

// ============================================================================
// Numeric64 Scaling (I64 Fixed-Point)
// ============================================================================

/// Scale a numeric string to I64 fixed-point representation.
///
/// Multiplies the value by 10^scale to convert to integer.
/// Uses `decimal_bytes::Decimal64NoScale` for precise conversion.
///
/// # Example
/// ```ignore
/// scale_i64("123.45", 2) // Returns Ok(12345)
/// ```
pub fn scale_i64(numeric_str: &str, scale: i16) -> Result<i64> {
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

/// Scale an OwnedValue to I64 fixed-point representation.
///
/// Handles Str, I64, U64, and F64 values, converting to scaled I64.
/// Uses direct primitive constructors (`from_i64`, `from_u64`, `from_f64`)
/// for efficient conversion without string intermediates.
///
/// # Example
/// ```ignore
/// scale_owned_value(OwnedValue::Str("123.45"), 2) // Returns Ok(OwnedValue::I64(12345))
/// scale_owned_value(OwnedValue::I64(100), 2) // Returns Ok(OwnedValue::I64(10000))
/// ```
pub fn scale_owned_value(value: OwnedValue, scale: i16) -> Result<OwnedValue> {
    use decimal_bytes::Decimal64NoScale;

    let scale_i32 = scale as i32;

    let scaled = match &value {
        // Use direct primitive constructors for efficiency
        OwnedValue::I64(i) => Decimal64NoScale::from_i64(*i, scale_i32)
            .map_err(|e| anyhow::anyhow!("Failed to scale i64 {}: {:?}", i, e))?
            .value(),
        OwnedValue::U64(u) => Decimal64NoScale::from_u64(*u, scale_i32)
            .map_err(|e| anyhow::anyhow!("Failed to scale u64 {}: {:?}", u, e))?
            .value(),
        OwnedValue::F64(f) => Decimal64NoScale::from_f64(*f, scale_i32)
            .map_err(|e| anyhow::anyhow!("Failed to scale f64 {}: {:?}", f, e))?
            .value(),
        // Fall back to string parsing for string values
        OwnedValue::Str(s) => scale_i64(s, scale)?,
        _ => {
            return Err(anyhow::anyhow!(
                "Cannot scale non-numeric value: {:?}",
                value
            ))
        }
    };

    Ok(OwnedValue::I64(scaled))
}

// ============================================================================
// NumericBytes Conversions
// ============================================================================

/// Convert a numeric value to a hex-encoded string for NumericBytes storage.
///
/// Used for NumericBytes storage where precision exceeds 18 digits.
/// Convert a numeric value to its Decimal representation.
/// Helper function used by both raw bytes and hex string conversions.
fn value_to_decimal(value: &OwnedValue) -> Result<Decimal> {
    match value {
        OwnedValue::I64(i) => Ok(Decimal::from(*i)),
        OwnedValue::U64(u) => Ok(Decimal::from(*u)),
        OwnedValue::F64(f) => Decimal::try_from(*f)
            .map_err(|e| anyhow::anyhow!("Failed to convert f64 {} to Decimal: {:?}", f, e)),
        OwnedValue::Str(s) => Decimal::from_str(s)
            .map_err(|e| anyhow::anyhow!("Failed to parse numeric '{}': {:?}", s, e)),
        _ => Err(anyhow::anyhow!(
            "Cannot convert non-numeric value: {:?}",
            value
        )),
    }
}

/// Convert a numeric value to raw bytes (OwnedValue::Bytes).
///
/// Uses `decimal_bytes::Decimal` for arbitrary-precision decimal encoding.
/// The byte encoding is lexicographically sortable for range queries.
///
/// Used for NumericBytes fields which are stored as Tantivy Bytes columns.
pub fn numeric_value_to_decimal_bytes(value: OwnedValue) -> Result<OwnedValue> {
    let decimal = value_to_decimal(&value)?;
    Ok(OwnedValue::Bytes(decimal.into_bytes()))
}

/// Convert a numeric value to a hex-encoded string (OwnedValue::Str).
///
/// Uses `decimal_bytes::Decimal` for arbitrary-precision decimal encoding.
/// The hex encoding preserves lexicographic byte ordering for range queries.
///
/// Used for NUMRANGEOID fields which are stored in JSON columns.
pub fn numeric_value_to_hex_string(value: OwnedValue) -> Result<OwnedValue> {
    let decimal = value_to_decimal(&value)?;
    Ok(OwnedValue::Str(bytes_to_hex(decimal.as_bytes())))
}

/// Convert a byte slice to a hex-encoded string.
///
/// This is a common utility for encoding lexicographically sortable decimal bytes
/// into strings that preserve the byte ordering. Used by NumericBytes storage
/// and NUMRANGE fields.
#[inline]
pub fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Convert a hex-encoded string back to bytes.
///
/// This is the inverse of `bytes_to_hex`. Used for deserializing hex-encoded
/// decimal values from storage.
///
/// Returns `None` if the string contains invalid hex characters or has odd length.
#[inline]
fn hex_to_bytes(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect()
}

/// Convert a hex-encoded decimal string back to a Decimal.
///
/// This is a convenience function that combines `hex_to_bytes` and `Decimal::from_bytes`
/// to convert from hex-encoded storage format to a `Decimal` object.
///
/// Returns `None` if the hex string is invalid or cannot be parsed as a Decimal.
#[inline]
pub fn hex_to_decimal(hex: &str) -> Option<Decimal> {
    let bytes = hex_to_bytes(hex)?;
    Decimal::from_bytes(&bytes).ok()
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

/// Convert a numeric bound to lexicographically sortable raw bytes.
/// Used for NumericBytes fields stored as Tantivy Bytes columns.
pub fn numeric_bound_to_bytes(bound: Bound<OwnedValue>) -> Result<Bound<OwnedValue>> {
    convert_bound(bound, numeric_value_to_decimal_bytes)
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
///
/// Uses direct type conversions where possible to avoid unnecessary string intermediates.
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

    // Convert based on the range's element type
    match oid.try_into() {
        Ok(BuiltinOid::INT4RANGEOID) | Ok(BuiltinOid::INT8RANGEOID) => {
            // Integer ranges: convert directly to i64
            match &value {
                OwnedValue::I64(i) => OwnedValue::I64(*i),
                OwnedValue::U64(u) => OwnedValue::I64(*u as i64),
                OwnedValue::F64(f) => OwnedValue::I64(*f as i64),
                OwnedValue::Str(s) => {
                    // Try parsing as i64 first to preserve precision
                    if let Ok(i) = s.parse::<i64>() {
                        return OwnedValue::I64(i);
                    }
                    // Fallback: try parsing as f64 for decimal values
                    if let Ok(f) = s.parse::<f64>() {
                        return OwnedValue::I64(f as i64);
                    }
                    value
                }
                _ => value,
            }
        }
        Ok(BuiltinOid::NUMRANGEOID) => {
            // Numeric ranges are indexed as hex-encoded sortable bytes
            // Use numeric_value_to_hex_string for JSON storage
            numeric_value_to_hex_string(value.clone()).unwrap_or(value)
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
        SearchFieldType::NumericBytes(_) => numeric_value_to_decimal_bytes(value),
        SearchFieldType::Json(_) => Ok(string_to_json_numeric(value)),
        SearchFieldType::I64(_) => Ok(string_to_i64(value)),
        SearchFieldType::U64(_) => Ok(string_to_u64(value)),
        SearchFieldType::F64(_) => Ok(string_to_f64(value)),
        _ => Ok(value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_i64() {
        assert_eq!(scale_i64("123.45", 2).unwrap(), 12345);
        assert_eq!(scale_i64("0.999", 3).unwrap(), 999);
        assert_eq!(scale_i64("-50.5", 1).unwrap(), -505);
    }

    #[test]
    fn test_scale_owned_value() {
        // String input
        assert_eq!(
            scale_owned_value(OwnedValue::Str("123.45".to_string()), 2).unwrap(),
            OwnedValue::I64(12345)
        );
        // F64 input (uses from_f64 directly)
        assert_eq!(
            scale_owned_value(OwnedValue::F64(0.999), 3).unwrap(),
            OwnedValue::I64(999)
        );
        // I64 input (uses from_i64 directly)
        assert_eq!(
            scale_owned_value(OwnedValue::I64(50), 1).unwrap(),
            OwnedValue::I64(500)
        );
        // U64 input (uses from_u64 directly)
        assert_eq!(
            scale_owned_value(OwnedValue::U64(100), 2).unwrap(),
            OwnedValue::I64(10000)
        );
        // Negative value via string
        assert_eq!(
            scale_owned_value(OwnedValue::Str("-50.5".to_string()), 1).unwrap(),
            OwnedValue::I64(-505)
        );
        // Negative I64 (uses from_i64 directly)
        assert_eq!(
            scale_owned_value(OwnedValue::I64(-25), 2).unwrap(),
            OwnedValue::I64(-2500)
        );
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
