use pgrx::{iter::TableIterator, *};
use tantivy::schema::*;

use crate::globals::SECONDS_IN_DAY;
use crate::postgres::utils::{
    get_search_index, pgrx_date_to_tantivy_value, pgrx_time_to_tantivy_value,
    pgrx_timestamp_to_tantivy_value, pgrx_timestamptz_to_tantivy_value,
    pgrx_timetz_to_tantivy_value,
};
use crate::query::SearchQueryInput;
use crate::schema::ToString;
use core::panic;
use std::ops::Bound;

#[allow(clippy::type_complexity)]
#[pg_extern]
pub fn schema_bm25(
    index_name: &str,
) -> TableIterator<(
    name!(name, String),
    name!(field_type, String),
    name!(stored, bool),
    name!(indexed, bool),
    name!(fast, bool),
    name!(fieldnorms, bool),
    name!(expand_dots, Option<bool>),
    name!(tokenizer, Option<String>),
    name!(record, Option<String>),
    name!(normalizer, Option<String>),
)> {
    let bm25_index_name = format!("{}_bm25_index", index_name);
    let search_index = get_search_index(&bm25_index_name);
    let schema = search_index.schema.schema.clone();
    let mut field_entries: Vec<_> = schema.fields().collect();

    // To ensure consistent ordering of outputs, we'll sort the results by field name.
    field_entries.sort_by_key(|(field, _)| schema.get_field_name(*field).to_string());

    let mut field_rows = Vec::new();

    for field in field_entries {
        let (field, field_entry) = field;
        let name = schema.get_field_name(field).to_string();

        let (field_type, tokenizer, record, normalizer, expand_dots) =
            match field_entry.field_type() {
                FieldType::I64(_) => ("I64".to_string(), None, None, None, None),
                FieldType::U64(_) => ("U64".to_string(), None, None, None, None),
                FieldType::F64(_) => ("F64".to_string(), None, None, None, None),
                FieldType::Bool(_) => ("Bool".to_string(), None, None, None, None),
                FieldType::Str(text_options) => {
                    let indexing_options = text_options.get_indexing_options();
                    let tokenizer = indexing_options.map(|opt| opt.tokenizer().to_string());
                    let record = indexing_options.map(|opt| opt.index_option().to_string());
                    let normalizer = text_options
                        .get_fast_field_tokenizer_name()
                        .map(|s| s.to_string());
                    ("Str".to_string(), tokenizer, record, normalizer, None)
                }
                FieldType::JsonObject(json_options) => {
                    let indexing_options = json_options.get_text_indexing_options();
                    let tokenizer = indexing_options.map(|opt| opt.tokenizer().to_string());
                    let record = indexing_options.map(|opt| opt.index_option().to_string());
                    let normalizer = json_options
                        .get_fast_field_tokenizer_name()
                        .map(|s| s.to_string());
                    let expand_dots = Some(json_options.is_expand_dots_enabled());
                    (
                        "JsonObject".to_string(),
                        tokenizer,
                        record,
                        normalizer,
                        expand_dots,
                    )
                }
                FieldType::Date(_) => ("Date".to_string(), None, None, None, None),
                _ => ("Other".to_string(), None, None, None, None),
            };

        let row = (
            name,
            field_type,
            field_entry.is_stored(),
            field_entry.is_indexed(),
            field_entry.is_fast(),
            field_entry.has_fieldnorms(),
            expand_dots,
            tokenizer,
            record,
            normalizer,
        );

        field_rows.push(row);
    }

    TableIterator::new(field_rows)
}

#[pg_extern(immutable, parallel_safe)]
pub fn all() -> SearchQueryInput {
    SearchQueryInput::All
}

#[pg_extern(name = "boolean", immutable, parallel_safe)]
pub fn boolean_arrays(
    must: default!(Vec<SearchQueryInput>, "ARRAY[]::searchqueryinput[]"),
    should: default!(Vec<SearchQueryInput>, "ARRAY[]::searchqueryinput[]"),
    must_not: default!(Vec<SearchQueryInput>, "ARRAY[]::searchqueryinput[]"),
) -> SearchQueryInput {
    SearchQueryInput::Boolean {
        must,
        should,
        must_not,
    }
}

#[pg_extern(name = "boolean", immutable, parallel_safe)]
pub fn boolean_singles(
    must: default!(Option<SearchQueryInput>, "NULL"),
    should: default!(Option<SearchQueryInput>, "NULL"),
    must_not: default!(Option<SearchQueryInput>, "NULL"),
) -> SearchQueryInput {
    boolean_arrays(
        must.map_or(vec![], |v| vec![v]),
        should.map_or(vec![], |v| vec![v]),
        must_not.map_or(vec![], |v| vec![v]),
    )
}

#[pg_extern(immutable, parallel_safe)]
pub fn boost(boost: f32, query: SearchQueryInput) -> SearchQueryInput {
    SearchQueryInput::Boost {
        query: Box::new(query),
        boost,
    }
}

#[pg_extern(immutable, parallel_safe)]
pub fn const_score(score: f32, query: SearchQueryInput) -> SearchQueryInput {
    SearchQueryInput::ConstScore {
        query: Box::new(query),
        score,
    }
}

#[pg_extern(immutable, parallel_safe)]
pub fn disjunction_max(
    disjuncts: Array<SearchQueryInput>,
    tie_breaker: default!(Option<f32>, "NULL"),
) -> SearchQueryInput {
    SearchQueryInput::DisjunctionMax {
        disjuncts: disjuncts.iter_deny_null().collect(),
        tie_breaker,
    }
}

#[pg_extern(immutable, parallel_safe)]
pub fn empty() -> SearchQueryInput {
    SearchQueryInput::Empty
}

// Not clear on whether this query makes sense to support, as only our "key_field" is a fast
// field... and the user can just use SQL to select a range. We'll keep the implementation here
// for now, but we should remove when we decide definitively that we don't need this.
#[allow(unused)]
pub fn fast_field_range_weight(field: String, range: pgrx::Range<i32>) -> SearchQueryInput {
    match range.into_inner() {
        None => SearchQueryInput::FastFieldRangeWeight {
            field,
            lower_bound: Bound::Included(0),
            upper_bound: Bound::Excluded(0),
        },
        Some((lower, upper)) => SearchQueryInput::FastFieldRangeWeight {
            field,
            lower_bound: match lower {
                RangeBound::Infinite => Bound::Unbounded,
                RangeBound::Inclusive(n) => Bound::Included(n as u64),
                RangeBound::Exclusive(n) => Bound::Excluded(n as u64),
            },
            upper_bound: match upper {
                RangeBound::Infinite => Bound::Unbounded,
                RangeBound::Inclusive(n) => Bound::Included(n as u64),
                RangeBound::Exclusive(n) => Bound::Excluded(n as u64),
            },
        },
    }
}

#[pg_extern(immutable, parallel_safe)]
pub fn fuzzy_term(
    field: String,
    value: String,
    distance: default!(Option<i32>, "NULL"),
    tranposition_cost_one: default!(Option<bool>, "NULL"),
    prefix: default!(Option<bool>, "NULL"),
) -> SearchQueryInput {
    SearchQueryInput::FuzzyTerm {
        field,
        value,
        distance: distance.map(|n| n as u8),
        tranposition_cost_one,
        prefix,
    }
}

// Avoid exposing more_like_this for now until we can decide on the exact API.
// Lucene and Elasticsearch seem to have different interfaces for this query,
// and Tantivy doesn't have any examples of its use, so its unclear what the best
// way to use it is with pg_search.
#[allow(unused)]
#[allow(clippy::too_many_arguments)]
pub fn more_like_this(
    min_doc_frequency: default!(Option<i32>, "NULL"),
    max_doc_frequency: default!(Option<i32>, "NULL"),
    min_term_frequency: default!(Option<i32>, "NULL"),
    max_query_terms: default!(Option<i32>, "NULL"),
    min_word_length: default!(Option<i32>, "NULL"),
    max_word_length: default!(Option<i32>, "NULL"),
    boost_factor: default!(Option<f32>, "NULL"),
    stop_words: default!(Option<Vec<String>>, "NULL"),
    fields: default!(Array<SearchQueryInput>, "ARRAY[]::searchqueryinput[]"),
) -> SearchQueryInput {
    let fields = fields.iter_deny_null().map(|input| match input {
        SearchQueryInput::Term { field, value, .. } => (field.unwrap_or("".into()), value),
        _ => panic!("only term queries can be passed to more_like_this"),
    });
    SearchQueryInput::MoreLikeThis {
        min_doc_frequency: min_doc_frequency.map(|n| n as u64),
        max_doc_frequency: max_doc_frequency.map(|n| n as u64),
        min_term_frequency: min_term_frequency.map(|n| n as usize),
        max_query_terms: max_query_terms.map(|n| n as usize),
        min_word_length: min_word_length.map(|n| n as usize),
        max_word_length: max_word_length.map(|n| n as usize),
        boost_factor,
        stop_words,
        fields: fields.collect(),
    }
}

#[pg_extern(immutable, parallel_safe)]
pub fn parse(query_string: String) -> SearchQueryInput {
    SearchQueryInput::Parse { query_string }
}

#[pg_extern(immutable, parallel_safe)]
pub fn phrase(
    field: String,
    phrases: Array<String>,
    slop: default!(Option<i32>, "NULL"),
) -> SearchQueryInput {
    SearchQueryInput::Phrase {
        field,
        phrases: phrases.iter_deny_null().collect(),
        slop: slop.map(|n| n as u32),
    }
}

#[pg_extern(immutable, parallel_safe)]
pub fn phrase_prefix(
    field: String,
    phrases: Array<String>,
    max_expansion: default!(Option<i32>, "NULL"),
) -> SearchQueryInput {
    SearchQueryInput::PhrasePrefix {
        field,
        phrases: phrases.iter_deny_null().collect(),
        max_expansions: max_expansion.map(|n| n as u32),
    }
}

// We'll also avoid exposing range queries for now, as they are slightly complex
// to map to Tantivy ranges, and it's not clear if they're necessary at all in the context of
// Postgres, which can query by range on its own.
// #[allow(unused)]
#[pg_extern(name = "range", immutable, parallel_safe)]
pub fn range_i32(field: String, range: Range<i32>) -> SearchQueryInput {
    match range.into_inner() {
        None => SearchQueryInput::Range {
            field,
            lower_bound: Bound::Included(Value::I64(0)),
            upper_bound: Bound::Excluded(Value::I64(0)),
        },
        Some((lower, upper)) => SearchQueryInput::Range {
            field,
            lower_bound: match lower {
                RangeBound::Infinite => Bound::Unbounded,
                RangeBound::Inclusive(n) => Bound::Included(Value::I64(n as i64)),
                RangeBound::Exclusive(n) => Bound::Excluded(Value::I64(n as i64)),
            },
            upper_bound: match upper {
                RangeBound::Infinite => Bound::Unbounded,
                RangeBound::Inclusive(n) => Bound::Included(Value::I64(n as i64)),
                RangeBound::Exclusive(n) => Bound::Excluded(Value::I64(n as i64)),
            },
        },
    }
}

// As with range_i32, we'll avoid exposing range queries for now.
// #[allow(unused)]
#[pg_extern(name = "range", immutable, parallel_safe)]
pub fn range_i64(field: String, range: Range<i64>) -> SearchQueryInput {
    match range.into_inner() {
        None => SearchQueryInput::Range {
            field,
            lower_bound: Bound::Included(Value::I64(0)),
            upper_bound: Bound::Excluded(Value::I64(0)),
        },
        Some((lower, upper)) => SearchQueryInput::Range {
            field,
            lower_bound: match lower {
                RangeBound::Infinite => Bound::Unbounded,
                RangeBound::Inclusive(n) => Bound::Included(Value::I64(n)),
                RangeBound::Exclusive(n) => Bound::Excluded(Value::I64(n)),
            },
            upper_bound: match upper {
                RangeBound::Infinite => Bound::Unbounded,
                RangeBound::Inclusive(n) => Bound::Included(Value::I64(n)),
                RangeBound::Exclusive(n) => Bound::Excluded(Value::I64(n)),
            },
        },
    }
}

#[pg_extern(name = "range", immutable, parallel_safe)]
pub fn range_date(field: String, range: Range<pgrx::Date>) -> SearchQueryInput {
    match range.into_inner() {
        None => SearchQueryInput::Range {
            field,
            lower_bound: Bound::Included(tantivy::schema::Value::Date(
                tantivy::DateTime::from_timestamp_micros(0),
            )),
            upper_bound: Bound::Excluded(tantivy::schema::Value::Date(
                tantivy::DateTime::from_timestamp_micros(0),
            )),
        },
        Some((lower, upper)) => SearchQueryInput::Range {
            field,
            lower_bound: match lower {
                RangeBound::Infinite => Bound::Unbounded,
                RangeBound::Inclusive(n) => Bound::Included(pgrx_date_to_tantivy_value(n)),
                RangeBound::Exclusive(n) => Bound::Excluded(pgrx_date_to_tantivy_value(n)),
            },
            upper_bound: match upper {
                RangeBound::Infinite => Bound::Unbounded,
                RangeBound::Inclusive(n) => Bound::Included(pgrx_date_to_tantivy_value(n)),
                RangeBound::Exclusive(n) => Bound::Excluded(pgrx_date_to_tantivy_value(n)),
            },
        },
    }
}

#[pg_extern(name = "range", immutable, parallel_safe)]
pub fn range_timestamp(field: String, range: Range<pgrx::Timestamp>) -> SearchQueryInput {
    match range.into_inner() {
        None => SearchQueryInput::Range {
            field,
            lower_bound: Bound::Included(tantivy::schema::Value::Date(
                tantivy::DateTime::from_timestamp_micros(0),
            )),
            upper_bound: Bound::Excluded(tantivy::schema::Value::Date(
                tantivy::DateTime::from_timestamp_micros(0),
            )),
        },
        Some((lower, upper)) => SearchQueryInput::Range {
            field,
            lower_bound: match lower {
                RangeBound::Infinite => Bound::Unbounded,
                RangeBound::Inclusive(n) => Bound::Included(pgrx_timestamp_to_tantivy_value(n)),
                RangeBound::Exclusive(n) => Bound::Excluded(pgrx_timestamp_to_tantivy_value(n)),
            },
            upper_bound: match upper {
                RangeBound::Infinite => Bound::Unbounded,
                RangeBound::Inclusive(n) => Bound::Included(pgrx_timestamp_to_tantivy_value(n)),
                RangeBound::Exclusive(n) => Bound::Excluded(pgrx_timestamp_to_tantivy_value(n)),
            },
        },
    }
}

#[pg_extern(name = "range", immutable, parallel_safe)]
pub fn range_timestamptz(
    field: String,
    range: Range<pgrx::TimestampWithTimeZone>,
) -> SearchQueryInput {
    match range.into_inner() {
        None => SearchQueryInput::Range {
            field,
            lower_bound: Bound::Included(tantivy::schema::Value::Date(
                tantivy::DateTime::from_timestamp_micros(0),
            )),
            upper_bound: Bound::Excluded(tantivy::schema::Value::Date(
                tantivy::DateTime::from_timestamp_micros(0),
            )),
        },
        Some((lower, upper)) => SearchQueryInput::Range {
            field,
            lower_bound: match lower {
                RangeBound::Infinite => Bound::Unbounded,
                RangeBound::Inclusive(n) => Bound::Included(pgrx_timestamptz_to_tantivy_value(n)),
                RangeBound::Exclusive(n) => Bound::Excluded(pgrx_timestamptz_to_tantivy_value(n)),
            },
            upper_bound: match upper {
                RangeBound::Infinite => Bound::Unbounded,
                RangeBound::Inclusive(n) => Bound::Included(pgrx_timestamptz_to_tantivy_value(n)),
                RangeBound::Exclusive(n) => Bound::Excluded(pgrx_timestamptz_to_tantivy_value(n)),
            },
        },
    }
}

#[pg_extern(immutable, parallel_safe)]
pub fn regex(field: String, pattern: String) -> SearchQueryInput {
    SearchQueryInput::Regex { field, pattern }
}

macro_rules! term_fn {
    ($func_name:ident, $value_type:ty, $conversion:expr) => {
        #[pg_extern(name = "term", immutable, parallel_safe)]
        pub fn $func_name(
            field: default!(Option<String>, "NULL"),
            value: default!(Option<$value_type>, "NULL"),
        ) -> SearchQueryInput {
            let convert = $conversion;
            if let Some(value) = value {
                SearchQueryInput::Term {
                    field,
                    value: convert(value),
                }
            } else {
                panic!("no value provided to term query")
            }
        }
    };
}

// Generate functions for each type
term_fn!(term_bytes, Vec<u8>, tantivy::schema::Value::Bytes);
term_fn!(term_str, String, tantivy::schema::Value::Str);
term_fn!(term_i8, i8, |v| tantivy::schema::Value::I64(v as i64));
term_fn!(term_i16, i16, |v| tantivy::schema::Value::I64(v as i64));
term_fn!(term_i32, i32, |v| tantivy::schema::Value::I64(v as i64));
term_fn!(term_i64, i64, tantivy::schema::Value::I64);
term_fn!(term_f32, f32, |v| tantivy::schema::Value::F64(v as f64));
term_fn!(term_f64, f64, tantivy::schema::Value::F64);
term_fn!(term_bool, bool, tantivy::schema::Value::Bool);
term_fn!(json, pgrx::Json, |pgrx::Json(v)| {
    tantivy::schema::Value::JsonObject(
        v.as_object()
            .expect("json passed to term query must be an object")
            .clone(),
    )
});
term_fn!(jsonb, pgrx::JsonB, |pgrx::JsonB(v)| {
    tantivy::schema::Value::JsonObject(
        v.as_object()
            .expect("jsonb passed to term query must be an object")
            .clone(),
    )
});
term_fn!(date, pgrx::Date, |v: pgrx::Date| {
    pgrx_date_to_tantivy_value(v)
});
term_fn!(time, pgrx::Time, pgrx_time_to_tantivy_value);
term_fn!(timestamp, pgrx::Timestamp, pgrx_timestamp_to_tantivy_value);
term_fn!(
    time_with_time_zone,
    pgrx::TimeWithTimeZone,
    pgrx_timetz_to_tantivy_value
);
term_fn!(
    timestamp_with_time_zome,
    pgrx::TimestampWithTimeZone,
    pgrx_timestamptz_to_tantivy_value
);
term_fn!(anyarray, pgrx::AnyArray, |_v| unimplemented!(
    "array in term query not implemented"
));
term_fn!(pg_box, pgrx::pg_sys::BOX, |_v| unimplemented!(
    "box in term query not implemented"
));
term_fn!(point, pgrx::pg_sys::Point, |_v| unimplemented!(
    "point in term query not implemented"
));
term_fn!(tid, pgrx::pg_sys::ItemPointerData, |_v| unimplemented!(
    "tid in term query not implemented"
));
term_fn!(inet, pgrx::Inet, |_v| unimplemented!(
    "inet in term query not implemented"
));
term_fn!(numeric, pgrx::AnyNumeric, |_v| unimplemented!(
    "numeric in term query not implemented"
));
term_fn!(int4range, pgrx::Range<i32>, |_v| unimplemented!(
    "int4 range in term query not implemented"
));
term_fn!(int8range, pgrx::Range<i64>, |_v| unimplemented!(
    "int8 range in term query not implemented"
));
term_fn!(
    numrange,
    pgrx::Range<pgrx::AnyNumeric>,
    |_v| unimplemented!("numeric range in term query not implemented")
);
term_fn!(daterange, pgrx::Range<pgrx::Date>, |_v| unimplemented!(
    "date range in term query not implemented"
));
term_fn!(tsrange, pgrx::Range<pgrx::Timestamp>, |_v| unimplemented!(
    "timestamp ranges in term query not implemented"
));
term_fn!(
    tstzrange,
    pgrx::Range<pgrx::TimestampWithTimeZone>,
    |_v| unimplemented!("timestamp ranges with time zone in term query not implemented")
);
term_fn!(uuid, pgrx::Uuid, |_v| unimplemented!(
    "uuid in term query not implemented"
));

#[pg_extern(immutable, parallel_safe)]
pub fn term_set(
    terms: default!(Vec<SearchQueryInput>, "ARRAY[]::searchqueryinput[]"),
) -> SearchQueryInput {
    let terms: Vec<_> = terms
        .into_iter()
        .filter_map(|input| match input {
            SearchQueryInput::Term { field, value, .. } => field.map(|field| (field, value)),
            _ => panic!("only term queries can be passed to term_set"),
        })
        .collect();

    SearchQueryInput::TermSet { terms }
}
