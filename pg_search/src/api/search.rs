use crate::env::needs_commit;
use crate::index::state::{SearchAlias, SearchStateManager};
use crate::schema::SearchConfig;
use crate::writer::{WriterClient, WriterDirectory};
use crate::{globals::WriterGlobal, index::SearchIndex, postgres::utils::get_search_index};
use anyhow::{anyhow, Result};
use pgrx::{prelude::TableIterator, *};
use tantivy::aggregation::agg_req::Aggregations;
use tantivy::aggregation::agg_result::AggregationResults;
use tantivy::aggregation::AggregationCollector;

const DEFAULT_SNIPPET_PREFIX: &str = "<b>";
const DEFAULT_SNIPPET_POSTFIX: &str = "</b>";

#[pg_extern]
pub fn rank_bm25(key: i64, alias: default!(Option<String>, "NULL")) -> f32 {
    SearchStateManager::get_score(key, alias.map(SearchAlias::from))
        .expect("could not lookup doc address for search query")
}

#[pg_extern]
pub fn highlight(
    key: i64,
    field: &str,
    prefix: default!(Option<String>, "NULL"),
    postfix: default!(Option<String>, "NULL"),
    max_num_chars: default!(Option<i32>, "NULL"),
    alias: default!(Option<String>, "NULL"),
) -> String {
    let mut snippet = SearchStateManager::get_snippet(
        key,
        field,
        max_num_chars.map(|n| n as usize),
        alias.map(SearchAlias::from),
    )
    .expect("could not create snippet for highlighting");

    match (prefix, postfix) {
        (Some(prefix), Some(postfix)) => snippet.set_snippet_prefix_postfix(&prefix, &postfix),
        (None, Some(postfix)) => {
            snippet.set_snippet_prefix_postfix(DEFAULT_SNIPPET_PREFIX, &postfix)
        }
        (Some(prefix), None) => {
            snippet.set_snippet_prefix_postfix(&prefix, DEFAULT_SNIPPET_POSTFIX)
        }
        _ => snippet.set_snippet_prefix_postfix(DEFAULT_SNIPPET_PREFIX, DEFAULT_SNIPPET_POSTFIX),
    }

    snippet.to_html()
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

    // Collect into a Vec to allow multiple iterations
    let top_docs: Vec<_> = scan_state.search_dedup(search_index.executor).collect();

    // Calculate min and max scores
    let (min_score, max_score) = top_docs
        .iter()
        .map(|(score, _)| score)
        .fold((f32::MAX, f32::MIN), |(min, max), bm25| {
            (min.min(*bm25), max.max(*bm25))
        });
    let score_range = max_score - min_score;

    // Now that we have min and max, iterate over the collected results
    let mut field_rows = Vec::new();
    for (score, doc_address) in top_docs {
        let key = scan_state.key_value(doc_address);
        let normalized_score = if score_range == 0.0 {
            1.0 // Avoid division by zero
        } else {
            (score - min_score) / score_range
        };

        field_rows.push((key, normalized_score));
    }
    TableIterator::new(field_rows)
}

#[pg_extern]
fn drop_bm25_internal(index_name: &str) {
    let writer_client = WriterGlobal::client();
    let writer_directory = WriterDirectory::from_index_name(index_name);
    if needs_commit() {
        writer_client
            .lock()
            .expect("could not lock writer on drop_bm25")
            .request(crate::writer::WriterRequest::Commit {
                directory: writer_directory,
            })
            .expect("error committing existing transaction during drop_bm25");
    }
    // Drop the Tantivy data directory.
    SearchIndex::drop_index(&writer_client, index_name)
        .unwrap_or_else(|err| panic!("error dropping index {index_name}: {err}"));
}

#[pg_extern]
#[allow(unused)]
pub fn aggregate_internal(
    aggs: String,
    config_json: default!(Option<JsonB>, "NULL"),
) -> Result<JsonB> {
    let JsonB(search_config_json) =
        config_json.ok_or(anyhow!("no config json passed to aggregate"))?;
    let search_config: SearchConfig =
        serde_json::from_value(search_config_json).expect("could not parse search config");
    let mut search_index = get_search_index(&search_config.index_name);

    // Must reload, or new results will not appear in the search.
    search_index.reader.reload()?;

    let tantivy_aggs: Aggregations = serde_json::from_str(&aggs)?;
    let tantivy_query = search_config
        .query
        .into_tantivy_query(&search_index.schema, &mut search_index.query_parser())?;
    let collector = AggregationCollector::from_aggs(tantivy_aggs, Default::default());

    let searcher = search_index.searcher();
    let results: AggregationResults = searcher.search_with_executor(
        &tantivy_query,
        &collector,
        &search_index.executor,
        tantivy::query::EnableScoring::Enabled {
            searcher: &searcher,
            statistics_provider: &searcher,
        },
    )?;
    Ok(JsonB(serde_json::to_value(results)?))
}
