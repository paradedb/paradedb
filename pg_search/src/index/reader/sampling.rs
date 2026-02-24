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

use arrow_array::{
    Array, ArrayRef, BinaryViewArray, BooleanArray, Float64Array, Int64Array, StringViewArray,
    TimestampNanosecondArray, UInt64Array,
};
use datafusion::common::{DataFusionError, Result};
use tantivy::SegmentReader;

use crate::index::fast_fields_helper::FFType;
use crate::postgres::types::TantivyValue;

/// Sample the given fast field for a number of documents, and return the sorted values.
pub fn sample_fast_field(
    reader: &SegmentReader,
    field_name: &str,
    num_samples: usize,
) -> Result<Vec<TantivyValue>> {
    let num_docs = reader.num_docs();
    if num_docs == 0 {
        return Ok(vec![]);
    }

    let fast_fields = reader.fast_fields();
    // This might panic if the field doesn't exist or isn't a fast field,
    // which is consistent with existing FFType behavior.
    let ff_type = FFType::new(fast_fields, field_name);

    let step = (num_docs as usize) / num_samples.max(1);
    let step = step.max(1);

    let ids: Vec<u32> = (0..num_docs)
        .step_by(step)
        .take(num_samples.min(num_docs as usize))
        .collect();

    if ids.is_empty() {
        return Ok(vec![]);
    }

    let array = ff_type.fetch_arrow_array(&ids)?;

    // Sort the array
    let sorted_indices = arrow_ord::sort::sort_to_indices(&array, None, None)
        .map_err(|e| DataFusionError::Internal(format!("Failed to sort sampled values: {e}")))?;
    let sorted_array = arrow_select::take::take(&array, &sorted_indices, None)
        .map_err(|e| DataFusionError::Internal(format!("Failed to take sorted values: {e}")))?;

    // Convert to TantivyValues
    // We return ALL sorted samples. The caller (leader) will pick the split points.
    array_to_tantivy_values(&sorted_array)
}

fn array_to_tantivy_values(array: &ArrayRef) -> Result<Vec<TantivyValue>> {
    use tantivy::schema::OwnedValue;

    let mut values = Vec::with_capacity(array.len());

    if let Some(a) = array.as_any().downcast_ref::<StringViewArray>() {
        for i in 0..a.len() {
            if a.is_null(i) {
                values.push(TantivyValue(OwnedValue::Null));
            } else {
                values.push(TantivyValue(OwnedValue::Str(a.value(i).to_string())));
            }
        }
    } else if let Some(a) = array.as_any().downcast_ref::<BinaryViewArray>() {
        for i in 0..a.len() {
            if a.is_null(i) {
                values.push(TantivyValue(OwnedValue::Null));
            } else {
                values.push(TantivyValue(OwnedValue::Bytes(a.value(i).to_vec())));
            }
        }
    } else if let Some(a) = array.as_any().downcast_ref::<Int64Array>() {
        for i in 0..a.len() {
            if a.is_null(i) {
                values.push(TantivyValue(OwnedValue::Null));
            } else {
                values.push(TantivyValue(OwnedValue::I64(a.value(i))));
            }
        }
    } else if let Some(a) = array.as_any().downcast_ref::<Float64Array>() {
        for i in 0..a.len() {
            if a.is_null(i) {
                values.push(TantivyValue(OwnedValue::Null));
            } else {
                values.push(TantivyValue(OwnedValue::F64(a.value(i))));
            }
        }
    } else if let Some(a) = array.as_any().downcast_ref::<UInt64Array>() {
        for i in 0..a.len() {
            if a.is_null(i) {
                values.push(TantivyValue(OwnedValue::Null));
            } else {
                values.push(TantivyValue(OwnedValue::U64(a.value(i))));
            }
        }
    } else if let Some(a) = array.as_any().downcast_ref::<BooleanArray>() {
        for i in 0..a.len() {
            if a.is_null(i) {
                values.push(TantivyValue(OwnedValue::Null));
            } else {
                values.push(TantivyValue(OwnedValue::Bool(a.value(i))));
            }
        }
    } else if let Some(a) = array.as_any().downcast_ref::<TimestampNanosecondArray>() {
        for i in 0..a.len() {
            if a.is_null(i) {
                values.push(TantivyValue(OwnedValue::Null));
            } else {
                let nanos = a.value(i);
                let dt = tantivy::DateTime::from_timestamp_nanos(nanos);
                values.push(TantivyValue(OwnedValue::Date(dt)));
            }
        }
    } else {
        return Err(DataFusionError::Internal(format!(
            "Unsupported arrow array type for sampling: {:?}",
            array.data_type()
        )));
    }

    Ok(values)
}
