// Copyright (c) 2023-2024 Retake, Inc.
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

#![allow(dead_code)]
use crate::index::reader::SearchIndexReader;
use crate::postgres::types::TantivyValue;
use crate::schema::SearchFieldType;
use std::sync::Arc;
use tantivy::columnar::{ColumnValues, StrColumn};
use tantivy::fastfield::FastFieldReaders;
use tantivy::schema::OwnedValue;
use tantivy::{DocAddress, DocId};

/// A helper for tracking specific "fast field" readers from a [`SearchIndexReader`] reference
///
/// They're organized by index positions and not names to eliminate as much runtime overhead as
/// possible when looking up the value of a specific fast field.
#[derive(Default)]
pub struct FFHelper(Vec<Vec<FFType>>);

impl FFHelper {
    pub fn empty() -> Self {
        Self(vec![])
    }

    pub fn with_fields(reader: &SearchIndexReader, fields: &[WhichFastField]) -> Self {
        let fast_fields = reader
            .searcher
            .segment_readers()
            .iter()
            .map(|reader| {
                let mut lookup = Vec::new();
                for field in fields {
                    match field {
                        WhichFastField::Ctid
                        | WhichFastField::TableOid
                        | WhichFastField::Score
                        | WhichFastField::Junk(_) => lookup.push(FFType::Junk),
                        WhichFastField::Named(name, _) => {
                            lookup.push(FFType::new(reader.fast_fields(), name))
                        }
                    }
                }
                lookup
            })
            .collect();
        Self(fast_fields)
    }

    #[track_caller]
    pub fn value(&self, field: usize, doc_address: DocAddress) -> Option<TantivyValue> {
        Some(self.0[doc_address.segment_ord as usize][field].value(doc_address.doc_id))
    }

    #[track_caller]
    pub fn i64(&self, field: usize, doc_address: DocAddress) -> Option<i64> {
        self.0[doc_address.segment_ord as usize][field].as_i64(doc_address.doc_id)
    }

    #[track_caller]
    pub fn string(&self, field: usize, doc_address: DocAddress, value: &mut String) -> Option<()> {
        self.0[doc_address.segment_ord as usize][field].string(doc_address.doc_id, value)
    }
}

/// Helper for working with different "fast field" types as if they're all one type
pub enum FFType {
    Junk,
    Text(StrColumn),
    I64(Arc<dyn ColumnValues<i64>>),
    F64(Arc<dyn ColumnValues<f64>>),
    U64(Arc<dyn ColumnValues<u64>>),
    Bool(Arc<dyn ColumnValues<bool>>),
    Date(Arc<dyn ColumnValues<tantivy::DateTime>>),
}

impl FFType {
    /// Construct the proper [`FFType`] for the specified `field_name`, which
    /// should be a known field name in the Tantivy index
    #[track_caller]
    pub fn new(ffr: &FastFieldReaders, field_name: &str) -> Self {
        if let Ok(Some(ff)) = ffr.str(field_name) {
            Self::Text(ff)
        } else if let Ok(ff) = ffr.u64(field_name) {
            Self::U64(ff.first_or_default_col(0))
        } else if let Ok(ff) = ffr.i64(field_name) {
            Self::I64(ff.first_or_default_col(0))
        } else if let Ok(ff) = ffr.f64(field_name) {
            Self::F64(ff.first_or_default_col(0.0))
        } else if let Ok(ff) = ffr.bool(field_name) {
            Self::Bool(ff.first_or_default_col(false))
        } else if let Ok(ff) = ffr.date(field_name) {
            Self::Date(ff.first_or_default_col(tantivy::DateTime::MIN))
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
            FFType::I64(ff) => TantivyValue(ff.get_val(doc).into()),
            FFType::F64(ff) => TantivyValue(ff.get_val(doc).into()),
            FFType::U64(ff) => TantivyValue(ff.get_val(doc).into()),
            FFType::Bool(ff) => TantivyValue(ff.get_val(doc).into()),
            FFType::Date(ff) => TantivyValue(ff.get_val(doc).into()),
        };

        value
    }

    #[inline(always)]
    pub fn string(&self, doc: DocId, value: &mut String) -> Option<()> {
        match self {
            FFType::Text(ff) => {
                value.clear();
                let ord = ff.term_ords(doc).next()?;
                ff.ord_to_str(ord, value).ok()?;
                Some(())
            }
            _ => None,
        }
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
            Some(ff.get_val(doc))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Ord, Eq, PartialOrd, PartialEq)]
pub enum WhichFastField {
    Junk(String),
    Ctid,
    TableOid,
    Score,
    Named(String, FastFieldType),
}

#[derive(Debug, Clone, Ord, Eq, PartialOrd, PartialEq)]
pub enum FastFieldType {
    String,
    Numeric,
}

impl From<SearchFieldType> for FastFieldType {
    fn from(value: SearchFieldType) -> Self {
        match value {
            SearchFieldType::Text => FastFieldType::String,
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
