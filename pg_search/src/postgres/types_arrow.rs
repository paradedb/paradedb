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

use crate::postgres::catalog::{facet_encoded_str_to_ltree_text, is_citext_oid, is_ltree_oid};
use crate::postgres::datetime::MICROSECONDS_IN_SECOND;

use arrow_array::cast::AsArray;
use arrow_array::Array;
use arrow_schema::DataType;
use decimal_bytes::{Decimal, Decimal64NoScale};
use pgrx::pg_sys;
use pgrx::{datum, IntoDatum, PgBuiltInOids, PgOid};

/// Convert an Arrow array slice into a Postgres `Datum`.
///
/// This effectively inlines `TantivyValue::try_into_datum` in order to avoid creating both
/// `OwnedValue` and `TantivyValue` wrappers around primitives (but particularly around strings).
///
/// The input Arrow arrays have types corresponding to "widened" storage types (see
/// [`WhichFastField`](crate::index::fast_fields_helper::WhichFastField)). This function is
/// responsible for converting those widened types (e.g. `Utf8View`) back into specific
/// Postgres OIDs where applicable.
///
/// The `numeric_scale` parameter is used for `Numeric64` and `NumericBytes` fields.
/// For `Numeric64`, the scale is used to convert scaled i64 values back to NUMERIC.
/// For `NumericBytes`, the scale is used to format the output with proper trailing zeros.
pub fn arrow_array_to_datum(
    array: &dyn Array,
    index: usize,
    oid: PgOid,
    numeric_scale: Option<i16>,
) -> Result<Option<pg_sys::Datum>, String> {
    if array.is_null(index) {
        return Ok(None);
    }

    // This switch statement primarily needs to support types which are produced by
    // `WhichFastField`/`FFType` (including Score/TableOid which are f32/u32). We widen any
    // narrower user types into those types. See the method docs about widening.
    let datum = match array.data_type() {
        DataType::Utf8View => {
            let arr = array.as_string_view();
            let s = arr.value(index);
            match &oid {
                PgOid::BuiltIn(PgBuiltInOids::TEXTOID)
                | PgOid::BuiltIn(PgBuiltInOids::VARCHAROID) => s.into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::JSONOID) => {
                    pgrx::Json(serde_json::Value::String(s.to_string())).into_datum()
                }
                PgOid::BuiltIn(PgBuiltInOids::JSONBOID) => {
                    datum::JsonB(serde_json::Value::String(s.to_string())).into_datum()
                }
                PgOid::BuiltIn(PgBuiltInOids::UUIDOID) => {
                    let uuid = uuid::Uuid::parse_str(s)
                        .map_err(|e| format!("Failed to decode as UUID: {e}"))?;
                    datum::Uuid::from_slice(uuid.as_bytes())?.into_datum()
                }
                PgOid::BuiltIn(PgBuiltInOids::INETOID) => {
                    datum::Inet::from(s.to_string()).into_datum()
                }

                PgOid::Custom(custom) => {
                    if is_citext_oid(*custom) {
                        s.into_datum()
                    } else if is_ltree_oid(*custom) {
                        // Facet fast fields store the value as a null-byte-separated string.
                        // Convert back to dot-separated ltree path using the shared helper.
                        let ltree_text = facet_encoded_str_to_ltree_text(s);
                        unsafe {
                            let mut typinput: pg_sys::Oid = pg_sys::InvalidOid;
                            let mut typioparam: pg_sys::Oid = pg_sys::InvalidOid;
                            pg_sys::getTypeInputInfo(*custom, &mut typinput, &mut typioparam);
                            let cstring = std::ffi::CString::new(ltree_text)
                                .map_err(|e| format!("Failed to create CString for ltree: {e}"))?;
                            let datum = pg_sys::OidInputFunctionCall(
                                typinput,
                                cstring.as_ptr() as *mut std::ffi::c_char,
                                typioparam,
                                -1,
                            );
                            Some(datum)
                        }
                    } else {
                        return Err(format!("Unsupported OID for Utf8 Arrow type: {oid:?}"));
                    }
                }
                _ => return Err(format!("Unsupported OID for Utf8 Arrow type: {oid:?}")),
            }
        }
        DataType::Utf8 => {
            let arr = array
                .as_any()
                .downcast_ref::<arrow_array::StringArray>()
                .unwrap();
            let s = arr.value(index);
            match &oid {
                PgOid::BuiltIn(PgBuiltInOids::TEXTOID)
                | PgOid::BuiltIn(PgBuiltInOids::VARCHAROID) => s.into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::JSONOID) => {
                    pgrx::Json(serde_json::Value::String(s.to_string())).into_datum()
                }
                PgOid::BuiltIn(PgBuiltInOids::JSONBOID) => {
                    datum::JsonB(serde_json::Value::String(s.to_string())).into_datum()
                }
                _ => return Err(format!("Unsupported OID for Utf8 Arrow type: {oid:?}")),
            }
        }
        DataType::LargeUtf8 => {
            let arr = array
                .as_any()
                .downcast_ref::<arrow_array::LargeStringArray>()
                .unwrap();
            let s = arr.value(index);
            match &oid {
                PgOid::BuiltIn(PgBuiltInOids::TEXTOID)
                | PgOid::BuiltIn(PgBuiltInOids::VARCHAROID) => s.into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::JSONOID) => {
                    pgrx::Json(serde_json::Value::String(s.to_string())).into_datum()
                }
                PgOid::BuiltIn(PgBuiltInOids::JSONBOID) => {
                    datum::JsonB(serde_json::Value::String(s.to_string())).into_datum()
                }
                _ => return Err(format!("Unsupported OID for LargeUtf8 Arrow type: {oid:?}")),
            }
        }
        DataType::BinaryView => {
            let arr = array.as_binary_view();
            let bytes = arr.value(index);
            match &oid {
                PgOid::BuiltIn(PgBuiltInOids::BYTEAOID) => bytes.into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::NUMERICOID) => {
                    // Bytes are stored as Decimal::as_bytes() - convert back to AnyNumeric
                    // via string representation since AnyNumeric implements FromStr
                    let decimal = Decimal::from_bytes(bytes)
                        .map_err(|e| format!("Failed to decode bytes as Decimal: {e:?}"))?;

                    // Format decimal with proper scale to preserve trailing zeros
                    let decimal_str = if let Some(scale) = numeric_scale {
                        decimal.to_string_with_scale(scale as i32)
                    } else {
                        decimal.to_string()
                    };

                    decimal_str
                        .parse::<pgrx::AnyNumeric>()
                        .map_err(|e| format!("Failed to parse Decimal string as AnyNumeric: {e}"))?
                        .into_datum()
                }
                _ => {
                    return Err(format!(
                        "Unsupported OID for BinaryView Arrow type: {oid:?}"
                    ))
                }
            }
        }
        DataType::Binary => {
            let arr = array
                .as_any()
                .downcast_ref::<arrow_array::BinaryArray>()
                .unwrap();
            let val = arr.value(index);
            match &oid {
                PgOid::BuiltIn(PgBuiltInOids::BYTEAOID) => val.to_vec().into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::UUIDOID) => {
                    let uuid =
                        pgrx::Uuid::from_bytes(val.try_into().map_err(|_| "Invalid UUID bytes")?);
                    uuid.into_datum()
                }
                _ => return Err(format!("Unsupported OID for Binary Arrow type: {oid:?}")),
            }
        }
        DataType::FixedSizeBinary(16) => {
            let arr = array
                .as_any()
                .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                .unwrap();
            let val = arr.value(index);
            match &oid {
                PgOid::BuiltIn(PgBuiltInOids::UUIDOID) => {
                    let uuid =
                        pgrx::Uuid::from_bytes(val.try_into().map_err(|_| "Invalid UUID bytes")?);
                    uuid.into_datum()
                }
                _ => {
                    return Err(format!(
                        "Unsupported OID for FixedSizeBinary(16) Arrow type: {oid:?}"
                    ))
                }
            }
        }
        DataType::UInt64 => {
            let arr = array.as_primitive::<arrow_array::types::UInt64Type>();
            let val = arr.value(index);
            match &oid {
                PgOid::BuiltIn(PgBuiltInOids::INT8OID) => (val as i64).into_datum(), // Convert u64 to i64 for INT8OID
                PgOid::BuiltIn(PgBuiltInOids::FLOAT8OID) => (val as f64).into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::OIDOID) => {
                    pgrx::pg_sys::Oid::from(val as u32).into_datum()
                } // Cast u64 to u32 for OID
                PgOid::BuiltIn(PgBuiltInOids::NUMERICOID) => {
                    let numeric_str = val.to_string();
                    numeric_str
                        .parse::<pgrx::AnyNumeric>()
                        .map_err(|e| format!("Failed to parse u64 string as AnyNumeric: {e}"))?
                        .into_datum()
                }
                // Consider other potential integer OIDs (INT2OID, INT4OID) if overflow is handled or guaranteed not to occur.
                _ => return Err(format!("Unsupported OID for UInt64 Arrow type: {oid:?}")),
            }
        }
        DataType::UInt32 => {
            let arr = array.as_primitive::<arrow_array::types::UInt32Type>();
            let val = arr.value(index);
            match &oid {
                PgOid::BuiltIn(PgBuiltInOids::OIDOID) => pgrx::pg_sys::Oid::from(val).into_datum(),
                _ => return Err(format!("Unsupported OID for UInt32 Arrow type: {oid:?}")),
            }
        }
        DataType::Int64 => {
            let arr = array.as_primitive::<arrow_array::types::Int64Type>();
            let val = arr.value(index);
            match &oid {
                PgOid::BuiltIn(PgBuiltInOids::INT8OID) => val.into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::INT4OID) => (val as i32).into_datum(), // Cast i64 to i32
                PgOid::BuiltIn(PgBuiltInOids::INT2OID) => (val as i16).into_datum(), // Cast i64 to i16
                PgOid::BuiltIn(PgBuiltInOids::FLOAT8OID) => (val as f64).into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::FLOAT4OID) => (val as f32).into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::NUMERICOID) => {
                    // Numeric64: convert scaled i64 back to NUMERIC
                    if let Some(scale) = numeric_scale {
                        let numeric_str =
                            Decimal64NoScale::from_raw(val).to_string_with_scale(scale as i32);
                        numeric_str
                            .parse::<pgrx::AnyNumeric>()
                            .map_err(|e| format!("Failed to parse scaled i64 as AnyNumeric: {e}"))?
                            .into_datum()
                    } else {
                        // Without scale, treat as raw integer value for NUMERIC
                        let numeric = pgrx::AnyNumeric::try_from(val.to_string().as_str())
                            .map_err(|e| format!("Failed to parse i64 as AnyNumeric: {e}"))?;
                        numeric.into_datum()
                    }
                }
                _ => {
                    if let Some(res) = try_convert_timestamp_nanos_to_datum(val, &oid) {
                        res?
                    } else {
                        return Err(format!("Unsupported OID for Int64 Arrow type: {oid:?}"));
                    }
                }
            }
        }
        DataType::Int32 => {
            let arr = array.as_primitive::<arrow_array::types::Int32Type>();
            let val = arr.value(index);
            match &oid {
                PgOid::BuiltIn(PgBuiltInOids::INT8OID) => (val as i64).into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::INT4OID) => val.into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::INT2OID) => (val as i16).into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::FLOAT8OID) => (val as f64).into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::FLOAT4OID) => (val as f32).into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::NUMERICOID) => {
                    let numeric = pgrx::AnyNumeric::try_from(val.to_string().as_str())
                        .map_err(|e| format!("Failed to parse i32 as AnyNumeric: {e}"))?;
                    numeric.into_datum()
                }
                _ => return Err(format!("Unsupported OID for Int32 Arrow type: {oid:?}")),
            }
        }
        DataType::Int16 => {
            let arr = array.as_primitive::<arrow_array::types::Int16Type>();
            let val = arr.value(index);
            match &oid {
                PgOid::BuiltIn(PgBuiltInOids::INT8OID) => (val as i64).into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::INT4OID) => (val as i32).into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::INT2OID) => val.into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::FLOAT8OID) => (val as f64).into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::FLOAT4OID) => (val as f32).into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::NUMERICOID) => {
                    let numeric = pgrx::AnyNumeric::try_from(val.to_string().as_str())
                        .map_err(|e| format!("Failed to parse i16 as AnyNumeric: {e}"))?;
                    numeric.into_datum()
                }
                _ => return Err(format!("Unsupported OID for Int16 Arrow type: {oid:?}")),
            }
        }
        DataType::Float64 => {
            let arr = array.as_primitive::<arrow_array::types::Float64Type>();
            let val = arr.value(index);
            match &oid {
                PgOid::BuiltIn(PgBuiltInOids::FLOAT8OID) => val.into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::FLOAT4OID) => (val as f32).into_datum(), // Cast f64 to f32
                PgOid::BuiltIn(PgBuiltInOids::INT8OID) => (val as i64).into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::INT4OID) => (val as i32).into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::INT2OID) => (val as i16).into_datum(),
                // Legacy pre-v0.22.0 indexes stored NUMERIC fields as F64. Reading those
                // fast fields back must round-trip through `AnyNumeric` to return a NUMERIC
                // datum rather than the FLOAT8 the column-type widening would imply.
                PgOid::BuiltIn(PgBuiltInOids::NUMERICOID) => {
                    let numeric = pgrx::AnyNumeric::try_from(val)
                        .map_err(|_| "Failed to convert f64 to AnyNumeric")?;
                    numeric.into_datum()
                }
                _ => return Err(format!("Unsupported OID for Float64 Arrow type: {oid:?}")),
            }
        }
        DataType::Float32 => {
            let arr = array.as_primitive::<arrow_array::types::Float32Type>();
            let val = arr.value(index);
            match &oid {
                PgOid::BuiltIn(PgBuiltInOids::FLOAT4OID) => val.into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::FLOAT8OID) => (val as f64).into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::INT8OID) => (val as f64 as i64).into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::INT4OID) => (val as f64 as i32).into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::INT2OID) => (val as f64 as i16).into_datum(),
                // Legacy pre-v0.22.0 indexes stored NUMERIC fields as F64. Reading those
                // fast fields back must round-trip through `AnyNumeric` to return a NUMERIC
                // datum rather than the FLOAT8 the column-type widening would imply.
                PgOid::BuiltIn(PgBuiltInOids::NUMERICOID) => {
                    let numeric = pgrx::AnyNumeric::try_from(val as f64)
                        .map_err(|_| "Failed to convert f32 to AnyNumeric")?;
                    numeric.into_datum()
                }
                _ => return Err(format!("Unsupported OID for Float32 Arrow type: {oid:?}")),
            }
        }
        DataType::Decimal128(_, scale) => {
            let arr = array.as_primitive::<arrow_array::types::Decimal128Type>();
            let val = arr.value(index);
            let scale = *scale as u32;
            match &oid {
                PgOid::BuiltIn(PgBuiltInOids::NUMERICOID) => {
                    let s = if scale == 0 {
                        format!("{}", val)
                    } else {
                        let divisor = 10i128.pow(scale);
                        let whole = val / divisor;
                        let frac = (val % divisor).unsigned_abs();
                        format!("{}.{:0>width$}", whole, frac, width = scale as usize)
                    };
                    let numeric = pgrx::AnyNumeric::try_from(s.as_str())
                        .map_err(|e| format!("Failed to parse Decimal128 as AnyNumeric: {e}"))?;
                    numeric.into_datum()
                }
                PgOid::BuiltIn(PgBuiltInOids::FLOAT8OID) => {
                    let divisor = 10_f64.powi(scale as i32);
                    let f_val = val as f64 / divisor;
                    f_val.into_datum()
                }
                PgOid::BuiltIn(PgBuiltInOids::FLOAT4OID) => {
                    let divisor = 10_f64.powi(scale as i32);
                    let f_val = val as f64 / divisor;
                    (f_val as f32).into_datum()
                }
                _ => {
                    return Err(format!(
                        "Unsupported OID for Decimal128 Arrow type: {oid:?}"
                    ))
                }
            }
        }
        DataType::Boolean => {
            let arr = array.as_boolean();
            let val = arr.value(index);
            match &oid {
                PgOid::BuiltIn(PgBuiltInOids::BOOLOID) => val.into_datum(),
                _ => return Err(format!("Unsupported OID for Boolean Arrow type: {oid:?}")),
            }
        }
        DataType::Date32 => {
            let arr = array.as_primitive::<arrow_array::types::Date32Type>();
            let days = arr.value(index);
            let nanos = (days as i64)
                .checked_mul(86_400_000_000_000)
                .ok_or("Overflow calculating nanoseconds from Date32")?;
            if let Some(res) = try_convert_timestamp_nanos_to_datum(nanos, &oid) {
                res?
            } else {
                return Err(format!("Unsupported OID for Date32 Arrow type: {oid:?}"));
            }
        }
        DataType::Date64 => {
            let arr = array.as_primitive::<arrow_array::types::Date64Type>();
            let millis = arr.value(index);
            let nanos = millis
                .checked_mul(1_000_000)
                .ok_or("Overflow calculating nanoseconds from Date64")?;
            if let Some(res) = try_convert_timestamp_nanos_to_datum(nanos, &oid) {
                res?
            } else {
                return Err(format!("Unsupported OID for Date64 Arrow type: {oid:?}"));
            }
        }
        DataType::Timestamp(unit, _tz) => {
            let nanos = match unit {
                arrow_schema::TimeUnit::Nanosecond => array
                    .as_primitive::<arrow_array::types::TimestampNanosecondType>()
                    .value(index),
                arrow_schema::TimeUnit::Microsecond => array
                    .as_primitive::<arrow_array::types::TimestampMicrosecondType>()
                    .value(index)
                    .checked_mul(1_000)
                    .ok_or("Overflow calculating nanoseconds from TimestampMicrosecond")?,
                arrow_schema::TimeUnit::Millisecond => array
                    .as_primitive::<arrow_array::types::TimestampMillisecondType>()
                    .value(index)
                    .checked_mul(1_000_000)
                    .ok_or("Overflow calculating nanoseconds from TimestampMillisecond")?,
                arrow_schema::TimeUnit::Second => array
                    .as_primitive::<arrow_array::types::TimestampSecondType>()
                    .value(index)
                    .checked_mul(1_000_000_000)
                    .ok_or("Overflow calculating nanoseconds from TimestampSecond")?,
            };
            if let Some(res) = try_convert_timestamp_nanos_to_datum(nanos, &oid) {
                res?
            } else {
                return Err(format!("Unsupported OID for Timestamp Arrow type: {oid:?}"));
            }
        }
        DataType::List(_) | DataType::LargeList(_) => {
            return arrow_array_to_datum_list(array, index)
        }
        dt => return Err(format!("Unsupported Arrow data type: {dt:?}")),
    };
    Ok(datum)
}

/// Helper function to extract lists.
/// Note: This ignores the target `PgOid` and `numeric_scale` passed to the parent `arrow_array_to_datum`,
/// producing a Datum matching the inner Arrow array type (e.g. an Int64 Arrow array always becomes int8[]).
/// Callers are expected to know the native type matching the Arrow list array.
fn arrow_array_to_datum_list(
    array: &dyn Array,
    index: usize,
) -> Result<Option<pg_sys::Datum>, String> {
    use arrow_array::*;
    use arrow_schema::DataType;

    macro_rules! collect_list {
        ($inner:expr, $ArrayType:ty, $RustType:ty) => {{
            let arr = $inner
                .as_any()
                .downcast_ref::<$ArrayType>()
                .ok_or("Failed to downcast inner list array")?;
            let vals: Vec<Option<$RustType>> = (0..arr.len())
                .map(|i| {
                    if arr.is_null(i) {
                        None
                    } else {
                        Some(arr.value(i).into())
                    }
                })
                .collect();
            Ok(vals.into_datum())
        }};
    }

    let list = array
        .as_any()
        .downcast_ref::<ListArray>()
        .ok_or("Failed to downcast to ListArray")?;
    let inner = list.value(index);
    let inner_type = inner.data_type();

    match inner_type {
        DataType::Utf8 => collect_list!(inner, StringArray, String),
        DataType::Utf8View => collect_list!(inner, StringViewArray, String),
        DataType::LargeUtf8 => collect_list!(inner, LargeStringArray, String),
        DataType::Int64 => collect_list!(inner, Int64Array, i64),
        DataType::Int32 => collect_list!(inner, Int32Array, i32),
        DataType::Float64 => collect_list!(inner, Float64Array, f64),
        DataType::Boolean => collect_list!(inner, BooleanArray, bool),
        _ => Err(format!(
            "Unsupported Arrow List element type {inner_type:?} for ARRAY_AGG projection"
        )),
    }
}

pub(crate) fn try_convert_timestamp_nanos_to_datum(
    ts_nanos: i64,
    oid: &PgOid,
) -> Option<Result<Option<pg_sys::Datum>, String>> {
    match &oid {
        PgOid::BuiltIn(PgBuiltInOids::TIMESTAMPTZOID)
        | PgOid::BuiltIn(PgBuiltInOids::TIMESTAMPOID)
        | PgOid::BuiltIn(PgBuiltInOids::DATEOID)
        | PgOid::BuiltIn(PgBuiltInOids::TIMEOID)
        | PgOid::BuiltIn(PgBuiltInOids::TIMETZOID) => {
            let dt = ts_nanos_to_date_time(ts_nanos);
            let prim_dt = dt.into_primitive();
            let res = match &oid {
                PgOid::BuiltIn(PgBuiltInOids::TIMESTAMPTZOID) => {
                    let (h, m, s, micro) = prim_dt.as_hms_micro();
                    datum::TimestampWithTimeZone::with_timezone(
                        prim_dt.year(),
                        prim_dt.month().into(),
                        prim_dt.day(),
                        h,
                        m,
                        s as f64 + ((micro as f64) / (MICROSECONDS_IN_SECOND as f64)),
                        "UTC",
                    )
                    .map_err(|e| format!("Failed to convert timestamp: {e}"))
                    .map(|d| d.into_datum())
                }
                PgOid::BuiltIn(PgBuiltInOids::TIMESTAMPOID) => {
                    let (h, m, s, micro) = prim_dt.as_hms_micro();
                    datum::Timestamp::new(
                        prim_dt.year(),
                        prim_dt.month().into(),
                        prim_dt.day(),
                        h,
                        m,
                        s as f64 + ((micro as f64) / (MICROSECONDS_IN_SECOND as f64)),
                    )
                    .map_err(|e| format!("Failed to convert timestamp: {e}"))
                    .map(|d| d.into_datum())
                }
                PgOid::BuiltIn(PgBuiltInOids::DATEOID) => {
                    datum::Date::new(prim_dt.year(), prim_dt.month().into(), prim_dt.day())
                        .map_err(|e| format!("Failed to convert timestamp: {e}"))
                        .map(|d| d.into_datum())
                }
                PgOid::BuiltIn(PgBuiltInOids::TIMEOID) => {
                    let (h, m, s, micro) = prim_dt.as_hms_micro();
                    datum::Time::new(
                        h,
                        m,
                        s as f64 + ((micro as f64) / (MICROSECONDS_IN_SECOND as f64)),
                    )
                    .map_err(|e| format!("Failed to convert timestamp: {e}"))
                    .map(|d| d.into_datum())
                }
                PgOid::BuiltIn(PgBuiltInOids::TIMETZOID) => {
                    let (h, m, s, micro) = prim_dt.as_hms_micro();
                    datum::TimeWithTimeZone::with_timezone(
                        h,
                        m,
                        s as f64 + ((micro as f64) / (MICROSECONDS_IN_SECOND as f64)),
                        "UTC",
                    )
                    .map_err(|e| format!("Failed to convert timestamp: {e}"))
                    .map(|d| d.into_datum())
                }
                _ => unreachable!(),
            };
            Some(res)
        }
        _ => None,
    }
}

pub fn ts_nanos_to_date_time(ts_nanos: i64) -> tantivy::DateTime {
    tantivy::DateTime::from_timestamp_nanos(ts_nanos)
}

pub fn date_time_to_ts_nanos(date_time: tantivy::DateTime) -> i64 {
    date_time.into_timestamp_nanos()
}

// ---------------------------------------------------------------------------
// Type mapping helpers (used by PgExprUdf for expression DISTINCT support)
// ---------------------------------------------------------------------------

/// Returns true if this PG type OID can be converted to an Arrow array
/// by the `eval_expr_to_arrow!` macro. Used at planning time to decline JoinScan
/// for unsupported expression result types, falling back to native PG.
pub fn is_arrow_convertible(type_oid: pg_sys::Oid) -> bool {
    matches!(
        type_oid,
        pg_sys::BOOLOID
            | pg_sys::INT2OID
            | pg_sys::INT4OID
            | pg_sys::INT8OID
            | pg_sys::FLOAT4OID
            | pg_sys::FLOAT8OID
            | pg_sys::TEXTOID
            | pg_sys::VARCHAROID
            | pg_sys::NAMEOID
    )
}

/// Arrow DataType for UDF INPUTS — matches what Tantivy fast fields produce.
/// Tantivy widens Int16/Int32 → Int64 and Float32 → Float64.
pub fn pg_type_to_tantivy_arrow(type_oid: pg_sys::Oid) -> DataType {
    match type_oid {
        pg_sys::BOOLOID => DataType::Boolean,
        pg_sys::INT2OID | pg_sys::INT4OID | pg_sys::INT8OID => DataType::Int64,
        pg_sys::FLOAT4OID | pg_sys::FLOAT8OID => DataType::Float64,
        pg_sys::TEXTOID | pg_sys::VARCHAROID | pg_sys::NAMEOID => DataType::Utf8,
        pg_sys::TIMESTAMPOID => DataType::Timestamp(arrow_schema::TimeUnit::Microsecond, None),
        pg_sys::TIMESTAMPTZOID => {
            DataType::Timestamp(arrow_schema::TimeUnit::Microsecond, Some("UTC".into()))
        }
        pg_sys::DATEOID => DataType::Date32,
        _ => DataType::Utf8,
    }
}

/// Arrow DataType for UDF OUTPUTS — preserves the PG expression result type
/// without Tantivy widening.
pub fn pg_type_to_arrow(type_oid: pg_sys::Oid) -> DataType {
    match type_oid {
        pg_sys::BOOLOID => DataType::Boolean,
        pg_sys::INT2OID => DataType::Int16,
        pg_sys::INT4OID => DataType::Int32,
        pg_sys::INT8OID => DataType::Int64,
        pg_sys::FLOAT4OID => DataType::Float32,
        pg_sys::FLOAT8OID => DataType::Float64,
        pg_sys::TEXTOID | pg_sys::VARCHAROID | pg_sys::NAMEOID => DataType::Utf8,
        pg_sys::TIMESTAMPOID => DataType::Timestamp(arrow_schema::TimeUnit::Microsecond, None),
        pg_sys::TIMESTAMPTZOID => {
            DataType::Timestamp(arrow_schema::TimeUnit::Microsecond, Some("UTC".into()))
        }
        pg_sys::DATEOID => DataType::Date32,
        _ => DataType::Utf8,
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;

    use crate::postgres::datetime::{MAX_SAFE_TANTIVY_NANOS, MIN_SAFE_TANTIVY_NANOS};

    use std::sync::Arc;

    use crate::postgres::types::TantivyValue;

    use arrow_array::builder::{
        ArrayBuilder, BooleanBuilder, Float64Builder, Int64Builder, StringViewBuilder,
        TimestampNanosecondBuilder, UInt64Builder,
    };
    use arrow_array::*;
    use pgrx::datum::{Date, Time, TimeWithTimeZone, Timestamp, TimestampWithTimeZone};
    use pgrx::pg_test;
    use pgrx::Spi;
    use proptest::prelude::*;

    fn create_string_view_array(s: &str) -> Arc<dyn Array> {
        let mut builder = StringViewBuilder::with_capacity(1);
        builder.append_value(s);
        Arc::new(builder.finish())
    }

    /// A generic helper function to test the full conversion roundtrip for a given value and OID.
    fn test_conversion_roundtrip<T, F1, F2, R>(
        original_val: T,
        create_array: F1,
        oid: PgOid,
        create_expected_value: F2,
    ) where
        T: Clone,
        F1: Fn(T) -> Arc<dyn Array>,
        F2: Fn(T) -> R,
        R: TryInto<TantivyValue>,
        R::Error: std::fmt::Debug,
    {
        let array = create_array(original_val.clone());
        let datum = arrow_array_to_datum(&array, 0, oid, None).unwrap().unwrap();
        let converted_val = unsafe { TantivyValue::try_from_datum(datum, oid) }.unwrap();
        let expected_val: TantivyValue = create_expected_value(original_val).try_into().unwrap();
        assert_eq!(expected_val, converted_val);
    }

    #[pg_test]
    fn test_arrow_int64_to_datum() {
        proptest!(|(
            original_val in any::<i64>()
        )| {
            let create_int64_array = |v: i64| {
                let mut builder = Int64Builder::with_capacity(1);
                builder.append_value(v);
                Arc::new(builder.finish()) as Arc<dyn Array>
            };

            // Test INT8OID
            let oid_i64 = PgOid::from(PgBuiltInOids::INT8OID.value());
            test_conversion_roundtrip(original_val, create_int64_array, oid_i64, |v| v);

            // Test INT4OID
            if original_val >= i32::MIN as i64 && original_val <= i32::MAX as i64 {
                let oid_i32 = PgOid::from(PgBuiltInOids::INT4OID.value());
                test_conversion_roundtrip(original_val, create_int64_array, oid_i32, |v| v as i32);
            }

            // Test INT2OID
            if original_val >= i16::MIN as i64 && original_val <= i16::MAX as i64 {
                let oid_i16 = PgOid::from(PgBuiltInOids::INT2OID.value());
                test_conversion_roundtrip(original_val, create_int64_array, oid_i16, |v| v as i16);
            }
        });
    }
    fn do_test_arrow_int64_as_timestamp_to_datum(original_nanos: i64) {
        let create_ts_array = |v: i64| {
            let mut builder = Int64Builder::with_capacity(1);
            builder.append_value(v);
            Arc::new(builder.finish()) as Arc<dyn Array>
        };

        let pdt = ts_nanos_to_date_time(original_nanos).into_primitive();

        // Test TIMESTAMPTZOID
        let oid_timestamptz = PgOid::from(PgBuiltInOids::TIMESTAMPTZOID.value());
        test_conversion_roundtrip(original_nanos, create_ts_array, oid_timestamptz, |_| {
            TimestampWithTimeZone::with_timezone(
                pdt.year(),
                pdt.month().into(),
                pdt.day(),
                pdt.hour(),
                pdt.minute(),
                pdt.second() as f64 + pdt.microsecond() as f64 / 1_000_000.0,
                "UTC",
            )
            .unwrap()
        });

        // Test TIMESTAMPOID
        let oid_timestamp = PgOid::from(PgBuiltInOids::TIMESTAMPOID.value());
        test_conversion_roundtrip(original_nanos, create_ts_array, oid_timestamp, |_| {
            Timestamp::new(
                pdt.year(),
                pdt.month().into(),
                pdt.day(),
                pdt.hour(),
                pdt.minute(),
                pdt.second() as f64 + pdt.microsecond() as f64 / 1_000_000.0,
            )
            .unwrap()
        });

        // Test DATEOID
        let oid_date = PgOid::from(PgBuiltInOids::DATEOID.value());
        test_conversion_roundtrip(original_nanos, create_ts_array, oid_date, |_| {
            Date::new(pdt.year(), pdt.month().into(), pdt.day()).unwrap()
        });

        // Test TIMEOID
        let oid_time = PgOid::from(PgBuiltInOids::TIMEOID.value());
        test_conversion_roundtrip(original_nanos, create_ts_array, oid_time, |_| {
            Time::new(
                pdt.hour(),
                pdt.minute(),
                pdt.second() as f64 + pdt.microsecond() as f64 / 1_000_000.0,
            )
            .unwrap()
        });

        // Test TIMETZOID
        let oid_timetz = PgOid::from(PgBuiltInOids::TIMETZOID.value());
        test_conversion_roundtrip(original_nanos, create_ts_array, oid_timetz, |_| {
            TimeWithTimeZone::with_timezone(
                pdt.hour(),
                pdt.minute(),
                pdt.second() as f64 + pdt.microsecond() as f64 / 1_000_000.0,
                "UTC",
            )
            .unwrap()
        });
    }

    #[pg_test]
    fn test_arrow_int64_as_timestamp_to_datum_bounds() {
        do_test_arrow_int64_as_timestamp_to_datum(MIN_SAFE_TANTIVY_NANOS);
        do_test_arrow_int64_as_timestamp_to_datum(MAX_SAFE_TANTIVY_NANOS);
    }

    #[pg_test]
    fn test_arrow_int64_as_timestamp_to_datum() {
        proptest!(|(original_nanos in MIN_SAFE_TANTIVY_NANOS..=MAX_SAFE_TANTIVY_NANOS)| {
            do_test_arrow_int64_as_timestamp_to_datum(original_nanos);
        });
    }

    #[pg_test]
    fn test_arrow_uint64_to_datum() {
        proptest!(|(
            original_val in any::<u64>()
        )| {
            let create_uint64_array = |v: u64| {
                let mut builder = UInt64Builder::with_capacity(1);
                builder.append_value(v);
                Arc::new(builder.finish()) as Arc<dyn Array>
            };

            // Test INT8OID
            if original_val <= i64::MAX as u64 {
                let oid_i64 = PgOid::from(PgBuiltInOids::INT8OID.value());
                test_conversion_roundtrip(original_val, create_uint64_array, oid_i64, |v| v as i64);
            }

            // Test OIDOID
            if original_val <= u32::MAX as u64 {
                let oid_u32 = PgOid::from(PgBuiltInOids::OIDOID.value());
                test_conversion_roundtrip(original_val, create_uint64_array, oid_u32, |v| v as u32);
            }
        });
    }

    #[pg_test]
    fn test_arrow_float64_to_datum() {
        proptest!(|(original_val in any::<f64>())| {
            prop_assume!(original_val.is_finite());
            let create_float64_array = |v: f64| {
                let mut builder = Float64Builder::with_capacity(1);
                builder.append_value(v);
                Arc::new(builder.finish()) as Arc<dyn Array>
            };

            // Test FLOAT8OID
            let oid_f64 = PgOid::from(PgBuiltInOids::FLOAT8OID.value());
            test_conversion_roundtrip(original_val, create_float64_array, oid_f64, |v| v);

            // Test FLOAT4OID
            if original_val >= f32::MIN as f64 && original_val <= f32::MAX as f64 {
                let oid_f32 = PgOid::from(PgBuiltInOids::FLOAT4OID.value());
                test_conversion_roundtrip(original_val, create_float64_array, oid_f32, |v| v as f32);
            }

            // Test NUMERICOID (legacy pre-v0.22.0 indexes stored NUMERIC as F64).
            let oid_numeric = PgOid::from(PgBuiltInOids::NUMERICOID.value());
            test_conversion_roundtrip(original_val, create_float64_array, oid_numeric, |v| {
                pgrx::AnyNumeric::try_from(v).unwrap()
            });
        });
    }

    #[pg_test]
    fn test_arrow_boolean_to_datum() {
        proptest!(|(original_val in any::<bool>())| {
            let create_bool_array = |v: bool| {
                let mut builder = BooleanBuilder::with_capacity(1);
                builder.append_value(v);
                Arc::new(builder.finish()) as Arc<dyn Array>
            };
            let oid_bool = PgOid::from(PgBuiltInOids::BOOLOID.value());
            test_conversion_roundtrip(original_val, create_bool_array, oid_bool, |v| v);
        });
    }

    #[pg_test]
    fn test_arrow_string_to_datum_text() {
        Spi::run("CREATE EXTENSION IF NOT EXISTS citext;").unwrap();
        let citext_oid = PgOid::from(
            Spi::get_one::<pg_sys::Oid>("SELECT 'citext'::regtype::oid")
                .expect("SPI failed")
                .expect("citext extension not installed"),
        );
        proptest!(|(ref s in ".*")| {
            let oid_text = PgOid::from(PgBuiltInOids::TEXTOID.value());
            let oid_varchar = PgOid::from(PgBuiltInOids::VARCHAROID.value());
            test_conversion_roundtrip(s.clone(), |s| create_string_view_array(&s), oid_text, |s| s);
            test_conversion_roundtrip(s.clone(), |s| create_string_view_array(&s), oid_varchar, |s| s);
            test_conversion_roundtrip(s.clone(), |s| create_string_view_array(&s), citext_oid, |s| s);
        });
    }

    #[pg_test]
    fn test_arrow_string_to_datum_json() {
        proptest!(|(s in r#"[a-zA-Z0-9]*"#)| {
            let oid_json = PgOid::from(PgBuiltInOids::JSONOID.value());
            test_conversion_roundtrip(s.clone(), |s| create_string_view_array(&s), oid_json, |v| {
                pgrx::datum::JsonString(serde_json::Value::String(v).to_string())
            });
        });
    }

    #[pg_test]
    fn test_arrow_string_to_datum_jsonb() {
        proptest!(|(s in r#"[a-zA-Z0-9]*"#)| {
            let oid_jsonb = PgOid::from(PgBuiltInOids::JSONBOID.value());
            test_conversion_roundtrip(s.clone(), |s| create_string_view_array(&s), oid_jsonb, |v| {
                pgrx::JsonB(serde_json::Value::String(v))
            });
        });
    }

    #[pg_test]
    fn test_arrow_string_to_datum_uuid() {
        proptest!(|(s in "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}")| {
            let oid_uuid = PgOid::from(PgBuiltInOids::UUIDOID.value());
            test_conversion_roundtrip(s.clone(), |s| create_string_view_array(&s), oid_uuid, |v| {
                let uuid = uuid::Uuid::parse_str(&v).unwrap();
                pgrx::Uuid::from_slice(uuid.as_bytes()).unwrap()
            });
        });
    }

    #[pg_test]
    fn test_arrow_string_to_datum_inet() {
        proptest!(|(b1 in 0..=255u8, b2 in 0..=255u8, b3 in 0..=255u8, b4 in 0..=255u8)| {
            let s = format!("{b1}.{b2}.{b3}.{b4}");
            let oid_inet = PgOid::from(PgBuiltInOids::INETOID.value());
            test_conversion_roundtrip(s.clone(), |s| create_string_view_array(&s), oid_inet, |v| {
                pgrx::Inet::from(v)
            });
        });
    }

    fn do_test_arrow_timestamp_to_datum(original_nanos: i64) {
        let create_ts_array = |v: i64| {
            let mut builder = TimestampNanosecondBuilder::with_capacity(1);
            builder.append_value(v);
            Arc::new(builder.finish()) as Arc<dyn Array>
        };

        let pdt = ts_nanos_to_date_time(original_nanos).into_primitive();

        // Test TIMESTAMPTZOID
        let oid_timestamptz = PgOid::from(PgBuiltInOids::TIMESTAMPTZOID.value());
        test_conversion_roundtrip(original_nanos, create_ts_array, oid_timestamptz, |_| {
            TimestampWithTimeZone::with_timezone(
                pdt.year(),
                pdt.month().into(),
                pdt.day(),
                pdt.hour(),
                pdt.minute(),
                pdt.second() as f64 + pdt.microsecond() as f64 / 1_000_000.0,
                "UTC",
            )
            .unwrap()
        });

        // Test TIMESTAMPOID
        let oid_timestamp = PgOid::from(PgBuiltInOids::TIMESTAMPOID.value());
        test_conversion_roundtrip(original_nanos, create_ts_array, oid_timestamp, |_| {
            Timestamp::new(
                pdt.year(),
                pdt.month().into(),
                pdt.day(),
                pdt.hour(),
                pdt.minute(),
                pdt.second() as f64 + pdt.microsecond() as f64 / 1_000_000.0,
            )
            .unwrap()
        });

        // Test DATEOID
        let oid_date = PgOid::from(PgBuiltInOids::DATEOID.value());
        test_conversion_roundtrip(original_nanos, create_ts_array, oid_date, |_| {
            Date::new(pdt.year(), pdt.month().into(), pdt.day()).unwrap()
        });

        // Test TIMEOID
        let oid_time = PgOid::from(PgBuiltInOids::TIMEOID.value());
        test_conversion_roundtrip(original_nanos, create_ts_array, oid_time, |_| {
            Time::new(
                pdt.hour(),
                pdt.minute(),
                pdt.second() as f64 + pdt.microsecond() as f64 / 1_000_000.0,
            )
            .unwrap()
        });

        // Test TIMETZOID
        let oid_timetz = PgOid::from(PgBuiltInOids::TIMETZOID.value());
        test_conversion_roundtrip(original_nanos, create_ts_array, oid_timetz, |_| {
            TimeWithTimeZone::with_timezone(
                pdt.hour(),
                pdt.minute(),
                pdt.second() as f64 + pdt.microsecond() as f64 / 1_000_000.0,
                "UTC",
            )
            .unwrap()
        });
    }

    #[pg_test]
    fn test_arrow_timestamp_to_datum_bounds() {
        do_test_arrow_timestamp_to_datum(MIN_SAFE_TANTIVY_NANOS);
        do_test_arrow_timestamp_to_datum(MAX_SAFE_TANTIVY_NANOS);
    }

    #[pg_test]
    fn test_arrow_timestamp_to_datum() {
        proptest!(|(original_nanos in MIN_SAFE_TANTIVY_NANOS..=MAX_SAFE_TANTIVY_NANOS)| {
            do_test_arrow_timestamp_to_datum(original_nanos);
        });
    }

    fn test_null_conversion<B: ArrayBuilder, F: FnOnce(&mut B)>(
        mut builder: B,
        oid: PgOid,
        append_null: F,
    ) {
        append_null(&mut builder);
        let array = builder.finish();
        let datum = arrow_array_to_datum(&array, 0, oid, None).unwrap();
        assert!(datum.is_none());
    }

    #[pg_test]
    fn test_arrow_to_datum_nulls() {
        Spi::run("CREATE EXTENSION IF NOT EXISTS citext;").unwrap();
        test_null_conversion(
            Int64Builder::with_capacity(1),
            PgOid::from(PgBuiltInOids::INT8OID.value()),
            |b| b.append_null(),
        );
        test_null_conversion(
            UInt64Builder::with_capacity(1),
            PgOid::from(PgBuiltInOids::INT8OID.value()),
            |b| b.append_null(),
        );
        test_null_conversion(
            Float64Builder::with_capacity(1),
            PgOid::from(PgBuiltInOids::FLOAT8OID.value()),
            |b| b.append_null(),
        );
        test_null_conversion(
            BooleanBuilder::with_capacity(1),
            PgOid::from(PgBuiltInOids::BOOLOID.value()),
            |b| b.append_null(),
        );
        test_null_conversion(
            TimestampNanosecondBuilder::with_capacity(1),
            PgOid::from(PgBuiltInOids::TIMESTAMPOID.value()),
            |b| b.append_null(),
        );
        test_null_conversion(
            StringViewBuilder::with_capacity(1),
            PgOid::from(PgBuiltInOids::TEXTOID.value()),
            |b| b.append_null(),
        );
        let citext_oid = PgOid::from(
            Spi::get_one::<pg_sys::Oid>("SELECT 'citext'::regtype::oid")
                .expect("SPI failed")
                .expect("citext extension not installed"),
        );
        test_null_conversion(StringViewBuilder::with_capacity(1), citext_oid, |b| {
            b.append_null()
        });
    }

    fn arrow_value_to_datum(
        col: &Arc<dyn Array>,
        row_idx: usize,
        typoid: pg_sys::Oid,
    ) -> Option<pg_sys::Datum> {
        arrow_array_to_datum(col.as_ref(), row_idx, pgrx::PgOid::from(typoid), None).unwrap_or(None)
    }

    #[pgrx::pg_test]
    fn test_agg_project_arrow_int64() {
        let arr: Arc<dyn Array> = Arc::new(Int64Array::from(vec![42]));
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::INT8OID);
        assert!(datum.is_some());

        let arr: Arc<dyn Array> = Arc::new(Int64Array::from(vec![100]));
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::INT4OID);
        assert!(datum.is_some());
    }

    #[pgrx::pg_test]
    fn test_agg_project_arrow_float64() {
        let arr: Arc<dyn Array> = Arc::new(Float64Array::from(vec![99.5]));
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::FLOAT8OID);
        assert!(datum.is_some());

        let datum = arrow_value_to_datum(&arr, 0, pg_sys::FLOAT4OID);
        assert!(datum.is_some());
    }

    #[pgrx::pg_test]
    fn test_agg_project_arrow_string() {
        let arr: Arc<dyn Array> = Arc::new(StringArray::from(vec!["hello"]));
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::TEXTOID);
        assert!(datum.is_some());
    }

    #[pgrx::pg_test]
    fn test_agg_project_arrow_bool() {
        let arr: Arc<dyn Array> = Arc::new(BooleanArray::from(vec![true]));
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::BOOLOID);
        assert!(datum.is_some());
    }

    #[pgrx::pg_test]
    fn test_agg_project_arrow_null() {
        // A nullable Int64 array with a null at index 0
        let arr: Arc<dyn Array> = Arc::new(Int64Array::from(vec![None, Some(1)]));
        // is_null check happens before arrow_value_to_datum in project_aggregate_row_to_slot,
        // but let's verify the array reports null correctly
        assert!(arr.is_null(0));
        assert!(!arr.is_null(1));
    }

    #[pgrx::pg_test]
    fn test_agg_project_arrow_utf8view() {
        let arr: Arc<dyn Array> = Arc::new(StringViewArray::from(vec!["world"]));
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::TEXTOID);
        assert!(datum.is_some());
    }

    #[pgrx::pg_test]
    fn test_agg_project_arrow_int32() {
        let arr: Arc<dyn Array> = Arc::new(Int32Array::from(vec![42]));
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::INT4OID);
        assert!(datum.is_some());

        // Int32 → INT8OID (widening)
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::INT8OID);
        assert!(datum.is_some());
    }

    #[pgrx::pg_test]
    fn test_agg_project_arrow_int16() {
        let arr: Arc<dyn Array> = Arc::new(Int16Array::from(vec![7]));
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::INT2OID);
        assert!(datum.is_some());

        // Int16 → INT4OID (widening)
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::INT4OID);
        assert!(datum.is_some());
    }

    #[pgrx::pg_test]
    fn test_agg_project_arrow_uint64() {
        let arr: Arc<dyn Array> = Arc::new(UInt64Array::from(vec![100u64]));
        // Within i64 range → int64_to_datum
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::INT8OID);
        assert!(datum.is_some());

        // Above i64::MAX → float64_to_datum fallback
        let arr: Arc<dyn Array> = Arc::new(UInt64Array::from(vec![u64::MAX]));
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::FLOAT8OID);
        assert!(datum.is_some());
    }

    #[pgrx::pg_test]
    fn test_agg_project_arrow_float32() {
        let arr: Arc<dyn Array> = Arc::new(Float32Array::from(vec![1.23f32]));
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::FLOAT4OID);
        assert!(datum.is_some());

        // Float32 → FLOAT8OID (widening)
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::FLOAT8OID);
        assert!(datum.is_some());
    }

    #[pgrx::pg_test]
    fn test_agg_project_arrow_large_utf8() {
        let arr: Arc<dyn Array> = Arc::new(LargeStringArray::from(vec!["large string"]));
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::TEXTOID);
        assert!(datum.is_some());
    }

    #[pgrx::pg_test]
    fn test_agg_project_arrow_decimal128() {
        // Decimal128 with scale=2 → NUMERICOID
        let arr: Arc<dyn Array> = Arc::new(
            arrow_array::Decimal128Array::from(vec![12345i128])
                .with_precision_and_scale(10, 2)
                .unwrap(),
        );
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::NUMERICOID);
        assert!(datum.is_some());

        // Decimal128 with scale=0 → NUMERICOID (integer-like)
        let arr: Arc<dyn Array> = Arc::new(
            arrow_array::Decimal128Array::from(vec![999i128])
                .with_precision_and_scale(10, 0)
                .unwrap(),
        );
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::NUMERICOID);
        assert!(datum.is_some());

        // Decimal128 → FLOAT8OID (non-NUMERIC target)
        let arr: Arc<dyn Array> = Arc::new(
            arrow_array::Decimal128Array::from(vec![12345i128])
                .with_precision_and_scale(10, 2)
                .unwrap(),
        );
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::FLOAT8OID);
        assert!(datum.is_some());
    }

    #[pgrx::pg_test]
    fn test_agg_project_int64_to_numeric() {
        // int64_to_datum with NUMERICOID — the SUM(bigint) crash fix
        let arr: Arc<dyn Array> = Arc::new(Int64Array::from(vec![9999i64]));
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::NUMERICOID);
        assert!(datum.is_some());
    }

    #[pgrx::pg_test]
    fn test_agg_project_int64_to_float() {
        // int64_to_datum with FLOAT8OID and FLOAT4OID
        let arr: Arc<dyn Array> = Arc::new(Int64Array::from(vec![42i64]));

        let datum = arrow_value_to_datum(&arr, 0, pg_sys::FLOAT8OID);
        assert!(datum.is_some());

        let datum = arrow_value_to_datum(&arr, 0, pg_sys::FLOAT4OID);
        assert!(datum.is_some());

        // INT2OID (narrowing)
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::INT2OID);
        assert!(datum.is_some());
    }

    #[pgrx::pg_test]
    fn test_agg_project_float64_to_numeric() {
        // float64_to_datum with NUMERICOID
        let arr: Arc<dyn Array> = Arc::new(Float64Array::from(vec![123.456]));
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::NUMERICOID);
        assert!(datum.is_some());
    }

    #[pgrx::pg_test]
    fn test_agg_project_float64_to_int() {
        // float64_to_datum with integer targets
        let arr: Arc<dyn Array> = Arc::new(Float64Array::from(vec![42.0]));

        let datum = arrow_value_to_datum(&arr, 0, pg_sys::INT8OID);
        assert!(datum.is_some());

        let datum = arrow_value_to_datum(&arr, 0, pg_sys::INT4OID);
        assert!(datum.is_some());

        let datum = arrow_value_to_datum(&arr, 0, pg_sys::INT2OID);
        assert!(datum.is_some());
    }

    #[pgrx::pg_test]
    fn test_agg_project_timestamp_type() {
        // TimestampNanosecondArray is now supported for TIMESTAMPOID
        let arr: Arc<dyn Array> = Arc::new(arrow_array::TimestampNanosecondArray::from(vec![
            1_000_000_000i64,
        ]));
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::TIMESTAMPOID);
        assert!(datum.is_some());
    }

    // --- Timestamp TimeUnit tests ---

    #[pgrx::pg_test]
    fn test_timestamp_nanosecond_projection() {
        let nanos: i64 = 1_705_314_600_000_000_000; // 2024-01-15 10:30:00 UTC
        let arr: Arc<dyn Array> =
            Arc::new(arrow_array::TimestampNanosecondArray::from(vec![nanos]));
        let result = arrow_value_to_datum(&arr, 0, pg_sys::TIMESTAMPOID);
        assert!(
            result.is_some(),
            "TimestampNanosecond should produce a datum"
        );
    }

    #[pgrx::pg_test]
    fn test_timestamp_microsecond_projection() {
        let micros: i64 = 1_705_314_600_000_000; // 2024-01-15 10:30:00 UTC
        let arr: Arc<dyn Array> =
            Arc::new(arrow_array::TimestampMicrosecondArray::from(vec![micros]));
        let result = arrow_value_to_datum(&arr, 0, pg_sys::TIMESTAMPOID);
        assert!(
            result.is_some(),
            "TimestampMicrosecond should produce a datum"
        );
    }

    #[pgrx::pg_test]
    fn test_timestamp_millisecond_projection() {
        let millis: i64 = 1_705_314_600_000; // 2024-01-15 10:30:00 UTC
        let arr: Arc<dyn Array> =
            Arc::new(arrow_array::TimestampMillisecondArray::from(vec![millis]));
        let result = arrow_value_to_datum(&arr, 0, pg_sys::TIMESTAMPOID);
        assert!(
            result.is_some(),
            "TimestampMillisecond should produce a datum"
        );
    }

    #[pgrx::pg_test]
    fn test_timestamp_second_projection() {
        let secs: i64 = 1_705_314_600; // 2024-01-15 10:30:00 UTC
        let arr: Arc<dyn Array> = Arc::new(arrow_array::TimestampSecondArray::from(vec![secs]));
        let result = arrow_value_to_datum(&arr, 0, pg_sys::TIMESTAMPOID);
        assert!(result.is_some(), "TimestampSecond should produce a datum");
    }

    // --- Date32 / Date64 tests ---

    #[pgrx::pg_test]
    fn test_date32_projection() {
        let days: i32 = 19_737; // 2024-01-15 = 19737 days since epoch
        let arr: Arc<dyn Array> = Arc::new(arrow_array::Date32Array::from(vec![days]));
        let result = arrow_value_to_datum(&arr, 0, pg_sys::DATEOID);
        assert!(
            result.is_some(),
            "Date32 should produce a datum for DATEOID"
        );
    }

    #[pgrx::pg_test]
    fn test_date64_projection() {
        let millis: i64 = 19_737 * 86_400_000; // 2024-01-15 in milliseconds since epoch
        let arr: Arc<dyn Array> = Arc::new(arrow_array::Date64Array::from(vec![millis]));
        let result = arrow_value_to_datum(&arr, 0, pg_sys::DATEOID);
        assert!(
            result.is_some(),
            "Date64 should produce a datum for DATEOID"
        );
    }

    // --- TIMESTAMPTZ vs TIMESTAMP vs DATE typoid routing ---

    #[pgrx::pg_test]
    fn test_timestamp_nanos_to_all_typoids() {
        let nanos: i64 = 1_705_314_600_000_000_000; // 2024-01-15 10:30:00 UTC
        let arr: Arc<dyn Array> =
            Arc::new(arrow_array::TimestampNanosecondArray::from(vec![nanos]));

        let ts = arrow_value_to_datum(&arr, 0, pg_sys::TIMESTAMPOID);
        assert!(ts.is_some(), "Should produce TIMESTAMP datum");

        let tstz = arrow_value_to_datum(&arr, 0, pg_sys::TIMESTAMPTZOID);
        assert!(tstz.is_some(), "Should produce TIMESTAMPTZ datum");

        let date = arrow_value_to_datum(&arr, 0, pg_sys::DATEOID);
        assert!(date.is_some(), "Should produce DATE datum");
    }

    // --- NULL handling ---
    // Note: project_aggregate_row_to_slot checks col.is_null(row_idx) before
    // calling arrow_value_to_datum, so we test null reporting at the array level.

    #[pgrx::pg_test]
    fn test_timestamp_null_reports_correctly() {
        let arr: Arc<dyn Array> = Arc::new(arrow_array::TimestampNanosecondArray::from(vec![
            None as Option<i64>,
        ]));
        assert!(arr.is_null(0), "Timestamp null should be reported");
    }

    #[pgrx::pg_test]
    fn test_date32_null_reports_correctly() {
        let arr: Arc<dyn Array> =
            Arc::new(arrow_array::Date32Array::from(vec![None as Option<i32>]));
        assert!(arr.is_null(0), "Date32 null should be reported");
    }

    #[pgrx::pg_test]
    fn test_date64_null_reports_correctly() {
        let arr: Arc<dyn Array> =
            Arc::new(arrow_array::Date64Array::from(vec![None as Option<i64>]));
        assert!(arr.is_null(0), "Date64 null should be reported");
    }

    // --- Unsupported type (negative test) ---

    #[pgrx::pg_test]
    fn test_unsupported_arrow_type_returns_none() {
        let arr: Arc<dyn Array> =
            Arc::new(arrow_array::Time64NanosecondArray::from(vec![1_000_000i64]));
        let result = arrow_value_to_datum(&arr, 0, pg_sys::TIMEOID);
        assert!(
            result.is_none(),
            "Time64 should not be supported — should return None"
        );
    }

    // --- Pre-epoch regression guards ---

    #[pgrx::pg_test]
    fn test_date32_pre_epoch() {
        let days: i32 = -1; // 1969-12-31
        let arr: Arc<dyn Array> = Arc::new(arrow_array::Date32Array::from(vec![days]));
        let result = arrow_value_to_datum(&arr, 0, pg_sys::DATEOID);
        assert!(result.is_some(), "Pre-epoch Date32 should produce a datum");
    }

    #[pgrx::pg_test]
    fn test_timestamp_pre_epoch() {
        let nanos: i64 = -1_000_000_000; // 1969-12-31 23:59:59 UTC
        let arr: Arc<dyn Array> =
            Arc::new(arrow_array::TimestampNanosecondArray::from(vec![nanos]));
        let result = arrow_value_to_datum(&arr, 0, pg_sys::TIMESTAMPOID);
        assert!(
            result.is_some(),
            "Pre-epoch timestamp should produce a datum"
        );
    }
}
