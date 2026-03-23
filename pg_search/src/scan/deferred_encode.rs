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

use arrow_array::{new_empty_array, ArrayRef, StructArray, UInt32Array, UInt64Array, UnionArray};
use arrow_buffer::ScalarBuffer;
use arrow_schema::{DataType, Field, UnionFields, UnionMode};
use std::sync::Arc;
use tantivy::{DocId, SegmentOrdinal};

pub const EXTENSION_DOC_ADDRESS: &str = "tantivy_doc_address";
pub const EXTENSION_TERM_ORDINAL: &str = "tantivy_term_ordinal";

// In Arrow, extension types are defined by the underlying storage type,
// and the extension name is attached as metadata to the Field later!
pub fn doc_address_type() -> DataType {
    DataType::UInt64
}

pub fn term_ordinal_type() -> DataType {
    // We use a Struct to safely hold both a u32 segment_ord and a u64 term_ord
    DataType::Struct(
        vec![
            Field::new("segment_ord", DataType::UInt32, false),
            Field::new("term_ord", DataType::UInt64, true),
        ]
        .into(),
    )
}

/// Packs segment ordinals and doc IDs into a single 64-bit integer array.
pub fn pack_doc_addresses(segment_ord: SegmentOrdinal, doc_ids: &[DocId]) -> UInt64Array {
    let mut b = arrow_array::builder::UInt64Builder::with_capacity(doc_ids.len());
    for doc_id in doc_ids {
        let packed = ((segment_ord as u64) << 32) | (*doc_id as u64);
        b.append_value(packed);
    }
    b.finish()
}

/// Unpacks a 64-bit integer into its segment ordinal and doc ID.
pub fn unpack_doc_address(packed: u64) -> (u32, u32) {
    let seg_ord = (packed >> 32) as u32;
    let doc_id = (packed & 0xFFFF_FFFF) as u32;
    (seg_ord, doc_id)
}

/// Helper to get just the UnionFields (required by UnionArray::try_new)
pub fn deferred_union_fields(is_bytes: bool) -> UnionFields {
    let fields = vec![
        Field::new("doc_address", doc_address_type(), true).with_metadata(
            [(
                "ARROW:extension:name".to_string(),
                EXTENSION_DOC_ADDRESS.to_string(),
            )]
            .into(),
        ),
        Field::new("term_ordinal", term_ordinal_type(), true).with_metadata(
            [(
                "ARROW:extension:name".to_string(),
                EXTENSION_TERM_ORDINAL.to_string(),
            )]
            .into(),
        ),
        Field::new(
            "materialized",
            if is_bytes {
                DataType::BinaryView
            } else {
                DataType::Utf8View
            },
            true,
        ),
    ];
    UnionFields::try_new(vec![0, 1, 2], fields).expect("Failed to create UnionFields")
}

/// The schema definition for our 3-way UnionArray
pub fn deferred_union_data_type(is_bytes: bool) -> DataType {
    DataType::Union(deferred_union_fields(is_bytes), UnionMode::Dense)
}

// State 0
pub fn build_state_doc_address(
    segment_ord: SegmentOrdinal,
    doc_ids: &[DocId],
    is_bytes: bool,
) -> ArrayRef {
    let len = doc_ids.len();
    let fields = deferred_union_fields(is_bytes);
    let type_ids = ScalarBuffer::from(vec![0_i8; len]);
    let offsets = ScalarBuffer::from((0..len).map(|i| i as i32).collect::<Vec<_>>());

    let children: Vec<ArrayRef> = vec![
        Arc::new(pack_doc_addresses(segment_ord, doc_ids)),
        new_empty_array(fields[1].1.data_type()),
        new_empty_array(fields[2].1.data_type()),
    ];

    Arc::new(
        UnionArray::try_new(fields, type_ids, Some(offsets), children)
            .expect("Failed to construct State 0 UnionArray"),
    )
}

// State 1
pub fn build_state_term_ordinals(
    segment_ord: SegmentOrdinal,
    ordinals: ArrayRef,
    is_bytes: bool,
) -> ArrayRef {
    let len = ordinals.len();
    let fields = deferred_union_fields(is_bytes);
    let type_ids = ScalarBuffer::from(vec![1_i8; len]);
    let offsets = ScalarBuffer::from((0..len).map(|i| i as i32).collect::<Vec<_>>());

    let seg_array = Arc::new(UInt32Array::from(vec![segment_ord; len])) as ArrayRef;
    let term_ord_struct = Arc::new(
        StructArray::try_new(
            if let DataType::Struct(f) = term_ordinal_type() {
                f.clone()
            } else {
                unreachable!()
            },
            vec![seg_array, ordinals],
            None,
        )
        .unwrap(),
    ) as ArrayRef;

    let children: Vec<ArrayRef> = vec![
        new_empty_array(fields[0].1.data_type()),
        term_ord_struct,
        new_empty_array(fields[2].1.data_type()),
    ];

    Arc::new(
        UnionArray::try_new(fields, type_ids, Some(offsets), children)
            .expect("Failed to construct State 1 UnionArray"),
    )
}

// State 2
pub fn build_state_hydrated(materialized: ArrayRef, is_bytes: bool) -> ArrayRef {
    let len = materialized.len();
    let fields = deferred_union_fields(is_bytes);
    let type_ids = ScalarBuffer::from(vec![2_i8; len]);
    let offsets = ScalarBuffer::from((0..len).map(|i| i as i32).collect::<Vec<_>>());

    let children: Vec<ArrayRef> = vec![
        new_empty_array(fields[0].1.data_type()),
        new_empty_array(fields[1].1.data_type()),
        materialized,
    ];

    Arc::new(
        UnionArray::try_new(fields, type_ids, Some(offsets), children)
            .expect("Failed to construct State 2 UnionArray"),
    )
}

/// Extracts the underlying materialized string/bytes data type from a deferred Union schema field.
/// The Union array uses a 3-state encoding:
/// 0 = doc_address (UInt64)
/// 1 = term_ordinal (Struct)
/// 2 = materialized (Utf8View / BinaryView)
pub fn extract_materialized_type_from_union(
    union_fields: &arrow_schema::UnionFields,
) -> arrow_schema::DataType {
    // The materialized type is always safely located at index 2
    union_fields
        .iter()
        .nth(2)
        .expect("Deferred Union schema is missing the materialized field variant")
        .1
        .data_type()
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_array::{Array, StringViewArray};

    #[test]
    fn pack_unpack_roundtrip() {
        let packed = pack_doc_addresses(3, &[10, 20, 30]);
        assert_eq!(packed.len(), 3);
        assert_eq!(unpack_doc_address(packed.value(0)), (3, 10));
        assert_eq!(unpack_doc_address(packed.value(1)), (3, 20));
        assert_eq!(unpack_doc_address(packed.value(2)), (3, 30));
    }

    #[test]
    fn pack_unpack_boundary_values() {
        let packed_max = ((u32::MAX as u64) << 32) | (u32::MAX as u64);
        assert_eq!(unpack_doc_address(packed_max), (u32::MAX, u32::MAX));
        assert_eq!(unpack_doc_address(0), (0, 0));
    }

    #[test]
    fn pack_doc_addresses_empty() {
        let packed = pack_doc_addresses(0, &[]);
        assert_eq!(packed.len(), 0);
    }

    #[test]
    fn build_state_doc_address_creates_state_0() {
        let array = build_state_doc_address(1, &[5, 10], false);
        let union_array = array.as_any().downcast_ref::<UnionArray>().unwrap();
        assert_eq!(union_array.len(), 2);
        assert!(union_array.type_ids().iter().all(|&id| id == 0));
        let child = union_array.child(0);
        let uint64_child = child.as_any().downcast_ref::<UInt64Array>().unwrap();
        assert_eq!(unpack_doc_address(uint64_child.value(0)), (1, 5));
        assert_eq!(unpack_doc_address(uint64_child.value(1)), (1, 10));
    }

    #[test]
    fn build_state_term_ordinals_creates_state_1() {
        let ordinals: ArrayRef = Arc::new(UInt64Array::from(vec![Some(100), Some(200)]));
        let array = build_state_term_ordinals(2, ordinals, false);
        let union_array = array.as_any().downcast_ref::<UnionArray>().unwrap();
        assert_eq!(union_array.len(), 2);
        assert!(union_array.type_ids().iter().all(|&id| id == 1));
        let child = union_array.child(1);
        let struct_array = child.as_any().downcast_ref::<StructArray>().unwrap();
        let seg_ords = struct_array
            .column(0)
            .as_any()
            .downcast_ref::<UInt32Array>()
            .unwrap();
        assert_eq!(seg_ords.value(0), 2);
        assert_eq!(seg_ords.value(1), 2);
        let term_ords = struct_array
            .column(1)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap();
        assert_eq!(term_ords.value(0), 100);
        assert_eq!(term_ords.value(1), 200);
    }

    #[test]
    fn build_state_hydrated_creates_state_2() {
        let strings: ArrayRef = Arc::new(StringViewArray::from(vec!["hello", "world"]));
        let array = build_state_hydrated(strings, false);
        let union_array = array.as_any().downcast_ref::<UnionArray>().unwrap();
        assert_eq!(union_array.len(), 2);
        assert!(union_array.type_ids().iter().all(|&id| id == 2));
        let child = union_array.child(2);
        let string_array = child.as_any().downcast_ref::<StringViewArray>().unwrap();
        assert_eq!(string_array.value(0), "hello");
        assert_eq!(string_array.value(1), "world");
    }

    #[test]
    fn deferred_union_fields_text_vs_bytes() {
        let text_fields = deferred_union_fields(false);
        let bytes_fields = deferred_union_fields(true);
        // Fields 0 and 1 are identical
        assert_eq!(text_fields[0].1.data_type(), bytes_fields[0].1.data_type());
        assert_eq!(text_fields[1].1.data_type(), bytes_fields[1].1.data_type());
        // Field 2 differs: Utf8View for text, BinaryView for bytes
        assert_eq!(*text_fields[2].1.data_type(), DataType::Utf8View);
        assert_eq!(*bytes_fields[2].1.data_type(), DataType::BinaryView);
    }
}
