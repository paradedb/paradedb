use pgrx::{iter::TableIterator, *};
use tantivy::schema::*;

use crate::postgres::utils::get_search_index;
use crate::query::SearchQueryInput;
use crate::schema::ToString;
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

#[pg_extern]
pub fn all() -> SearchQueryInput {
    SearchQueryInput::All
}

#[pg_extern]
pub fn boolean(
    must: Option<Vec<SearchQueryInput>>,
    should: Option<Vec<SearchQueryInput>>,
    must_not: Option<Vec<SearchQueryInput>>,
) -> SearchQueryInput {
    SearchQueryInput::Boolean {
        must: must.map(|v| v.into_iter().map(Box::new).collect()),
        should: should.map(|v| v.into_iter().map(Box::new).collect()),
        must_not: must_not.map(|v| v.into_iter().map(Box::new).collect()),
    }
}

#[pg_extern]
pub fn boost(query: Option<SearchQueryInput>, boost: Option<f32>) -> SearchQueryInput {
    SearchQueryInput::Boost {
        query: query.map(Box::new),
        boost,
    }
}

#[pg_extern]
pub fn const_score(query: Option<SearchQueryInput>, score: Option<f32>) -> SearchQueryInput {
    SearchQueryInput::ConstScore {
        query: query.map(Box::new),
        score,
    }
}

#[pg_extern]
pub fn disjunction_max(
    disjuncts: Option<Vec<SearchQueryInput>>,
    tie_breaker: Option<f32>,
) -> SearchQueryInput {
    SearchQueryInput::DisjunctionMax {
        disjuncts: disjuncts.map(|v| v.into_iter().map(Box::new).collect()),
        tie_breaker,
    }
}

#[pg_extern]
pub fn empty() -> SearchQueryInput {
    SearchQueryInput::Empty
}

#[pg_extern]
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

#[pg_extern]
pub fn fuzzy_term(
    field: String,
    text: String,
    distance: i16,
    tranposition_cost_one: bool,
    prefix: bool,
) -> SearchQueryInput {
    SearchQueryInput::FuzzyTerm {
        field,
        text,
        distance: distance as u8,
        tranposition_cost_one,
        prefix,
    }
}

#[pg_extern]
pub fn more_like_this(
    min_doc_frequency: Option<i32>,
    max_doc_frequency: Option<i32>,
    min_term_frequency: Option<i32>,
    max_query_terms: Option<i32>,
    min_word_length: Option<i32>,
    max_word_length: Option<i32>,
    boost_factor: Option<f32>,
    stop_words: Option<Vec<String>>,
    fields: Json,
) -> SearchQueryInput {
    SearchQueryInput::MoreLikeThis {
        min_doc_frequency: min_doc_frequency.map(|n| n as u64),
        max_doc_frequency: max_doc_frequency.map(|n| n as u64),
        min_term_frequency: min_term_frequency.map(|n| n as usize),
        max_query_terms: max_query_terms.map(|n| n as usize),
        min_word_length: min_word_length.map(|n| n as usize),
        max_word_length: max_word_length.map(|n| n as usize),
        boost_factor,
        stop_words,
        fields: fields.0.into(),
    }
}

#[pg_extern]
pub fn phrase_prefix(
    field: String,
    prefix: Option<String>,
    phrases: Option<Array<String>>,
    max_expansion: Option<i32>,
) -> SearchQueryInput {
    SearchQueryInput::PhrasePrefix {
        field,
        prefix,
        phrases: phrases.map(|arr| arr.iter_deny_null().collect()),
        max_expansion: max_expansion.map(|n| n as u32),
    }
}

#[pg_extern]
pub fn phrase(
    field: String,
    phrases: Option<Array<String>>,
    slop: Option<i32>,
) -> SearchQueryInput {
    SearchQueryInput::Phrase {
        field,
        phrases: phrases.map(|arr| arr.iter_deny_null().collect()),
        slop: slop.map(|n| n as u32),
    }
}

#[pg_extern]
pub fn range(field: String, range: Range<i32>) -> SearchQueryInput {
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

#[pg_extern]
pub fn regex(field: String, pattern: String) -> SearchQueryInput {
    SearchQueryInput::Regex { field, pattern }
}

#[pg_extern]
pub fn term(
    field: String,
    text: String,
    freqs: Option<bool>,
    position: Option<bool>,
) -> SearchQueryInput {
    SearchQueryInput::Term {
        field,
        text,
        freqs,
        position,
    }
}

#[pg_extern]
pub fn term_set(fields: Json) -> SearchQueryInput {
    SearchQueryInput::TermSet { fields: fields.0 }
}
