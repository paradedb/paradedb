use crate::{directory::SQLDirectory, helpers::build_tantivy_schema, index::ParadeIndex};
use pgrx::prelude::*;
use serde_json::{Map, Value};
use tantivy::{collector::TopDocs, query::QueryParser, Index, IndexSettings};

#[pg_trigger]
pub fn sync_index<'a>(
    trigger: &'a pgrx::PgTrigger<'a>,
) -> Result<Option<PgHeapTuple<'a, impl WhoAllocated>>, pgrx::PgTriggerError> {
    let table_name = trigger
        .table_name()
        .expect("failed to get table name from trigger");
    let args = trigger
        .extra_args()
        .expect("failed to get extra arguments from trigger");
    let new = trigger.new().expect("failed to get new rows from trigger");

    let index_name = args.get(0).unwrap().to_string();
    let target_columns: Vec<String> = args
        .get(1)
        .unwrap()
        .trim_matches(|c| c == '{' || c == '}' || c == ',' || c == ' ')
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    let (schema, fields) = build_tantivy_schema(&table_name, &target_columns);
    let settings = IndexSettings {
        docstore_compress_dedicated_thread: false, // Must run on single thread, or pgrx will panic
        ..Default::default()
    };

    let mut index = ParadeIndex::new(index_name, table_name, schema, fields, settings);
    index.sync(&new);

    Ok(Some(new))
}

#[pg_extern]
pub fn index_bm25(table_name: String, index_name: String, target_columns: Vec<String>) {
    let (schema, fields) = build_tantivy_schema(&table_name, &target_columns);
    let settings = IndexSettings {
        docstore_compress_dedicated_thread: false, // Must run on single thread, or pgrx will panic
        ..Default::default()
    };
    let mut index = ParadeIndex::new(index_name, table_name, schema, fields, settings);
    index.build();
}

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
    let tantivy_query = query_parser.parse_query(&query).unwrap();
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
                tantivy::schema::FieldType::Bool(_) => {
                    if let Some(val) = retrieved_doc.get_first(field).and_then(|f| f.as_bool()) {
                        json_map.insert(field_name.to_string(), Value::Bool(val));
                    }
                }
                _ => {} // For now, we ignore fields we don't handle
            }
        }

        (score, pgrx::JsonB(Value::Object(json_map)))
    });

    TableIterator::new(results)
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use super::*;
    use pgrx::Spi;
    use pgrx::*;
    use pgrx_macros::pg_test;
    use std::collections::HashSet;

    const TABLE_NAME: &str = "products";
    const INDEX_NAME: &str = "products_index";
    const COLUMNS: [&str; 3] = ["description", "rating", "category"];

    #[pg_test]
    fn test_search_bm25() {
        bootstrap_test_db();

        // Call index_bm25
        let columns_vec: Vec<String> = COLUMNS.iter().cloned().map(String::from).collect();
        index_bm25(
            TABLE_NAME.to_string(),
            INDEX_NAME.to_string(),
            columns_vec.clone(),
        );

        // Check that index was created correctly
        let column_names: HashSet<String> = crate::helpers::extract_table_def(INDEX_NAME)
            .expect("Failed to extract index definition")
            .into_iter()
            .map(|(col_name, _)| col_name)
            .collect();

        let required_columns: HashSet<_> = ["path", "content"]
            .iter()
            .cloned()
            .map(String::from)
            .collect();

        assert!(
            column_names.is_superset(&required_columns),
            "The index does not contain the required columns 'path' and 'content'."
        );

        // Check that search_bm25 returns results
        let query: &str = "description:keyboard";
        let k: i32 = 10;

        let results: Vec<_> = search_bm25(query.to_string(), INDEX_NAME.to_string(), k).collect();

        assert!(
            results.len() == 2,
            "Expected exactly two results for the search query, but found {}.",
            results.len()
        );

        // Check that search_bm25 returns no results for a query that does not match
        let query: &str = "description:ajskda";

        let results: Vec<_> = search_bm25(query.to_string(), INDEX_NAME.to_string(), k).collect();

        assert!(
            results.is_empty(),
            "Expected no results for the search query."
        );
    }

    #[pg_test]
    fn test_index_sync() {
        bootstrap_test_db();

        // Call index_bm25
        let columns_vec: Vec<String> = COLUMNS.iter().cloned().map(String::from).collect();
        index_bm25(
            TABLE_NAME.to_string(),
            INDEX_NAME.to_string(),
            columns_vec.clone(),
        );

        // Check that index was created correctly
        let column_names: HashSet<String> = crate::helpers::extract_table_def(INDEX_NAME)
            .expect("Failed to extract index definition")
            .into_iter()
            .map(|(col_name, _)| col_name)
            .collect();

        let required_columns: HashSet<_> = ["path", "content"]
            .iter()
            .cloned()
            .map(String::from)
            .collect();

        assert!(
            column_names.is_superset(&required_columns),
            "The index does not contain the required columns 'path' and 'content'."
        );

        // Insert new rows
        let query = format!("INSERT INTO {} (description, rating, category) VALUES ('Smart watch', 5, 'Electronics')", TABLE_NAME);
        Spi::run(&query).expect("SPI failed inserting new row");

        // Search for new row
        let query: &str = "description:watch";
        let k: i32 = 10;

        let results: Vec<_> = search_bm25(query.to_string(), INDEX_NAME.to_string(), k).collect();

        assert!(
            results.len() == 1,
            "Expected exactly one result for the search query, but found {}.",
            results.len()
        );

        // Search for old rows to make sure
        // they weren't overwritten
        let query: &str = "description:keyboard";
        let k: i32 = 10;

        let results: Vec<_> = search_bm25(query.to_string(), INDEX_NAME.to_string(), k).collect();

        assert!(
            results.len() == 2,
            "Expected exactly two results for the search query, but found {}.",
            results.len()
        );
    }

    fn bootstrap_test_db() {
        let mut path = std::path::PathBuf::from(
            std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"),
        );
        path.push("sql");
        path.push("_bootstrap_test.sql");

        let sql_content = std::fs::read_to_string(&path).expect("Unable to read the SQL file");

        Spi::run(&sql_content).expect("SPI failed executing SQL content");
    }
}
