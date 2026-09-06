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
use crate::postgres::customscan::datafusion::numeric_agg::decode_avg_blob;
use crate::postgres::types_arrow::decimal_bytes_to_anynumeric;
use arrow_array::cast::AsArray;
use arrow_array::{Array, RecordBatch};
use pgrx::{AnyNumeric, IntoDatum, JsonB, pg_sys};

/// Project a single row from an aggregate `RecordBatch` into a Postgres `TupleTableSlot`.
///
/// The DataFusion output schema is: `[group_col_0, ..., group_col_N, agg_0, ..., agg_M]`.
/// Each column is mapped to the correct position in the Postgres tuple via `output_index`.
/// `pdb.agg()` entries have no column of their own; their assembled documents come
/// in `pdb_agg_json`, one per entry in target-list order.
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
    pdb_agg_json: Vec<serde_json::Value>,
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
    let mut pdb_agg_json = pdb_agg_json.into_iter();

    for agg in &targetlist.aggregates {
        let pg_idx = agg.output_index;
        if let AggKind::PdbAgg(_) = agg.agg_kind {
            let document = pdb_agg_json
                .next()
                .expect("one assembled document per pdb.agg entry");
            if pg_idx < natts {
                datums[pg_idx] = JsonB(document).into_datum().expect("jsonb datum");
                isnull[pg_idx] = false;
            }
            continue;
        }
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
            // Aggregate results arrive in the fast field's storage encoding. A
            // numeric AVG carries its row count beside the sum as
            // `[count u64 BE, decimal-bytes sum]` and divides through
            // `AnyNumeric` so the result scale follows Postgres' numeric
            // division rules, matching a non-pushed-down AVG. Everything else
            // converts straight out of Arrow with the column's declared scale.
            let datum = match (&agg.agg_kind, agg.numeric) {
                (AggKind::Avg, Some(_)) => {
                    let blob = col.as_binary::<i32>().value(row_idx);
                    let (count, sum_bytes) = decode_avg_blob(blob)
                        .unwrap_or_else(|e| panic!("BUG: failed to decode numeric AVG blob: {e}"));
                    (count != 0)
                        .then(|| {
                            let sum =
                                decimal_bytes_to_anynumeric(sum_bytes, None).unwrap_or_else(|e| {
                                    panic!("BUG: failed to decode numeric AVG sum: {e}")
                                });
                            (sum / AnyNumeric::from(count as i64)).into_datum()
                        })
                        .flatten()
                }
                (_, numeric) => crate::postgres::types_arrow::arrow_array_to_datum(
                    col.as_ref(),
                    row_idx,
                    pgrx::PgOid::from(agg.result_type_oid),
                    numeric.and_then(|field_type| field_type.numeric_scale()),
                )
                .unwrap_or_else(|e| panic!("BUG: Aggregate projection failed: {e}")),
            };
            match datum {
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
