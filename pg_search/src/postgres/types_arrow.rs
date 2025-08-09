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

use crate::postgres::datetime::MICROSECONDS_IN_SECOND;

use arrow_array::cast::AsArray;
use arrow_array::Array;
use arrow_schema::DataType;
use pgrx::pg_sys;
use pgrx::{datum, AnyNumeric, IntoDatum, PgBuiltInOids, PgOid};

/// Get a value of the given type from the given index/row of the given array.
///
/// This effectively inlines `TantivyValue::try_into_datum` in order to avoid creating both
/// `OwnedValue` and `TantivyValue` wrappers around primitives (but particularly around strings).
pub fn arrow_array_to_datum(
    array: &dyn Array,
    index: usize,
    oid: PgOid,
) -> Result<Option<pg_sys::Datum>, String> {
    if array.is_null(index) {
        return Ok(None);
    }

    let datum = match array.data_type() {
        DataType::Utf8View => {
            let arr = array.as_string_view();
            let s = arr.value(index);
            match &oid {
                PgOid::BuiltIn(PgBuiltInOids::TEXTOID)
                | PgOid::BuiltIn(PgBuiltInOids::VARCHAROID) => s.into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::JSONOID) => pgrx::Json(
                    serde_json::from_str(s)
                        .map_err(|e| format!("Failed to decode as JSON: {e}"))?,
                )
                .into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::JSONBOID) => datum::JsonB(
                    serde_json::from_str(s)
                        .map_err(|e| format!("Failed to decode as JSON: {e}"))?,
                )
                .into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::UUIDOID) => {
                    let uuid = uuid::Uuid::parse_str(s)
                        .map_err(|e| format!("Failed to decode as UUID: {e}"))?;
                    datum::Uuid::from_slice(uuid.as_bytes())?.into_datum()
                }
                PgOid::BuiltIn(PgBuiltInOids::INETOID) => {
                    datum::Inet::from(s.to_string()).into_datum()
                }
                _ => return Err(format!("Unsupported OID for Utf8 Arrow type: {oid:?}")),
            }
        }
        DataType::UInt64 => {
            let arr = array.as_primitive::<arrow_array::types::UInt64Type>();
            let val = arr.value(index);
            match &oid {
                PgOid::BuiltIn(PgBuiltInOids::INT8OID) => (val as i64).into_datum(), // Convert u64 to i64 for INT8OID
                PgOid::BuiltIn(PgBuiltInOids::OIDOID) => {
                    pgrx::pg_sys::Oid::from(val as u32).into_datum()
                } // Cast u64 to u32 for OID
                PgOid::BuiltIn(PgBuiltInOids::NUMERICOID) => AnyNumeric::from(val).into_datum(),
                // Consider other potential integer OIDs (INT2OID, INT4OID) if overflow is handled or guaranteed not to occur.
                _ => return Err(format!("Unsupported OID for UInt64 Arrow type: {oid:?}")),
            }
        }
        DataType::Int64 => {
            let arr = array.as_primitive::<arrow_array::types::Int64Type>();
            let val = arr.value(index);
            match &oid {
                PgOid::BuiltIn(PgBuiltInOids::INT8OID) => val.into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::INT4OID) => (val as i32).into_datum(), // Cast i64 to i32
                PgOid::BuiltIn(PgBuiltInOids::INT2OID) => (val as i16).into_datum(), // Cast i64 to i16
                PgOid::BuiltIn(PgBuiltInOids::NUMERICOID) => AnyNumeric::from(val).into_datum(),
                _ => return Err(format!("Unsupported OID for Int64 Arrow type: {oid:?}")),
            }
        }
        DataType::Float64 => {
            let arr = array.as_primitive::<arrow_array::types::Float64Type>();
            let val = arr.value(index);
            match &oid {
                PgOid::BuiltIn(PgBuiltInOids::FLOAT8OID) => val.into_datum(),
                PgOid::BuiltIn(PgBuiltInOids::FLOAT4OID) => (val as f32).into_datum(), // Cast f64 to f32
                PgOid::BuiltIn(PgBuiltInOids::NUMERICOID) => AnyNumeric::try_from(val)
                    .map_err(|e| format!("Failed to encode: {e}"))?
                    .into_datum(),
                _ => return Err(format!("Unsupported OID for Float64 Arrow type: {oid:?}")),
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
        DataType::Timestamp(arrow_schema::TimeUnit::Nanosecond, None) => {
            let arr = array.as_primitive::<arrow_array::types::TimestampNanosecondType>();
            let ts_nanos = arr.value(index);
            let dt = ts_nanos_to_date_time(ts_nanos);
            let prim_dt = dt.into_primitive();
            match &oid {
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
                    .map_err(|e| format!("Failed to convert timestamp: {e}"))?
                    .into_datum()
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
                    .map_err(|e| format!("Failed to convert timestamp: {e}"))?
                    .into_datum()
                }
                PgOid::BuiltIn(PgBuiltInOids::DATEOID) => {
                    datum::Date::new(prim_dt.year(), prim_dt.month().into(), prim_dt.day())
                        .map_err(|e| format!("Failed to convert timestamp: {e}"))?
                        .into_datum()
                }
                PgOid::BuiltIn(PgBuiltInOids::TIMEOID) => {
                    let (h, m, s, micro) = prim_dt.as_hms_micro();
                    datum::Time::new(
                        h,
                        m,
                        s as f64 + ((micro as f64) / (MICROSECONDS_IN_SECOND as f64)),
                    )
                    .map_err(|e| format!("Failed to convert timestamp: {e}"))?
                    .into_datum()
                }
                PgOid::BuiltIn(PgBuiltInOids::TIMETZOID) => {
                    let (h, m, s, micro) = prim_dt.as_hms_micro();
                    datum::TimeWithTimeZone::with_timezone(
                        h,
                        m,
                        s as f64 + ((micro as f64) / (MICROSECONDS_IN_SECOND as f64)),
                        "UTC",
                    )
                    .map_err(|e| format!("Failed to convert timestamp: {e}"))?
                    .into_datum()
                }
                _ => {
                    return Err(format!(
                        "Unsupported OID for TimestampNanosecond Arrow type: {oid:?}"
                    ))
                }
            }
        }
        dt => return Err(format!("Unsupported Arrow data type: {dt:?}")),
    };
    Ok(datum)
}

pub fn ts_nanos_to_date_time(ts_nanos: i64) -> tantivy::DateTime {
    tantivy::DateTime::from_timestamp_nanos(ts_nanos)
}

pub fn date_time_to_ts_nanos(date_time: tantivy::DateTime) -> i64 {
    date_time.into_timestamp_nanos()
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;

    use std::sync::Arc;

    use crate::postgres::types::TantivyValue;

    use arrow_array::builder::{
        ArrayBuilder, BooleanBuilder, Float64Builder, Int64Builder, StringViewBuilder,
        TimestampNanosecondBuilder, UInt64Builder,
    };
    use arrow_array::Array;
    use pgrx::datum::{Date, Time, TimeWithTimeZone, Timestamp, TimestampWithTimeZone};
    use pgrx::pg_test;
    use proptest::prelude::*;
    use serde_json::Value as JsonValue;

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
        let datum = arrow_array_to_datum(&array, 0, oid).unwrap().unwrap();
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

            // Test NUMERICOID
            let oid_numeric = PgOid::from(PgBuiltInOids::NUMERICOID.value());
            test_conversion_roundtrip(original_val, create_int64_array, oid_numeric, AnyNumeric::from);
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

            // Test NUMERICOID
            let oid_numeric = PgOid::from(PgBuiltInOids::NUMERICOID.value());
            test_conversion_roundtrip(original_val, create_uint64_array, oid_numeric, AnyNumeric::from);
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

            // Test NUMERICOID
            let oid_numeric = PgOid::from(PgBuiltInOids::NUMERICOID.value());
            test_conversion_roundtrip(original_val, create_float64_array, oid_numeric, |v| AnyNumeric::try_from(v).unwrap());
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
        proptest!(|(ref s in ".*")| {
            let oid_text = PgOid::from(PgBuiltInOids::TEXTOID.value());
            let oid_varchar = PgOid::from(PgBuiltInOids::VARCHAROID.value());

            test_conversion_roundtrip(s.clone(), |s| create_string_view_array(&s), oid_text, |s| s);
            test_conversion_roundtrip(s.clone(), |s| create_string_view_array(&s), oid_varchar, |s| s);
        });
    }

    #[pg_test]
    fn test_arrow_string_to_datum_json() {
        proptest!(|(s in r#""[a-zA-Z0-9]*""#)| {
            let oid_json = PgOid::from(PgBuiltInOids::JSONOID.value());
            test_conversion_roundtrip(s.clone(), |s| create_string_view_array(&s), oid_json, pgrx::datum::JsonString);
        });
    }

    #[pg_test]
    fn test_arrow_string_to_datum_jsonb() {
        proptest!(|(s in r#""[a-zA-Z0-9]*""#)| {
            let oid_jsonb = PgOid::from(PgBuiltInOids::JSONBOID.value());
            test_conversion_roundtrip(s.clone(), |s| create_string_view_array(&s), oid_jsonb, |v| {
                let expected_val: JsonValue = serde_json::from_str(&v).unwrap();
                pgrx::JsonB(expected_val)
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

    #[pg_test]
    fn test_arrow_timestamp_to_datum() {
        proptest!(|(original_nanos in any::<i64>())| {
            let create_ts_array = |v: i64| {
                let mut builder = TimestampNanosecondBuilder::with_capacity(1);
                builder.append_value(v);
                Arc::new(builder.finish()) as Arc<dyn Array>
            };

            let pdt = ts_nanos_to_date_time(original_nanos).into_primitive();

            // Test TIMESTAMPTZOID
            let oid_timestamptz = PgOid::from(PgBuiltInOids::TIMESTAMPTZOID.value());
            test_conversion_roundtrip(original_nanos, create_ts_array, oid_timestamptz, |_| {
                TimestampWithTimeZone::with_timezone(pdt.year(), pdt.month().into(), pdt.day(), pdt.hour(), pdt.minute(), pdt.second() as f64 + pdt.microsecond() as f64 / 1_000_000.0, "UTC").unwrap()
            });

            // Test TIMESTAMPOID
            let oid_timestamp = PgOid::from(PgBuiltInOids::TIMESTAMPOID.value());
            test_conversion_roundtrip(original_nanos, create_ts_array, oid_timestamp, |_| {
                Timestamp::new(pdt.year(), pdt.month().into(), pdt.day(), pdt.hour(), pdt.minute(), pdt.second() as f64 + pdt.microsecond() as f64 / 1_000_000.0).unwrap()
            });

            // Test DATEOID
            let oid_date = PgOid::from(PgBuiltInOids::DATEOID.value());
            test_conversion_roundtrip(original_nanos, create_ts_array, oid_date, |_| {
                Date::new(pdt.year(), pdt.month().into(), pdt.day()).unwrap()
            });

            // Test TIMEOID
            let oid_time = PgOid::from(PgBuiltInOids::TIMEOID.value());
            test_conversion_roundtrip(original_nanos, create_ts_array, oid_time, |_| {
                Time::new(pdt.hour(), pdt.minute(), pdt.second() as f64 + pdt.microsecond() as f64 / 1_000_000.0).unwrap()
            });

            // Test TIMETZOID
            let oid_timetz = PgOid::from(PgBuiltInOids::TIMETZOID.value());
            test_conversion_roundtrip(original_nanos, create_ts_array, oid_timetz, |_| {
                TimeWithTimeZone::with_timezone(pdt.hour(), pdt.minute(), pdt.second() as f64 + pdt.microsecond() as f64 / 1_000_000.0, "UTC").unwrap()
            });
        });
    }

    fn test_null_conversion<B: ArrayBuilder, F: FnOnce(&mut B)>(
        mut builder: B,
        oid: PgOid,
        append_null: F,
    ) {
        append_null(&mut builder);
        let array = builder.finish();
        let datum = arrow_array_to_datum(&array, 0, oid).unwrap();
        assert!(datum.is_none());
    }

    #[pg_test]
    fn test_arrow_to_datum_nulls() {
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
    }
}
