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

//! The encoding of a deferred string/bytes column.
//!
//! A deferred column is a `UInt64` column. A row is a packed doc address (State 0) or a
//! packed term ordinal (State 1), told apart by the top bit, and a NULL is an Arrow NULL.
//! One primitive word per row is what makes the column cheap to carry: a join's `take`
//! copies one buffer and keeps a validity bitmap for the rows it null-extends, a group-by
//! hashes and compares the word itself, and the fetch changes a row's state without
//! changing the column's type, so it can run in the scan or anywhere above it.

use arrow_array::{Array, ArrayRef, UInt64Array};
use arrow_buffer::{NullBuffer, ScalarBuffer};
use arrow_schema::{DataType, Field};
use datafusion::common::{DataFusionError, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tantivy::termdict::TermOrdinal;
use tantivy::{DocId, SegmentOrdinal};

/// The Arrow extension name on a deferred column's field, which is how a schema tells one
/// apart from any other `UInt64` column.
pub const EXTENSION_NAME: &str = "tantivy_deferred";
const EXTENSION_KEY: &str = "ARROW:extension:name";

/// Set on a packed term ordinal, clear on a packed doc address.
const TERM_ORDINAL_BIT: u64 = 1 << 63;
/// A term ordinal keeps the low 40 bits of the word and the segment ordinal the 23 above
/// them. A segment holds fewer than 2^32 documents, so no dictionary comes near 2^40 terms.
const TERM_ORDINAL_BITS: u32 = 40;
const TERM_ORDINAL_MASK: u64 = (1 << TERM_ORDINAL_BITS) - 1;
const MAX_STATE1_SEGMENT_ORD: u64 = (1 << (63 - TERM_ORDINAL_BITS)) - 1;
/// A doc address keeps the doc id in the low 32 bits and the segment ordinal above it.
const MAX_STATE0_SEGMENT_ORD: u64 = (1 << 31) - 1;

pub fn deferred_data_type() -> DataType {
    DataType::UInt64
}

/// The metadata a deferred column's field carries.
pub fn deferred_metadata() -> HashMap<String, String> {
    [(EXTENSION_KEY.to_string(), EXTENSION_NAME.to_string())].into()
}

pub fn deferred_field(name: &str) -> Field {
    Field::new(name, deferred_data_type(), true).with_metadata(deferred_metadata())
}

pub fn is_deferred_field(field: &Field) -> bool {
    field.data_type() == &DataType::UInt64
        && field.metadata().get(EXTENSION_KEY).map(String::as_str) == Some(EXTENSION_NAME)
}

/// Packs a segment ordinal and a doc id into one word (State 0).
///
/// This preserves enough Tantivy row identity to resolve a surviving row back to a real
/// ctid or a term ordinal later, without paying for either up front.
pub fn pack_doc_address(segment_ord: SegmentOrdinal, doc_id: DocId) -> u64 {
    assert!(
        (segment_ord as u64) <= MAX_STATE0_SEGMENT_ORD,
        "segment ordinal {segment_ord} does not fit a packed doc address"
    );
    ((segment_ord as u64) << 32) | (doc_id as u64)
}

pub fn pack_doc_addresses(segment_ord: SegmentOrdinal, doc_ids: &[DocId]) -> UInt64Array {
    UInt64Array::from_iter_values(
        doc_ids
            .iter()
            .map(|doc_id| pack_doc_address(segment_ord, *doc_id)),
    )
}

/// Unpacks a doc address into its segment ordinal and doc id.
pub fn unpack_doc_address(packed: u64) -> (SegmentOrdinal, DocId) {
    debug_assert_eq!(packed & TERM_ORDINAL_BIT, 0, "not a doc address");
    ((packed >> 32) as u32, (packed & 0xFFFF_FFFF) as u32)
}

/// Packs a segment ordinal and a term ordinal in that segment's dictionary into one word
/// (State 1). Equal words name equal terms of one segment, which is what a group-by on the
/// column relies on.
pub fn pack_term_ordinal(segment_ord: SegmentOrdinal, term_ord: TermOrdinal) -> u64 {
    assert!(
        (segment_ord as u64) <= MAX_STATE1_SEGMENT_ORD && term_ord <= TERM_ORDINAL_MASK,
        "segment ordinal {segment_ord} or term ordinal {term_ord} does not fit a packed term ordinal"
    );
    TERM_ORDINAL_BIT | ((segment_ord as u64) << TERM_ORDINAL_BITS) | term_ord
}

/// State 0 for one segment's rows.
pub fn build_state_doc_address(segment_ord: SegmentOrdinal, doc_ids: &[DocId]) -> ArrayRef {
    Arc::new(pack_doc_addresses(segment_ord, doc_ids))
}

/// State 1 for one segment's rows, from their raw ordinals (NULL where the row has no value).
pub fn build_state_term_ordinals(segment_ord: SegmentOrdinal, ordinals: &UInt64Array) -> ArrayRef {
    Arc::new(UInt64Array::from_iter(
        ordinals
            .iter()
            .map(|ord| ord.map(|ord| pack_term_ordinal(segment_ord, ord))),
    ))
}

/// One row of a deferred column, as read through [`DeferredColumn`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeferredValue {
    /// State 0: the row still carries its packed `(segment_ord, doc_id)`.
    DocAddress(u64),
    /// State 1: the row's term ordinal in `segment_ord`'s dictionary.
    TermOrdinal {
        segment_ord: SegmentOrdinal,
        term_ord: TermOrdinal,
    },
    /// The row's value is NULL, or the row was null-extended by an outer join.
    Null,
}

/// Typed read access to a deferred column, so the fetch, the decode and the segmented
/// Top-K share one reading of the encoding.
pub struct DeferredColumn<'a> {
    values: &'a UInt64Array,
}

impl<'a> DeferredColumn<'a> {
    pub fn try_new(array: &'a dyn Array) -> Result<Self> {
        let values = array
            .as_any()
            .downcast_ref::<UInt64Array>()
            .ok_or_else(|| {
                DataFusionError::Execution(format!(
                    "expected a deferred UInt64 column, found {:?}",
                    array.data_type()
                ))
            })?;
        Ok(Self { values })
    }

    pub fn value(&self, row: usize) -> DeferredValue {
        if self.values.is_null(row) {
            return DeferredValue::Null;
        }
        let packed = self.values.value(row);
        if packed & TERM_ORDINAL_BIT == 0 {
            DeferredValue::DocAddress(packed)
        } else {
            DeferredValue::TermOrdinal {
                segment_ord: ((packed & !TERM_ORDINAL_BIT) >> TERM_ORDINAL_BITS) as u32,
                term_ord: packed & TERM_ORDINAL_MASK,
            }
        }
    }

    pub fn values(&self) -> impl Iterator<Item = DeferredValue> + '_ {
        (0..self.values.len()).map(|row| self.value(row))
    }

    /// A copy of the column with `resolved` rows written as term ordinals. A row resolved to
    /// no ordinal becomes NULL; every other row keeps its value.
    pub fn with_term_ordinals(
        &self,
        resolved: impl IntoIterator<Item = (usize, SegmentOrdinal, Option<TermOrdinal>)>,
    ) -> ArrayRef {
        let mut words: Vec<u64> = self.values.values().to_vec();
        let mut valid: Vec<bool> = (0..words.len())
            .map(|row| self.values.is_valid(row))
            .collect();
        for (row, segment_ord, term_ord) in resolved {
            match term_ord {
                Some(term_ord) => {
                    words[row] = pack_term_ordinal(segment_ord, term_ord);
                    valid[row] = true;
                }
                None => valid[row] = false,
            }
        }
        Arc::new(UInt64Array::new(
            ScalarBuffer::from(words),
            Some(NullBuffer::from(valid)),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let packed_max = pack_doc_address(MAX_STATE0_SEGMENT_ORD as u32, u32::MAX);
        assert_eq!(
            unpack_doc_address(packed_max),
            (MAX_STATE0_SEGMENT_ORD as u32, u32::MAX)
        );
        assert_eq!(unpack_doc_address(0), (0, 0));
    }

    #[test]
    fn pack_doc_addresses_empty() {
        let packed = pack_doc_addresses(0, &[]);
        assert_eq!(packed.len(), 0);
    }

    #[test]
    fn a_term_ordinal_keeps_its_segment_and_ordinal() {
        let packed = pack_term_ordinal(MAX_STATE1_SEGMENT_ORD as u32, TERM_ORDINAL_MASK);
        let array = UInt64Array::from(vec![packed]);
        let column = DeferredColumn::try_new(&array).unwrap();
        assert_eq!(
            column.value(0),
            DeferredValue::TermOrdinal {
                segment_ord: MAX_STATE1_SEGMENT_ORD as u32,
                term_ord: TERM_ORDINAL_MASK,
            }
        );
    }

    #[test]
    fn the_two_states_never_collide() {
        let doc = pack_doc_address(7, 8);
        let ord = pack_term_ordinal(7, 8);
        assert_ne!(doc, ord);
        assert_eq!(doc & TERM_ORDINAL_BIT, 0);
        assert_ne!(ord & TERM_ORDINAL_BIT, 0);
    }

    #[test]
    fn a_deferred_field_is_told_apart_from_a_plain_integer() {
        assert!(is_deferred_field(&deferred_field("category")));
        assert!(!is_deferred_field(&Field::new(
            "id",
            DataType::UInt64,
            true
        )));
        assert!(!is_deferred_field(
            &Field::new("category", DataType::Utf8View, true).with_metadata(deferred_metadata())
        ));
    }

    #[test]
    fn deferred_column_reads_both_states_and_nulls() {
        let state_0 = build_state_doc_address(3, &[7, 8]);
        let view = DeferredColumn::try_new(state_0.as_ref()).unwrap();
        assert_eq!(view.values().count(), 2);
        assert_eq!(
            view.value(0),
            DeferredValue::DocAddress(pack_doc_address(3, 7))
        );

        let ordinals = UInt64Array::from(vec![Some(5), None]);
        let state_1 = build_state_term_ordinals(1, &ordinals);
        let view = DeferredColumn::try_new(state_1.as_ref()).unwrap();
        assert_eq!(
            view.values().collect::<Vec<_>>(),
            vec![
                DeferredValue::TermOrdinal {
                    segment_ord: 1,
                    term_ord: 5
                },
                DeferredValue::Null,
            ]
        );

        let not_deferred = arrow_array::Int64Array::from(vec![1]);
        assert!(DeferredColumn::try_new(&not_deferred).is_err());
    }

    #[test]
    fn resolving_rows_keeps_the_others_and_nulls_the_missing() {
        let state_0 = build_state_doc_address(2, &[1, 2, 3]);
        let view = DeferredColumn::try_new(state_0.as_ref()).unwrap();
        let resolved = view.with_term_ordinals([(0, 2, Some(40)), (2, 2, None)]);
        let view = DeferredColumn::try_new(resolved.as_ref()).unwrap();
        assert_eq!(
            view.values().collect::<Vec<_>>(),
            vec![
                DeferredValue::TermOrdinal {
                    segment_ord: 2,
                    term_ord: 40
                },
                DeferredValue::DocAddress(pack_doc_address(2, 2)),
                DeferredValue::Null,
            ]
        );
    }
}
