use crate::schema::{
    ParadeJsonOptions, SearchDocument, SearchFieldConfig, SearchFieldName, SearchIndexSchema,
};
use serde_json::json;
use tantivy::schema::Schema;

pub fn simple_json() -> SearchDocument {
    // Setup search document
    let default_json_options = SearchFieldConfig::Json(ParadeJsonOptions::default());
    let schema = SearchIndexSchema::new(vec![
        (SearchFieldName("ctid".into()), SearchFieldConfig::Ctid),
        (SearchFieldName("key".into()), SearchFieldConfig::Key),
        (SearchFieldName("metadata".into()), default_json_options),
    ])
    .unwrap();

    let mut search_document = schema.new_document();

    let data = json!(
        {"color": "blue"}
    );

    let ids: Vec<_> = schema.fields.into_iter().map(|f| f.id).collect();

    search_document.insert(ids[0].clone(), 0u64.into());
    search_document.insert(ids[1].clone(), 0i64.into());
    search_document.insert(ids[2].clone(), data.into());

    search_document
}

pub fn simple_tantivy_schema() -> Schema {
    let serialized = r#"
      [{"name":"category","type":"text","options":{"indexing":{"record":"position","fieldnorms":true,"tokenizer":"default"},"stored":true,"fast":false}},{"name":"description","type":"text","options":{"indexing":{"record":"position","fieldnorms":true,"tokenizer":"default"},"stored":true,"fast":false}},{"name":"rating","type":"i64","options":{"indexed":true,"fieldnorms":false,"fast":true,"stored":true}},{"name":"in_stock","type":"bool","options":{"indexed":true,"fieldnorms":false,"fast":true,"stored":true}},{"name":"metadata","type":"json_object","options":{"stored":true,"indexing":{"record":"position","fieldnorms":true,"tokenizer":"default"},"fast":false,"expand_dots_enabled":true}},{"name":"id","type":"i64","options":{"indexed":true,"fieldnorms":true,"fast":true,"stored":true}},{"name":"ctid","type":"u64","options":{"indexed":true,"fieldnorms":true,"fast":true,"stored":true}}]
    "#;

    let schema: Schema = serde_json::from_str(&serialized).unwrap();
    schema
}
