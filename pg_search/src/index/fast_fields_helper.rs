// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use crate::index::reader::index::SearchIndexReader;
use crate::postgres::types::TantivyValue;
use crate::schema::SearchFieldType;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tantivy::columnar::StrColumn;
use tantivy::fastfield::{Column, FastFieldReaders};
use tantivy::schema::OwnedValue;
use tantivy::{DocAddress, DocId};

/// A fast-field index position value.
pub type FFIndex = usize;

type FastFieldReadersCache = Vec<Vec<(FastFieldReaders, String, OnceLock<FFType>)>>;
/// A helper for tracking specific "fast field" readers from a [`SearchIndexReader`] reference
///
/// They're organized by index positions and not names to eliminate as much runtime overhead as
/// possible when looking up the value of a specific fast field.
#[derive(Default)]
pub struct FFHelper(FastFieldReadersCache);
impl FFHelper {
    pub fn empty() -> Self {
        Self(vec![])
    }

    pub fn with_fields(reader: &SearchIndexReader, fields: &[WhichFastField]) -> Self {
        let fast_fields = reader
            .segment_readers()
            .iter()
            .map(|reader| {
                let fast_fields_reader = reader.fast_fields().clone();
                let mut lookup = Vec::new();
                for field in fields {
                    match field {
                        WhichFastField::Named(name, _) => lookup.push((
                            fast_fields_reader.clone(),
                            name.to_string(),
                            OnceLock::default(),
                        )),
                        WhichFastField::Ctid
                        | WhichFastField::TableOid
                        | WhichFastField::Score
                        | WhichFastField::Junk(_) => lookup.push((
                            fast_fields_reader.clone(),
                            String::from("junk"),
                            OnceLock::from(FFType::Junk),
                        )),
                    }
                }
                lookup
            })
            .collect();
        Self(fast_fields)
    }

    #[track_caller]
    pub fn value(&self, field: FFIndex, doc_address: DocAddress) -> Option<TantivyValue> {
        let entry = &self.0[doc_address.segment_ord as usize][field];
        Some(
            entry
                .2
                .get_or_init(|| FFType::new(&entry.0, &entry.1))
                .value(doc_address.doc_id),
        )
    }

    #[track_caller]
    pub fn i64(&self, field: FFIndex, doc_address: DocAddress) -> Option<i64> {
        let entry = &self.0[doc_address.segment_ord as usize][field];
        entry
            .2
            .get_or_init(|| FFType::new(&entry.0, &entry.1))
            .as_i64(doc_address.doc_id)
    }
}

/// Helper for working with different "fast field" types as if they're all one type
#[derive(Debug)]
pub enum FFType {
    Junk,
    Text(StrColumn),
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

    /// Given a [`DocId`], what is its "fast field" value?  In the case of a String field, we
    /// don't reconstruct the full string, and instead return the term ord as a u64
    #[inline(always)]
    #[allow(dead_code)]
    pub fn value_fast(&self, doc: DocId) -> TantivyValue {
        let value = match self {
            FFType::Text(ff) => {
                // just use the first term ord here.  that's enough to do a tie-break quickly
                let ord = ff
                    .term_ords(doc)
                    .next()
                    .expect("term ord should be retrievable");
                TantivyValue(ord.into())
            }
            other => other.value(doc),
        };

        value
    }

    /// Given a [`DocId`], what is its i64 "fast field" value?
    ///
    /// If this [`FFType`] isn't [`FFType::I64`], this function returns [`None`].
    #[inline(always)]
    pub fn as_i64(&self, doc: DocId) -> Option<i64> {
        if let FFType::I64(ff) = self {
            ff.first(doc)
        } else {
            None
        }
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

#[derive(Debug, Clone, Ord, Eq, PartialOrd, PartialEq, Serialize, Deserialize, Hash)]
pub enum WhichFastField {
    Junk(String),
    Ctid,
    TableOid,
    Score,
    Named(String, FastFieldType),
}

#[derive(Debug, Clone, Ord, Eq, PartialOrd, PartialEq, Serialize, Deserialize, Hash)]
pub enum FastFieldType {
    String,
    Numeric,
}

impl From<SearchFieldType> for FastFieldType {
    fn from(value: SearchFieldType) -> Self {
        match value {
            SearchFieldType::Text(_) => FastFieldType::String,
            _ => FastFieldType::Numeric,
        }
    }
}

impl<S: AsRef<str>> From<(S, FastFieldType)> for WhichFastField {
    fn from(value: (S, FastFieldType)) -> Self {
        let name = value.0.as_ref();
        match name {
            "ctid" => WhichFastField::Ctid,
            "tableoid" => WhichFastField::TableOid,
            "paradedb.score()" => WhichFastField::Score,
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
            WhichFastField::Score => "paradedb.score()".into(),
            WhichFastField::Named(s, _) => s.clone(),
        }
    }
}
