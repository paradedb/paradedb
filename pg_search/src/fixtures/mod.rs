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
    SearchIndexSchema::new(default_fields).unwrap()
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
        ("id".into(), SearchFieldConfig::Key, SearchFieldType::I64),
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
        ("id".into(), SearchFieldConfig::Key, SearchFieldType::I64),
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
    MockSearchIndex::new(default_fields)
}

#[fixture]
pub fn chinese_index(
    chinese_fields: Vec<(SearchFieldName, SearchFieldConfig, SearchFieldType)>,
) -> MockSearchIndex {
    MockSearchIndex::new(chinese_fields)
}
