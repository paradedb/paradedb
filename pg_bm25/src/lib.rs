use directory::SQLDirectory;
use pgrx::prelude::*;
use serde_json::{Map, Value};
use tantivy::collector::TopDocs;
use tantivy::doc;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::Index;
use tantivy::IndexSettings;
use tantivy::SingleSegmentIndexWriter;

mod directory;
mod sql_writer;

pgrx::pg_module_magic!();

fn extract_table_def(table_name: &str) -> Result<Vec<(String, String)>, spi::Error> {
    let query = format!(
        "SELECT attname::text            AS column_name,
                atttypid::regtype::text  AS data_type
         FROM   pg_attribute
         WHERE  attrelid = '{table_name}'::regclass
         AND    attnum > 0
         AND    NOT attisdropped
         ORDER  BY attnum;"
    );

    Spi::connect(|client| {
        let mut results: Vec<(String, String)> = Vec::new();
        let tup_table = client.select(&query, None, None)?;

        for row in tup_table {
            let column_name = row["column_name"]
                .value::<String>()
                .expect("no column name")
                .unwrap();

            let data_type = row["data_type"]
                .value::<String>()
                .expect("no data type")
                .unwrap();

            results.push((column_name, data_type));
        }

        Ok(results)
    })
}

fn build_tantivy_schema(
    table_name: &str,
    column_names: &[String],
) -> (Schema, Vec<(String, String, Field)>) {
    let table_def: Vec<(String, String)> =
        extract_table_def(table_name).expect("failed to return table definition");
    let mut schema_builder = Schema::builder();
    let mut fields = Vec::new();

    for (col_name, data_type) in &table_def {
        if column_names.contains(col_name) {
            // TODO: Support JSON, byte, and date fields
            match data_type.as_str() {
                "text" | "varchar" => {
                    let field = schema_builder.add_text_field(col_name, TEXT | STORED);
                    fields.push((col_name.clone(), data_type.clone(), field));
                }
                "int2" | "int4" | "int8" => {
                    let field = schema_builder.add_i64_field(col_name, INDEXED | STORED);
                    fields.push((col_name.clone(), data_type.clone(), field));
                }
                "float4" | "float8" | "numeric" => {
                    let field = schema_builder.add_f64_field(col_name, INDEXED | STORED);
                    fields.push((col_name.clone(), data_type.clone(), field));
                }
                "bool" => {
                    let field = schema_builder.add_bool_field(col_name, STORED);
                    fields.push((col_name.clone(), data_type.clone(), field));
                }
                _ => {
                    let field = schema_builder.add_text_field(col_name, TEXT | STORED);
                    fields.push((col_name.clone(), data_type.clone(), field));
                }
            }
        }
    }

    (schema_builder.build(), fields)
}

fn create_docs(
    mut writer: SingleSegmentIndexWriter,
    table_name: &str,
    fields: &Vec<(String, String, Field)>,
) {
    let query: String = format!("SELECT * FROM {}", table_name);
    let _ = Spi::connect(|client| {
        let tup_table = client.select(&query, None, None);

        match tup_table {
            Ok(tup_table) => {
                for row in tup_table {
                    let mut doc = Document::new();

                    for (col_name, data_type, field) in fields {
                        match data_type.as_str() {
                            // TODO: Support JSON, byte, and date fields
                            "text" | "varchar" => {
                                let value: String = row[col_name.as_str()]
                                    .value()
                                    .expect("failed to get value for col")
                                    .unwrap();
                                doc.add_text(*field, &value);
                            }
                            "int2" | "int4" | "int8" => {
                                let value: i64 = row[col_name.as_str()]
                                    .value()
                                    .expect("failed to get value for col")
                                    .unwrap();
                                doc.add_i64(*field, value);
                            }
                            "float4" | "float8" | "numeric" => {
                                let value: f64 = row[col_name.as_str()]
                                    .value()
                                    .expect("failed to get value for col")
                                    .unwrap();
                                doc.add_f64(*field, value);
                            }
                            "bool" => {
                                let value: bool = row[col_name.as_str()]
                                    .value()
                                    .expect("failed to get value for col")
                                    .unwrap();
                                doc.add_bool(*field, value);
                            }
                            _ => panic!("Unsupported data type: {}", data_type),
                        }
                    }
                    let _ = writer.add_document(doc);
                }
                Ok(())
            }
            Err(_) => Err(()),
        }
    });
    writer.finalize().expect("failed to finalize index writer");
}

#[pg_extern]
fn index_bm25(table_name: String, index_name: String, column_names: Vec<String>) {
    let (schema, fields) = build_tantivy_schema(&table_name, &column_names);
    let dir = SQLDirectory::new(index_name.clone());
    let settings = IndexSettings {
        docstore_compress_dedicated_thread: false, // Must run on single thread, or pgrx will panic
        ..Default::default()
    };

    let index = Index::builder()
        .schema(schema)
        .settings(settings)
        .open_or_create(dir)
        .expect("failed to create index");

    let index_writer =
        SingleSegmentIndexWriter::new(index, 50_000_000).expect("failed to create index writer");

    create_docs(index_writer, &table_name, &fields);
}

#[pg_extern]
fn search_bm25(
    query: String,
    index_name: String,
    field_names: Vec<String>,
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

    // Convert all column names to fields
    let fields: Vec<Field> = field_names
        .iter()
        .map(|field| schema.get_field(field).expect("failed to get field"))
        .collect();

    let query_parser = QueryParser::for_index(&index, fields.clone());
    let tantivy_query = query_parser.parse_query(&query).unwrap();
    let top_docs = searcher
        .search(&tantivy_query, &TopDocs::with_limit(k as usize))
        .unwrap();

    let results = top_docs.into_iter().map(move |(score, doc_address)| {
        let retrieved_doc = searcher.doc(doc_address).unwrap();

        let mut json_map = Map::new();
        for (&field, column_name) in fields.iter().zip(field_names.iter()) {
            if let Some(value) = retrieved_doc.get_first(field).and_then(|f| f.as_text()) {
                json_map.insert(column_name.to_string(), Value::String(value.to_string()));
            }
        }

        (score, pgrx::JsonB(Value::Object(json_map)))
    });

    TableIterator::new(results)
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {}

/// This module is required by `cargo pgrx test` invocations.
/// It must be visible at the root of your extension crate.
#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        // return any postgresql.conf settings that are required for your tests
        vec![]
    }
}
