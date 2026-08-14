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

use super::join_targetlist::{AggKind, JoinAggregateEntry, JoinAggregateTargetList};
use crate::postgres::customscan::datafusion::numeric_agg::decode_avg_blob;
use arrow_array::cast::AsArray;
use arrow_array::{Array, ArrayRef, RecordBatch};
use decimal_bytes::Decimal;
use pgrx::{AnyNumeric, IntoDatum, pg_sys};

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
                gc.numeric_scale,
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
                    panic!("BUG: Aggregate projection failed: {}", e);
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
            match agg_value_to_datum(agg, col, row_idx) {
                Ok(Some(datum)) => {
                    datums[pg_idx] = datum;
                    isnull[pg_idx] = false;
                }
                Ok(None) => {
                    isnull[pg_idx] = true;
                    datums[pg_idx] = pg_sys::Datum::null();
                }
                Err(e) => {
                    panic!("BUG: Aggregate projection failed: {}", e);
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

/// Aggregate results arrive in the fast field's storage encoding. A numeric
/// AVG carries its row count beside the sum, and the other numeric results
/// need the column's declared scale to render. Everything else converts
/// straight out of Arrow.
fn agg_value_to_datum(
    agg: &JoinAggregateEntry,
    col: &ArrayRef,
    row_idx: usize,
) -> Result<Option<pg_sys::Datum>, String> {
    match (&agg.agg_kind, agg.numeric) {
        (AggKind::Avg, Some(_)) => numeric_avg_to_datum(col, row_idx),
        (_, numeric) => crate::postgres::types_arrow::arrow_array_to_datum(
            col.as_ref(),
            row_idx,
            pgrx::PgOid::from(agg.result_type_oid),
            numeric.and_then(|field_type| field_type.numeric_scale()),
        ),
    }
}

/// Convert a numeric AVG blob (`[count u64 BE, decimal-bytes sum]`) into a
/// NUMERIC datum. The division runs through `AnyNumeric` so the result scale
/// follows Postgres' numeric division rules, matching a non-pushed-down AVG.
fn numeric_avg_to_datum(col: &ArrayRef, row_idx: usize) -> Result<Option<pg_sys::Datum>, String> {
    let blob = col.as_binary::<i32>().value(row_idx);
    let (count, sum_bytes) = decode_avg_blob(blob).map_err(|e| e.to_string())?;
    if count == 0 {
        return Ok(None);
    }
    let sum = Decimal::from_bytes(sum_bytes)
        .map_err(|e| format!("failed to decode numeric AVG sum: {e:?}"))?;
    let sum: AnyNumeric = sum
        .to_string()
        .parse()
        .map_err(|e| format!("failed to parse numeric AVG sum: {e}"))?;
    let count = AnyNumeric::from(count as i64);
    Ok((sum / count).into_datum())
}
