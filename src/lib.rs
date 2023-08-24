use pgrx::prelude::*;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::Index;

pgrx::pg_module_magic!();

#[pg_extern]
fn hello_retake_extension() -> &'static str {
    "Hello, retake_extension"
}

#[pg_extern]
fn hello_tantivy() -> String {
    // Create a schema
    let mut schema_builder = Schema::builder();
    let title = schema_builder.add_text_field("title", TEXT | STORED);
    let schema = schema_builder.build();

    // Create an in-memory index
    let index = Index::create_in_ram(schema.clone());

    // Index a document
    {
        let mut index_writer = index.writer(50_000_000).unwrap();
        let mut doc = Document::new();
        doc.add_text(title, "Hello, Tantivy");
        index_writer.add_document(doc);
        index_writer.commit().unwrap();
    }

    // Search for the document
    let reader = index.reader().unwrap();
    let searcher = reader.searcher();
    let query_parser = QueryParser::for_index(&index, vec![title]);
    let query = query_parser.parse_query("Tantivy").unwrap();
    let top_docs = searcher.search(&query, &TopDocs::with_limit(1)).unwrap();

    // Return the document's content
    if let Some((_score, doc_address)) = top_docs.first() {
        let retrieved_doc = searcher.doc(*doc_address).unwrap();
        let retrieved_value = retrieved_doc.get_first(title).unwrap().as_text().unwrap();
        return retrieved_value.to_string();
    }

    "No results found".to_string()
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    fn test_hello_retake_extension() {
        assert_eq!("Hello, retake_extension", crate::hello_retake_extension());
    }

    #[pg_test]
    fn test_hello_tantivy() {
        assert_eq!("Hello, Tantivy", crate::hello_tantivy());
    }
}

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
