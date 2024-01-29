use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, RegexQuery};
use tantivy::query_grammar::Occur;
use tantivy::{
    query::{Query, QueryParser},
    schema::*,
    DocAddress, Score, Searcher,
};
use tantivy::{DocId, SegmentReader};

use super::index::ParadeIndex;
use super::score::ParadeIndexScore;
use crate::schema::{SearchConfig, SearchIndexSchema};

pub struct SearchState {
    pub schema: SearchIndexSchema,
    pub query: Box<dyn Query>,
    pub parser: QueryParser,
    pub searcher: Searcher,
    pub iterator: *mut std::vec::IntoIter<(ParadeIndexScore, DocAddress)>,
    pub config: SearchConfig,
    pub key_field_name: String,
}

impl SearchState {
    pub fn new(parade_index: &ParadeIndex, config: &SearchConfig) -> Self {
        let schema = parade_index.schema.clone();
        let mut parser = parade_index.query_parser();
        let query = Self::query(config, &schema, &mut parser);
        let key_field_name = schema.key_field().name.0;
        SearchState {
            schema,
            query,
            parser,
            config: config.clone(),
            searcher: parade_index.searcher(),
            iterator: std::ptr::null_mut(),
            key_field_name,
        }
    }

    pub fn key_field_value(&mut self, doc_address: DocAddress) -> i64 {
        let retrieved_doc = self.searcher.doc(doc_address).expect("could not find doc");

        let key_field = self
            .schema
            .schema
            .get_field(&self.key_field_name)
            .expect("field '{key_field_name}' not found in schema");

        if let tantivy::schema::Value::I64(key_field_value) =
            retrieved_doc.get_first(key_field).unwrap_or_else(|| {
                panic!(
                    "value for key_field '{}' not found in doc",
                    &self.key_field_name,
                )
            })
        {
            *key_field_value
        } else {
            panic!("error unwrapping ctid value")
        }
    }

    pub fn search(&mut self) -> Vec<(ParadeIndexScore, DocAddress)> {
        // Extract limit and offset from the query config or set defaults.
        let limit = self.config.limit_rows.unwrap_or_else(|| {
            // We use unwrap_or_else here so this block doesn't run unless
            // we actually need the default value. This is important, because there can
            // be some cost to Tantivy API calls.
            let num_docs = self.searcher.num_docs() as usize;
            if num_docs > 0 {
                num_docs // The collector will panic if it's passed a limit of 0.
            } else {
                1 // Since there's no docs to return anyways, just use 1.
            }
        });

        let offset = self.config.offset_rows.unwrap_or(0);
        let key_field_name = self.key_field_name.clone();
        let top_docs_by_custom_score = TopDocs::with_limit(limit).and_offset(offset).tweak_score(
            // tweak_score expects a function that will return a function. A little unusual for
            // Rust, but not too much of a problem as long as you don't need to reference
            // many variables outside the function scope.
            move |segment_reader: &SegmentReader| {
                let key_field_reader = segment_reader
                    .fast_fields()
                    .i64(&key_field_name)
                    .unwrap_or_else(|err| {
                        panic!("key field {} is not a u64: {err:?}", &key_field_name)
                    })
                    .first_or_default_col(0);

                move |doc: DocId, original_score: Score| ParadeIndexScore {
                    bm25: original_score,
                    key: key_field_reader.get_val(doc),
                }
            },
        );

        self.searcher
            .search(&self.query, &top_docs_by_custom_score)
            .expect("failed to search")
    }

    pub fn doc(&self, doc_address: DocAddress) -> tantivy::Result<Document> {
        self.searcher.doc(doc_address)
    }

    fn query(
        query_config: &SearchConfig,
        schema: &SearchIndexSchema,
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
                if let Ok(field) = schema.schema.get_field(field_name) {
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
                if let Ok(field) = schema.schema.get_field(field_name) {
                    parser.set_field_fuzzy(field, require_prefix, max_distance, transpose_cost_one);
                }
            }

            // Construct the query using the lenient parser to tolerate minor errors in the input.
            parser.parse_query_lenient(&query_config.query).0
        };

        tantivy_query
    }
}
