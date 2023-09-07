use crate::{
    parade_index::directory::SQLDirectory, parade_index::helpers::build_tantivy_schema,
    parade_index::index::ParadeIndex,
};
use pgrx::prelude::*;
use serde_json::{Map, Value};
use tantivy::{collector::TopDocs, query::QueryParser, Index, IndexSettings};

#[pg_extern]
pub fn search_bm25(
    query: String,
    index_name: String,
    k: i32,
) -> TableIterator<'static, (name!(score, f32), name!(hits, pgrx::JsonB))> {
    let dir = SQLDirectory::new(index_name.clone());
    let index = Index::open(dir).unwrap_or_else(|_| panic!("{} does not exist", &index_name));
    let schema = index.schema();

    // Search for the document
    let reader = index
        .reader_builder()
        .reload_policy(tantivy::ReloadPolicy::Manual)
        .try_into()
        .expect("failed to create index reader");
    let searcher = reader.searcher();

    let query_parser = QueryParser::for_index(
        &index,
        schema.fields().map(|(field, _)| field).collect::<Vec<_>>(),
    );
    let (tantivy_query, _) = query_parser.parse_query_lenient(&query);
    let top_docs = searcher
        .search(&tantivy_query, &TopDocs::with_limit(k as usize))
        .unwrap();

    let results = top_docs.into_iter().map(move |(score, doc_address)| {
        let retrieved_doc = searcher.doc(doc_address).unwrap();

        let mut json_map = Map::new();
        for (field, _) in schema.fields() {
            let field_entry = schema.get_field_entry(field);
            let field_name = field_entry.name();
            match field_entry.field_type() {
                // TODO: Handle remaining field types
                tantivy::schema::FieldType::Str(_) => {
                    if let Some(text) = retrieved_doc.get_first(field).and_then(|f| f.as_text()) {
                        json_map.insert(field_name.to_string(), Value::String(text.to_string()));
                    }
                }
                tantivy::schema::FieldType::U64(_) => {
                    if let Some(val) = retrieved_doc.get_first(field).and_then(|f| f.as_u64()) {
                        json_map.insert(field_name.to_string(), Value::Number(val.into()));
                    }
                }
                tantivy::schema::FieldType::I64(_) => {
                    if let Some(val) = retrieved_doc.get_first(field).and_then(|f| f.as_i64()) {
                        json_map.insert(field_name.to_string(), Value::Number(val.into()));
                    }
                }
                tantivy::schema::FieldType::F64(_) => {
                    if let Some(val) = retrieved_doc.get_first(field).and_then(|f| f.as_f64()) {
                        json_map.insert(field_name.to_string(), Value::from(val));
                    }
                }
                tantivy::schema::FieldType::Bool(_) => {
                    if let Some(val) = retrieved_doc.get_first(field).and_then(|f| f.as_bool()) {
                        json_map.insert(field_name.to_string(), Value::Bool(val));
                    }
                }
                tantivy::schema::FieldType::JsonObject(_) => {
                    if let Some(val) = retrieved_doc.get_first(field).and_then(|f| f.as_json()) {
                        json_map.insert(field_name.to_string(), Value::Object(val.clone()));
                    }
                }
                _ => {} // For now, we ignore fields we don't handle
            }
        }

        (score, pgrx::JsonB(Value::Object(json_map)))
    });

    TableIterator::new(results)
}
