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

<<<<<<< HEAD
use arrow_array::{new_empty_array, ArrayRef, StructArray, UInt32Array, UInt64Array, UnionArray};
=======
use arrow_array::{
    Array, ArrayRef, StructArray, UInt32Array, UInt64Array, UnionArray, new_empty_array,
};
>>>>>>> 1c2667ad (refactor: decouple column fetching from string decoding (#6219))
use arrow_buffer::ScalarBuffer;
use arrow_schema::{DataType, Field, UnionFields, UnionMode};
use datafusion::common::{DataFusionError, Result};
use std::sync::Arc;
use tantivy::termdict::TermOrdinal;
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
    let seg_array = UInt32Array::from(vec![segment_ord; ordinals.len()]);
    build_state_term_ordinals_per_row(seg_array, ordinals)
}

/// State 1 with a segment ordinal per row, for a batch whose rows were fetched out of
/// several segments (anything above a join no longer groups rows by segment).
pub fn build_state_term_ordinals_per_row(
    segment_ords: UInt32Array,
    ordinals: ArrayRef,
) -> ArrayRef {
    let len = ordinals.len();
    let fields = deferred_union_fields();
    let type_ids = ScalarBuffer::from(vec![1_i8; len]);
    let offsets = ScalarBuffer::from((0..len).map(|i| i as i32).collect::<Vec<_>>());

    let term_ord_struct = Arc::new(
        StructArray::try_new(
            if let DataType::Struct(f) = term_ordinal_type() {
                f.clone()
            } else {
                unreachable!()
            },
            vec![Arc::new(segment_ords) as ArrayRef, ordinals],
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

/// One row of a deferred column, as read through [`DeferredUnion`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeferredValue {
    /// State 0: the row still carries its packed `(segment_ord, doc_id)`.
    DocAddress(u64),
    /// State 1: the row's term ordinal in `segment_ord`'s dictionary. `None` is a NULL term.
    TermOrdinal {
        segment_ord: SegmentOrdinal,
        term_ord: Option<TermOrdinal>,
    },
    /// The row is NULL in either state.
    Null,
}

/// Typed read access to a deferred column's dense 2-way `UnionArray`, so the fetch and
/// decode nodes share one reading of the encoding.
pub struct DeferredUnion<'a> {
    type_ids: &'a [i8],
    offsets: &'a [i32],
    doc_addresses: &'a UInt64Array,
    term_ords: &'a StructArray,
    segment_ords: &'a UInt32Array,
    ordinals: &'a UInt64Array,
}

impl<'a> DeferredUnion<'a> {
    pub fn try_new(array: &'a dyn Array) -> Result<Self> {
        let union = array.as_any().downcast_ref::<UnionArray>().ok_or_else(|| {
            DataFusionError::Execution(format!(
                "expected a deferred UnionArray, found {:?}",
                array.data_type()
            ))
        })?;
        let offsets = union.offsets().ok_or_else(|| {
            DataFusionError::Execution(
                "expected dense union with offsets in deferred column".into(),
            )
        })?;
        let doc_addresses = union
            .child(0)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .ok_or_else(|| {
                DataFusionError::Execution(
                    "expected UInt64Array for doc_address child in deferred union".into(),
                )
            })?;
        let term_ords = union
            .child(1)
            .as_any()
            .downcast_ref::<StructArray>()
            .ok_or_else(|| {
                DataFusionError::Execution(
                    "expected StructArray for term_ord child in deferred union".into(),
                )
            })?;
        let segment_ords = term_ords
            .column(0)
            .as_any()
            .downcast_ref::<UInt32Array>()
            .ok_or_else(|| {
                DataFusionError::Execution("expected UInt32Array for seg_ord column".into())
            })?;
        let ordinals = term_ords
            .column(1)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .ok_or_else(|| {
                DataFusionError::Execution("expected UInt64Array for term_ord column".into())
            })?;
        Ok(Self {
            type_ids: union.type_ids(),
            offsets,
            doc_addresses,
            term_ords,
            segment_ords,
            ordinals,
        })
    }

    pub fn value(&self, row: usize) -> DeferredValue {
        let ci = self.offsets[row] as usize;
        match self.type_ids[row] {
            0 => {
                if self.doc_addresses.is_null(ci) {
                    DeferredValue::Null
                } else {
                    DeferredValue::DocAddress(self.doc_addresses.value(ci))
                }
            }
            1 => {
                if self.term_ords.is_null(ci) || self.segment_ords.is_null(ci) {
                    DeferredValue::Null
                } else {
                    DeferredValue::TermOrdinal {
                        segment_ord: self.segment_ords.value(ci),
                        term_ord: (!self.ordinals.is_null(ci)).then(|| self.ordinals.value(ci)),
                    }
                }
            }
            other => unreachable!("invalid deferred union state {other}"),
        }
    }

    pub fn values(&self) -> impl Iterator<Item = DeferredValue> + '_ {
        (0..self.type_ids.len()).map(|row| self.value(row))
    }
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
    fn deferred_union_reads_both_states_and_nulls() {
        let state_0 = build_state_doc_address(3, &[7, 8]);
        let view = DeferredUnion::try_new(state_0.as_ref()).unwrap();
        assert_eq!(view.values().count(), 2);
        assert_eq!(
            view.value(0),
            DeferredValue::DocAddress(pack_doc_addresses(3, &[7]).value(0))
        );

        let ordinals: ArrayRef = Arc::new(UInt64Array::from(vec![Some(5), None]));
        let state_1 = build_state_term_ordinals_per_row(UInt32Array::from(vec![1, 2]), ordinals);
        let view = DeferredUnion::try_new(state_1.as_ref()).unwrap();
        assert_eq!(
            view.values().collect::<Vec<_>>(),
            vec![
                DeferredValue::TermOrdinal {
                    segment_ord: 1,
                    term_ord: Some(5)
                },
                DeferredValue::TermOrdinal {
                    segment_ord: 2,
                    term_ord: None
                },
            ]
        );

        let not_a_union: ArrayRef = Arc::new(UInt64Array::from(vec![1]));
        assert!(DeferredUnion::try_new(not_a_union.as_ref()).is_err());
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
