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

/// The output mode for deferred column values.
///
/// `DocAddress` emits a `UnionArray` of `DocAddress` (State 0) or `TermOrdinal` (State 1).
/// `TermOrdinal` emits a `StructArray` containing `segment_ord` and `term_ord` directly.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize, Hash)]
pub enum DeferMode {
    #[default]
    DocAddress,
    TermOrdinal,
}

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
///
/// This preserves enough Tantivy row identity to resolve a surviving row back to a
/// real ctid later, without paying heap-access costs up front.
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
pub fn deferred_union_fields() -> UnionFields {
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
    ];
    UnionFields::try_new(vec![0, 1], fields).expect("Failed to create UnionFields")
}

/// The schema definition for our 2-way UnionArray
pub fn deferred_union_data_type() -> DataType {
    DataType::Union(deferred_union_fields(), UnionMode::Dense)
}

// State 0
pub fn build_state_doc_address(segment_ord: SegmentOrdinal, doc_ids: &[DocId]) -> ArrayRef {
    let len = doc_ids.len();
    let fields = deferred_union_fields();
    let type_ids = ScalarBuffer::from(vec![0_i8; len]);
    let offsets = ScalarBuffer::from((0..len).map(|i| i as i32).collect::<Vec<_>>());

    let children: Vec<ArrayRef> = vec![
        Arc::new(pack_doc_addresses(segment_ord, doc_ids)),
        new_empty_array(fields[1].1.data_type()),
    ];

    Arc::new(
        UnionArray::try_new(fields, type_ids, Some(offsets), children)
            .expect("Failed to construct State 0 UnionArray"),
    )
}

// State 1
pub fn build_state_term_ordinals(segment_ord: SegmentOrdinal, ordinals: ArrayRef) -> ArrayRef {
    let len = ordinals.len();
    let fields = deferred_union_fields();
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

    let children: Vec<ArrayRef> = vec![new_empty_array(fields[0].1.data_type()), term_ord_struct];

    Arc::new(
        UnionArray::try_new(fields, type_ids, Some(offsets), children)
            .expect("Failed to construct State 1 UnionArray"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_array::Array;

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
        let array = build_state_doc_address(1, &[5, 10]);
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
        let array = build_state_term_ordinals(2, ordinals);
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
}
