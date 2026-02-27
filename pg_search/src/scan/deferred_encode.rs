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

use arrow_array::{new_null_array, ArrayRef, StructArray, UInt32Array, UInt64Array, UnionArray};
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
    DataType::Union(deferred_union_fields(is_bytes), UnionMode::Sparse)
}

// State 0
pub fn build_state_doc_address(
    segment_ord: SegmentOrdinal,
    doc_ids: &[DocId],
    is_bytes: bool,
) -> ArrayRef {
    let len = doc_ids.len();
    let type_ids = ScalarBuffer::from(vec![0_i8; len]);

    let doc_addr_array = Arc::new(pack_doc_addresses(segment_ord, doc_ids)) as ArrayRef;
    let term_ord_array = new_null_array(&term_ordinal_type(), len);
    let mat_array = new_null_array(
        &if is_bytes {
            DataType::BinaryView
        } else {
            DataType::Utf8View
        },
        len,
    );

    let union_array = UnionArray::try_new(
        deferred_union_fields(is_bytes), // Pass the Fields, not the DataType!
        type_ids,
        None,
        vec![doc_addr_array, term_ord_array, mat_array],
    )
    .expect("Failed to construct State 0 UnionArray");

    Arc::new(union_array)
}

// State 1
pub fn build_state_term_ordinals(
    segment_ord: SegmentOrdinal,
    ordinals: ArrayRef,
    is_bytes: bool,
) -> ArrayRef {
    let len = ordinals.len();
    let type_ids = ScalarBuffer::from(vec![1_i8; len]);

    let doc_addr_array = new_null_array(&doc_address_type(), len);

    // Build the StructArray containing the Segment ID and the Ordinals
    let seg_array = Arc::new(UInt32Array::from(vec![segment_ord; len])) as ArrayRef;
    let term_ord_array = Arc::new(
        StructArray::try_new(
            if let DataType::Struct(fields) = term_ordinal_type() {
                fields.clone()
            } else {
                unreachable!()
            },
            vec![seg_array, ordinals],
            None,
        )
        .unwrap(),
    ) as ArrayRef;

    let mat_array = new_null_array(
        &if is_bytes {
            DataType::BinaryView
        } else {
            DataType::Utf8View
        },
        len,
    );

    let union_array = UnionArray::try_new(
        deferred_union_fields(is_bytes),
        type_ids,
        None,
        vec![doc_addr_array, term_ord_array, mat_array],
    )
    .expect("Failed to construct State 1 UnionArray");

    Arc::new(union_array)
}

// State 2
pub fn build_state_hydrated(materialized: ArrayRef, is_bytes: bool) -> ArrayRef {
    let len = materialized.len();
    let type_ids = ScalarBuffer::from(vec![2_i8; len]);

    let doc_addr_array = new_null_array(&doc_address_type(), len);
    let term_ord_array = new_null_array(&term_ordinal_type(), len);
    let mat_array = materialized; // Passed straight from the memoized cache!

    let union_array = UnionArray::try_new(
        deferred_union_fields(is_bytes),
        type_ids,
        None,
        vec![doc_addr_array, term_ord_array, mat_array],
    )
    .expect("Failed to construct State 2 UnionArray");

    Arc::new(union_array)
}
