mod client;
mod directory;
mod document;
mod handler;
mod index;

use crate::schema::{
    ParadeBooleanOptions, ParadeJsonOptions, ParadeNumericOptions, ParadeTextOptions,
    ParadeTokenizer, SearchDocument, SearchFieldConfig, SearchFieldName,
};
pub use client::*;
pub use directory::*;
pub use document::*;
pub use handler::*;
pub use index::*;
pub use rstest::*;
use tantivy::schema::Schema;

#[fixture]
pub fn json_doc() -> SearchDocument {
    document::simple_json()
}

#[fixture]
pub fn mock_dir() -> MockWriterDirectory {
    MockWriterDirectory::new("mock_writer_directory")
}

#[fixture]
pub fn default_fields() -> Vec<(SearchFieldName, SearchFieldConfig)> {
    let key = SearchFieldConfig::Key;
    let ctid = SearchFieldConfig::Ctid;
    let text = SearchFieldConfig::Text(ParadeTextOptions::default());
    let numeric = SearchFieldConfig::Numeric(ParadeNumericOptions::default());
    let boolean = SearchFieldConfig::Boolean(ParadeBooleanOptions::default());
    let json = SearchFieldConfig::Json(ParadeJsonOptions::default());

    vec![
        (SearchFieldName("id".into()), key),
        (SearchFieldName("ctid".into()), ctid),
        (SearchFieldName("description".into()), text.clone()),
        (SearchFieldName("rating".into()), numeric),
        (SearchFieldName("category".into()), text.clone()),
        (SearchFieldName("in_stock".into()), boolean),
        (SearchFieldName("metadata".into()), json),
    ]
}

#[fixture]
pub fn chinese_fields() -> Vec<(SearchFieldName, SearchFieldConfig)> {
    let key = SearchFieldConfig::Key;
    let ctid = SearchFieldConfig::Ctid;
    let numeric = SearchFieldConfig::Numeric(ParadeNumericOptions::default());
    let json = SearchFieldConfig::Json(ParadeJsonOptions::default());

    let mut text_options = ParadeTextOptions::default();
    text_options.tokenizer = ParadeTokenizer::ChineseCompatible;
    let text = SearchFieldConfig::Text(text_options);

    vec![
        (SearchFieldName("id".into()), key),
        (SearchFieldName("ctid".into()), ctid),
        (SearchFieldName("author".into()), text.clone()),
        (SearchFieldName("title".into()), text.clone()),
        (SearchFieldName("message".into()), numeric.clone()),
        (SearchFieldName("content".into()), json.clone()),
        (SearchFieldName("like_count".into()), numeric.clone()),
        (SearchFieldName("dislike_count".into()), numeric.clone()),
        (SearchFieldName("comment_count".into()), numeric.clone()),
        (SearchFieldName("unix_timestamp_milli".into()), numeric),
    ]
}

#[fixture]
pub fn default_index(default_fields: Vec<(SearchFieldName, SearchFieldConfig)>) -> MockParadeIndex {
    MockParadeIndex::new(default_fields)
}

#[fixture]
pub fn chinese_index(chinese_fields: Vec<(SearchFieldName, SearchFieldConfig)>) -> MockParadeIndex {
    MockParadeIndex::new(chinese_fields)
}

#[allow(dead_code)]
#[fixture]
pub fn tantivy_schema() -> Schema {
    simple_tantivy_schema()
}
