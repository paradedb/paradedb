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

use super::join_targetlist::{
    AggKind, JoinAggregateTargetList, SupportedWindowAggType, WindowAggKind,
};
use crate::postgres::customscan::datafusion::numeric_agg::decode_avg_blob;
use crate::postgres::types_arrow::decimal_bytes_to_anynumeric;
use crate::schema::SearchFieldType;
use arrow_array::cast::AsArray;
use arrow_array::{Array, RecordBatch};
use pgrx::{AnyNumeric, IntoDatum, JsonB, pg_sys};

fn write_slot_datum(
    datums: &mut [pg_sys::Datum],
    isnull: &mut [bool],
    pg_idx: usize,
    datum: Option<pg_sys::Datum>,
) {
    match datum {
        Some(datum) => {
            datums[pg_idx] = datum;
            isnull[pg_idx] = false;
        }
        None => {
            datums[pg_idx] = pg_sys::Datum::null();
            isnull[pg_idx] = true;
        }
    }
}

/// Project a single row from an aggregate `RecordBatch` into a Postgres `TupleTableSlot`.
///
/// The DataFusion output schema is:
/// `[group_col_0, ..., group_col_N, agg_0, ..., agg_M, row_val_0, ..., window_agg_0, ...]`.
/// The raw Postgres tuple has layout: `[groups..., aggregates..., window_aggregates..., resjunk...]`.
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
    num_group_exprs: usize,
    pdb_agg_json: Vec<serde_json::Value>,
) {
    let tupdesc = (*slot).tts_tupleDescriptor;
    let natts = (*tupdesc).natts as usize;
    let raw_columns = targetlist.group_columns.len()
        + targetlist.aggregates.len()
        + targetlist.window_aggregates.len();
    assert!(
        natts >= raw_columns,
        "aggregate raw tuple must contain every group, aggregate, and window aggregate column"
    );
    let datums = std::slice::from_raw_parts_mut((*slot).tts_values, natts);
    let isnull = std::slice::from_raw_parts_mut((*slot).tts_isnull, natts);

    // Fill GROUP BY columns
    for (pg_idx, gc) in targetlist.group_columns.iter().enumerate() {
        // Use the pre-calculated DataFusion column index for this GROUP BY column
        let df_col_idx = group_df_indices[pg_idx];
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

        let datum = crate::postgres::types_arrow::arrow_array_to_datum(
            col.as_ref(),
            row_idx,
            pgrx::PgOid::from(expected_type),
            gc.numeric_scale,
        )
        .unwrap_or_else(|e| panic!("BUG: Aggregate projection failed: {e}"));
        write_slot_datum(datums, isnull, pg_idx, datum);
    }

    // Aggregate columns follow the deduplicated GROUP BY columns in the
    // RecordBatch.
    let mut df_col_idx = num_group_exprs;
    let mut pdb_agg_json = pdb_agg_json.into_iter();

    for (offset, agg) in targetlist.aggregates.iter().enumerate() {
        let pg_idx = targetlist.group_columns.len() + offset;
        if let AggKind::PdbAgg(_) = agg.agg_kind {
            let document = pdb_agg_json
                .next()
                .expect("one assembled document per pdb.agg entry");
            write_slot_datum(datums, isnull, pg_idx, JsonB(document).into_datum());
            continue;
        }

        let col = batch.column(df_col_idx);
        let datum = datafusion_agg_to_datum(
            matches!(agg.agg_kind, AggKind::Avg),
            agg.numeric.as_ref(),
            agg.result_type_oid,
            col.as_ref(),
            row_idx,
        )
        .unwrap_or_else(|e| panic!("BUG: Aggregate projection failed: {e}"));
        write_slot_datum(datums, isnull, pg_idx, datum);
        df_col_idx += 1;
    }

    // Row-value group columns sit between standard SQL aggregates and window
    // aggregates in the RecordBatch.
    if !targetlist.window_aggregates.is_empty() {
        let num_row_values = targetlist
            .group_columns
            .iter()
            .filter(|gc| gc.row_value)
            .count();
        df_col_idx += num_row_values;
        let win_base = targetlist.group_columns.len() + targetlist.aggregates.len();
        for (offset, win) in targetlist.window_aggregates.iter().enumerate() {
            let pg_idx = win_base + offset;
            let col = batch.column(df_col_idx);
            let datum = match &win.kind {
                WindowAggKind::CombinePdbAgg(_) => {
                    crate::postgres::customscan::datafusion::pdb_agg_udaf::json_document_to_datum(
                        col.as_ref(),
                        row_idx,
                    )
                    .unwrap_or_else(|e| {
                        panic!("BUG: Window pdb.combine_agg projection failed: {e}")
                    })
                }
                WindowAggKind::SqlAgg { op, numeric, .. } => datafusion_agg_to_datum(
                    matches!(op, SupportedWindowAggType::Avg),
                    numeric.as_ref(),
                    win.result_type_oid,
                    col.as_ref(),
                    row_idx,
                )
                .unwrap_or_else(|e| panic!("BUG: Window aggregate projection failed: {e}")),
            };
            write_slot_datum(datums, isnull, pg_idx, datum);
            df_col_idx += 1;
        }
    }

    // Positions past the raw columns are resjunk predicate Vars appended by
    // add_vars_to_tlist. ExecProject never reads them; null them rather than
    // leave stale values behind.
    datums[raw_columns..].fill(pg_sys::Datum::null());
    isnull[raw_columns..].fill(true);

    // Mark slot as non-empty
    (*slot).tts_flags &= !(pg_sys::TTS_FLAG_EMPTY as u16);
    (*slot).tts_nvalid = natts as i16;
}

/// Aggregate results arrive in the fast field's storage encoding. A
/// numeric AVG carries its row count beside the sum as
/// `[count u64 BE, decimal-bytes sum]` and divides through
/// `AnyNumeric` so the result scale follows Postgres' numeric
/// division rules, matching a non-pushed-down AVG. Everything else
/// converts straight out of Arrow with the column's declared scale.
pub fn datafusion_agg_to_datum(
    is_avg: bool,
    field_type: Option<&SearchFieldType>,
    result_type_oid: pg_sys::Oid,
    col: &dyn Array,
    row_idx: usize,
) -> anyhow::Result<Option<pg_sys::Datum>> {
    // A NULL aggregate value decodes to NULL regardless of encoding. Checked
    // up front because the numeric AVG blob arm below reads the raw bytes,
    // where a null entry would surface as an empty (undecodable) slice.
    // The Null *type* check is separate: a NullArray (e.g. the constant a
    // pruned window argument becomes) has no validity bitmap, so its
    // `is_null()` reports false even though every entry is logically null.
    if col.data_type() == &arrow_schema::DataType::Null || col.is_null(row_idx) {
        return Ok(None);
    }
    let numeric_field = field_type.filter(|f| f.is_numeric());
    match (is_avg, numeric_field) {
        (true, Some(_)) => {
            let blob = col.as_binary::<i32>().value(row_idx);
            let (count, sum_bytes) = decode_avg_blob(blob)
                .unwrap_or_else(|e| panic!("BUG: failed to decode numeric AVG blob: {e}"));
            let res = (count != 0)
                .then(|| {
                    let sum = decimal_bytes_to_anynumeric(sum_bytes, None)
                        .unwrap_or_else(|e| panic!("BUG: failed to decode numeric AVG sum: {e}"));
                    (sum / AnyNumeric::from(count as i64)).into_datum()
                })
                .flatten();
            Ok(res)
        }
        (_, numeric) => crate::postgres::types_arrow::arrow_array_to_datum(
            col,
            row_idx,
            result_type_oid.into(),
            numeric.and_then(|field_type| field_type.numeric_scale()),
        ),
    }
}
