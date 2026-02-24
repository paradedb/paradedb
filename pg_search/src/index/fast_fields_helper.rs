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

use std::sync::OnceLock;

use crate::index::reader::index::SearchIndexReader;
use crate::postgres::types::TantivyValue;
use crate::schema::SearchFieldType;

use serde::{Deserialize, Serialize};
use tantivy::columnar::{BytesColumn, StrColumn};
use tantivy::fastfield::{Column, FastFieldReaders};
use tantivy::schema::OwnedValue;
use tantivy::termdict::TermOrdinal;
use tantivy::SegmentOrdinal;
use tantivy::{DocAddress, DocId};

use arrow_array::builder::{BinaryViewBuilder, StringViewBuilder};
use arrow_array::ArrayRef;
use arrow_buffer::Buffer;

/// A fast-field index position value.
pub type FFIndex = usize;

/// A cache of fast field columns for a single segment, indexed by FFIndex.
type ColumnCache = Vec<(String, OnceLock<FFType>)>;

/// A helper for tracking specific "fast field" readers from a [`SearchIndexReader`] reference
///
/// They're organized by index positions and not names to eliminate as much runtime overhead as
/// possible when looking up the value of a specific fast field.
#[derive(Default)]
pub struct FFHelper {
    // A cache of columns and a ctid column for each segment.
    segment_caches: Vec<(FastFieldReaders, ColumnCache, OnceLock<FFType>)>,
}

impl FFHelper {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn with_fields(reader: &SearchIndexReader, fields: &[WhichFastField]) -> Self {
        let segment_caches = reader
            .segment_readers()
            .iter()
            .map(|reader| {
                let fast_fields_reader = reader.fast_fields().clone();
                let mut lookup = Vec::new();
                for field in fields {
                    match field {
                        WhichFastField::Named(name, _) | WhichFastField::Deferred(name, _, _) => {
                            lookup.push((name.to_string(), OnceLock::default()))
                        }
                        WhichFastField::Ctid
                        | WhichFastField::TableOid
                        | WhichFastField::Score
                        | WhichFastField::Junk(_) => {
                            lookup.push((String::from("junk"), OnceLock::from(FFType::Junk)))
                        }
                    }
                }
                (fast_fields_reader, lookup, OnceLock::default())
            })
            .collect();
        Self { segment_caches }
    }

    pub fn ctid(&self, segment_ord: SegmentOrdinal) -> &FFType {
        let (ff_readers, _, ctid) = &self.segment_caches[segment_ord as usize];
        ctid.get_or_init(|| FFType::new_ctid(ff_readers))
    }

    pub fn column(&self, segment_ord: SegmentOrdinal, field: FFIndex) -> &FFType {
        let (ff_readers, columns, _) = &self.segment_caches[segment_ord as usize];
        let column = &columns[field];
        column.1.get_or_init(|| FFType::new(ff_readers, &column.0))
    }

    #[track_caller]
    pub fn value(&self, field: FFIndex, doc_address: DocAddress) -> Option<TantivyValue> {
        let (ff_readers, columns, _) = &self.segment_caches[doc_address.segment_ord as usize];
        let column = &columns[field];
        Some(
            column
                .1
                .get_or_init(|| FFType::new(ff_readers, &column.0))
                .value(doc_address.doc_id),
        )
    }
}

/// Helper for working with different "fast field" types as if they're all one type.
///
/// This enum is used *after* a column is open to provide a typed wrapper around the underlying
/// Tantivy column readers.
#[derive(Debug)]
pub enum FFType {
    Junk,
    Text(StrColumn),
    Bytes(BytesColumn),
    I64(Column<i64>),
    F64(Column<f64>),
    U64(Column<u64>),
    Bool(Column<bool>),
    Date(Column<tantivy::DateTime>),
}

impl FFType {
    /// Construct the proper [`FFType`] for the internal `ctid` field, which
    /// should be a known field name in the Tantivy index
    pub fn new_ctid(ffr: &FastFieldReaders) -> Self {
        Self::U64(ffr.u64("ctid").expect("ctid should be a u64 fast field"))
    }

    /// Construct the proper [`FFType`] for the specified `field_name`, which
    /// should be a known field name in the Tantivy index
    #[track_caller]
    pub fn new(ffr: &FastFieldReaders, field_name: &str) -> Self {
        if let Ok(ff) = ffr.i64(field_name) {
            Self::I64(ff)
        } else if let Ok(Some(ff)) = ffr.str(field_name) {
            Self::Text(ff)
        } else if let Ok(Some(ff)) = ffr.bytes(field_name) {
            Self::Bytes(ff)
        } else if let Ok(ff) = ffr.u64(field_name) {
            Self::U64(ff)
        } else if let Ok(ff) = ffr.f64(field_name) {
            Self::F64(ff)
        } else if let Ok(ff) = ffr.bool(field_name) {
            Self::Bool(ff)
        } else if let Ok(ff) = ffr.date(field_name) {
            Self::Date(ff)
        } else {
            panic!("`{field_name}` is missing or is not configured as a fast field")
        }
    }

    /// Given a [`DocId`], what is its "fast field" value?
    #[inline(always)]
    pub fn value(&self, doc: DocId) -> TantivyValue {
        let value = match self {
            FFType::Junk => TantivyValue(OwnedValue::Null),
            FFType::Text(ff) => {
                let mut s = String::new();
                let ord = ff
                    .term_ords(doc)
                    .next()
                    .expect("term ord should be retrievable");
                ff.ord_to_str(ord, &mut s)
                    .expect("string should be retrievable for term ord");
                TantivyValue(s.into())
            }
            FFType::Bytes(ff) => {
                let mut bytes = Vec::new();
                let ord = ff
                    .term_ords(doc)
                    .next()
                    .expect("term ord should be retrievable");
                ff.ord_to_bytes(ord, &mut bytes)
                    .expect("bytes should be retrievable for term ord");
                TantivyValue(OwnedValue::Bytes(bytes))
            }
            FFType::I64(ff) => TantivyValue(
                ff.first(doc)
                    .map(|first| first.into())
                    .unwrap_or(OwnedValue::Null),
            ),
            FFType::F64(ff) => TantivyValue(
                ff.first(doc)
                    .map(|first| first.into())
                    .unwrap_or(OwnedValue::Null),
            ),
            FFType::U64(ff) => TantivyValue(
                ff.first(doc)
                    .map(|first| first.into())
                    .unwrap_or(OwnedValue::Null),
            ),
            FFType::Bool(ff) => TantivyValue(
                ff.first(doc)
                    .map(|first| first.into())
                    .unwrap_or(OwnedValue::Null),
            ),
            FFType::Date(ff) => TantivyValue(
                ff.first(doc)
                    .map(|first| first.into())
                    .unwrap_or(OwnedValue::Null),
            ),
        };

        value
    }

    /// Given a [`DocId`], what is its u64 "fast field" value?
    ///
    /// If this [`FFType`] isn't [`FFType::U64`], this function returns [`None`].
    #[inline(always)]
    pub fn as_u64(&self, doc: DocId) -> Option<u64> {
        if let FFType::U64(ff) = self {
            ff.first(doc)
        } else {
            None
        }
    }

    /// Given [`DocId`]s, what are their u64 "fast field" values?
    ///
    /// The given `output` slice must be the same length as the docs slice.
    #[inline(always)]
    pub fn as_u64s(&self, docs: &[DocId], output: &mut [Option<u64>]) {
        let FFType::U64(ff) = self else {
            panic!("Expected a u64 column.");
        };
        ff.first_vals(docs, output);
    }
}

/// A request for a specific fast field, used *before* the column is open.
///
/// This enum allows consumers to specify which columns to retrieve and their expected types.
///
/// # Type Widening
///
/// Currently, we "widen" various Postgres types into larger underlying storage types (e.g.
/// based on how they are stored in Tantivy). For instance, JSON and UUID are both stored as Strings.
/// The consumer of the data (e.g. the Arrow conversion layer) is responsible for interpreting
/// these widened types back into their original Postgres OIDs via `SearchFieldType::typeoid()`.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub enum WhichFastField {
    Junk(String),
    Ctid,
    TableOid,
    Score,
    Named(String, SearchFieldType),
    Deferred(String, SearchFieldType, bool),
}

impl<S: AsRef<str>> From<(S, SearchFieldType)> for WhichFastField {
    fn from(value: (S, SearchFieldType)) -> Self {
        let name = value.0.as_ref();
        match name {
            "ctid" => WhichFastField::Ctid,
            "tableoid" => WhichFastField::TableOid,
            "pdb.score()" => WhichFastField::Score,
            other => {
                if other.starts_with("junk(") && other.ends_with(")") {
                    WhichFastField::Junk(String::from(
                        other.trim_start_matches("junk(").trim_end_matches(")"),
                    ))
                } else {
                    WhichFastField::Named(String::from(other), value.1)
                }
            }
        }
    }
}

impl WhichFastField {
    pub fn name(&self) -> String {
        match self {
            WhichFastField::Junk(s) => format!("junk({s})"),
            WhichFastField::Ctid => "ctid".into(),
            WhichFastField::TableOid => "tableoid".into(),
            WhichFastField::Score => "pdb.score()".into(),
            WhichFastField::Named(s, _) => s.clone(),
            WhichFastField::Deferred(s, _, _) => s.clone(),
        }
    }

    /// Returns the SearchFieldType if this is a Named fast field, None otherwise.
    pub fn field_type(&self) -> Option<&SearchFieldType> {
        match self {
            WhichFastField::Named(_, field_type) => Some(field_type),
            WhichFastField::Deferred(_, field_type, _) => Some(field_type),
            _ => None,
        }
    }

    /// Returns the Arrow DataType for this fast field.
    pub fn arrow_data_type(&self) -> arrow_schema::DataType {
        use arrow_schema::DataType;
        match self {
            WhichFastField::Ctid => DataType::UInt64,
            WhichFastField::TableOid => DataType::UInt32,
            WhichFastField::Score => DataType::Float32,
            WhichFastField::Named(_, field_type) => field_type.arrow_data_type(),
            WhichFastField::Junk(_) => DataType::Null,
            WhichFastField::Deferred(_, _, is_bytes) => {
                crate::scan::deferred_encode::deferred_union_data_type(*is_bytes)
            }
        }
    }
}

/// Build an Arrow schema from a list of fast fields.
///
/// This is used by Scanner and MixedFastFieldExecState to create consistent
/// Arrow schemas for DataFusion execution.
pub fn build_arrow_schema(which_fast_fields: &[WhichFastField]) -> arrow_schema::SchemaRef {
    use arrow_schema::{Field, Schema};
    use std::sync::Arc;

    let fields: Vec<Field> = which_fast_fields
        .iter()
        .map(|wff| Field::new(wff.name(), wff.arrow_data_type(), true))
        .collect();
    Arc::new(Schema::new(fields))
}

pub(crate) const NULL_TERM_ORDINAL: TermOrdinal = u64::MAX;

/// `NULL_TERM_ORDINAL` represents NULL, and will be emitted last in the sorted order.
pub(crate) fn ords_to_string_array(
    str_ff: StrColumn,
    term_ords: impl IntoIterator<Item = TermOrdinal>,
) -> ArrayRef {
    // Enumerate the term ordinals to preserve their positions, and then sort them by ordinal.
    let mut term_ords = term_ords.into_iter().enumerate().collect::<Vec<_>>();
    term_ords.sort_unstable_by_key(|(_, term_ord)| *term_ord);

    // Iterate over the sorted term ordinals: as we visit each term ordinal, we will append the
    // term to a StringViewBuilder's data buffer, and record a view to be appended later in sorted
    // order.
    let mut builder = StringViewBuilder::with_capacity(term_ords.len());
    let mut views: Vec<Option<(u32, u32)>> = Vec::with_capacity(term_ords.len());
    views.resize(term_ords.len(), None);

    let mut buffer = Vec::new();
    let mut bytes = Vec::new();
    let mut current_block_addr = str_ff.dictionary().sstable_index.get_block_with_ord(0);
    let mut current_sstable_delta_reader = str_ff
        .dictionary()
        .sstable_delta_reader_block(current_block_addr.clone())
        .expect("Failed to open term dictionary.");
    let mut current_ordinal = 0;
    let mut previous_term: Option<(TermOrdinal, (u32, u32))> = None;
    for (row_idx, ord) in term_ords {
        if ord == NULL_TERM_ORDINAL {
            // NULL_TERM_ORDINAL sorts highest, so all remaining ords will have `None` views, and
            // be appended to the builder as null.
            break;
        }

        // only advance forward if the new ord is different than the one we just processed
        //
        // this allows the input TermOrdinal iterator to contain and reuse duplicates, so long as
        // it's still sorted
        match &previous_term {
            Some((previous_ord, previous_view)) if *previous_ord == ord => {
                // This is the same term ordinal: reuse the previous view.
                views[row_idx] = Some(*previous_view);
                continue;
            }
            // Fall through.
            _ => {}
        }

        // This is a new term ordinal: decode it and append it to the builder.
        assert!(ord >= current_ordinal);
        // check if block changed for new term_ord
        let new_block_addr = str_ff.dictionary().sstable_index.get_block_with_ord(ord);
        if new_block_addr != current_block_addr {
            current_block_addr = new_block_addr;
            current_ordinal = current_block_addr.first_ordinal;
            current_sstable_delta_reader = str_ff
                .dictionary()
                .sstable_delta_reader_block(current_block_addr.clone())
                .unwrap_or_else(|e| panic!("Failed to fetch next dictionary block: {e}"));
            bytes.clear();
        }

        // Move to ord inside that block
        for _ in current_ordinal..=ord {
            match current_sstable_delta_reader.advance() {
                Ok(true) => {}
                Ok(false) => {
                    panic!("Term ordinal {ord} did not exist in the dictionary.");
                }
                Err(e) => {
                    panic!("Failed to decode dictionary block: {e}")
                }
            }
            bytes.truncate(current_sstable_delta_reader.common_prefix_len());
            bytes.extend_from_slice(current_sstable_delta_reader.suffix());
        }
        current_ordinal = ord + 1;

        // Set the view for this row_idx.
        let offset: u32 = buffer
            .len()
            .try_into()
            .expect("Too many terms requested in `ords_to_string_array`");
        let len: u32 = bytes
            .len()
            .try_into()
            .expect("Single term is too long in `ords_to_string_array`");
        buffer.extend_from_slice(&bytes);
        previous_term = Some((ord, (offset, len)));
        views[row_idx] = Some((offset, len));
    }

    // Append all the rows' views to the builder.
    let block_no = builder.append_block(Buffer::from(buffer));
    for view in views {
        // Each view is an offset and len in our single block, or None for a null.
        match view {
            Some((offset, len)) => unsafe {
                builder.append_view_unchecked(block_no, offset, len);
            },
            None => builder.append_null(),
        }
    }

    std::sync::Arc::new(builder.finish())
}

/// Given an unordered collection of TermOrdinals for the given BytesColumn, return a
/// `BinaryViewArray` with one row per input term ordinal (in the input order).
///
/// This is identical to `ords_to_string_array` but uses `BinaryViewBuilder` for binary data.
///
/// `NULL_TERM_ORDINAL` represents NULL, and will be emitted last in the sorted order.
pub(crate) fn ords_to_bytes_array(
    bytes_ff: BytesColumn,
    term_ords: impl IntoIterator<Item = TermOrdinal>,
) -> ArrayRef {
    // Enumerate the term ordinals to preserve their positions, and then sort them by ordinal.
    let mut term_ords = term_ords.into_iter().enumerate().collect::<Vec<_>>();
    term_ords.sort_unstable_by_key(|(_, term_ord)| *term_ord);

    // Iterate over the sorted term ordinals: as we visit each term ordinal, we will append the
    // term to a BinaryViewBuilder's data buffer, and record a view to be appended later in sorted
    // order.
    let mut builder = BinaryViewBuilder::with_capacity(term_ords.len());
    let mut views: Vec<Option<(u32, u32)>> = Vec::with_capacity(term_ords.len());
    views.resize(term_ords.len(), None);

    let mut buffer = Vec::new();
    let mut bytes = Vec::new();
    let mut current_block_addr = bytes_ff.dictionary().sstable_index.get_block_with_ord(0);
    let mut current_sstable_delta_reader = bytes_ff
        .dictionary()
        .sstable_delta_reader_block(current_block_addr.clone())
        .expect("Failed to open term dictionary.");
    let mut current_ordinal = 0;
    let mut previous_term: Option<(TermOrdinal, (u32, u32))> = None;
    for (row_idx, ord) in term_ords {
        if ord == NULL_TERM_ORDINAL {
            // NULL_TERM_ORDINAL sorts highest, so all remaining ords will have `None` views, and
            // be appended to the builder as null.
            break;
        }

        // only advance forward if the new ord is different than the one we just processed
        //
        // this allows the input TermOrdinal iterator to contain and reuse duplicates, so long as
        // it's still sorted
        match &previous_term {
            Some((previous_ord, previous_view)) if *previous_ord == ord => {
                // This is the same term ordinal: reuse the previous view.
                views[row_idx] = Some(*previous_view);
                continue;
            }
            // Fall through.
            _ => {}
        }

        // This is a new term ordinal: decode it and append it to the builder.
        assert!(ord >= current_ordinal);
        // check if block changed for new term_ord
        let new_block_addr = bytes_ff.dictionary().sstable_index.get_block_with_ord(ord);
        if new_block_addr != current_block_addr {
            current_block_addr = new_block_addr;
            current_ordinal = current_block_addr.first_ordinal;
            current_sstable_delta_reader = bytes_ff
                .dictionary()
                .sstable_delta_reader_block(current_block_addr.clone())
                .unwrap_or_else(|e| panic!("Failed to fetch next dictionary block: {e}"));
            bytes.clear();
        }

        // Move to ord inside that block
        for _ in current_ordinal..=ord {
            match current_sstable_delta_reader.advance() {
                Ok(true) => {}
                Ok(false) => {
                    panic!("Term ordinal {ord} did not exist in the dictionary.");
                }
                Err(e) => {
                    panic!("Failed to decode dictionary block: {e}")
                }
            }
            bytes.truncate(current_sstable_delta_reader.common_prefix_len());
            bytes.extend_from_slice(current_sstable_delta_reader.suffix());
        }
        current_ordinal = ord + 1;

        // Set the view for this row_idx.
        let offset: u32 = buffer
            .len()
            .try_into()
            .expect("Too many terms requested in `ords_to_bytes_array`");
        let len: u32 = bytes
            .len()
            .try_into()
            .expect("Single term is too long in `ords_to_bytes_array`");
        buffer.extend_from_slice(&bytes);
        previous_term = Some((ord, (offset, len)));
        views[row_idx] = Some((offset, len));
    }

    // Append all the rows' views to the builder.
    let block_no = builder.append_block(Buffer::from(buffer));
    for view in views {
        // Each view is an offset and len in our single block, or None for a null.
        match view {
            Some((offset, len)) => unsafe {
                builder.append_view_unchecked(block_no, offset, len);
            },
            None => builder.append_null(),
        }
    }

    std::sync::Arc::new(builder.finish())
}
