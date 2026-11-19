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
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

pub use common::postgres::pdb_owned_value::*;

use arrow_array::Array;
use arrow_schema::DataType;
use datafusion::common::ScalarValue;

use crate::postgres::datetime::PostgresDateTime;
use crate::schema::SearchFieldType;

/// DataFusion [`ScalarValue`] and Arrow array conversion methods for [`PdbOwnedValue`].
pub trait PdbOwnedValueScalarExt: Sized {
    /// The value of an Arrow cell, for a column whose field type is not at hand:
    /// integers widen to `I64` or `U64`, floats to `F64`, a microsecond timestamp
    /// becomes a `Date`, and a `Date32` day count stays an `I64`. `None` for an
    /// Arrow type with no variant here.
    fn from_arrow(array: &dyn Array, row: usize) -> Option<Self>;

    /// Converts a DataFusion [`ScalarValue`] into a [`PdbOwnedValue`], guided by the
    /// field's [`SearchFieldType`].
    fn from_scalar(scalar: &ScalarValue, field_type: &SearchFieldType) -> Option<Self>;

    /// Converts a DataFusion execution [`ScalarValue`] (Arrow column representation)
    /// into a [`PdbOwnedValue`] ready for Tantivy term construction.
    ///
    /// Unlike [`Self::from_scalar`], which treats `Int64` as a logical SQL value and
    /// may apply `Numeric64` scale, this path assumes values already match the field's
    /// Arrow encoding. Hash-join dynamic `InList` members are built from join-key
    /// arrays, so `Numeric64` arrives as an already-scaled `Int64`.
    fn from_execution_scalar(scalar: &ScalarValue, field_type: &SearchFieldType) -> Option<Self>;

    /// Converts this [`PdbOwnedValue`] into a DataFusion [`ScalarValue`] representation matching
    /// the column's Arrow [`DataType`].
    ///
    /// **Contract**: Returns `Some` only for exact, lossless conversions. Returns `None` for
    /// NULLs, out-of-range bounds, precision-losing casts, or unsupported conversions.
    /// `RangeSplitPoints::to_datafusion` depends on this exactness guarantee.
    fn to_scalar(&self, data_type: &DataType) -> Option<ScalarValue>;
}

impl PdbOwnedValueScalarExt for PdbOwnedValue {
    fn from_arrow(array: &dyn Array, row: usize) -> Option<Self> {
        if array.is_null(row) {
            return Some(Self::Null);
        }
        Some(match ScalarValue::try_from_array(array, row).ok()? {
            ScalarValue::Utf8(Some(v))
            | ScalarValue::LargeUtf8(Some(v))
            | ScalarValue::Utf8View(Some(v)) => Self::Str(v),
            ScalarValue::Int8(Some(v)) => Self::I64(v as i64),
            ScalarValue::Int16(Some(v)) => Self::I64(v as i64),
            ScalarValue::Int32(Some(v)) => Self::I64(v as i64),
            ScalarValue::Int64(Some(v)) => Self::I64(v),
            ScalarValue::UInt8(Some(v)) => Self::U64(v as u64),
            ScalarValue::UInt16(Some(v)) => Self::U64(v as u64),
            ScalarValue::UInt32(Some(v)) => Self::U64(v as u64),
            ScalarValue::UInt64(Some(v)) => Self::U64(v),
            ScalarValue::Float32(Some(v)) => Self::F64(v as f64),
            ScalarValue::Float64(Some(v)) => Self::F64(v),
            ScalarValue::Boolean(Some(v)) => Self::Bool(v),
            // A value outside the timestamp range keeps its raw microseconds.
            ScalarValue::TimestampMicrosecond(Some(v), _) => PostgresDateTime::try_from_raw(v)
                .map(Self::Date)
                .unwrap_or(Self::I64(v)),
            ScalarValue::Date32(Some(v)) => Self::I64(v as i64),
            ScalarValue::Binary(Some(v))
            | ScalarValue::LargeBinary(Some(v))
            | ScalarValue::BinaryView(Some(v)) => Self::Bytes(v),
            _ => return None,
        })
    }

    fn from_scalar(scalar: &ScalarValue, field_type: &SearchFieldType) -> Option<Self> {
        match (scalar, field_type) {
            // Integer types (I64)
            (ScalarValue::Int8(Some(v)), SearchFieldType::I64(_)) => {
                Some(PdbOwnedValue::I64(*v as i64))
            }
            (ScalarValue::Int16(Some(v)), SearchFieldType::I64(_)) => {
                Some(PdbOwnedValue::I64(*v as i64))
            }
            (ScalarValue::Int32(Some(v)), SearchFieldType::I64(_)) => {
                Some(PdbOwnedValue::I64(*v as i64))
            }
            (ScalarValue::Int64(Some(v)), SearchFieldType::I64(_)) => Some(PdbOwnedValue::I64(*v)),

            // Unsigned integer types (U64)
            (ScalarValue::UInt8(Some(v)), SearchFieldType::U64(_)) => {
                Some(PdbOwnedValue::U64(*v as u64))
            }
            (ScalarValue::UInt16(Some(v)), SearchFieldType::U64(_)) => {
                Some(PdbOwnedValue::U64(*v as u64))
            }
            (ScalarValue::UInt32(Some(v)), SearchFieldType::U64(_)) => {
                Some(PdbOwnedValue::U64(*v as u64))
            }
            (ScalarValue::UInt64(Some(v)), SearchFieldType::U64(_)) => Some(PdbOwnedValue::U64(*v)),

            // Cross-type integer conversions
            (ScalarValue::Int8(Some(v)), SearchFieldType::U64(_)) if *v >= 0 => {
                Some(PdbOwnedValue::U64(*v as u64))
            }
            (ScalarValue::Int16(Some(v)), SearchFieldType::U64(_)) if *v >= 0 => {
                Some(PdbOwnedValue::U64(*v as u64))
            }
            (ScalarValue::Int32(Some(v)), SearchFieldType::U64(_)) if *v >= 0 => {
                Some(PdbOwnedValue::U64(*v as u64))
            }
            (ScalarValue::Int64(Some(v)), SearchFieldType::U64(_)) if *v >= 0 => {
                Some(PdbOwnedValue::U64(*v as u64))
            }

            // Float types (F64)
            (ScalarValue::Float32(Some(v)), SearchFieldType::F64(_)) => {
                Some(PdbOwnedValue::F64(*v as f64))
            }
            (ScalarValue::Float64(Some(v)), SearchFieldType::F64(_)) => {
                Some(PdbOwnedValue::F64(*v))
            }

            // Integer to float conversion
            (ScalarValue::Int64(Some(v)), SearchFieldType::F64(_)) => {
                Some(PdbOwnedValue::F64(*v as f64))
            }
            (ScalarValue::Int32(Some(v)), SearchFieldType::F64(_)) => {
                Some(PdbOwnedValue::F64(*v as f64))
            }

            // Boolean
            (ScalarValue::Boolean(Some(v)), SearchFieldType::Bool(_)) => {
                Some(PdbOwnedValue::Bool(*v))
            }

            // String/Text types. `Uuid` indexes the whole value as one text term and
            // `Tokenized` is a text column carrying an explicit tokenizer, so both take
            // the same `Str` representation as `Text`.
            //
            // The other Utf8View-backed types are deliberately absent. `Inet` stores as a
            // tantivy IP field, and `Json`, `Range` and `Ltree` carry path, bound or facet
            // encoding. A bare string cannot reproduce any of those, and a term built the
            // wrong way silently drops the rows it should match.
            (
                ScalarValue::Utf8(Some(v))
                | ScalarValue::LargeUtf8(Some(v))
                | ScalarValue::Utf8View(Some(v)),
                SearchFieldType::Text(_)
                | SearchFieldType::Tokenized(..)
                | SearchFieldType::Uuid(_),
            ) => Some(PdbOwnedValue::Str(v.clone())),

            // Numeric64 (scaled integers)
            (ScalarValue::Int64(Some(v)), SearchFieldType::Numeric64(_, scale)) => {
                let multiplier = 10i64.pow(*scale as u32);
                Some(PdbOwnedValue::I64(v * multiplier))
            }
            (ScalarValue::Float64(Some(v)), SearchFieldType::Numeric64(_, scale)) => {
                let multiplier = 10f64.powi(*scale as i32);
                Some(PdbOwnedValue::I64((v * multiplier).round() as i64))
            }

            _ => None,
        }
    }

    fn from_execution_scalar(scalar: &ScalarValue, field_type: &SearchFieldType) -> Option<Self> {
        match (scalar, field_type) {
            (ScalarValue::Int64(Some(v)), SearchFieldType::Numeric64(_, _)) => {
                Some(PdbOwnedValue::I64(*v))
            }
            _ => Self::from_scalar(scalar, field_type),
        }
    }

    fn to_scalar(&self, data_type: &DataType) -> Option<ScalarValue> {
        use arrow_schema::TimeUnit;

        match (data_type, self) {
            (DataType::Int64, PdbOwnedValue::I64(v)) => Some(ScalarValue::Int64(Some(*v))),
            (DataType::Int32, PdbOwnedValue::I64(v))
                if *v >= i32::MIN as i64 && *v <= i32::MAX as i64 =>
            {
                Some(ScalarValue::Int32(Some(*v as i32)))
            }
            (DataType::Int16, PdbOwnedValue::I64(v))
                if *v >= i16::MIN as i64 && *v <= i16::MAX as i64 =>
            {
                Some(ScalarValue::Int16(Some(*v as i16)))
            }
            (DataType::Int8, PdbOwnedValue::I64(v))
                if *v >= i8::MIN as i64 && *v <= i8::MAX as i64 =>
            {
                Some(ScalarValue::Int8(Some(*v as i8)))
            }
            (DataType::UInt64, PdbOwnedValue::U64(v)) => Some(ScalarValue::UInt64(Some(*v))),
            (DataType::UInt32, PdbOwnedValue::U64(v)) if *v <= u32::MAX as u64 => {
                Some(ScalarValue::UInt32(Some(*v as u32)))
            }
            (DataType::UInt16, PdbOwnedValue::U64(v)) if *v <= u16::MAX as u64 => {
                Some(ScalarValue::UInt16(Some(*v as u16)))
            }
            (DataType::UInt8, PdbOwnedValue::U64(v)) if *v <= u8::MAX as u64 => {
                Some(ScalarValue::UInt8(Some(*v as u8)))
            }
            // The sampler classifies integer fields using Tantivy's FFType. An Int64 field with
            // only non-negative values in a sample may be extracted as U64 (and vice-versa).
            // Allow exact cross-conversions.
            (DataType::Int64, PdbOwnedValue::U64(v)) if *v <= i64::MAX as u64 => {
                Some(ScalarValue::Int64(Some(*v as i64)))
            }
            (DataType::Int32, PdbOwnedValue::U64(v)) if *v <= i32::MAX as u64 => {
                Some(ScalarValue::Int32(Some(*v as i32)))
            }
            (DataType::UInt64, PdbOwnedValue::I64(v)) if *v >= 0 => {
                Some(ScalarValue::UInt64(Some(*v as u64)))
            }
            (DataType::UInt32, PdbOwnedValue::I64(v)) if *v >= 0 && *v <= u32::MAX as i64 => {
                Some(ScalarValue::UInt32(Some(*v as u32)))
            }
            (DataType::Float64, PdbOwnedValue::F64(v)) => Some(ScalarValue::Float64(Some(*v))),
            (DataType::Float32, PdbOwnedValue::F64(v)) if (*v as f32) as f64 == *v => {
                Some(ScalarValue::Float32(Some(*v as f32)))
            }
            (DataType::Boolean, PdbOwnedValue::Bool(v)) => Some(ScalarValue::Boolean(Some(*v))),
            (DataType::BinaryView, PdbOwnedValue::Bytes(v)) => {
                Some(ScalarValue::BinaryView(Some(v.clone())))
            }
            (DataType::Utf8View, PdbOwnedValue::Str(v)) => {
                Some(ScalarValue::Utf8View(Some(v.clone())))
            }
            (DataType::Utf8, PdbOwnedValue::Str(v)) => Some(ScalarValue::Utf8(Some(v.clone()))),
            // Fast fields store Timestamp/Date values as i64 microseconds.
            (DataType::Timestamp(TimeUnit::Microsecond, None), PdbOwnedValue::I64(v)) => {
                Some(ScalarValue::TimestampMicrosecond(Some(*v), None))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pgrx::pg_sys;

    #[test]
    fn from_execution_scalar_preserves_already_scaled_numeric64() {
        // 104.25 stored as Int64(10425) for numeric(..., 2). Execution values
        // must not be scaled again; logical from_scalar still multiplies.
        let field = SearchFieldType::Numeric64(pg_sys::InvalidOid, 2);
        let execution = ScalarValue::Int64(Some(10_425));

        assert_eq!(
            PdbOwnedValue::from_execution_scalar(&execution, &field),
            Some(PdbOwnedValue::I64(10_425))
        );
        assert_eq!(
            PdbOwnedValue::from_scalar(&execution, &field),
            Some(PdbOwnedValue::I64(1_042_500))
        );
    }

    #[test]
    fn from_execution_scalar_delegates_non_numeric64() {
        let field = SearchFieldType::I64(pg_sys::InvalidOid);
        let value = ScalarValue::Int64(Some(42));
        assert_eq!(
            PdbOwnedValue::from_execution_scalar(&value, &field),
            PdbOwnedValue::from_scalar(&value, &field)
        );

        let numeric = SearchFieldType::Numeric64(pg_sys::InvalidOid, 2);
        let logical_float = ScalarValue::Float64(Some(104.25));
        assert_eq!(
            PdbOwnedValue::from_execution_scalar(&logical_float, &numeric),
            PdbOwnedValue::from_scalar(&logical_float, &numeric)
        );
        assert_eq!(
            PdbOwnedValue::from_execution_scalar(&logical_float, &numeric),
            Some(PdbOwnedValue::I64(10_425))
        );
    }

    #[test]
    fn from_execution_scalar_handles_negative_and_zero_scale() {
        let scale2 = SearchFieldType::Numeric64(pg_sys::InvalidOid, 2);
        assert_eq!(
            PdbOwnedValue::from_execution_scalar(&ScalarValue::Int64(Some(-10_425)), &scale2),
            Some(PdbOwnedValue::I64(-10_425))
        );

        let scale0 = SearchFieldType::Numeric64(pg_sys::InvalidOid, 0);
        assert_eq!(
            PdbOwnedValue::from_execution_scalar(&ScalarValue::Int64(Some(104)), &scale0),
            Some(PdbOwnedValue::I64(104))
        );
        // scale 0: from_scalar multiplier is 1, so both paths agree.
        assert_eq!(
            PdbOwnedValue::from_execution_scalar(&ScalarValue::Int64(Some(104)), &scale0),
            PdbOwnedValue::from_scalar(&ScalarValue::Int64(Some(104)), &scale0)
        );
    }
}
