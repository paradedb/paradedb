use tantivy::aggregation::agg_req::{Aggregation, AggregationVariants};
use tantivy::aggregation::bucket::{
    DateHistogramAggregationReq, HistogramAggregation, HistogramBounds,
};
use tantivy::TantivyError;

use crate::postgres::datetime::{unix_millis_to_pg_micros, PostgresDateTime};
use crate::postgres::types::is_pgoid_datetime_type;
use crate::schema::SearchIndexSchema;

fn is_a_datetime_field(key: &str, schema: &SearchIndexSchema) -> bool {
    if let Some(field) = schema.search_field(key) {
        if is_pgoid_datetime_type(field.field_type().typeoid()) {
            return true;
        }
    }
    false
}

fn i64_value_to_timestamp_string(v: &serde_json::Value) -> Option<String> {
    // only rewrite numbers. If this is a string already, it doesn't need rewriting. Any other kind
    // of value should be left untouched since it's not what we expected to find.
    if let serde_json::Value::Number(num) = v {
        let pg_micros = num
            .as_i64()
            // some responses return floats
            .or_else(|| num.as_f64().map(|f| f as i64))
            .expect("This should always be a valid i64 since it's a stored timestamp value");
        let string = PostgresDateTime::try_from_raw(pg_micros)
            .expect("should always be a valid timestamp")
            .to_string();
        Some(string)
    } else {
        None
    }
}

fn rewrite_i64_value_to_timestamp_string(v: &mut serde_json::Value) {
    if let Some(string) = i64_value_to_timestamp_string(v) {
        *v = serde_json::Value::String(string);
    }
}

pub fn rewrite_aggregate_result_json_timestamps(
    output_json: &mut serde_json::Value,
    agg_json: &serde_json::Value,
    schema: &SearchIndexSchema,
) {
    // top level keys are either variant names or "aggs"
    // "aggs" contoins user-provided keys which hold more "agg" objects

    // METRICS
    // tophits
    if let Some(keys) = agg_json
        .pointer("/top_hits/docvalue_fields")
        .and_then(|v| v.as_array())
    {
        let keys: Vec<String> = keys
            .iter()
            .filter_map(|k| k.as_str())
            .filter(|k| is_a_datetime_field(k, schema))
            .map(|k| k.to_string())
            .collect();
        if !keys.is_empty() {
            // rewrite hits[].docvalue_fields.{key}
            if let Some(hits_objects) = output_json.get_mut("hits").and_then(|v| v.as_array_mut()) {
                for item in hits_objects.iter_mut().filter_map(|v| v.as_object_mut()) {
                    if let Some(fields) = item
                        .get_mut("docvalue_fields")
                        .and_then(|v| v.as_object_mut())
                    {
                        for key in keys.iter() {
                            if let Some(values) = fields.get_mut(key).and_then(|v| v.as_array_mut())
                            {
                                for v in values {
                                    rewrite_i64_value_to_timestamp_string(v);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // BUCKETS
    // (path, (rewrite key, add key as string))
    let path_and_rewrite_rules = [
        ("/terms/field", (true, false)),
        ("/histogram/field", (false, true)),
    ];
    let (rewrite_key, add_key_as_string) = path_and_rewrite_rules
        .into_iter()
        .find_map(|(path, rules)| {
            if let Some(key) = agg_json.pointer(path).and_then(|v| v.as_str()) {
                if is_a_datetime_field(key, schema) {
                    Some(rules)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .unwrap_or((false, false));

    // process buckets
    if let Some(buckets) = output_json
        .get_mut("buckets")
        .and_then(|v| v.as_array_mut())
    {
        for bucket in buckets.iter_mut().filter_map(|v| v.as_object_mut()) {
            if rewrite_key {
                if let Some(v) = bucket.get_mut("key") {
                    rewrite_i64_value_to_timestamp_string(v);
                }
            }
            if add_key_as_string {
                if let Some(v) = bucket.get("key") {
                    if let Some(key_as_str) = i64_value_to_timestamp_string(v) {
                        bucket.insert(
                            "key_as_string".to_string(),
                            serde_json::Value::String(key_as_str),
                        );
                    }
                }
            }
            // sub-aggs
            if let Some(subaggs) = agg_json.get("aggs").and_then(|v| v.as_object()) {
                for (key, subagg_json) in subaggs.iter() {
                    if let Some(suboutput_json) = bucket.get_mut(key) {
                        rewrite_aggregate_result_json_timestamps(
                            suboutput_json,
                            subagg_json,
                            schema,
                        );
                    }
                }
            }
        }
    }
}

fn unix_millis_bounds_to_pg_micros(bounds: HistogramBounds) -> HistogramBounds {
    HistogramBounds {
        min: unix_millis_to_pg_micros(bounds.min as i64) as f64,
        max: unix_millis_to_pg_micros(bounds.max as i64) as f64,
    }
}

fn date_histogram_req_to_histogram_agg(
    date_histogram: &DateHistogramAggregationReq,
) -> Result<HistogramAggregation, TantivyError> {
    let mut histogram = date_histogram.to_histogram_req()?;
    // tantivy converts the intervals to milliseconds, so we need to convert that to microseconds
    histogram.interval *= 1_000.0;
    histogram.offset = histogram.offset.map(|v| v * 1_000.0);
    // the bounds are specified as unix milliseconds, so we need to convert that to pg microseconds
    histogram.hard_bounds = histogram.hard_bounds.map(unix_millis_bounds_to_pg_micros);
    histogram.extended_bounds = histogram
        .extended_bounds
        .map(unix_millis_bounds_to_pg_micros);

    Ok(histogram)
}

/// If this agg contains date_histograms, rewrite them as regular histograms against the underlying
/// pg_micros I64 representation
pub fn rewrite_date_histogram_to_histogram(agg: &mut Aggregation) -> Result<(), TantivyError> {
    println!("rewriting: {agg:?}");
    if let AggregationVariants::DateHistogram(date_histogram) = &agg.agg {
        agg.agg =
            AggregationVariants::Histogram(date_histogram_req_to_histogram_agg(date_histogram)?);
    }
    for subagg in agg.sub_aggregation.values_mut() {
        rewrite_date_histogram_to_histogram(subagg)?;
    }
    println!("rewritten: {agg:?}");
    Ok(())
}
