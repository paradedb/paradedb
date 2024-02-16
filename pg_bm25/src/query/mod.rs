#![allow(dead_code)]

use pgrx::PostgresType;
use serde::{Deserialize, Serialize};
use tantivy::{
    query::{
        AllQuery, BooleanQuery, BoostQuery, ConstScoreQuery, DisjunctionMaxQuery, EmptyQuery,
        FastFieldRangeWeight, Query,
    },
    query_grammar::Occur,
    schema::FieldType,
};

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
        query: Box<SearchQueryInput>,
        boost: f32,
    },
    ConstScore {
        query: Box<SearchQueryInput>,
        score: f32,
    },
    DisjunctionMax {
        disjuncts: Vec<Box<SearchQueryInput>>,
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
        value: String,
        distance: Option<u8>,
        tranposition_cost_one: Option<bool>,
        prefix: Option<bool>,
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

trait AsFieldType<T> {
    fn as_field_type(&self, from: T) -> Option<FieldType>;

    fn as_str(&self, from: T) -> Option<FieldType> {
        self.as_field_type(from).and_then(|ft| match ft {
            FieldType::Str(_) => Some(ft),
            _ => None,
        })
    }
    fn as_u64(&self, from: T) -> Option<FieldType> {
        self.as_field_type(from).and_then(|ft| match ft {
            FieldType::U64(_) => Some(ft),
            _ => None,
        })
    }
    fn as_i64(&self, from: T) -> Option<FieldType> {
        self.as_field_type(from).and_then(|ft| match ft {
            FieldType::I64(_) => Some(ft),
            _ => None,
        })
    }
    fn as_f64(&self, from: T) -> Option<FieldType> {
        self.as_field_type(from).and_then(|ft| match ft {
            FieldType::F64(_) => Some(ft),
            _ => None,
        })
    }
    fn as_bool(&self, from: T) -> Option<FieldType> {
        self.as_field_type(from).and_then(|ft| match ft {
            FieldType::Bool(_) => Some(ft),
            _ => None,
        })
    }
    fn as_date(&self, from: T) -> Option<FieldType> {
        self.as_field_type(from).and_then(|ft| match ft {
            FieldType::Date(_) => Some(ft),
            _ => None,
        })
    }
    fn as_facet(&self, from: T) -> Option<FieldType> {
        self.as_field_type(from).and_then(|ft| match ft {
            FieldType::Facet(_) => Some(ft),
            _ => None,
        })
    }
    fn as_bytes(&self, from: T) -> Option<FieldType> {
        self.as_field_type(from).and_then(|ft| match ft {
            FieldType::Bytes(_) => Some(ft),
            _ => None,
        })
    }
    fn as_json_object(&self, from: T) -> Option<FieldType> {
        self.as_field_type(from).and_then(|ft| match ft {
            FieldType::JsonObject(_) => Some(ft),
            _ => None,
        })
    }
    fn as_ip_addr(&self, from: T) -> Option<FieldType> {
        self.as_field_type(from).and_then(|ft| match ft {
            FieldType::IpAddr(_) => Some(ft),
            _ => None,
        })
    }
}

impl SearchQueryInput {
    fn into_tantivy_query<'a>(
        self,
        lookup: &'a impl AsFieldType<&'a str>,
    ) -> Result<Box<dyn Query>, Box<dyn std::error::Error>> {
        match self {
            Self::All => Ok(Box::new(AllQuery)),
            Self::Boolean {
                must,
                should,
                must_not,
            } => {
                let mut subqueries = vec![];
                for input in must.unwrap_or_default() {
                    subqueries.push((Occur::Must, input.into_tantivy_query(lookup)?));
                }
                for input in should.unwrap_or_default() {
                    subqueries.push((Occur::Should, input.into_tantivy_query(lookup)?));
                }
                for input in must_not.unwrap_or_default() {
                    subqueries.push((Occur::MustNot, input.into_tantivy_query(lookup)?));
                }
                Ok(Box::new(BooleanQuery::new(subqueries)))
            }
            Self::Boost { query, boost } => Ok(Box::new(BoostQuery::new(
                query.into_tantivy_query(lookup)?,
                boost,
            ))),
            Self::ConstScore { query, score } => Ok(Box::new(ConstScoreQuery::new(
                query.into_tantivy_query(lookup)?,
                score,
            ))),
            Self::DisjunctionMax {
                disjuncts,
                tie_breaker,
            } => {
                let disjuncts = disjuncts
                    .into_iter()
                    .map(|query| query.into_tantivy_query(lookup))
                    .collect::<Result<_, _>>()?;
                if let Some(tie_breaker) = tie_breaker {
                    Ok(Box::new(DisjunctionMaxQuery::with_tie_breaker(
                        disjuncts,
                        tie_breaker,
                    )))
                } else {
                    Ok(Box::new(DisjunctionMaxQuery::new(disjuncts)))
                }
            }
            Self::Empty => Ok(Box::new(EmptyQuery)),
            Self::FastFieldRangeWeight {
                field,
                lower_bound,
                upper_bound,
            } => Ok(Box::new(FastFieldRangeWeight::new(
                field,
                lower_bound,
                upper_bound,
            ))),
            _ => unimplemented!(""),
        }
    }
}
