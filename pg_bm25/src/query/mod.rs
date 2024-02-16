#![allow(dead_code)]

use pgrx::PostgresType;
use serde::{Deserialize, Serialize};

// enum SearchQuery {
//     AllQuery,
//     BooleanQuery {
//         must: Option<Vec<Box<SearchQuery>>>,
//         should: Option<Vec<Box<SearchQuery>>>,
//         must_not: Option<Vec<Box<SearchQuery>>>,
//     },
//     BoostQuery {
//         query: Option<Box<SearchQuery>>,
//         boost: Option<f32>,
//     },
//     ConstScoreQuery {
//         query: Option<Box<SearchQuery>>,
//         score: Option<f32>,
//     },
//     DisjunctionMaxQuery {
//         disjuncts: Option<Vec<Box<SearchQuery>>>,
//         tie_breaker: Option<f32>,
//     },
//     EmptyQuery,
//     FastFieldRangeWeight {
//         field: tantivy::schema::Field,
//         lower_bound: Option<std::ops::Bound<u64>>,
//         upper_bound: Option<std::ops::Bound<u64>>,
//     },
//     FuzzyTermQuery {
//         field: tantivy::schema::Field,
//         text: String,
//         distance: u8,
//         tranposition_cost_one: bool,
//         prefix: bool,
//     },
//     MoreLikeThisQuery {
//         min_doc_frequency: Option<u64>,
//         max_doc_frequency: Option<u64>,
//         min_term_frequency: Option<usize>,
//         max_query_terms: Option<usize>,
//         min_word_length: Option<usize>,
//         max_word_length: Option<usize>,
//         boost_factor: Option<f32>,
//         stop_words: Option<Vec<String>>,
//         fields: Option<HashMap<tantivy::schema::Field, Vec<tantivy::schema::Value>>>,
//     },
//     PhrasePrefixQuery {
//         field: tantivy::schema::Field,
//         prefix: Option<String>,
//         phrases: Option<Vec<String>>,
//         max_expansion: Option<u32>,
//     },
//     PhraseQuery {
//         field: tantivy::schema::Field,
//         phrases: Option<Vec<String>>,
//         slop: Option<u32>,
//     },
//     RangeQuery {
//         field: tantivy::schema::Field,
//         lower_bound: Option<std::ops::Bound<u64>>,
//         upper_bound: Option<std::ops::Bound<u64>>,
//         schema_type: tantivy::schema::Type,
//     },
//     RegexQuery {
//         field: tantivy::schema::Field,
//         pattern: String,
//     },
//     TermQuery {
//         text: String,
//         freqs: Option<bool>,
//         position: Option<bool>,
//     },
//     TermSetQuery {
//         fields: HashMap<tantivy::schema::Field, Vec<tantivy::schema::Value>>,
//     },
// }

#[derive(PostgresType, Deserialize, Serialize)]
pub enum SearchQueryInput {
    All,
    Boolean {
        must: Option<Vec<Box<SearchQueryInput>>>,
        should: Option<Vec<Box<SearchQueryInput>>>,
        must_not: Option<Vec<Box<SearchQueryInput>>>,
    },
    Boost {
        query: Option<Box<SearchQueryInput>>,
        boost: Option<f32>,
    },
    ConstScore {
        query: Option<Box<SearchQueryInput>>,
        score: Option<f32>,
    },
    DisjunctionMax {
        disjuncts: Option<Vec<Box<SearchQueryInput>>>,
        tie_breaker: Option<f32>,
    },
    Empty,
    FastFieldRangeWeight {
        field: String,
        lower_bound: std::ops::Bound<u64>,
        upper_bound: std::ops::Bound<u64>,
    },
    FuzzyTerm {
        field: String,
        text: String,
        distance: u8,
        tranposition_cost_one: bool,
        prefix: bool,
    },
    MoreLikeThis {
        min_doc_frequency: Option<u64>,
        max_doc_frequency: Option<u64>,
        min_term_frequency: Option<usize>,
        max_query_terms: Option<usize>,
        min_word_length: Option<usize>,
        max_word_length: Option<usize>,
        boost_factor: Option<f32>,
        stop_words: Option<Vec<String>>,
        fields: serde_json::Value,
    },
    PhrasePrefix {
        field: String,
        prefix: Option<String>,
        phrases: Option<Vec<String>>,
        max_expansion: Option<u32>,
    },
    Phrase {
        field: String,
        phrases: Option<Vec<String>>,
        slop: Option<u32>,
    },
    Range {
        field: String,
        lower_bound: std::ops::Bound<u64>,
        upper_bound: std::ops::Bound<u64>,
    },
    Regex {
        field: String,
        pattern: String,
    },
    Term {
        field: String,
        text: String,
        freqs: Option<bool>,
        position: Option<bool>,
    },
    TermSet {
        fields: serde_json::Value,
    },
}
