use json5::from_str;
use pgrx::*;
use serde_json::Value as JsonValue;
use tantivy::aggregation::agg_req::Aggregations;
use tantivy::aggregation::agg_result::AggregationResults;
use tantivy::aggregation::AggregationCollector;
use tantivy::query::AllQuery;

use crate::index_access::utils::get_parade_index;

#[pg_extern]
pub fn aggregation(index_name: &str, query: &str) -> JsonB {
    // Get Parade index
    let parade_index = get_parade_index(index_name.to_string());

    // Initialize aggregation searcher + collector
    let searcher = parade_index.searcher();
    let agg_req: Aggregations = from_str(query).expect("error parsing query");
    let collector = AggregationCollector::from_aggs(agg_req, Default::default());

    // Collect aggregation results
    let agg_res: AggregationResults = searcher
        .search(&AllQuery, &collector)
        .expect("error collecting aggregation results");
    let res: JsonValue = serde_json::to_value(agg_res).unwrap();

    JsonB(res)
}

#[cfg(feature = "pg_test")]
#[pgrx::pg_schema]
mod tests {
    use super::aggregation;
    use pgrx::*;

    const SETUP_SQL: &str = include_str!("../../sql/index_setup.sql");

    #[pg_test]
    fn test_histogram_aggregation() {
        Spi::run(SETUP_SQL).expect("failed to setup index");
        let query = r#"
            {
                aggs: {
                    histogram: {
                        field: "release_year",
                        interval: 1,
                    }
                }
            }
        "#;

        let res = aggregation("idx_one_republic", query);
        let res = res.0.as_object().unwrap();
        let aggs = res["aggs"].as_object().unwrap();
        let buckets = aggs["buckets"].as_array().unwrap();

        assert!(buckets.is_empty());
    }

    #[pg_test]
    fn test_terms_aggregation() {
        Spi::run(SETUP_SQL).expect("failed to setup index");
        let query = r#"
            {
                aggs: {
                    terms: {
                        field: "lyrics",
                    }
                }
            }
        "#;

        let res = aggregation("idx_one_republic", query);
        let res = res.0.as_object().unwrap();
        let aggs = res["aggs"].as_object().unwrap();
        let buckets = aggs["buckets"].as_array().unwrap();

        assert!(buckets.is_empty());
    }

    #[pg_test]
    fn test_metric_max() {
        Spi::run(SETUP_SQL).expect("failed to setup index");
        let query = r#"
          {  aggs: {
                avg: {
                    field: "release_year"
                }
            }
        }
        "#;

        let res = aggregation("idx_one_republic", query);
        let res = res.0.as_object().unwrap();
        let aggs = res["aggs"].as_object().unwrap();
        let value = &aggs["value"];

        assert!(value.is_null());
    }
}
