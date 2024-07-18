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

mod client;
mod directory;
mod handler;
mod index;

use crate::schema::{
    SearchDocument, SearchFieldConfig, SearchFieldName, SearchFieldType, SearchIndexSchema,
};
pub use crate::writer::SearchFs;
pub use client::*;
pub use directory::*;
pub use handler::*;
pub use index::*;
pub use rstest::*;
use serde_json::json;

#[fixture]
pub fn simple_schema(
    default_fields: Vec<(SearchFieldName, SearchFieldConfig, SearchFieldType)>,
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
    MockWriterDirectory::new("mock_writer_directory")
}

#[fixture]
pub fn default_fields() -> Vec<(SearchFieldName, SearchFieldConfig, SearchFieldType)> {
    let text: SearchFieldConfig = serde_json::from_value(json!({"Text": {}})).unwrap();
    let numeric: SearchFieldConfig = serde_json::from_value(json!({"Numeric": {}})).unwrap();
    let json: SearchFieldConfig = serde_json::from_value(json!({"Json": {}})).unwrap();
    let boolean: SearchFieldConfig = serde_json::from_value(json!({"Boolean": {}})).unwrap();

    vec![
        ("id".into(), numeric.clone().into(), SearchFieldType::I64),
        ("ctid".into(), SearchFieldConfig::Ctid, SearchFieldType::U64),
        ("description".into(), text.clone(), SearchFieldType::Text),
        ("rating".into(), numeric.clone(), SearchFieldType::I64),
        ("category".into(), text.clone(), SearchFieldType::Text),
        ("in_stock".into(), boolean.clone(), SearchFieldType::Bool),
        ("metadata".into(), json.clone(), SearchFieldType::Json),
    ]
}

#[fixture]
pub fn chinese_fields() -> Vec<(SearchFieldName, SearchFieldConfig, SearchFieldType)> {
    let text: SearchFieldConfig =
        serde_json::from_value(json!({"Text": {"tokenizer": {"type": "chinese_compatible"}}}))
            .unwrap();
    let numeric: SearchFieldConfig = serde_json::from_value(json!({"Numeric": {}})).unwrap();
    let json: SearchFieldConfig = serde_json::from_value(json!({"Json": {}})).unwrap();

    vec![
        ("id".into(), numeric.clone().into(), SearchFieldType::I64),
        ("ctid".into(), SearchFieldConfig::Ctid, SearchFieldType::U64),
        ("author".into(), text.clone(), SearchFieldType::Text),
        ("title".into(), text.clone(), SearchFieldType::Text),
        ("message".into(), numeric.clone(), SearchFieldType::I64),
        ("content".into(), json.clone(), SearchFieldType::Json),
        ("like_count".into(), numeric.clone(), SearchFieldType::I64),
        (
            "dislike_count".into(),
            numeric.clone(),
            SearchFieldType::I64,
        ),
        (
            "comment_count".into(),
            numeric.clone(),
            SearchFieldType::I64,
        ),
        (
            "unix_timestamp_milli".into(),
            numeric.clone(),
            SearchFieldType::I64,
        ),
    ]
}

#[fixture]
pub fn default_index(
    default_fields: Vec<(SearchFieldName, SearchFieldConfig, SearchFieldType)>,
) -> MockSearchIndex {
    // Key field index is 0 (id) for default_fields.
    MockSearchIndex::new(default_fields, 0)
}

#[fixture]
pub fn chinese_index(
    chinese_fields: Vec<(SearchFieldName, SearchFieldConfig, SearchFieldType)>,
) -> MockSearchIndex {
    // Key field index is 0 (id) for chinese_fields.
    MockSearchIndex::new(chinese_fields, 0)
}
