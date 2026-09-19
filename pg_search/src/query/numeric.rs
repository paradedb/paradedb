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
// Query-literal placement on the Numeric64 grid
// ============================================================================

/// Which side of a range a bound belongs to.
///
/// A query literal that falls between two grid points has to move outwards to the grid
/// point that preserves PostgreSQL comparison semantics, and which way "outwards" is
/// depends on the side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundSide {
    Lower,
    Upper,
}

/// Where a query literal falls on the fixed-point grid of a `Numeric64` field.
///
/// A `Numeric64` field stores values as `i64` multiples of `10^-scale`. PostgreSQL does not
/// apply the column's typmod to a comparison operand, so a literal with more fractional
/// digits than the field's scale is compared exactly and simply does not land on the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridPosition {
    /// The literal is exactly representable at the field's scale.
    Exact(i64),
    /// The literal lies strictly between the grid points `floor` and `floor + 1`.
    Between { floor: i64 },
}

impl GridPosition {
    /// The largest grid value that is less than or equal to the literal.
    pub fn floor(self) -> i64 {
        match self {
            GridPosition::Exact(v) => v,
            GridPosition::Between { floor } => floor,
        }
    }

    /// The smallest grid value that is greater than or equal to the literal.
    pub fn ceil(self) -> i64 {
        match self {
            GridPosition::Exact(v) => v,
            GridPosition::Between { floor } => floor + 1,
        }
    }

    pub fn is_exact(self) -> bool {
        matches!(self, GridPosition::Exact(_))
    }
}

/// Render a numeric value as a plain decimal string.
///
/// `f64` goes through the shortest round-trip representation, which is exact for the value
/// actually held, rather than through a scale-aware constructor that would round it.
fn owned_value_decimal_string(value: &PdbOwnedValue) -> Result<String> {
    Ok(match value {
        PdbOwnedValue::Str(s) => s.clone(),
        PdbOwnedValue::I64(i) => i.to_string(),
        PdbOwnedValue::U64(u) => u.to_string(),
        PdbOwnedValue::F64(f) => {
            if !f.is_finite() {
                return Err(anyhow::anyhow!(
                    "Cannot place non-finite value {} on the Numeric64 grid",
                    f
                ));
            }
            format!("{f}")
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Cannot place non-numeric value on the Numeric64 grid: {:?}",
                value
            ));
        }
    })
}

/// Locate a decimal literal on the fixed-point grid of a field with the given scale,
/// **without rounding it**.
///
/// This is the query-side counterpart of [`scale_i64`]. `scale_i64` rounds, which is what
/// encoding a stored column value needs; a comparison operand must instead report which grid
/// points it falls between, so the caller can pick the one that preserves PostgreSQL
/// semantics.
pub fn locate_on_grid(numeric_str: &str, scale: i16) -> Result<GridPosition> {
    let text = numeric_str.trim();
    if text.is_empty() {
        return Err(anyhow::anyhow!("Cannot parse empty numeric literal"));
    }

    let (negative, rest) = match text.as_bytes()[0] {
        b'-' => (true, &text[1..]),
        b'+' => (false, &text[1..]),
        _ => (false, text),
    };

    // Split off an exponent, so that `1.5e3` is handled as exactly as `1500`.
    let (mantissa, exponent) = match rest.find(['e', 'E']) {
        Some(idx) => {
            let exp: i32 = rest[idx + 1..].parse().map_err(|_| {
                anyhow::anyhow!("Invalid exponent in numeric literal '{}'", numeric_str)
            })?;
            (&rest[..idx], exp)
        }
        None => (rest, 0),
    };

    let (int_part, frac_part) = match mantissa.split_once('.') {
        Some((i, f)) => (i, f),
        None => (mantissa, ""),
    };

    if int_part.is_empty() && frac_part.is_empty() {
        return Err(anyhow::anyhow!("Invalid numeric literal '{}'", numeric_str));
    }
    if !int_part.bytes().all(|b| b.is_ascii_digit())
        || !frac_part.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(anyhow::anyhow!("Invalid numeric literal '{}'", numeric_str));
    }

    // value == digits * 10^(exponent - frac_len), and we want floor(value * 10^scale),
    // which is floor(digits * 10^shift) for the shift below.
    let digits_str = format!("{int_part}{frac_part}");
    let shift = exponent - (frac_part.len() as i32) + (scale as i32);

    let overflow = || {
        anyhow::anyhow!(
            "Numeric literal '{}' exceeds i64 range at scale {}",
            numeric_str,
            scale
        )
    };

    let mut digits: i128 = 0;
    for b in digits_str.bytes() {
        digits = digits
            .checked_mul(10)
            .and_then(|d| d.checked_add((b - b'0') as i128))
            .ok_or_else(overflow)?;
    }

    let position = if shift >= 0 {
        let mut scaled = digits;
        for _ in 0..shift {
            scaled = scaled.checked_mul(10).ok_or_else(overflow)?;
        }
        let signed = if negative { -scaled } else { scaled };
        GridPosition::Exact(i64::try_from(signed).map_err(|_| overflow())?)
    } else {
        let mut divisor: i128 = 1;
        for _ in 0..(-shift) {
            divisor = divisor.checked_mul(10).ok_or_else(overflow)?;
        }
        let quotient = digits / divisor;
        let remainder = digits % divisor;
        if remainder == 0 {
            let signed = if negative { -quotient } else { quotient };
            GridPosition::Exact(i64::try_from(signed).map_err(|_| overflow())?)
        } else if negative {
            // -(quotient + fraction) sits strictly between -(quotient + 1) and -quotient.
            GridPosition::Between {
                floor: i64::try_from(-(quotient + 1)).map_err(|_| overflow())?,
            }
        } else {
            GridPosition::Between {
                floor: i64::try_from(quotient).map_err(|_| overflow())?,
            }
        }
    };

    Ok(position)
}

/// [`locate_on_grid`] for an already-typed value.
pub fn locate_owned_value_on_grid(value: &PdbOwnedValue, scale: i16) -> Result<GridPosition> {
    locate_on_grid(&owned_value_decimal_string(value)?, scale)
}

/// Whether a query literal is exactly representable at the field's scale.
///
/// An equality term against a literal that is not on the grid can never match a stored value,
/// so callers turn it into an empty query rather than rounding it onto a neighbour.
///
/// Literals that are not ordinary decimals — `NaN` and the infinities, which PostgreSQL accepts
/// as `numeric` values — report `true`. They have no place on the grid, and the point of the
/// grid check is only to catch a literal that was silently rounded onto a neighbour. Saying
/// `true` here leaves them to the existing scaling path, which has dedicated representations
/// for them, so their behaviour is unchanged.
pub fn literal_is_on_grid(value: &PdbOwnedValue, scale: i16) -> Result<bool> {
    match locate_owned_value_on_grid(value, scale) {
        Ok(position) => Ok(position.is_exact()),
        Err(_) => Ok(true),
    }
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

/// Scale a numeric bound for Numeric64 storage, preserving PostgreSQL comparison semantics.
///
/// PostgreSQL does not apply the column typmod to a comparison operand, so a literal with more
/// fractional digits than the field scale is compared exactly. When such a literal falls
/// between two grid points, the bound moves to the neighbouring grid point that keeps the same
/// set of matching values, and becomes inclusive:
///
/// * an upper bound (`<` or `<=`) becomes `<=` the grid point below the literal,
/// * a lower bound (`>` or `>=`) becomes `>=` the grid point above it.
///
/// A literal that is exactly on the grid keeps its own inclusive or exclusive bound.
pub fn scale_numeric_bound(
    bound: Bound<PdbOwnedValue>,
    scale: i16,
    side: BoundSide,
) -> Result<Bound<PdbOwnedValue>> {
    let (value, inclusive) = match bound {
        Bound::Unbounded => return Ok(Bound::Unbounded),
        Bound::Included(v) => (v, true),
        Bound::Excluded(v) => (v, false),
    };

    match locate_owned_value_on_grid(&value, scale) {
        Ok(GridPosition::Exact(scaled)) => Ok(if inclusive {
            Bound::Included(PdbOwnedValue::I64(scaled))
        } else {
            Bound::Excluded(PdbOwnedValue::I64(scaled))
        }),
        Ok(position @ GridPosition::Between { .. }) => {
            let grid = match side {
                BoundSide::Lower => position.ceil(),
                BoundSide::Upper => position.floor(),
            };
            Ok(Bound::Included(PdbOwnedValue::I64(grid)))
        }
        // Not an ordinary decimal — `NaN`, the infinities, or a value out of `i64` range. The
        // grid has nothing to say about these, so hand them back to the path that handled them
        // before this function learned about grids, and keep its behaviour exactly.
        Err(_) => {
            let scaled = scale_owned_value(value, scale)?;
            Ok(if inclusive {
                Bound::Included(scaled)
            } else {
                Bound::Excluded(scaled)
            })
        }
    }
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

    fn s(v: &str) -> PdbOwnedValue {
        PdbOwnedValue::Str(v.to_string())
    }

    #[test]
    fn test_locate_on_grid_exact_values() {
        // A literal with no more fractional digits than the scale lands on the grid.
        assert_eq!(
            locate_on_grid("12.34", 2).unwrap(),
            GridPosition::Exact(1234)
        );
        assert_eq!(
            locate_on_grid("12.3", 2).unwrap(),
            GridPosition::Exact(1230)
        );
        assert_eq!(locate_on_grid("12", 2).unwrap(), GridPosition::Exact(1200));
        assert_eq!(
            locate_on_grid("-12.34", 2).unwrap(),
            GridPosition::Exact(-1234)
        );
        assert_eq!(locate_on_grid("0", 2).unwrap(), GridPosition::Exact(0));
        assert_eq!(locate_on_grid("-0.00", 2).unwrap(), GridPosition::Exact(0));
        // Trailing zeros beyond the scale are still exact.
        assert_eq!(
            locate_on_grid("12.3400", 2).unwrap(),
            GridPosition::Exact(1234)
        );
    }

    #[test]
    fn test_locate_on_grid_between_values() {
        // 12.345 sits strictly between the grid points 12.34 and 12.35.
        assert_eq!(
            locate_on_grid("12.345", 2).unwrap(),
            GridPosition::Between { floor: 1234 }
        );
        // The floor of a negative literal is the more negative neighbour.
        assert_eq!(
            locate_on_grid("-12.345", 2).unwrap(),
            GridPosition::Between { floor: -1235 }
        );
        assert_eq!(
            locate_on_grid("0.001", 2).unwrap(),
            GridPosition::Between { floor: 0 }
        );
        assert_eq!(
            locate_on_grid("-0.001", 2).unwrap(),
            GridPosition::Between { floor: -1 }
        );
    }

    #[test]
    fn test_locate_on_grid_zero_and_negative_scales() {
        // Scale 0: the grid is the integers.
        assert_eq!(locate_on_grid("12", 0).unwrap(), GridPosition::Exact(12));
        assert_eq!(
            locate_on_grid("12.5", 0).unwrap(),
            GridPosition::Between { floor: 12 }
        );
        assert_eq!(
            locate_on_grid("-12.5", 0).unwrap(),
            GridPosition::Between { floor: -13 }
        );

        // Negative scale: the grid is multiples of 10^|scale|, so whole numbers can be
        // off-grid too.
        assert_eq!(locate_on_grid("1200", -2).unwrap(), GridPosition::Exact(12));
        assert_eq!(
            locate_on_grid("1234", -2).unwrap(),
            GridPosition::Between { floor: 12 }
        );
        assert_eq!(
            locate_on_grid("-1234", -2).unwrap(),
            GridPosition::Between { floor: -13 }
        );
    }

    #[test]
    fn test_locate_on_grid_exponent_notation() {
        assert_eq!(
            locate_on_grid("1.5e3", 0).unwrap(),
            GridPosition::Exact(1500)
        );
        assert_eq!(
            locate_on_grid("1E2", 2).unwrap(),
            GridPosition::Exact(10000)
        );
        assert_eq!(
            locate_on_grid("1e-3", 2).unwrap(),
            GridPosition::Between { floor: 0 }
        );
    }

    #[test]
    fn test_locate_on_grid_rejects_garbage() {
        assert!(locate_on_grid("", 2).is_err());
        assert!(locate_on_grid("abc", 2).is_err());
        assert!(locate_on_grid("1.2.3", 2).is_err());
        // Beyond i64 once scaled.
        assert!(locate_on_grid("99999999999999999999", 2).is_err());
    }

    #[test]
    fn test_grid_position_floor_and_ceil() {
        assert_eq!(GridPosition::Exact(7).floor(), 7);
        assert_eq!(GridPosition::Exact(7).ceil(), 7);
        assert!(GridPosition::Exact(7).is_exact());

        assert_eq!(GridPosition::Between { floor: 7 }.floor(), 7);
        assert_eq!(GridPosition::Between { floor: 7 }.ceil(), 8);
        assert!(!GridPosition::Between { floor: 7 }.is_exact());
    }

    #[test]
    fn test_literal_is_on_grid() {
        assert!(literal_is_on_grid(&s("12.34"), 2).unwrap());
        assert!(!literal_is_on_grid(&s("12.345"), 2).unwrap());
        assert!(literal_is_on_grid(&PdbOwnedValue::I64(12), 2).unwrap());
        // With a negative scale even an integer can miss the grid.
        assert!(!literal_is_on_grid(&PdbOwnedValue::I64(1234), -2).unwrap());
        assert!(literal_is_on_grid(&PdbOwnedValue::I64(1200), -2).unwrap());
    }

    #[test]
    fn test_scale_numeric_bound_on_grid_keeps_its_bound() {
        // An exactly representable literal keeps whichever bound the operator asked for.
        assert_eq!(
            scale_numeric_bound(Bound::Excluded(s("12.34")), 2, BoundSide::Upper).unwrap(),
            Bound::Excluded(PdbOwnedValue::I64(1234))
        );
        assert_eq!(
            scale_numeric_bound(Bound::Included(s("12.34")), 2, BoundSide::Upper).unwrap(),
            Bound::Included(PdbOwnedValue::I64(1234))
        );
        assert_eq!(
            scale_numeric_bound(Bound::Excluded(s("12.34")), 2, BoundSide::Lower).unwrap(),
            Bound::Excluded(PdbOwnedValue::I64(1234))
        );
        assert_eq!(
            scale_numeric_bound(Bound::Included(s("12.34")), 2, BoundSide::Lower).unwrap(),
            Bound::Included(PdbOwnedValue::I64(1234))
        );
    }

    #[test]
    fn test_scale_numeric_bound_off_grid_moves_outwards() {
        // `< 12.345` and `<= 12.345` both mean `<= 12.34` on a scale-2 grid.
        assert_eq!(
            scale_numeric_bound(Bound::Excluded(s("12.345")), 2, BoundSide::Upper).unwrap(),
            Bound::Included(PdbOwnedValue::I64(1234))
        );
        assert_eq!(
            scale_numeric_bound(Bound::Included(s("12.345")), 2, BoundSide::Upper).unwrap(),
            Bound::Included(PdbOwnedValue::I64(1234))
        );
        // `> 12.345` and `>= 12.345` both mean `>= 12.35`.
        assert_eq!(
            scale_numeric_bound(Bound::Excluded(s("12.345")), 2, BoundSide::Lower).unwrap(),
            Bound::Included(PdbOwnedValue::I64(1235))
        );
        assert_eq!(
            scale_numeric_bound(Bound::Included(s("12.345")), 2, BoundSide::Lower).unwrap(),
            Bound::Included(PdbOwnedValue::I64(1235))
        );
    }

    #[test]
    fn test_scale_numeric_bound_negative_values() {
        // -12.345 sits between -12.35 and -12.34.
        assert_eq!(
            scale_numeric_bound(Bound::Included(s("-12.345")), 2, BoundSide::Upper).unwrap(),
            Bound::Included(PdbOwnedValue::I64(-1235))
        );
        assert_eq!(
            scale_numeric_bound(Bound::Included(s("-12.345")), 2, BoundSide::Lower).unwrap(),
            Bound::Included(PdbOwnedValue::I64(-1234))
        );
    }

    #[test]
    fn test_scale_numeric_bound_zero_and_negative_scales() {
        assert_eq!(
            scale_numeric_bound(Bound::Included(s("12.5")), 0, BoundSide::Upper).unwrap(),
            Bound::Included(PdbOwnedValue::I64(12))
        );
        assert_eq!(
            scale_numeric_bound(Bound::Included(s("12.5")), 0, BoundSide::Lower).unwrap(),
            Bound::Included(PdbOwnedValue::I64(13))
        );
        assert_eq!(
            scale_numeric_bound(Bound::Included(s("1234")), -2, BoundSide::Upper).unwrap(),
            Bound::Included(PdbOwnedValue::I64(12))
        );
        assert_eq!(
            scale_numeric_bound(Bound::Included(s("1234")), -2, BoundSide::Lower).unwrap(),
            Bound::Included(PdbOwnedValue::I64(13))
        );
    }

    #[test]
    fn test_scale_numeric_bound_unbounded() {
        assert_eq!(
            scale_numeric_bound(Bound::Unbounded, 2, BoundSide::Lower).unwrap(),
            Bound::Unbounded
        );
        assert_eq!(
            scale_numeric_bound(Bound::Unbounded, 2, BoundSide::Upper).unwrap(),
            Bound::Unbounded
        );
    }

    #[test]
    fn test_special_values_fall_back_to_the_previous_path() {
        // `NaN` and the infinities are valid PostgreSQL numerics but are not points on any
        // grid, so the parser rejects them outright.
        assert!(locate_on_grid("NaN", 2).is_err());
        assert!(locate_on_grid("Infinity", 2).is_err());

        // The callers must not turn that into a failed query. `literal_is_on_grid` reports
        // `true` so the term is built by the existing path rather than dropped.
        assert!(literal_is_on_grid(&s("NaN"), 2).unwrap());
        assert!(literal_is_on_grid(&s("Infinity"), 2).unwrap());
        assert!(literal_is_on_grid(&s("-Infinity"), 2).unwrap());

        // And a bound carrying one of them scales exactly as it did before.
        for literal in ["NaN", "Infinity", "-Infinity"] {
            let via_bound = scale_numeric_bound(Bound::Included(s(literal)), 2, BoundSide::Upper);
            let via_previous_path = scale_owned_value(s(literal), 2);
            assert_eq!(
                via_bound.is_ok(),
                via_previous_path.is_ok(),
                "{literal} changed whether the conversion succeeds"
            );
            if let (Ok(Bound::Included(from_bound)), Ok(direct)) = (via_bound, via_previous_path) {
                assert_eq!(from_bound, direct, "{literal} scaled to a different value");
            }
        }
    }

    #[test]
    fn test_scale_i64_still_rounds_for_stored_values() {
        // The storage path is unchanged: encoding a column value rounds to the column scale.
        assert_eq!(scale_i64("12.345", 2).unwrap(), 1235);
        // The query path must not.
        assert_eq!(
            locate_on_grid("12.345", 2).unwrap(),
            GridPosition::Between { floor: 1234 }
        );
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
}
