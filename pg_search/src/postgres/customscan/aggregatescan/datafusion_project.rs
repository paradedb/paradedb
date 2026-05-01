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
use arrow_array::{Array, RecordBatch};
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
    group_df_indices: &[usize],
) -> *mut pg_sys::TupleTableSlot {
    let tupdesc = (*slot).tts_tupleDescriptor;
    let natts = (*tupdesc).natts as usize;
    let datums = std::slice::from_raw_parts_mut((*slot).tts_values, natts);
    let isnull = std::slice::from_raw_parts_mut((*slot).tts_isnull, natts);

    // Fill GROUP BY columns
    for (i, gc) in targetlist.group_columns.iter().enumerate() {
        let pg_idx = gc.output_index;
        if pg_idx >= natts {
            continue;
        }

        // Use the pre-calculated DataFusion column index for this GROUP BY column
        let df_col_idx = group_df_indices[i];
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
            match crate::postgres::types_arrow::arrow_array_to_datum(
                col.as_ref(),
                row_idx,
                pgrx::PgOid::from(expected_type),
                None,
            ) {
                Ok(Some(datum)) => {
                    datums[pg_idx] = datum;
                    isnull[pg_idx] = false;
                }
                Ok(None) => {
                    isnull[pg_idx] = true;
                    datums[pg_idx] = pg_sys::Datum::null();
                }
                Err(e) => {
                    pgrx::warning!("Aggregate projection failed: {}", e);
                    isnull[pg_idx] = true;
                    datums[pg_idx] = pg_sys::Datum::null();
                }
            }
        }
    }

    // Fill aggregate columns
    // Aggregate columns always follow ALL deduplicated GROUP BY columns in the
    // RecordBatch. The number of deduplicated group columns is the number of
    // unique indices in group_df_indices.
    let num_unique_group_cols = group_df_indices.iter().max().map(|&m| m + 1).unwrap_or(0);
    let mut df_col_idx = num_unique_group_cols;

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
            match crate::postgres::types_arrow::arrow_array_to_datum(
                col.as_ref(),
                row_idx,
                pgrx::PgOid::from(agg.result_type_oid),
                None,
            ) {
                Ok(Some(datum)) => {
                    datums[pg_idx] = datum;
                    isnull[pg_idx] = false;
                }
                Ok(None) => {
                    isnull[pg_idx] = true;
                    datums[pg_idx] = pg_sys::Datum::null();
                }
                Err(e) => {
                    pgrx::warning!("Aggregate projection failed: {}", e);
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

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use arrow_array::*;
    use std::sync::Arc;

    fn arrow_value_to_datum(
        col: &ArrayRef,
        row_idx: usize,
        typoid: pg_sys::Oid,
    ) -> Option<pg_sys::Datum> {
        crate::postgres::types_arrow::arrow_array_to_datum(
            col.as_ref(),
            row_idx,
            pgrx::PgOid::from(typoid),
            None,
        )
        .unwrap_or(None)
    }

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

    #[pgrx::pg_test]
    fn test_agg_project_arrow_int32() {
        let arr: ArrayRef = Arc::new(Int32Array::from(vec![42]));
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::INT4OID);
        assert!(datum.is_some());

        // Int32 → INT8OID (widening)
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::INT8OID);
        assert!(datum.is_some());
    }

    #[pgrx::pg_test]
    fn test_agg_project_arrow_int16() {
        let arr: ArrayRef = Arc::new(Int16Array::from(vec![7]));
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::INT2OID);
        assert!(datum.is_some());

        // Int16 → INT4OID (widening)
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::INT4OID);
        assert!(datum.is_some());
    }

    #[pgrx::pg_test]
    fn test_agg_project_arrow_uint64() {
        let arr: ArrayRef = Arc::new(UInt64Array::from(vec![100u64]));
        // Within i64 range → int64_to_datum
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::INT8OID);
        assert!(datum.is_some());

        // Above i64::MAX → float64_to_datum fallback
        let arr: ArrayRef = Arc::new(UInt64Array::from(vec![u64::MAX]));
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::FLOAT8OID);
        assert!(datum.is_some());
    }

    #[pgrx::pg_test]
    fn test_agg_project_arrow_float32() {
        let arr: ArrayRef = Arc::new(Float32Array::from(vec![1.23f32]));
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::FLOAT4OID);
        assert!(datum.is_some());

        // Float32 → FLOAT8OID (widening)
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::FLOAT8OID);
        assert!(datum.is_some());
    }

    #[pgrx::pg_test]
    fn test_agg_project_arrow_large_utf8() {
        let arr: ArrayRef = Arc::new(LargeStringArray::from(vec!["large string"]));
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::TEXTOID);
        assert!(datum.is_some());
    }

    #[pgrx::pg_test]
    fn test_agg_project_arrow_decimal128() {
        // Decimal128 with scale=2 → NUMERICOID
        let arr: ArrayRef = Arc::new(
            Decimal128Array::from(vec![12345i128])
                .with_precision_and_scale(10, 2)
                .unwrap(),
        );
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::NUMERICOID);
        assert!(datum.is_some());

        // Decimal128 with scale=0 → NUMERICOID (integer-like)
        let arr: ArrayRef = Arc::new(
            Decimal128Array::from(vec![999i128])
                .with_precision_and_scale(10, 0)
                .unwrap(),
        );
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::NUMERICOID);
        assert!(datum.is_some());

        // Decimal128 → FLOAT8OID (non-NUMERIC target)
        let arr: ArrayRef = Arc::new(
            Decimal128Array::from(vec![12345i128])
                .with_precision_and_scale(10, 2)
                .unwrap(),
        );
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::FLOAT8OID);
        assert!(datum.is_some());
    }

    #[pgrx::pg_test]
    fn test_agg_project_int64_to_numeric() {
        // int64_to_datum with NUMERICOID — the SUM(bigint) crash fix
        let arr: ArrayRef = Arc::new(Int64Array::from(vec![9999i64]));
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::NUMERICOID);
        assert!(datum.is_some());
    }

    #[pgrx::pg_test]
    fn test_agg_project_int64_to_float() {
        // int64_to_datum with FLOAT8OID and FLOAT4OID
        let arr: ArrayRef = Arc::new(Int64Array::from(vec![42i64]));

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
        let arr: ArrayRef = Arc::new(Float64Array::from(vec![123.456]));
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::NUMERICOID);
        assert!(datum.is_some());
    }

    #[pgrx::pg_test]
    fn test_agg_project_float64_to_int() {
        // float64_to_datum with integer targets
        let arr: ArrayRef = Arc::new(Float64Array::from(vec![42.0]));

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
        let arr: ArrayRef = Arc::new(arrow_array::TimestampNanosecondArray::from(vec![
            1_000_000_000i64,
        ]));
        let datum = arrow_value_to_datum(&arr, 0, pg_sys::TIMESTAMPOID);
        assert!(datum.is_some());
    }

    // --- Timestamp TimeUnit tests ---

    #[pgrx::pg_test]
    fn test_timestamp_nanosecond_projection() {
        let nanos: i64 = 1_705_314_600_000_000_000; // 2024-01-15 10:30:00 UTC
        let arr: ArrayRef = Arc::new(TimestampNanosecondArray::from(vec![nanos]));
        let result = arrow_value_to_datum(&arr, 0, pg_sys::TIMESTAMPOID);
        assert!(
            result.is_some(),
            "TimestampNanosecond should produce a datum"
        );
    }

    #[pgrx::pg_test]
    fn test_timestamp_microsecond_projection() {
        let micros: i64 = 1_705_314_600_000_000; // 2024-01-15 10:30:00 UTC
        let arr: ArrayRef = Arc::new(TimestampMicrosecondArray::from(vec![micros]));
        let result = arrow_value_to_datum(&arr, 0, pg_sys::TIMESTAMPOID);
        assert!(
            result.is_some(),
            "TimestampMicrosecond should produce a datum"
        );
    }

    #[pgrx::pg_test]
    fn test_timestamp_millisecond_projection() {
        let millis: i64 = 1_705_314_600_000; // 2024-01-15 10:30:00 UTC
        let arr: ArrayRef = Arc::new(TimestampMillisecondArray::from(vec![millis]));
        let result = arrow_value_to_datum(&arr, 0, pg_sys::TIMESTAMPOID);
        assert!(
            result.is_some(),
            "TimestampMillisecond should produce a datum"
        );
    }

    #[pgrx::pg_test]
    fn test_timestamp_second_projection() {
        let secs: i64 = 1_705_314_600; // 2024-01-15 10:30:00 UTC
        let arr: ArrayRef = Arc::new(TimestampSecondArray::from(vec![secs]));
        let result = arrow_value_to_datum(&arr, 0, pg_sys::TIMESTAMPOID);
        assert!(result.is_some(), "TimestampSecond should produce a datum");
    }

    // --- Date32 / Date64 tests ---

    #[pgrx::pg_test]
    fn test_date32_projection() {
        let days: i32 = 19_737; // 2024-01-15 = 19737 days since epoch
        let arr: ArrayRef = Arc::new(Date32Array::from(vec![days]));
        let result = arrow_value_to_datum(&arr, 0, pg_sys::DATEOID);
        assert!(
            result.is_some(),
            "Date32 should produce a datum for DATEOID"
        );
    }

    #[pgrx::pg_test]
    fn test_date64_projection() {
        let millis: i64 = 19_737 * 86_400_000; // 2024-01-15 in milliseconds since epoch
        let arr: ArrayRef = Arc::new(Date64Array::from(vec![millis]));
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
        let arr: ArrayRef = Arc::new(TimestampNanosecondArray::from(vec![nanos]));

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
        let arr: ArrayRef = Arc::new(TimestampNanosecondArray::from(vec![None as Option<i64>]));
        assert!(arr.is_null(0), "Timestamp null should be reported");
    }

    #[pgrx::pg_test]
    fn test_date32_null_reports_correctly() {
        let arr: ArrayRef = Arc::new(Date32Array::from(vec![None as Option<i32>]));
        assert!(arr.is_null(0), "Date32 null should be reported");
    }

    #[pgrx::pg_test]
    fn test_date64_null_reports_correctly() {
        let arr: ArrayRef = Arc::new(Date64Array::from(vec![None as Option<i64>]));
        assert!(arr.is_null(0), "Date64 null should be reported");
    }

    // --- Unsupported type (negative test) ---

    #[pgrx::pg_test]
    fn test_unsupported_arrow_type_returns_none() {
        let arr: ArrayRef = Arc::new(Time64NanosecondArray::from(vec![1_000_000i64]));
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
        let arr: ArrayRef = Arc::new(Date32Array::from(vec![days]));
        let result = arrow_value_to_datum(&arr, 0, pg_sys::DATEOID);
        assert!(result.is_some(), "Pre-epoch Date32 should produce a datum");
    }

    #[pgrx::pg_test]
    fn test_timestamp_pre_epoch() {
        let nanos: i64 = -1_000_000_000; // 1969-12-31 23:59:59 UTC
        let arr: ArrayRef = Arc::new(TimestampNanosecondArray::from(vec![nanos]));
        let result = arrow_value_to_datum(&arr, 0, pg_sys::TIMESTAMPOID);
        assert!(
            result.is_some(),
            "Pre-epoch timestamp should produce a datum"
        );
    }
}
