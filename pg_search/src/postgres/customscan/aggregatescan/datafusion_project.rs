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

//! Arrow-to-Postgres result projection for aggregate `RecordBatch`es.
//!
//! Converts DataFusion aggregate results (Arrow arrays) into Postgres tuples.
//! This is simpler than JoinScan's projection because:
//! - No heap fetch / CTID extraction needed
//! - The aggregate result schema directly maps to the SQL output
//! - Type conversion is limited to aggregate-relevant types

use super::join_targetlist::{AggKind, JoinAggregateTargetList};
use arrow_array::{Array, ArrayRef, RecordBatch};
use pgrx::{pg_sys, IntoDatum};

/// Project a single row from an aggregate `RecordBatch` into a Postgres `TupleTableSlot`.
///
/// The DataFusion output schema is: `[group_col_0, ..., group_col_N, agg_0, ..., agg_M]`.
/// Each column is mapped to the correct position in the Postgres tuple via `output_index`.
///
/// # Safety
///
/// Caller must ensure:
/// - `slot` is a valid, cleared `TupleTableSlot`
/// - `row_idx` is within bounds of `batch.num_rows()`
/// - The tuple descriptor on `slot` matches the expected output schema
pub unsafe fn project_aggregate_row_to_slot(
    slot: *mut pg_sys::TupleTableSlot,
    batch: &RecordBatch,
    row_idx: usize,
    targetlist: &JoinAggregateTargetList,
) -> *mut pg_sys::TupleTableSlot {
    let tupdesc = (*slot).tts_tupleDescriptor;
    let natts = (*tupdesc).natts as usize;
    let datums = std::slice::from_raw_parts_mut((*slot).tts_values, natts);
    let isnull = std::slice::from_raw_parts_mut((*slot).tts_isnull, natts);

    // DataFusion output column index: group columns come first, then aggregates
    let mut df_col_idx = 0;

    // Fill GROUP BY columns
    for gc in &targetlist.group_columns {
        let pg_idx = gc.output_index;
        if pg_idx >= natts {
            df_col_idx += 1;
            continue;
        }

        let col = batch.column(df_col_idx);
        let expected_type = {
            #[cfg(any(feature = "pg15", feature = "pg16", feature = "pg17"))]
            {
                (*tupdesc).attrs.as_slice(natts)[pg_idx].atttypid
            }
            #[cfg(feature = "pg18")]
            {
                (*pg_sys::TupleDescAttr(tupdesc, pg_idx as i32)).atttypid
            }
        };

        if col.is_null(row_idx) {
            isnull[pg_idx] = true;
            datums[pg_idx] = pg_sys::Datum::null();
        } else {
            match arrow_value_to_datum(col, row_idx, expected_type) {
                Some(datum) => {
                    datums[pg_idx] = datum;
                    isnull[pg_idx] = false;
                }
                None => {
                    isnull[pg_idx] = true;
                    datums[pg_idx] = pg_sys::Datum::null();
                }
            }
        }
        df_col_idx += 1;
    }

    // Fill aggregate columns
    for agg in &targetlist.aggregates {
        let pg_idx = agg.output_index;
        if pg_idx >= natts {
            df_col_idx += 1;
            continue;
        }

        let col = batch.column(df_col_idx);

        if col.is_null(row_idx) {
            // COUNT returns 0 for NULL, other aggregates return NULL
            match agg.agg_kind {
                AggKind::CountStar | AggKind::Count => {
                    datums[pg_idx] = 0i64.into_datum().unwrap_or(pg_sys::Datum::null());
                    isnull[pg_idx] = false;
                }
                _ => {
                    isnull[pg_idx] = true;
                    datums[pg_idx] = pg_sys::Datum::null();
                }
            }
        } else {
            match arrow_value_to_datum(col, row_idx, agg.result_type_oid) {
                Some(datum) => {
                    datums[pg_idx] = datum;
                    isnull[pg_idx] = false;
                }
                None => {
                    isnull[pg_idx] = true;
                    datums[pg_idx] = pg_sys::Datum::null();
                }
            }
        }
        df_col_idx += 1;
    }

    // Mark slot as non-empty
    (*slot).tts_flags &= !(pg_sys::TTS_FLAG_EMPTY as u16);
    (*slot).tts_nvalid = natts as i16;

    slot
}

/// Convert a single value from an Arrow array to a Postgres `Datum`.
///
/// Dispatches on the Arrow data type and converts to the target Postgres type OID.
/// Returns `None` for unsupported type combinations.
fn arrow_value_to_datum(
    col: &ArrayRef,
    row_idx: usize,
    typoid: pg_sys::Oid,
) -> Option<pg_sys::Datum> {
    use arrow_array::*;
    use arrow_schema::DataType;

    match col.data_type() {
        DataType::Int64 => {
            let val = col.as_any().downcast_ref::<Int64Array>()?.value(row_idx);
            int64_to_datum(val, typoid)
        }
        DataType::Int32 => {
            let val = col.as_any().downcast_ref::<Int32Array>()?.value(row_idx);
            int64_to_datum(val as i64, typoid)
        }
        DataType::Int16 => {
            let val = col.as_any().downcast_ref::<Int16Array>()?.value(row_idx);
            int64_to_datum(val as i64, typoid)
        }
        DataType::UInt64 => {
            let val = col.as_any().downcast_ref::<UInt64Array>()?.value(row_idx);
            // Use checked conversion to avoid silent overflow for values > i64::MAX
            match i64::try_from(val) {
                Ok(i_val) => int64_to_datum(i_val, typoid),
                Err(_) => float64_to_datum(val as f64, typoid),
            }
        }
        DataType::Float64 => {
            let val = col.as_any().downcast_ref::<Float64Array>()?.value(row_idx);
            float64_to_datum(val, typoid)
        }
        DataType::Float32 => {
            let val = col.as_any().downcast_ref::<Float32Array>()?.value(row_idx);
            float64_to_datum(val as f64, typoid)
        }
        DataType::Utf8 => {
            let val = col.as_any().downcast_ref::<StringArray>()?.value(row_idx);
            val.to_string().into_datum()
        }
        DataType::Utf8View => {
            let val = col
                .as_any()
                .downcast_ref::<StringViewArray>()?
                .value(row_idx);
            val.to_string().into_datum()
        }
        DataType::LargeUtf8 => {
            let val = col
                .as_any()
                .downcast_ref::<LargeStringArray>()?
                .value(row_idx);
            val.to_string().into_datum()
        }
        DataType::Boolean => {
            let val = col.as_any().downcast_ref::<BooleanArray>()?.value(row_idx);
            val.into_datum()
        }
        DataType::Decimal128(_, scale) => {
            let val = col
                .as_any()
                .downcast_ref::<Decimal128Array>()?
                .value(row_idx);
            let scale = *scale as u32;
            if typoid == pg_sys::NUMERICOID {
                // Convert via string to preserve precision for NUMERIC targets
                use pgrx::AnyNumeric;
                let s = if scale == 0 {
                    format!("{}", val)
                } else {
                    let divisor = 10i128.pow(scale);
                    let whole = val / divisor;
                    let frac = (val % divisor).unsigned_abs();
                    format!("{}.{:0>width$}", whole, frac, width = scale as usize)
                };
                let numeric = AnyNumeric::try_from(s.as_str()).ok()?;
                numeric.into_datum()
            } else {
                let divisor = 10_f64.powi(scale as i32);
                let f_val = val as f64 / divisor;
                float64_to_datum(f_val, typoid)
            }
        }
        _ => {
            pgrx::warning!(
                "Unsupported Arrow type {:?} for aggregate projection",
                col.data_type()
            );
            None
        }
    }
}

/// Convert an i64 value to the appropriate Postgres integer datum.
///
/// Handles NUMERICOID explicitly because NUMERIC is pass-by-reference in
/// Postgres — returning a raw i64 for a NUMERIC slot would be interpreted
/// as a pointer, causing a segfault.
fn int64_to_datum(val: i64, typoid: pg_sys::Oid) -> Option<pg_sys::Datum> {
    match typoid {
        pg_sys::INT8OID => val.into_datum(),
        pg_sys::INT4OID => (val as i32).into_datum(),
        pg_sys::INT2OID => (val as i16).into_datum(),
        pg_sys::FLOAT8OID => (val as f64).into_datum(),
        pg_sys::FLOAT4OID => (val as f32).into_datum(),
        pg_sys::NUMERICOID => {
            use pgrx::AnyNumeric;
            let numeric = AnyNumeric::try_from(val as f64).ok()?;
            numeric.into_datum()
        }
        _ => val.into_datum(), // Default to i64
    }
}

/// Convert an f64 value to the appropriate Postgres numeric datum.
fn float64_to_datum(val: f64, typoid: pg_sys::Oid) -> Option<pg_sys::Datum> {
    match typoid {
        pg_sys::FLOAT8OID => val.into_datum(),
        pg_sys::FLOAT4OID => (val as f32).into_datum(),
        pg_sys::INT8OID => (val as i64).into_datum(),
        pg_sys::INT4OID => (val as i32).into_datum(),
        pg_sys::INT2OID => (val as i16).into_datum(),
        pg_sys::NUMERICOID => {
            // Convert f64 to Postgres NUMERIC via pgrx AnyNumeric
            use pgrx::AnyNumeric;
            let numeric = AnyNumeric::try_from(val).ok()?;
            numeric.into_datum()
        }
        _ => val.into_datum(), // Default to f64
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use arrow_array::*;
    use std::sync::Arc;

    #[pgrx::pg_test]
    fn test_agg_project_arrow_int64() {
        let arr: ArrayRef = Arc::new(Int64Array::from(vec![42]));
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::INT8OID);
        assert!(datum.is_some());

        let arr: ArrayRef = Arc::new(Int64Array::from(vec![100]));
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::INT4OID);
        assert!(datum.is_some());
    }

    #[pgrx::pg_test]
    fn test_agg_project_arrow_float64() {
        let arr: ArrayRef = Arc::new(Float64Array::from(vec![99.5]));
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::FLOAT8OID);
        assert!(datum.is_some());

        let datum = arrow_value_to_datum(&arr, 0, pg_sys::FLOAT4OID);
        assert!(datum.is_some());
    }

    #[pgrx::pg_test]
    fn test_agg_project_arrow_string() {
        let arr: ArrayRef = Arc::new(StringArray::from(vec!["hello"]));
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::TEXTOID);
        assert!(datum.is_some());
    }

    #[pgrx::pg_test]
    fn test_agg_project_arrow_bool() {
        let arr: ArrayRef = Arc::new(BooleanArray::from(vec![true]));
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::BOOLOID);
        assert!(datum.is_some());
    }

    #[pgrx::pg_test]
    fn test_agg_project_arrow_null() {
        // A nullable Int64 array with a null at index 0
        let arr: ArrayRef = Arc::new(Int64Array::from(vec![None, Some(1)]));
        // is_null check happens before arrow_value_to_datum in project_aggregate_row_to_slot,
        // but let's verify the array reports null correctly
        assert!(arr.is_null(0));
        assert!(!arr.is_null(1));
    }

    #[pgrx::pg_test]
    fn test_agg_project_arrow_utf8view() {
        let arr: ArrayRef = Arc::new(StringViewArray::from(vec!["world"]));
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::TEXTOID);
        assert!(datum.is_some());
    }
}
