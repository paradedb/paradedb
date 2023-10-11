use pgrx::*;
use serde_json::{from_str, Value as JsonValue};
use tantivy::aggregation::agg_req::Aggregations;
use tantivy::aggregation::agg_result::AggregationResults;
use tantivy::aggregation::AggregationCollector;
use tantivy::query::AllQuery;

use crate::index_access::utils::get_parade_index;

#[pg_extern]
pub fn aggregation(index_name: &str, query: &str) -> JsonB {
    // Lookup the index by name, and setup its tokenizer functions.
    let mut parade_index = get_parade_index(index_name.to_string());
    parade_index.setup_tokenizers();

    let underlying_index = parade_index.underlying_index;

    // Initialize aggregation searcher + collector
    let reader = underlying_index
        .reader()
        .expect("failed to create index reader");
    let searcher = reader.searcher();
    let agg_req: Aggregations = from_str(query).expect("error parsing query");
    let collector = AggregationCollector::from_aggs(agg_req, Default::default());

    // Collect aggregation results
    let agg_res: AggregationResults = searcher
        .search(&AllQuery, &collector)
        .expect("error collecting aggregation results");
    let res: JsonValue = serde_json::to_value(agg_res).unwrap();

    JsonB(res)
}
