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

mod directory;
mod index;

pub use crate::index::SearchFs;
use crate::schema::{
    SearchDocument, SearchFieldConfig, SearchFieldName, SearchFieldType, SearchIndexSchema,
};
pub use directory::*;
pub use index::*;
use pgrx::{pg_sys::BuiltinOid, PgOid};
pub use rstest::*;
use serde_json::json;

#[fixture]
pub fn simple_schema(
    default_fields: Vec<(SearchFieldName, SearchFieldConfig, SearchFieldType, PgOid)>,
) -> SearchIndexSchema {
    // As defined in the default_fields fixture, the key_field is the first
    // entry in the vectory.
    let default_fields_key_index = 0;
    SearchIndexSchema::new(default_fields, default_fields_key_index).unwrap()
}

#[fixture]
pub fn simple_doc(simple_schema: SearchIndexSchema) -> SearchDocument {
    let mut search_document = simple_schema.new_document();

    let ids: Vec<_> = simple_schema.fields.into_iter().map(|f| f.id).collect();

    search_document.insert(ids[0], 0i64.into());
    search_document.insert(ids[1], 0u64.into());
    search_document.insert(ids[2], "Ergonomic metal keyboard".into());
    search_document.insert(ids[3], 4i64.into());
    search_document.insert(ids[4], "Electronics".into());
    search_document.insert(ids[5], true.into());
    search_document.insert(
        ids[6],
        json!({"color":"Silver","location":"United States"}).into(),
    );

    search_document
}

#[fixture]
pub fn mock_dir() -> MockWriterDirectory {
    // We can pass a fixed index OID as a mock.
    MockWriterDirectory::new(42)
}

#[fixture]
pub fn default_fields() -> Vec<(SearchFieldName, SearchFieldConfig, SearchFieldType, PgOid)> {
    let text: SearchFieldConfig = serde_json::from_value(json!({"Text": {}})).unwrap();
    let numeric: SearchFieldConfig = serde_json::from_value(json!({"Numeric": {}})).unwrap();
    let json: SearchFieldConfig = serde_json::from_value(json!({"Json": {}})).unwrap();
    let boolean: SearchFieldConfig = serde_json::from_value(json!({"Boolean": {}})).unwrap();
    vec![
        (
            "id".into(),
            numeric.clone(),
            SearchFieldType::I64,
            PgOid::BuiltIn(BuiltinOid::INT8OID),
        ),
        (
            "ctid".into(),
            SearchFieldConfig::Ctid,
            SearchFieldType::U64,
            PgOid::BuiltIn(BuiltinOid::TIDOID),
        ),
        (
            "description".into(),
            text.clone(),
            SearchFieldType::Text,
            PgOid::BuiltIn(BuiltinOid::TEXTOID),
        ),
        (
            "rating".into(),
            numeric.clone(),
            SearchFieldType::I64,
            PgOid::BuiltIn(BuiltinOid::INT8OID),
        ),
        (
            "category".into(),
            text.clone(),
            SearchFieldType::Text,
            PgOid::BuiltIn(BuiltinOid::TEXTOID),
        ),
        (
            "in_stock".into(),
            boolean.clone(),
            SearchFieldType::Bool,
            PgOid::BuiltIn(BuiltinOid::BOOLOID),
        ),
        (
            "metadata".into(),
            json.clone(),
            SearchFieldType::Json,
            PgOid::BuiltIn(BuiltinOid::JSONBOID),
        ),
    ]
}

#[fixture]
pub fn chinese_fields() -> Vec<(SearchFieldName, SearchFieldConfig, SearchFieldType, PgOid)> {
    let text: SearchFieldConfig =
        serde_json::from_value(json!({"Text": {"tokenizer": {"type": "chinese_compatible"}}}))
            .unwrap();
    let numeric: SearchFieldConfig = serde_json::from_value(json!({"Numeric": {}})).unwrap();
    let json: SearchFieldConfig = serde_json::from_value(json!({"Json": {}})).unwrap();
    vec![
        (
            "id".into(),
            numeric.clone(),
            SearchFieldType::I64,
            PgOid::BuiltIn(BuiltinOid::INT8OID),
        ),
        (
            "ctid".into(),
            SearchFieldConfig::Ctid,
            SearchFieldType::U64,
            PgOid::BuiltIn(BuiltinOid::TIDOID),
        ),
        (
            "author".into(),
            text.clone(),
            SearchFieldType::Text,
            PgOid::BuiltIn(BuiltinOid::TEXTOID),
        ),
        (
            "title".into(),
            text.clone(),
            SearchFieldType::Text,
            PgOid::BuiltIn(BuiltinOid::TEXTOID),
        ),
        (
            "message".into(),
            numeric.clone(),
            SearchFieldType::I64,
            PgOid::BuiltIn(BuiltinOid::INT8OID),
        ),
        (
            "content".into(),
            json.clone(),
            SearchFieldType::Json,
            PgOid::BuiltIn(BuiltinOid::JSONBOID),
        ),
        (
            "like_count".into(),
            numeric.clone(),
            SearchFieldType::I64,
            PgOid::BuiltIn(BuiltinOid::INT8OID),
        ),
        (
            "dislike_count".into(),
            numeric.clone(),
            SearchFieldType::I64,
            PgOid::BuiltIn(BuiltinOid::INT8OID),
        ),
        (
            "comment_count".into(),
            numeric.clone(),
            SearchFieldType::I64,
            PgOid::BuiltIn(BuiltinOid::INT8OID),
        ),
        (
            "unix_timestamp_milli".into(),
            numeric.clone(),
            SearchFieldType::I64,
            PgOid::BuiltIn(BuiltinOid::INT8OID),
        ),
    ]
}

#[fixture]
pub fn default_index(
    default_fields: Vec<(SearchFieldName, SearchFieldConfig, SearchFieldType, PgOid)>,
) -> MockSearchIndex {
    // Key field index is 0 (id) for default_fields.
    MockSearchIndex::new(default_fields, 0)
}

#[fixture]
pub fn chinese_index(
    chinese_fields: Vec<(SearchFieldName, SearchFieldConfig, SearchFieldType, PgOid)>,
) -> MockSearchIndex {
    // Key field index is 0 (id) for chinese_fields.
    MockSearchIndex::new(chinese_fields, 0)
}
