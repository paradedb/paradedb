use std::collections::HashMap;

use pgrx::{prelude::TableIterator, *};
use tantivy::{schema::FieldType, DocAddress, Document, SnippetGenerator};

use crate::{
    index_access::utils::{get_parade_index, SearchConfig},
    parade_index::{
        index::{ParadeIndex, ParadeIndexKey},
        state::TantivyScanState,
    },
};

#[pg_extern]
pub fn rank_bm25(
    config_json: JsonB,
) -> TableIterator<'static, (name!(id, i64), name!(rank_bm25, f32))> {
    let JsonB(search_config_json) = config_json;
    let search_config: SearchConfig =
        serde_json::from_value(search_config_json).expect("could not parse search config");
    let parade_index = get_parade_index(&search_config.index_name);

    let mut scan_state = parade_index.scan_state(&search_config);
    let top_docs = scan_state.search();
    let dedupe = DedupeResults::new(&scan_state, &parade_index, top_docs);

    let mut field_rows = Vec::new();
    for DedupedDoc {
        document, score, ..
    } in dedupe.into_iter()
    {
        #[allow(unreachable_patterns)]
        let key = match parade_index.get_key_value(&document) {
            ParadeIndexKey::Number(k) => k,
            _ => unimplemented!("non-integer index keys are not yet implemented"),
        };
        field_rows.push((key, score));
    }
    TableIterator::new(field_rows)
}

#[pg_extern]
pub fn highlight_bm25(
    config_json: JsonB,
) -> TableIterator<'static, (name!(id, i64), name!(highlight_bm25, String))> {
    let JsonB(search_config_json) = config_json;
    let search_config: SearchConfig =
        serde_json::from_value(search_config_json).expect("could not parse search config");
    let parade_index = get_parade_index(&search_config.index_name);
    let schema = parade_index.schema();
    let function_schema = &search_config.schema_name;
    let field_name = search_config.highlight_field.as_ref().unwrap_or_else(|| {
        panic!("highlight_field parameter required for {function_schema}.highlight function")
    });
    let mut scan_state = parade_index.scan_state(&search_config);
    let top_docs = scan_state.search();

    let highlight_field = schema
        .get_field(field_name)
        .unwrap_or_else(|err| panic!("error highlighting field {field_name}: {err:?}"));
    let highlight_field_entry = schema.get_field_entry(highlight_field);

    let mut snippet_generator = if let FieldType::Str(_) = highlight_field_entry.field_type() {
        SnippetGenerator::create(&parade_index.searcher(), &scan_state.query, highlight_field)
            .unwrap_or_else(|err| {
                panic!("failed to create snippet generator for field: {field_name}... {err}")
            })
    } else {
        panic!("can only highlight text fields")
    };

    if let Some(max_num_chars) = search_config.max_num_chars {
        snippet_generator.set_max_num_chars(max_num_chars);
    }

    let dedupe = DedupeResults::new(&scan_state, &parade_index, top_docs);
    let mut field_rows = Vec::new();
    for DedupedDoc { document, .. } in dedupe.into_iter() {
        let snippet = snippet_generator.snippet_from_doc(&document);
        let html = snippet.to_html();

        #[allow(unreachable_patterns)]
        let key = match parade_index.get_key_value(&document) {
            ParadeIndexKey::Number(k) => k,
            _ => unimplemented!("non-integer index keys are not yet implemented"),
        };
        field_rows.push((key, html));
    }

    TableIterator::new(field_rows)
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[pg_extern]
pub fn minmax_bm25(
    config_json: JsonB,
) -> TableIterator<'static, (name!(id, i64), name!(rank_bm25, f32))> {
    let JsonB(search_config_json) = config_json;
    let search_config: SearchConfig =
        serde_json::from_value(search_config_json).expect("could not parse search config");
    let parade_index = get_parade_index(&search_config.index_name);

    let mut scan_state = parade_index.scan_state(&search_config);
    let top_docs = scan_state.search();
    let (min_score, max_score) = top_docs
        .iter()
        .map(|(score, _)| *score)
        .fold((f32::MAX, f32::MIN), |(min, max), score| {
            (min.min(score), max.max(score))
        });
    let score_range = max_score - min_score;
    let mut field_rows = Vec::new();

    let dedupe = DedupeResults::new(&scan_state, &parade_index, top_docs);
    for DedupedDoc {
        score, document, ..
    } in dedupe.into_iter()
    {
        #[allow(unreachable_patterns)]
        let key = match parade_index.get_key_value(&document) {
            ParadeIndexKey::Number(k) => k,
            _ => unimplemented!("non-integer index keys are not yet implemented"),
        };

        let normalized_score = if score_range == 0.0 {
            1.0
        } else {
            (score - min_score) / score_range
        };

        field_rows.push((key, normalized_score));
    }
    TableIterator::new(field_rows)
}

struct DedupedDoc {
    timestamp: i64,
    index: usize,
    score: f32,
    document: Document,
}

struct DedupeResults {
    map: HashMap<ParadeIndexKey, DedupedDoc>,
}

impl DedupeResults {
    pub fn new(
        scan_state: &TantivyScanState,
        parade_index: &ParadeIndex,
        top_docs: Vec<(f32, DocAddress)>,
    ) -> Self {
        let map = HashMap::new();
        let mut new_self = Self { map };

        for (index, (score, doc_address)) in top_docs.into_iter().enumerate() {
            let document = scan_state.doc(doc_address).unwrap_or_else(|err| {
                panic!("error retrieving document for highlighting: {err:?}")
            });

            let key = parade_index.get_key_value(&document);

            let timestamp = parade_index.get_timestamp_value(&document);
            new_self.insert(
                key,
                DedupedDoc {
                    timestamp,
                    index,
                    score,
                    document,
                },
            );
        }

        new_self
    }

    fn insert(&mut self, key: ParadeIndexKey, doc: DedupedDoc) {
        if let Some(existing) = self.map.get(&key) {
            if doc.timestamp > existing.timestamp {
                self.map.insert(key, doc);
            }
        } else {
            self.map.insert(key, doc);
        }
    }
}

// Custom iterator that will iterate over DedupedDocs
struct DedupeResultsIterator {
    inner: Vec<DedupedDoc>,
}

// Implement IntoIterator for DedupeResults
impl IntoIterator for DedupeResults {
    type Item = DedupedDoc;
    type IntoIter = DedupeResultsIterator;

    fn into_iter(self) -> Self::IntoIter {
        let mut docs: Vec<DedupedDoc> = self.map.into_values().collect();
        // Sort the documents by index
        docs.sort_by_key(|doc| doc.index);
        DedupeResultsIterator { inner: docs }
    }
}

// Implement Iterator for DedupeResultsIterator
impl Iterator for DedupeResultsIterator {
    type Item = DedupedDoc;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.pop()
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx::*;
    use shared::testing::{test_table, ExpectedRow, SETUP_SQL};

    #[pg_test]
    fn test_rank_bm25() {
        Spi::run(SETUP_SQL).expect("failed to create index and table");
        let ctid = Spi::get_one::<pg_sys::ItemPointerData>(
            "SELECT ctid FROM one_republic_songs WHERE title = 'If I Lose Myself'",
        )
        .expect("could not get ctid");

        assert!(ctid.is_some());
        let ctid = ctid.unwrap();
        assert_eq!(ctid.ip_posid, 3);

        let query = r#"
            SELECT rank_bm25 FROM one_republic_songs.rank('lyrics:im AND description:song')
        "#;

        let rank = Spi::get_one::<f32>(query)
            .expect("failed to rank query")
            .unwrap();
        assert!(rank > 1.0);
    }

    #[pg_test]
    fn test_highlight() {
        Spi::run(SETUP_SQL).expect("failed to create index and table");

        let query = r#"
            SELECT highlight_bm25
            FROM one_republic_songs.highlight('lyrics:im', highlight_field => 'lyrics', max_num_chars => 10);
        "#;

        let highlight = Spi::get_one::<&str>(query)
            .expect("failed to highlight lyrics")
            .unwrap();
        assert_eq!(highlight, "<b>Im</b> shaking");
    }

    #[pg_test]
    fn highlight_without_max_num_chars() -> spi::Result<()> {
        Spi::run(SETUP_SQL).expect("failed to setup index");
        Spi::connect(|client| {
            let table = client.select(
                "SELECT description, rating, category, highlight_bm25 FROM search_config.search('description:keyboard OR category:electronics') as s LEFT JOIN search_config.highlight('description:keyboard OR category:electronics', highlight_field => 'description') as h ON s.id = H.id LEFT JOIN search_config.rank('description:keyboard OR category:electronics') as r ON s.id = r.id ORDER BY rank_bm25 DESC LIMIT 5;",
                None,
                None,
            )?;

            let expect = vec![
                ExpectedRow {
                    description: Some("Plastic Keyboard"),
                    rating: Some(4),
                    category: Some("Electronics"),
                    highlight_bm25: Some("Plastic <b>Keyboard</b>"),
                    ..Default::default() // Other fields default to None
                },
                ExpectedRow {
                    description: Some("Ergonomic metal keyboard"),
                    rating: Some(4),
                    category: Some("Electronics"),
                    highlight_bm25: Some("Ergonomic metal <b>keyboard</b>"),
                    ..Default::default() // Other fields default to None
                },
                ExpectedRow {
                    description: Some("Innovative wireless earbuds"),
                    rating: Some(5),
                    category: Some("Electronics"),
                    highlight_bm25: Some(""),
                    ..Default::default() // Other fields default to None
                },
                ExpectedRow {
                    description: Some("Fast charging power bank"),
                    rating: Some(4),
                    category: Some("Electronics"),
                    highlight_bm25: Some(""),
                    ..Default::default() // Other fields default to None
                },
                ExpectedRow {
                    description: Some("Bluetooth-enabled speaker"),
                    rating: Some(3),
                    category: Some("Electronics"),
                    highlight_bm25: Some(""),
                    ..Default::default() // Other fields default to None
                },
            ];

            let _ = test_table(table, expect);

            Ok(())
        })
    }

    #[pg_test]
    fn highlight_with_max_num_chars() -> spi::Result<()> {
        Spi::run(SETUP_SQL).expect("failed to setup index");
        Spi::connect(|client| {
            let table = client.select(
                "SELECT description, rating, category, highlight_bm25 FROM search_config.search('description:keyboard OR category:electronics', max_num_chars => 14) as s LEFT JOIN search_config.highlight('description:keyboard OR category:electronics', highlight_field => 'description', max_num_chars => 14) as h ON s.id = H.id LEFT JOIN search_config.rank('description:keyboard OR category:electronics', max_num_chars => 14) as r ON s.id = r.id ORDER BY rank_bm25 DESC LIMIT 5;",
                None,
                None,
            )?;

            let expect = vec![
                ExpectedRow {
                    description: Some("Plastic Keyboard"),
                    rating: Some(4),
                    category: Some("Electronics"),
                    highlight_bm25: Some("<b>Keyboard</b>"),
                    ..Default::default() // Other fields default to None
                },
                ExpectedRow {
                    description: Some("Ergonomic metal keyboard"),
                    rating: Some(4),
                    category: Some("Electronics"),
                    highlight_bm25: Some("metal <b>keyboard</b>"),
                    ..Default::default() // Other fields default to None
                },
                ExpectedRow {
                    description: Some("Innovative wireless earbuds"),
                    rating: Some(5),
                    category: Some("Electronics"),
                    highlight_bm25: Some(""),
                    ..Default::default() // Other fields default to None
                },
                ExpectedRow {
                    description: Some("Fast charging power bank"),
                    rating: Some(4),
                    category: Some("Electronics"),
                    highlight_bm25: Some(""),
                    ..Default::default() // Other fields default to None
                },
                ExpectedRow {
                    description: Some("Bluetooth-enabled speaker"),
                    rating: Some(3),
                    category: Some("Electronics"),
                    highlight_bm25: Some(""),
                    ..Default::default() // Other fields default to None
                },
            ];

            let _ = test_table(table, expect);

            Ok(())
        })
    }
}
