use pgrx::{iter::TableIterator, *};
use serde_json::{Map, Value};
use tantivy::{collector::*, query::AllQuery, schema::*};

use crate::index_access::utils::get_parade_index;
use crate::parade_index::fields::ToString;

#[allow(clippy::type_complexity)]
#[pg_extern]
pub fn schema_bm25(
    index_name: &str,
) -> TableIterator<(
    name!(name, String),
    name!(field_type, String),
    name!(stored, bool),
    name!(indexed, bool),
    name!(fast, bool),
    name!(fieldnorms, bool),
    name!(expand_dots, Option<bool>),
    name!(tokenizer, Option<String>),
    name!(record, Option<String>),
    name!(normalizer, Option<String>),
)> {
    let parade_index = get_parade_index(index_name.to_string());
    let schema = parade_index.schema();

    let mut field_rows = Vec::new();

    for field in schema.fields() {
        let (field, field_entry) = field;
        let name = schema.get_field_name(field).to_string();

        let (field_type, tokenizer, record, normalizer, expand_dots) =
            match field_entry.field_type() {
                FieldType::I64(_) => ("I64".to_string(), None, None, None, None),
                FieldType::U64(_) => ("U64".to_string(), None, None, None, None),
                FieldType::F64(_) => ("F64".to_string(), None, None, None, None),
                FieldType::Bool(_) => ("Bool".to_string(), None, None, None, None),
                FieldType::Str(text_options) => {
                    let indexing_options = text_options.get_indexing_options();
                    let tokenizer = indexing_options.map(|opt| opt.tokenizer().to_string());
                    let record = indexing_options.map(|opt| opt.index_option().to_string());
                    let normalizer = text_options
                        .get_fast_field_tokenizer_name()
                        .map(|s| s.to_string());
                    ("Str".to_string(), tokenizer, record, normalizer, None)
                }
                FieldType::JsonObject(json_options) => {
                    let indexing_options = json_options.get_text_indexing_options();
                    let tokenizer = indexing_options.map(|opt| opt.tokenizer().to_string());
                    let record = indexing_options.map(|opt| opt.index_option().to_string());
                    let normalizer = json_options
                        .get_fast_field_tokenizer_name()
                        .map(|s| s.to_string());
                    let expand_dots = Some(json_options.is_expand_dots_enabled());
                    (
                        "JsonObject".to_string(),
                        tokenizer,
                        record,
                        normalizer,
                        expand_dots,
                    )
                }
                _ => ("Other".to_string(), None, None, None, None),
            };

        let row = (
            name,
            field_type,
            field_entry.is_stored(),
            field_entry.is_indexed(),
            field_entry.is_fast(),
            field_entry.has_fieldnorms(),
            expand_dots,
            tokenizer,
            record,
            normalizer,
        );

        field_rows.push(row);
    }

    TableIterator::new(field_rows)
}

#[pg_extern]
pub fn dump_bm25(
    index_name: String,
) -> TableIterator<'static, (name!(heap_tid, i64), name!(content, pgrx::JsonB))> {
    let parade_index = get_parade_index(index_name.to_string());
    let state = parade_index.scan();
    let searcher = state.searcher;
    let schema = parade_index.schema();

    let heap_tid_field = schema.get_field("heap_tid").unwrap();
    let top_docs = searcher
        .search(&AllQuery, &DocSetCollector)
        .expect("failed to search");

    let results = top_docs.into_iter().map(move |doc_address| {
        let retrieved_doc = searcher.doc(doc_address).unwrap();
        let heap_tid = retrieved_doc
            .get_first(heap_tid_field)
            .expect("Could not get heap_tid field")
            .as_u64()
            .expect("Could not convert heap_tid to u64") as i64;

        let mut json_map = Map::new();
        for (field, _) in schema.fields() {
            if field == heap_tid_field {
                continue;
            }

            let field_entry = schema.get_field_entry(field);
            let field_name = field_entry.name();
            match field_entry.field_type() {
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
                _ => {}
            }
        }

        (heap_tid, pgrx::JsonB(Value::Object(json_map)))
    });

    TableIterator::new(results)
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::schema_bm25;
    use pgrx::*;
    use shared::testing::SETUP_SQL;

    #[pg_test]
    fn test_schema_bm25() {
        Spi::run(SETUP_SQL).expect("failed to setup index");
        let schemas = schema_bm25("idx_one_republic").collect::<Vec<_>>();
        let names = schemas
            .iter()
            .map(|schema| schema.0.as_str())
            .collect::<Vec<_>>();

        assert_eq!(schemas.len(), 7);
        assert_eq!(
            names,
            vec![
                "title",
                "album",
                "release_year",
                "genre",
                "description",
                "lyrics",
                "heap_tid"
            ]
        );
    }
}
