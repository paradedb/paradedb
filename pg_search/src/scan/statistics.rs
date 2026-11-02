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

//! Metadata-only estimates for logical scalar fast fields.

use arrow_schema::DataType;
use datafusion::common::ScalarValue;
use datafusion::common::stats::{ColumnStatistics, Precision};

use crate::index::fast_fields_helper::{FFHelper, FFType, FieldCardinality, WhichFastField};
use crate::schema::SearchFieldType;

/// These estimates describe the unfiltered reader. Dictionaries can overlap between segments,
/// deleted rows can remain in metadata, and predicates can narrow the output, so every value is
/// inexact. Never open a mutable segment: opening its fast fields materializes heap rows.
/// Callers decoding deferred columns may request their logical type; packed values themselves
/// must never inherit these statistics.
pub(crate) fn column_statistics(
    ffhelper: &FFHelper,
    ff_index: usize,
    data_type: &DataType,
    estimated_rows: Option<usize>,
) -> ColumnStatistics {
    let Some(WhichFastField::Named {
        field_type,
        cardinality: FieldCardinality::Scalar,
        ..
    }) = ffhelper.fields().get(ff_index)
    else {
        return ColumnStatistics::default();
    };
    if matches!(field_type, SearchFieldType::Vector(..))
        || &field_type.arrow_data_type() != data_type
        || !matches!(
            data_type,
            DataType::Int64
                | DataType::UInt64
                | DataType::Float64
                | DataType::Utf8View
                | DataType::BinaryView
        )
    {
        return ColumnStatistics::default();
    }

    let mut statistics = ffhelper
        .statistics(ff_index)
        .get_or_init(|| metadata_statistics(ffhelper, ff_index, data_type))
        .clone();
    if let (Precision::Inexact(distinct_count), Some(rows)) =
        (&mut statistics.distinct_count, estimated_rows)
    {
        *distinct_count = (*distinct_count).min(rows);
    }
    statistics
}

fn metadata_statistics(
    ffhelper: &FFHelper,
    ff_index: usize,
    data_type: &DataType,
) -> ColumnStatistics {
    // One mutable segment makes the whole-column estimate unknown. Check every identity
    // before opening even the immutable segments' columns.
    if (0..ffhelper.num_segments())
        .any(|ord| ffhelper.immutable_segment_reader(ord as u32).is_none())
    {
        return ColumnStatistics::default();
    }
    let name = ffhelper.fields()[ff_index].name();
    let dictionary = matches!(data_type, DataType::Utf8View | DataType::BinaryView);
    let mut distinct_count = 0usize;
    let mut min_value: Option<ScalarValue> = None;
    let mut max_value: Option<ScalarValue> = None;
    for segment_ord in 0..ffhelper.num_segments() {
        let Some(segment) = ffhelper.immutable_segment_reader(segment_ord as u32) else {
            return ColumnStatistics::default();
        };
        if segment.num_docs() == 0 {
            continue;
        }
        // An absent column (e.g. an all-null JSON path) contributes no values. Unlike
        // FFHelper::column, a metadata probe need not require the column to exist.
        let Some(column) = FFType::try_new(segment.fast_fields(), &name) else {
            continue;
        };
        macro_rules! bounds {
            ($column:expr, $variant:ident) => {{
                let column = $column;
                if column.values.num_vals() == 0 {
                    continue;
                }
                (
                    ScalarValue::$variant(Some(column.min_value())),
                    ScalarValue::$variant(Some(column.max_value())),
                )
            }};
        }
        let (min, max) = match (&column, data_type) {
            (FFType::Text(column), DataType::Utf8View) => {
                distinct_count = distinct_count.saturating_add(column.num_terms());
                continue;
            }
            (FFType::Bytes(column), DataType::BinaryView) => {
                distinct_count = distinct_count.saturating_add(column.num_terms());
                continue;
            }
            (FFType::I64(column), DataType::Int64) => bounds!(column, Int64),
            (FFType::U64(column), DataType::UInt64) => bounds!(column, UInt64),
            (FFType::F64(column), DataType::Float64) => {
                if column.values.num_vals() == 0 {
                    continue;
                }
                let (min, max) = (column.min_value(), column.max_value());
                if !min.is_finite() || !max.is_finite() {
                    return ColumnStatistics::default();
                }
                (
                    ScalarValue::Float64(Some(min)),
                    ScalarValue::Float64(Some(max)),
                )
            }
            _ => return ColumnStatistics::default(),
        };
        if min_value.as_ref().is_none_or(|current| min < *current) {
            min_value = Some(min);
        }
        if max_value.as_ref().is_none_or(|current| max > *current) {
            max_value = Some(max);
        }
    }

    ColumnStatistics {
        distinct_count: if dictionary {
            Precision::Inexact(distinct_count)
        } else {
            Precision::Absent
        },
        min_value: min_value.map_or(Precision::Absent, Precision::Inexact),
        max_value: max_value.map_or(Precision::Absent, Precision::Inexact),
        ..ColumnStatistics::default()
    }
}
