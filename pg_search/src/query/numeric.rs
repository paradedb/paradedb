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

use std::num::IntErrorKind;
use std::ops::Bound;
use std::str::FromStr;

use crate::api::version::{Version, VersionInfo};
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::schema::SearchFieldType;
use anyhow::Result;
use decimal_bytes::Decimal;

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
pub fn scale_owned_value(value: PdbOwnedValue, scale: i16) -> Result<PdbOwnedValue> {
    use decimal_bytes::Decimal64NoScale;

    let scale_i32 = scale as i32;

    let scaled = match &value {
        // Use direct primitive constructors for efficiency
        PdbOwnedValue::I64(i) => Decimal64NoScale::from_i64(*i, scale_i32)
            .map_err(|e| anyhow::anyhow!("Failed to scale i64 {}: {:?}", i, e))?
            .value(),
        PdbOwnedValue::U64(u) => Decimal64NoScale::from_u64(*u, scale_i32)
            .map_err(|e| anyhow::anyhow!("Failed to scale u64 {}: {:?}", u, e))?
            .value(),
        PdbOwnedValue::F64(f) => Decimal64NoScale::from_f64(*f, scale_i32)
            .map_err(|e| anyhow::anyhow!("Failed to scale f64 {}: {:?}", f, e))?
            .value(),
        // Fall back to string parsing for string values
        PdbOwnedValue::Str(s) => scale_i64(s, scale)?,
        _ => {
            return Err(anyhow::anyhow!(
                "Cannot scale non-numeric value: {:?}",
                value
            ));
        }
    };

    Ok(PdbOwnedValue::I64(scaled))
}

// ============================================================================
// NumericBytes Conversions
// ============================================================================

/// Convert a numeric value to its Decimal representation.
/// Helper function used by both raw bytes and hex string conversions.
fn value_to_decimal(value: &PdbOwnedValue) -> Result<Decimal> {
    match value {
        PdbOwnedValue::I64(i) => Ok(Decimal::from(*i)),
        PdbOwnedValue::U64(u) => Ok(Decimal::from(*u)),
        PdbOwnedValue::F64(f) => Decimal::try_from(*f)
            .map_err(|e| anyhow::anyhow!("Failed to convert f64 {} to Decimal: {:?}", f, e)),
        PdbOwnedValue::Str(s) => Decimal::from_str(s)
            .map_err(|e| anyhow::anyhow!("Failed to parse numeric '{}': {:?}", s, e)),
        _ => Err(anyhow::anyhow!(
            "Cannot convert non-numeric value: {:?}",
            value
        )),
    }
}

/// Encode `decimal` in the byte layout used by the index that `index_created_by_version`
/// identifies. Stored values and query terms have to agree on the layout, because the index
/// compares them bytewise.
///
/// See [`crate::api::version::NUMERIC_BYTES_SORTABLE_NEGATIVES_VERSION`].
pub fn decimal_to_index_bytes(
    decimal: Decimal,
    index_created_by_version: Option<Version>,
) -> Vec<u8> {
    if index_created_by_version.stores_sortable_negative_numeric_bytes() {
        decimal.into_bytes()
    } else {
        decimal.to_legacy_bytes()
    }
}

/// Convert a numeric value to raw bytes (PdbOwnedValue::Bytes).
///
/// Uses `decimal_bytes::Decimal` for arbitrary-precision decimal encoding.
/// The byte encoding is lexicographically sortable for range queries.
///
/// Used for NumericBytes fields which are stored as Tantivy Bytes columns.
pub fn numeric_value_to_decimal_bytes(
    value: PdbOwnedValue,
    index_created_by_version: Option<Version>,
) -> Result<PdbOwnedValue> {
    let decimal = value_to_decimal(&value)?;
    Ok(PdbOwnedValue::Bytes(decimal_to_index_bytes(
        decimal,
        index_created_by_version,
    )))
}

/// Convert a numeric value to a hex-encoded string (PdbOwnedValue::Str).
///
/// Uses `decimal_bytes::Decimal` for arbitrary-precision decimal encoding.
/// The hex encoding preserves lexicographic byte ordering for range queries.
///
/// Used for NUMRANGEOID fields which store bounds in JSON columns.
/// JSON doesn't support raw bytes, so we hex-encode to preserve lexicographic ordering.
///
/// TODO: Consider changing Range field storage to support raw bytes instead of JSON,
/// which would eliminate the hex encoding overhead.
pub fn numeric_value_to_hex_string(
    value: PdbOwnedValue,
    index_created_by_version: Option<Version>,
) -> Result<PdbOwnedValue> {
    let decimal = value_to_decimal(&value)?;
    Ok(PdbOwnedValue::Str(bytes_to_hex(&decimal_to_index_bytes(
        decimal,
        index_created_by_version,
    ))))
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
pub fn string_to_json_numeric(value: PdbOwnedValue) -> PdbOwnedValue {
    let s = match &value {
        PdbOwnedValue::Str(s) => s,
        _ => return value,
    };

    let trimmed = s.trim();

    if is_plain_integer(trimmed) {
        // Try i64 first (handles negative and small positive integers)
        if let Ok(i) = trimmed.parse::<i64>() {
            return PdbOwnedValue::I64(i);
        }
        // Try u64 for large positive integers beyond i64::MAX
        if let Ok(u) = trimmed.parse::<u64>() {
            return PdbOwnedValue::U64(u);
        }
    }

    // For decimal values, scientific notation, or fallback, use F64
    if let Ok(f) = trimmed.parse::<f64>() {
        return PdbOwnedValue::F64(f);
    }

    value
}

/// Convert a string-encoded numeric value to I64.
///
/// Parses directly as i64 to preserve precision (f64 loses precision for large integers).
pub fn string_to_i64(value: PdbOwnedValue) -> PdbOwnedValue {
    let s = match &value {
        PdbOwnedValue::Str(s) => s,
        _ => return value,
    };

    let trimmed = s.trim();

    // Try to parse directly as i64 first to preserve precision
    if let Ok(i) = trimmed.parse::<i64>() {
        return PdbOwnedValue::I64(i);
    }
    // Fall back to f64 parsing for decimal values, then truncate
    if let Ok(f) = trimmed.parse::<f64>() {
        return PdbOwnedValue::I64(f as i64);
    }

    value
}

/// Convert a string-encoded numeric value to U64.
///
/// Parses directly as u64 to preserve precision (f64 loses precision for large integers).
pub fn string_to_u64(value: PdbOwnedValue) -> PdbOwnedValue {
    let s = match &value {
        PdbOwnedValue::Str(s) => s,
        _ => return value,
    };

    let trimmed = s.trim();

    // Try to parse directly as u64 first to preserve precision
    if let Ok(u) = trimmed.parse::<u64>() {
        return PdbOwnedValue::U64(u);
    }
    // Fall back to f64 parsing for decimal values, then truncate
    if let Ok(f) = trimmed.parse::<f64>()
        && f >= 0.0
    {
        return PdbOwnedValue::U64(f as u64);
    }

    value
}

/// Convert a string-encoded numeric value to F64.
pub fn string_to_f64(value: PdbOwnedValue) -> PdbOwnedValue {
    if let PdbOwnedValue::Str(s) = &value
        && let Ok(f) = s.parse::<f64>()
    {
        return PdbOwnedValue::F64(f);
    }
    value
}

// ============================================================================
// Integer Field Domains
// ============================================================================

/// The values that an `I64` or a `U64` field can hold.
#[derive(Clone, Copy)]
enum IntDomain {
    I64,
    U64,
}

impl IntDomain {
    fn of(field_type: &SearchFieldType) -> Option<Self> {
        match field_type {
            SearchFieldType::I64(_) => Some(Self::I64),
            SearchFieldType::U64(_) => Some(Self::U64),
            _ => None,
        }
    }

    /// The smallest value, as an exact integer and as a value of the field's type.
    fn min(self) -> (i128, PdbOwnedValue) {
        match self {
            Self::I64 => (i64::MIN.into(), PdbOwnedValue::I64(i64::MIN)),
            Self::U64 => (u64::MIN.into(), PdbOwnedValue::U64(u64::MIN)),
        }
    }

    /// The largest value, as an exact integer and as a value of the field's type.
    fn max(self) -> (i128, PdbOwnedValue) {
        match self {
            Self::I64 => (i64::MAX.into(), PdbOwnedValue::I64(i64::MAX)),
            Self::U64 => (u64::MAX.into(), PdbOwnedValue::U64(u64::MAX)),
        }
    }
}

/// The integer that `value` holds, if it is an integer or a string of one. A string of an integer
/// too large for an `i128` lies outside every domain, so it saturates.
fn exact_integer(value: &PdbOwnedValue) -> Option<i128> {
    match value {
        PdbOwnedValue::I64(n) => Some((*n).into()),
        PdbOwnedValue::U64(n) => Some((*n).into()),
        PdbOwnedValue::Str(s) => match s.trim().parse::<i128>() {
            Ok(n) => Some(n),
            Err(e) => match e.kind() {
                IntErrorKind::PosOverflow => Some(i128::MAX),
                IntErrorKind::NegOverflow => Some(i128::MIN),
                _ => None,
            },
        },
        _ => None,
    }
}

/// Clamp the bounds of a range on an integer field to the field's range. A bound that admits no
/// value becomes a lower `Excluded(max)` or an upper `Excluded(min)`, and one that admits every
/// value becomes a lower `Included(min)` or an upper `Included(max)`. Other bounds are unchanged.
pub fn clamp_int_bounds(
    field_type: &SearchFieldType,
    lower: Bound<PdbOwnedValue>,
    upper: Bound<PdbOwnedValue>,
) -> (Bound<PdbOwnedValue>, Bound<PdbOwnedValue>) {
    let Some(domain) = IntDomain::of(field_type) else {
        return (lower, upper);
    };
    let ((min, min_value), (max, max_value)) = (domain.min(), domain.max());
    let lower = match lower.as_ref().map(exact_integer) {
        Bound::Included(Some(n)) if n > max => Bound::Excluded(max_value.clone()),
        Bound::Excluded(Some(n)) if n >= max => Bound::Excluded(max_value.clone()),
        Bound::Included(Some(n)) | Bound::Excluded(Some(n)) if n < min => {
            Bound::Included(min_value.clone())
        }
        _ => lower,
    };
    let upper = match upper.as_ref().map(exact_integer) {
        Bound::Included(Some(n)) | Bound::Excluded(Some(n)) if n < min => {
            Bound::Excluded(min_value)
        }
        Bound::Included(Some(n)) if n >= max => Bound::Included(max_value),
        Bound::Excluded(Some(n)) if n > max => Bound::Included(max_value),
        _ => upper,
    };
    (lower, upper)
}

/// Check if `value` is an integer that a field of `field_type` cannot hold.
pub fn is_outside_int_domain(value: &PdbOwnedValue, field_type: &SearchFieldType) -> bool {
    let (Some(domain), Some(n)) = (IntDomain::of(field_type), exact_integer(value)) else {
        return false;
    };
    n < domain.min().0 || n > domain.max().0
}

// ============================================================================
// Generic Bound Conversion
// ============================================================================

/// Convert a bound using a fallible conversion function.
///
/// This generic helper eliminates the need for separate `convert_bound_to_*` functions.
pub fn convert_bound<F>(bound: Bound<PdbOwnedValue>, converter: F) -> Result<Bound<PdbOwnedValue>>
where
    F: Fn(PdbOwnedValue) -> Result<PdbOwnedValue>,
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
pub fn map_bound<F>(bound: Bound<PdbOwnedValue>, converter: F) -> Bound<PdbOwnedValue>
where
    F: Fn(PdbOwnedValue) -> PdbOwnedValue,
{
    match bound {
        Bound::Included(v) => Bound::Included(converter(v)),
        Bound::Excluded(v) => Bound::Excluded(converter(v)),
        Bound::Unbounded => Bound::Unbounded,
    }
}

/// Scale a numeric bound value for Numeric64 storage.
pub fn scale_numeric_bound(
    bound: Bound<PdbOwnedValue>,
    scale: i16,
) -> Result<Bound<PdbOwnedValue>> {
    convert_bound(bound, |v| scale_owned_value(v, scale))
}

/// Convert a numeric bound to lexicographically sortable raw bytes.
/// Used for NumericBytes fields stored as Tantivy Bytes columns.
pub fn numeric_bound_to_bytes(
    bound: Bound<PdbOwnedValue>,
    index_created_by_version: Option<Version>,
) -> Result<Bound<PdbOwnedValue>> {
    convert_bound(bound, |v| {
        numeric_value_to_decimal_bytes(v, index_created_by_version)
    })
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
    value: PdbOwnedValue,
    field_type: &SearchFieldType,
    index_created_by_version: Option<Version>,
) -> PdbOwnedValue {
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
                PdbOwnedValue::I64(i) => PdbOwnedValue::I64(*i),
                PdbOwnedValue::U64(u) => PdbOwnedValue::I64(*u as i64),
                PdbOwnedValue::F64(f) => PdbOwnedValue::I64(*f as i64),
                PdbOwnedValue::Str(s) => {
                    // Try parsing as i64 first to preserve precision
                    if let Ok(i) = s.parse::<i64>() {
                        return PdbOwnedValue::I64(i);
                    }
                    // Fallback: try parsing as f64 for decimal values
                    if let Ok(f) = s.parse::<f64>() {
                        return PdbOwnedValue::I64(f as i64);
                    }
                    value
                }
                _ => value,
            }
        }
        Ok(BuiltinOid::NUMRANGEOID) => {
            // Numeric ranges are indexed as hex-encoded sortable bytes
            // Use numeric_value_to_hex_string for JSON storage
            numeric_value_to_hex_string(value.clone(), index_created_by_version).unwrap_or(value)
        }
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
/// * `index_created_by_version` - Selects the `NumericBytes` byte layout
///
/// # Returns
/// The converted value, or an error if conversion fails for Numeric64/NumericBytes.
pub fn convert_value_for_field(
    value: PdbOwnedValue,
    field_type: &SearchFieldType,
    index_created_by_version: Option<Version>,
) -> Result<PdbOwnedValue> {
    match field_type {
        SearchFieldType::Numeric64(_, scale) => scale_owned_value(value, *scale),
        SearchFieldType::NumericBytes(..) => {
            numeric_value_to_decimal_bytes(value, index_created_by_version)
        }
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
    use rstest::rstest;
    use std::ops::Bound::{Excluded, Included, Unbounded};

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
            scale_owned_value(PdbOwnedValue::Str("123.45".to_string()), 2).unwrap(),
            PdbOwnedValue::I64(12345)
        );
        // F64 input (uses from_f64 directly)
        assert_eq!(
            scale_owned_value(PdbOwnedValue::F64(0.999), 3).unwrap(),
            PdbOwnedValue::I64(999)
        );
        // I64 input (uses from_i64 directly)
        assert_eq!(
            scale_owned_value(PdbOwnedValue::I64(50), 1).unwrap(),
            PdbOwnedValue::I64(500)
        );
        // U64 input (uses from_u64 directly)
        assert_eq!(
            scale_owned_value(PdbOwnedValue::U64(100), 2).unwrap(),
            PdbOwnedValue::I64(10000)
        );
        // Negative value via string
        assert_eq!(
            scale_owned_value(PdbOwnedValue::Str("-50.5".to_string()), 1).unwrap(),
            PdbOwnedValue::I64(-505)
        );
        // Negative I64 (uses from_i64 directly)
        assert_eq!(
            scale_owned_value(PdbOwnedValue::I64(-25), 2).unwrap(),
            PdbOwnedValue::I64(-2500)
        );
    }

    #[test]
    fn test_string_to_json_numeric() {
        // Plain integers
        assert_eq!(
            string_to_json_numeric(PdbOwnedValue::Str("42".to_string())),
            PdbOwnedValue::I64(42)
        );
        assert_eq!(
            string_to_json_numeric(PdbOwnedValue::Str("-42".to_string())),
            PdbOwnedValue::I64(-42)
        );

        // Decimal values
        assert_eq!(
            string_to_json_numeric(PdbOwnedValue::Str("2.5".to_string())),
            PdbOwnedValue::F64(2.5)
        );

        // Scientific notation
        assert_eq!(
            string_to_json_numeric(PdbOwnedValue::Str("1e10".to_string())),
            PdbOwnedValue::F64(1e10)
        );
    }

    #[test]
    fn test_decimal_to_index_bytes_follows_index_version() {
        use crate::api::version::NUMERIC_BYTES_SORTABLE_NEGATIVES_VERSION;

        let decimal = Decimal::from_str("-49990").unwrap();
        let current = decimal.clone().into_bytes();
        let legacy = decimal.to_legacy_bytes();
        assert_ne!(current, legacy);

        assert_eq!(decimal_to_index_bytes(decimal.clone(), None), legacy);
        // The releases that shipped `decimal-bytes` 0.4 wrote the legacy layout.
        for legacy_version in [Version::new(0, 25, 3), Version::new(0, 25, 4)] {
            assert_eq!(
                decimal_to_index_bytes(decimal.clone(), Some(legacy_version)),
                legacy
            );
        }
        assert_eq!(
            decimal_to_index_bytes(
                decimal.clone(),
                Some(NUMERIC_BYTES_SORTABLE_NEGATIVES_VERSION)
            ),
            current
        );
        assert_eq!(
            decimal_to_index_bytes(decimal, Some(Version::new(1, 0, 0))),
            current
        );

        // Both layouts decode to the same value.
        assert_eq!(Decimal::from_bytes(&legacy).unwrap().to_string(), "-49990");
    }

    #[test]
    fn test_map_bound() {
        let bound = Bound::Included(PdbOwnedValue::Str("42".to_string()));
        let result = map_bound(bound, string_to_i64);
        assert_eq!(result, Bound::Included(PdbOwnedValue::I64(42)));

        let unbounded: Bound<PdbOwnedValue> = Bound::Unbounded;
        let result = map_bound(unbounded, string_to_i64);
        assert_eq!(result, Bound::Unbounded);
    }

    fn int(n: i64) -> PdbOwnedValue {
        PdbOwnedValue::I64(n)
    }

    fn uint(n: u64) -> PdbOwnedValue {
        PdbOwnedValue::U64(n)
    }

    fn text(s: &str) -> PdbOwnedValue {
        PdbOwnedValue::Str(s.to_string())
    }

    const INT8: SearchFieldType = SearchFieldType::I64(pgrx::pg_sys::INT8OID);
    const OID: SearchFieldType = SearchFieldType::U64(pgrx::pg_sys::OIDOID);
    const HUGE: &str = "100000000000000000000000000000000000000000";

    #[rstest]
    #[case::above_max(INT8, Included(uint(1 << 63)), Excluded(int(i64::MAX)))]
    #[case::beyond_i128(INT8, Included(text(HUGE)), Excluded(int(i64::MAX)))]
    #[case::at_max(INT8, Included(int(i64::MAX)), Included(int(i64::MAX)))]
    #[case::excluded_at_max(INT8, Excluded(uint(i64::MAX as u64)), Excluded(int(i64::MAX)))]
    #[case::excluded_below_min(
        INT8,
        Excluded(text("-9223372036854775809")),
        Included(int(i64::MIN))
    )]
    #[case::inside(INT8, Excluded(uint(5)), Excluded(uint(5)))]
    #[case::u64_below_min(OID, Included(int(-5)), Included(uint(0)))]
    fn clamp_int_lower_bound(
        #[case] field_type: SearchFieldType,
        #[case] bound: Bound<PdbOwnedValue>,
        #[case] expected: Bound<PdbOwnedValue>,
    ) {
        assert_eq!(
            clamp_int_bounds(&field_type, bound, Unbounded),
            (expected, Unbounded)
        );
    }

    #[rstest]
    #[case::below_min(INT8, Included(text("-9223372036854775809")), Excluded(int(i64::MIN)))]
    #[case::beyond_i128(INT8, Excluded(text(&format!("-{HUGE}"))), Excluded(int(i64::MIN)))]
    #[case::at_max(INT8, Included(uint(i64::MAX as u64)), Included(int(i64::MAX)))]
    #[case::above_max(INT8, Included(text("18000000000000000000")), Included(int(i64::MAX)))]
    #[case::excluded_above_max(INT8, Excluded(uint(1 << 63)), Included(int(i64::MAX)))]
    #[case::excluded_at_max(INT8, Excluded(int(i64::MAX)), Excluded(int(i64::MAX)))]
    #[case::u64_below_min(OID, Included(int(-5)), Excluded(uint(0)))]
    #[case::u64_at_max(OID, Included(uint(u64::MAX)), Included(uint(u64::MAX)))]
    fn clamp_int_upper_bound(
        #[case] field_type: SearchFieldType,
        #[case] bound: Bound<PdbOwnedValue>,
        #[case] expected: Bound<PdbOwnedValue>,
    ) {
        assert_eq!(
            clamp_int_bounds(&field_type, Unbounded, bound),
            (Unbounded, expected)
        );
    }

    #[rstest]
    #[case::decimal(Included(text("1.5")))]
    #[case::exponent(Included(text("1e30")))]
    #[case::infinity(Excluded(text("infinity")))]
    #[case::date(Included(text("2024-01-01")))]
    fn clamp_int_bounds_leaves_non_integers(#[case] bound: Bound<PdbOwnedValue>) {
        for field_type in [INT8, OID] {
            assert_eq!(
                clamp_int_bounds(&field_type, bound.clone(), bound.clone()),
                (bound.clone(), bound.clone())
            );
        }
    }

    #[rstest]
    #[case::at_max(INT8, int(i64::MAX), false)]
    #[case::above_max(INT8, uint(1 << 63), true)]
    #[case::at_min(INT8, text("-9223372036854775808"), false)]
    #[case::below_min(INT8, text("-9223372036854775809"), true)]
    #[case::infinity(INT8, text("infinity"), false)]
    #[case::u64_negative(OID, int(-1), true)]
    #[case::float_field(SearchFieldType::F64(pgrx::pg_sys::FLOAT8OID), uint(1 << 63), false)]
    fn test_is_outside_int_domain(
        #[case] field_type: SearchFieldType,
        #[case] value: PdbOwnedValue,
        #[case] expected: bool,
    ) {
        assert_eq!(is_outside_int_domain(&value, &field_type), expected);
    }
}
