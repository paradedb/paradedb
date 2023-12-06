use std::collections::HashMap;

use crate::index_access::utils::SearchConfig;
use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, RegexQuery};
use tantivy::query_grammar::Occur;
use tantivy::SnippetGenerator;
use tantivy::{
    query::{Query, QueryParser},
    schema::*,
    DocAddress, Score, Searcher,
};

use super::index::ParadeIndex;

pub struct TantivyScanState {
    pub schema: Schema,
    pub query: Box<dyn Query>,
    pub parser: QueryParser,
    pub searcher: Searcher,
    pub iterator: *mut std::vec::IntoIter<(Score, DocAddress)>,
    pub config: SearchConfig,
    pub key_field_name: String,
}

impl TantivyScanState {
    pub fn new(parade_index: &ParadeIndex, config: &SearchConfig) -> Self {
        let schema = parade_index.schema();
        let mut parser = parade_index.query_parser();
        let query = Self::query(config, &schema, &mut parser);
        TantivyScanState {
            schema,
            query,
            parser,
            config: config.clone(),
            searcher: parade_index.searcher(),
            iterator: std::ptr::null_mut(),
            key_field_name: parade_index.key_field_name.clone(),
        }
    }

    pub fn ctid(&mut self, doc_address: DocAddress) -> u64 {
        let retrieved_doc = self.searcher.doc(doc_address).expect("could not find doc");

        let ctid_field = self
            .schema
            .get_field("ctid")
            .expect("field 'ctid' not found in schema");

        if let tantivy::schema::Value::U64(ctid_value) = retrieved_doc
            .get_first(ctid_field)
            .expect("ctid field not found in doc")
        {
            *ctid_value
        } else {
            panic!("error unwrapping ctid value")
        }
    }

    pub fn search(&mut self) -> Vec<(f32, DocAddress)> {
        // Extract limit and offset from the query config or set defaults.
        let limit = self.config.limit_rows.unwrap_or({
            let num_docs = self.searcher.num_docs() as usize;
            if num_docs > 0 {
                num_docs // The collector will panic if it's passed a limit of 0.
            } else {
                1 // Since there's no docs to return anyways, just use 1.
            }
        });

        let offset = self.config.offset_rows.unwrap_or(0);

        self.searcher
            .search(&self.query, &TopDocs::with_limit(limit).and_offset(offset))
            .expect("failed to search")
    }

    pub fn doc(&self, doc_address: DocAddress) -> tantivy::Result<Document> {
        self.searcher.doc(doc_address)
    }

    fn query(
        query_config: &SearchConfig,
        schema: &Schema,
        parser: &mut QueryParser,
    ) -> Box<dyn Query> {
        let fuzzy_fields = &query_config.fuzzy_fields;
        let regex_fields = &query_config.regex_fields;

        // Determine if we're using regex fields based on the presence or absence of prefix and fuzzy fields.
        // It panics if both are provided as that's considered an invalid input.
        let using_regex_fields = match (!regex_fields.is_empty(), !fuzzy_fields.is_empty()) {
            (true, true) => panic!("cannot search with both regex_fields and fuzzy_fields"),
            (true, false) => true,
            _ => false,
        };

        // Construct the actual Tantivy search query based on the mode determined above.
        let tantivy_query: Box<dyn Query> = if using_regex_fields {
            let regex_pattern = format!("{}.*", &query_config.query);
            let mut queries: Vec<Box<dyn Query>> = Vec::new();

            // Build a regex query for each specified regex field.
            for field_name in &mut regex_fields.iter() {
                if let Ok(field) = schema.get_field(field_name) {
                    let regex_query =
                        Box::new(RegexQuery::from_pattern(&regex_pattern, field).unwrap());
                    queries.push(regex_query);
                }
            }

            // If there's only one query, use it directly; otherwise, combine the queries.
            if queries.len() == 1 {
                queries.remove(0)
            } else {
                let boolean_query =
                    BooleanQuery::new(queries.into_iter().map(|q| (Occur::Should, q)).collect());
                Box::new(boolean_query)
            }
        } else {
            let require_prefix = query_config.prefix.unwrap_or(true);
            let transpose_cost_one = query_config.transpose_cost_one.unwrap_or(true);
            let max_distance = query_config.distance.unwrap_or(2);

            for field_name in &mut fuzzy_fields.iter() {
                if let Ok(field) = schema.get_field(field_name) {
                    parser.set_field_fuzzy(field, require_prefix, max_distance, transpose_cost_one);
                }
            }

            // Construct the query using the lenient parser to tolerate minor errors in the input.
            parser.parse_query_lenient(&query_config.query).0
        };

        tantivy_query
    }

    #[allow(dead_code)]
    pub fn snippet_generators(
        &mut self,
        query_config: &SearchConfig,
    ) -> HashMap<String, SnippetGenerator> {
        let mut snippet_generators = HashMap::new();
        for field in &mut self.schema.fields() {
            let field_name = field.1.name().to_string();

            if let FieldType::Str(_) = field.1.field_type() {
                let mut snippet_generator = SnippetGenerator::create(
                    &self.searcher,
                    &self.query,
                    field.0,
                )
                .unwrap_or_else(|err| {
                    panic!("failed to create snippet generator for field: {field_name}... {err}")
                });

                if let Some(max_num_chars) = query_config.max_num_chars {
                    snippet_generator.set_max_num_chars(max_num_chars);
                }

                snippet_generators.insert(field_name, snippet_generator);
            }
        }

        snippet_generators
    }
}
