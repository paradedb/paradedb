use crate::env::needs_commit;
use crate::schema::SearchConfig;
use crate::writer::WriterClient;
use crate::{globals::WriterGlobal, index::SearchIndex, postgres::utils::get_search_index};
use pgrx::{prelude::TableIterator, *};
use tantivy::{schema::FieldType, SnippetGenerator};

#[pg_extern]
pub fn rank_bm25(
    config_json: JsonB,
) -> TableIterator<'static, (name!(id, i64), name!(rank_bm25, f32))> {
    let JsonB(search_config_json) = config_json;
    let search_config: SearchConfig =
        serde_json::from_value(search_config_json).expect("could not parse search config");
    let search_index = get_search_index(&search_config.index_name);

    let writer_client = WriterGlobal::client();
    let mut scan_state = search_index
        .search_state(&writer_client, &search_config, needs_commit())
        .unwrap();
    let top_docs = scan_state.search();

    let mut field_rows = Vec::new();
    for (score, _) in top_docs.into_iter() {
        field_rows.push((score.key, score.bm25));
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
    let search_index = get_search_index(&search_config.index_name);
    let schema = search_index.schema.schema.clone();
    let field_name = search_config
        .highlight_field
        .as_ref()
        .unwrap_or_else(|| panic!("highlight_field parameter required for highlight function"));
    let writer_client = WriterGlobal::client();
    let mut scan_state = search_index
        .search_state(&writer_client, &search_config, needs_commit())
        .unwrap();
    let top_docs = scan_state.search();

    let highlight_field = schema
        .get_field(field_name)
        .unwrap_or_else(|err| panic!("error highlighting field {field_name}: {err:?}"));
    let highlight_field_entry = schema.get_field_entry(highlight_field);

    let mut snippet_generator = if let FieldType::Str(_) = highlight_field_entry.field_type() {
        SnippetGenerator::create(&search_index.searcher(), &scan_state.query, highlight_field)
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
        let key = search_index.get_key_value(&document);
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
    let search_index = get_search_index(&search_config.index_name);

    let writer_client = WriterGlobal::client();
    let mut scan_state = search_index
        .search_state(&writer_client, &search_config, needs_commit())
        .unwrap();
    let top_docs = scan_state.search();
    let (min_score, max_score) = top_docs
        .iter()
        .map(|(score, _)| *score)
        .fold((f32::MAX, f32::MIN), |(min, max), score| {
            (min.min(score.bm25), max.max(score.bm25))
        });
    let score_range = max_score - min_score;
    let mut field_rows = Vec::new();

    for (score, doc_address) in top_docs.into_iter() {
        let document = scan_state
            .doc(doc_address)
            .unwrap_or_else(|err| panic!("error retrieving document for rank_hybrid: {err:?}"));
        let key = search_index.get_key_value(&document);

        let normalized_score = if score_range == 0.0 {
            1.0
        } else {
            (score.bm25 - min_score) / score_range
        };

        field_rows.push((key, normalized_score));
    }
    TableIterator::new(field_rows)
}

#[pg_extern]
fn drop_bm25_internal(index_name: &str) {
    let writer_client = WriterGlobal::client();
    if needs_commit() {
        writer_client
            .lock()
            .expect("could not lock writer on drop_bm25")
            .request(crate::writer::WriterRequest::Commit)
            .expect("error committing existing transaction during drop_bm25");
    }
    // Drop the Tantivy data directory.
    SearchIndex::drop_index(&writer_client, index_name)
        .unwrap_or_else(|err| panic!("error dropping index {index_name}: {err}"));
}
