use crate::{
    index_access::utils::{get_parade_index, SearchConfig},
    parade_index::index::ParadeIndex,
};
use pgrx::{prelude::TableIterator, *};
use tantivy::{schema::FieldType, SnippetGenerator};

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

    let mut field_rows = Vec::new();
    for (score, doc_address) in top_docs.into_iter() {
        let document = scan_state
            .doc(doc_address)
            .unwrap_or_else(|err| panic!("error retrieving document for rank: {err:?}"));
        let key = parade_index.get_key_value(&document);
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

    let mut field_rows = Vec::new();
    for (_score, doc_address) in top_docs.into_iter() {
        let document = scan_state
            .doc(doc_address)
            .unwrap_or_else(|err| panic!("error retrieving document for highlight: {err:?}"));
        let snippet = snippet_generator.snippet_from_doc(&document);
        let html = snippet.to_html();
        let key = parade_index.get_key_value(&document);
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

    for (score, doc_address) in top_docs.into_iter() {
        let document = scan_state
            .doc(doc_address)
            .unwrap_or_else(|err| panic!("error retrieving document for rank_hybrid: {err:?}"));
        let key = parade_index.get_key_value(&document);

        let normalized_score = if score_range == 0.0 {
            1.0
        } else {
            (score - min_score) / score_range
        };

        field_rows.push((key, normalized_score));
    }
    TableIterator::new(field_rows)
}

#[pg_extern]
fn drop_bm25_internal(index_name: &str) {
    // Drop the Tantivy data directory.
    ParadeIndex::drop_index(index_name).expect(&format!("error dropping index {index_name}"));
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx::*;
    use shared::testing::SETUP_SQL;

    #[pg_test]
    fn test_rank_bm25() {
        crate::setup_background_workers();
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
        crate::setup_background_workers();
        Spi::run(SETUP_SQL).expect("failed to create index and table");

        let query = r#"
            SELECT highlight_bm25
            FROM one_republic_songs.highlight('lyrics:im', highlight_field => 'lyrics', max_num_chars => 10);
        "#;

        let highlight = Spi::get_one::<&str>(query)
            .expect("failed to highlight lyrics")
            .unwrap();
        assert_eq!(highlight, "<b>Im</b> holding");
    }
}
