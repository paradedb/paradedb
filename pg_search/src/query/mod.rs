// Copyright (c) 2023-2025 Retake, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod range;

use crate::postgres::utils::convert_pg_date_string;
use crate::query::range::{Comparison, RangeField};
use crate::schema::IndexRecordOption;
use anyhow::Result;
use core::panic;
use pgrx::{pg_sys, PgBuiltInOids, PgOid, PostgresType};
use range::{deserialize_bound, serialize_bound};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, ops::Bound};
use tantivy::query::{NestedQuery, ScoreMode};
use tantivy::DateTime;
use tantivy::{
    collector::DocSetCollector,
    json_utils::split_json_path,
    query::{
        AllQuery, BooleanQuery, BoostQuery, ConstScoreQuery, DisjunctionMaxQuery, EmptyQuery,
        ExistsQuery, FastFieldRangeQuery, FuzzyTermQuery, MoreLikeThisQuery, PhrasePrefixQuery,
        PhraseQuery, Query, QueryParser, RangeQuery, RegexPhraseQuery, RegexQuery, TermQuery,
        TermSetQuery,
    },
    query_grammar::Occur,
    schema::{Field, FieldType, OwnedValue, DATE_TIME_PRECISION_INDEXED},
    Searcher, Term,
};
use thiserror::Error;
use tokenizers::SearchTokenizer;

#[derive(Debug, PostgresType, Deserialize, Serialize, Clone, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SearchQueryInput {
    All,
    Boolean {
        #[serde(default)]
        #[serde(skip_serializing_if = "Vec::is_empty")]
        must: Vec<SearchQueryInput>,

        #[serde(default)]
        #[serde(skip_serializing_if = "Vec::is_empty")]
        should: Vec<SearchQueryInput>,

        #[serde(default)]
        #[serde(skip_serializing_if = "Vec::is_empty")]
        must_not: Vec<SearchQueryInput>,
    },
    Boost {
        query: Box<SearchQueryInput>,
        factor: f32,
    },
    ConstScore {
        query: Box<SearchQueryInput>,
        score: f32,
    },
    DisjunctionMax {
        disjuncts: Vec<SearchQueryInput>,
        tie_breaker: Option<f32>,
    },
    #[default]
    Empty,
    Exists {
        field: String,
    },
    FastFieldRangeWeight {
        field: String,
        #[serde(
            serialize_with = "serialize_bound",
            deserialize_with = "deserialize_bound"
        )]
        lower_bound: std::ops::Bound<u64>,
        #[serde(
            serialize_with = "serialize_bound",
            deserialize_with = "deserialize_bound"
        )]
        upper_bound: std::ops::Bound<u64>,
    },
    FuzzyTerm {
        field: String,
        value: String,
        distance: Option<u8>,
        transposition_cost_one: Option<bool>,
        prefix: Option<bool>,
    },
    Match {
        field: String,
        value: String,
        tokenizer: Option<serde_json::Value>,
        distance: Option<u8>,
        transposition_cost_one: Option<bool>,
        prefix: Option<bool>,
        conjunction_mode: Option<bool>,
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
        document_fields: Option<Vec<(String, tantivy::schema::OwnedValue)>>,
        document_id: Option<tantivy::schema::OwnedValue>,
    },
    Nested {
        /// The dot-notated path that identifies children
        path: Vec<String>,
        /// The query that should match on child docs
        query: Box<SearchQueryInput>,
        /// Aggregate child doc scores into the parent doc
        /// e.g. "avg", "sum", "max", "none" (defaults to "avg")
        score_mode: Option<NestedScoreMode>,
        /// If true, do not error if this path is not mapped
        /// in the schema. Just return no results instead.
        #[serde(default)]
        ignore_unmapped: bool,
    },
    Parse {
        query_string: String,
        lenient: Option<bool>,
        conjunction_mode: Option<bool>,
    },
    ParseWithField {
        field: String,
        query_string: String,
        lenient: Option<bool>,
        conjunction_mode: Option<bool>,
    },
    Phrase {
        field: String,
        phrases: Vec<String>,
        slop: Option<u32>,
    },
    PhrasePrefix {
        field: String,
        phrases: Vec<String>,
        max_expansions: Option<u32>,
    },
    Range {
        field: String,
        #[serde(
            serialize_with = "serialize_bound",
            deserialize_with = "deserialize_bound"
        )]
        lower_bound: std::ops::Bound<tantivy::schema::OwnedValue>,
        #[serde(
            serialize_with = "serialize_bound",
            deserialize_with = "deserialize_bound"
        )]
        upper_bound: std::ops::Bound<tantivy::schema::OwnedValue>,
        #[serde(default)]
        is_datetime: bool,
    },
    RangeContains {
        field: String,
        #[serde(
            serialize_with = "serialize_bound",
            deserialize_with = "deserialize_bound"
        )]
        lower_bound: std::ops::Bound<tantivy::schema::OwnedValue>,
        #[serde(
            serialize_with = "serialize_bound",
            deserialize_with = "deserialize_bound"
        )]
        upper_bound: std::ops::Bound<tantivy::schema::OwnedValue>,
        #[serde(default)]
        is_datetime: bool,
    },
    RangeIntersects {
        field: String,
        #[serde(
            serialize_with = "serialize_bound",
            deserialize_with = "deserialize_bound"
        )]
        lower_bound: std::ops::Bound<tantivy::schema::OwnedValue>,
        #[serde(
            serialize_with = "serialize_bound",
            deserialize_with = "deserialize_bound"
        )]
        upper_bound: std::ops::Bound<tantivy::schema::OwnedValue>,
        #[serde(default)]
        is_datetime: bool,
    },
    RangeTerm {
        field: String,
        value: tantivy::schema::OwnedValue,
        #[serde(default)]
        is_datetime: bool,
    },
    RangeWithin {
        field: String,
        #[serde(
            serialize_with = "serialize_bound",
            deserialize_with = "deserialize_bound"
        )]
        lower_bound: std::ops::Bound<tantivy::schema::OwnedValue>,
        #[serde(
            serialize_with = "serialize_bound",
            deserialize_with = "deserialize_bound"
        )]
        upper_bound: std::ops::Bound<tantivy::schema::OwnedValue>,
        #[serde(default)]
        is_datetime: bool,
    },
    Regex {
        field: String,
        pattern: String,
    },
    RegexPhrase {
        field: String,
        regexes: Vec<String>,
        slop: Option<u32>,
        max_expansions: Option<u32>,
    },
    Term {
        field: Option<String>,
        value: tantivy::schema::OwnedValue,
        #[serde(default)]
        is_datetime: bool,
    },
    TermSet {
        terms: Vec<TermInput>,
    },
    WithIndex {
        oid: pg_sys::Oid,
        query: Box<SearchQueryInput>,
    },
}

impl SearchQueryInput {
    pub fn contains_more_like_this(&self) -> bool {
        match self {
            SearchQueryInput::Boolean {
                must,
                should,
                must_not,
            } => must
                .iter()
                .chain(should.iter())
                .chain(must_not.iter())
                .any(Self::contains_more_like_this),
            SearchQueryInput::Boost { query, .. } => Self::contains_more_like_this(query),
            SearchQueryInput::ConstScore { query, .. } => Self::contains_more_like_this(query),
            SearchQueryInput::DisjunctionMax { disjuncts, .. } => {
                disjuncts.iter().any(Self::contains_more_like_this)
            }
            SearchQueryInput::WithIndex { query, .. } => Self::contains_more_like_this(query),
            SearchQueryInput::MoreLikeThis { .. } => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TermInput {
    pub field: String,
    pub value: tantivy::schema::OwnedValue,
    #[serde(default)]
    pub is_datetime: bool,
}

impl TryFrom<SearchQueryInput> for TermInput {
    type Error = &'static str;

    fn try_from(query: SearchQueryInput) -> Result<Self, Self::Error> {
        match query {
            SearchQueryInput::Term {
                field,
                value,
                is_datetime,
            } => Ok(TermInput {
                field: field.expect("field string must not be empty"),
                value,
                is_datetime,
            }),
            _ => Err("Only Term variants can be converted to TermInput"),
        }
    }
}

#[allow(dead_code)]
pub trait AsFieldType<T> {
    fn fields(&self) -> Vec<(FieldType, PgOid, Field)>;

    fn key_field(&self) -> (FieldType, PgOid, Field);

    fn as_field_type(&self, from: &T) -> Option<(FieldType, PgOid, Field)>;

    fn is_field_type(&self, from: &T, value: &OwnedValue) -> bool {
        matches!(
            (self.as_field_type(from), value),
            (Some((FieldType::Str(_), _, _)), OwnedValue::Str(_))
                | (Some((FieldType::U64(_), _, _)), OwnedValue::U64(_))
                | (Some((FieldType::I64(_), _, _)), OwnedValue::I64(_))
                | (Some((FieldType::F64(_), _, _)), OwnedValue::F64(_))
                | (Some((FieldType::Bool(_), _, _)), OwnedValue::Bool(_))
                | (Some((FieldType::Date(_), _, _)), OwnedValue::Date(_))
                | (Some((FieldType::Facet(_), _, _)), OwnedValue::Facet(_))
                | (Some((FieldType::Bytes(_), _, _)), OwnedValue::Bytes(_))
                | (
                    Some((FieldType::JsonObject(_), _, _)),
                    OwnedValue::Object(_)
                )
                | (Some((FieldType::IpAddr(_), _, _)), OwnedValue::IpAddr(_))
        )
    }

    fn as_str(&self, from: &T) -> Option<Field> {
        self.as_field_type(from)
            .and_then(|(ft, _, field)| match ft {
                FieldType::Str(_) => Some(field),
                _ => None,
            })
    }
    fn as_u64(&self, from: &T) -> Option<Field> {
        self.as_field_type(from)
            .and_then(|(ft, _, field)| match ft {
                FieldType::U64(_) => Some(field),
                _ => None,
            })
    }
    fn as_i64(&self, from: &T) -> Option<Field> {
        self.as_field_type(from)
            .and_then(|(ft, _, field)| match ft {
                FieldType::I64(_) => Some(field),
                _ => None,
            })
    }
    fn as_f64(&self, from: &T) -> Option<Field> {
        self.as_field_type(from)
            .and_then(|(ft, _, field)| match ft {
                FieldType::F64(_) => Some(field),
                _ => None,
            })
    }
    fn as_bool(&self, from: &T) -> Option<Field> {
        self.as_field_type(from)
            .and_then(|(ft, _, field)| match ft {
                FieldType::Bool(_) => Some(field),
                _ => None,
            })
    }
    fn as_date(&self, from: &T) -> Option<Field> {
        self.as_field_type(from)
            .and_then(|(ft, _, field)| match ft {
                FieldType::Date(_) => Some(field),
                _ => None,
            })
    }
    fn as_facet(&self, from: &T) -> Option<Field> {
        self.as_field_type(from)
            .and_then(|(ft, _, field)| match ft {
                FieldType::Facet(_) => Some(field),
                _ => None,
            })
    }
    fn as_bytes(&self, from: &T) -> Option<Field> {
        self.as_field_type(from)
            .and_then(|(ft, _, field)| match ft {
                FieldType::Bytes(_) => Some(field),
                _ => None,
            })
    }
    fn as_json_object(&self, from: &T) -> Option<Field> {
        self.as_field_type(from)
            .and_then(|(ft, _, field)| match ft {
                FieldType::JsonObject(_) => Some(field),
                _ => None,
            })
    }
    fn as_ip_addr(&self, from: &T) -> Option<Field> {
        self.as_field_type(from)
            .and_then(|(ft, _, field)| match ft {
                FieldType::IpAddr(_) => Some(field),
                _ => None,
            })
    }
}

fn is_datetime_typeoid(typeoid: PgOid) -> bool {
    matches!(
        typeoid,
        PgOid::BuiltIn(
            PgBuiltInOids::DATEOID
                | PgBuiltInOids::DATERANGEOID
                | PgBuiltInOids::TIMESTAMPOID
                | PgBuiltInOids::TSRANGEOID
                | PgBuiltInOids::TIMESTAMPTZOID
                | PgBuiltInOids::TSTZRANGEOID
                | PgBuiltInOids::TIMEOID
                | PgBuiltInOids::TIMETZOID
        )
    )
}

fn check_range_bounds(
    typeoid: PgOid,
    lower_bound: Bound<OwnedValue>,
    upper_bound: Bound<OwnedValue>,
) -> Result<(Bound<OwnedValue>, Bound<OwnedValue>)> {
    let one_day_nanos: i64 = 86_400_000_000_000;
    let lower_bound = match (typeoid, lower_bound.clone()) {
        // Excluded U64 needs to be canonicalized
        (_, Bound::Excluded(OwnedValue::U64(n))) => Bound::Included(OwnedValue::U64(n + 1)),
        // Excluded I64 needs to be canonicalized
        (_, Bound::Excluded(OwnedValue::I64(n))) => Bound::Included(OwnedValue::I64(n + 1)),
        // Excluded Date needs to be canonicalized
        (
            PgOid::BuiltIn(PgBuiltInOids::DATEOID | PgBuiltInOids::DATERANGEOID),
            Bound::Excluded(OwnedValue::Str(date_string)),
        ) => {
            let datetime = convert_pg_date_string(typeoid, &date_string);
            let nanos = datetime.into_timestamp_nanos();
            Bound::Included(OwnedValue::Date(DateTime::from_timestamp_nanos(
                nanos + one_day_nanos,
            )))
        }
        (
            PgOid::BuiltIn(
                PgBuiltInOids::TIMESTAMPOID
                | PgBuiltInOids::TSRANGEOID
                | PgBuiltInOids::TIMESTAMPTZOID
                | pg_sys::BuiltinOid::TSTZRANGEOID,
            ),
            Bound::Excluded(OwnedValue::Str(date_string)),
        ) => {
            let datetime = convert_pg_date_string(typeoid, &date_string);
            Bound::Excluded(OwnedValue::Date(datetime))
        }
        (
            PgOid::BuiltIn(
                PgBuiltInOids::DATEOID
                | PgBuiltInOids::DATERANGEOID
                | PgBuiltInOids::TIMESTAMPOID
                | PgBuiltInOids::TSRANGEOID
                | PgBuiltInOids::TIMESTAMPTZOID
                | pg_sys::BuiltinOid::TSTZRANGEOID,
            ),
            Bound::Included(OwnedValue::Str(date_string)),
        ) => {
            let datetime = convert_pg_date_string(typeoid, &date_string);
            Bound::Included(OwnedValue::Date(datetime))
        }
        _ => lower_bound,
    };

    let upper_bound = match (typeoid, upper_bound.clone()) {
        // Included U64 needs to be canonicalized
        (_, Bound::Included(OwnedValue::U64(n))) => Bound::Excluded(OwnedValue::U64(n + 1)),
        // Included I64 needs to be canonicalized
        (_, Bound::Included(OwnedValue::I64(n))) => Bound::Excluded(OwnedValue::I64(n + 1)),
        // Included Date needs to be canonicalized
        (
            PgOid::BuiltIn(PgBuiltInOids::DATEOID | PgBuiltInOids::DATERANGEOID),
            Bound::Included(OwnedValue::Str(date_string)),
        ) => {
            let datetime = convert_pg_date_string(typeoid, &date_string);
            let nanos = datetime.into_timestamp_nanos();
            Bound::Excluded(OwnedValue::Date(DateTime::from_timestamp_nanos(
                nanos + one_day_nanos,
            )))
        }
        (
            PgOid::BuiltIn(
                PgBuiltInOids::TIMESTAMPOID
                | PgBuiltInOids::TSRANGEOID
                | PgBuiltInOids::TIMESTAMPTZOID
                | pg_sys::BuiltinOid::TSTZRANGEOID,
            ),
            Bound::Included(OwnedValue::Str(date_string)),
        ) => {
            let datetime = convert_pg_date_string(typeoid, &date_string);
            Bound::Included(OwnedValue::Date(datetime))
        }
        (
            PgOid::BuiltIn(
                PgBuiltInOids::DATEOID
                | PgBuiltInOids::DATERANGEOID
                | PgBuiltInOids::TIMESTAMPOID
                | PgBuiltInOids::TSRANGEOID
                | PgBuiltInOids::TIMESTAMPTZOID
                | pg_sys::BuiltinOid::TSTZRANGEOID,
            ),
            Bound::Excluded(OwnedValue::Str(date_string)),
        ) => {
            let datetime = convert_pg_date_string(typeoid, &date_string);
            Bound::Excluded(OwnedValue::Date(datetime))
        }
        _ => upper_bound,
    };
    Ok((lower_bound, upper_bound))
}

impl SearchQueryInput {
    pub fn into_tantivy_query(
        self,
        field_lookup: &impl AsFieldType<String>,
        parser: &mut QueryParser,
        searcher: &Searcher,
    ) -> Result<Box<dyn Query>, Box<dyn std::error::Error>> {
        match self {
            Self::All => Ok(Box::new(AllQuery)),
            Self::Boolean {
                must,
                should,
                must_not,
            } => {
                let mut subqueries = vec![];
                for input in must {
                    subqueries.push((
                        Occur::Must,
                        input.into_tantivy_query(field_lookup, parser, searcher)?,
                    ));
                }
                for input in should {
                    subqueries.push((
                        Occur::Should,
                        input.into_tantivy_query(field_lookup, parser, searcher)?,
                    ));
                }
                for input in must_not {
                    subqueries.push((
                        Occur::MustNot,
                        input.into_tantivy_query(field_lookup, parser, searcher)?,
                    ));
                }
                Ok(Box::new(BooleanQuery::new(subqueries)))
            }
            Self::Boost { query, factor } => Ok(Box::new(BoostQuery::new(
                query.into_tantivy_query(field_lookup, parser, searcher)?,
                factor,
            ))),
            Self::ConstScore { query, score } => Ok(Box::new(ConstScoreQuery::new(
                query.into_tantivy_query(field_lookup, parser, searcher)?,
                score,
            ))),
            Self::DisjunctionMax {
                disjuncts,
                tie_breaker,
            } => {
                let disjuncts = disjuncts
                    .into_iter()
                    .map(|query| query.into_tantivy_query(field_lookup, parser, searcher))
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
            Self::Exists { field } => Ok(Box::new(ExistsQuery::new(field, false))),
            Self::FastFieldRangeWeight {
                field,
                lower_bound,
                upper_bound,
            } => {
                let field = field_lookup
                    .as_u64(&field)
                    .or_else(|| field_lookup.as_i64(&field))
                    .ok_or_else(|| QueryError::WrongFieldType(field.clone()))?;

                let new_lower_bound = match lower_bound {
                    Bound::Excluded(v) => Bound::Excluded(Term::from_field_u64(field, v)),
                    Bound::Included(v) => Bound::Included(Term::from_field_u64(field, v)),
                    Bound::Unbounded => Bound::Unbounded,
                };

                let new_upper_bound = match upper_bound {
                    Bound::Excluded(v) => Bound::Excluded(Term::from_field_u64(field, v)),
                    Bound::Included(v) => Bound::Included(Term::from_field_u64(field, v)),
                    Bound::Unbounded => Bound::Unbounded,
                };

                Ok(Box::new(FastFieldRangeQuery::new(
                    new_lower_bound,
                    new_upper_bound,
                )))
            }
            Self::FuzzyTerm {
                field,
                value,
                distance,
                transposition_cost_one,
                prefix,
            } => {
                let (field, path) = split_field_and_path(&field);
                let (field_type, _, field) = field_lookup
                    .as_field_type(&field)
                    .ok_or(QueryError::NonIndexedField(field))?;
                let term = value_to_term(
                    field,
                    &OwnedValue::Str(value),
                    &field_type,
                    path.as_deref(),
                    false,
                )?;
                let distance = distance.unwrap_or(2);
                let transposition_cost_one = transposition_cost_one.unwrap_or(true);
                if prefix.unwrap_or(false) {
                    Ok(Box::new(FuzzyTermQuery::new_prefix(
                        term,
                        distance,
                        transposition_cost_one,
                    )))
                } else {
                    Ok(Box::new(FuzzyTermQuery::new(
                        term,
                        distance,
                        transposition_cost_one,
                    )))
                }
            }
            Self::Match {
                field,
                value,
                tokenizer,
                distance,
                transposition_cost_one,
                prefix,
                conjunction_mode,
            } => {
                let (field, path) = split_field_and_path(&field);
                let distance = distance.unwrap_or(0);
                let transposition_cost_one = transposition_cost_one.unwrap_or(true);
                let conjunction_mode = conjunction_mode.unwrap_or(false);
                let prefix = prefix.unwrap_or(false);

                let (field_type, _, field) = field_lookup
                    .as_field_type(&field)
                    .ok_or(QueryError::NonIndexedField(field))?;

                let mut analyzer = match tokenizer {
                    Some(tokenizer) => {
                        let tokenizer = SearchTokenizer::from_json_value(&tokenizer)
                            .map_err(|_| QueryError::InvalidTokenizer)?;
                        tokenizer
                            .to_tantivy_tokenizer()
                            .ok_or(QueryError::InvalidTokenizer)?
                    }
                    None => searcher.index().tokenizer_for_field(field)?,
                };
                let mut stream = analyzer.token_stream(&value);
                let mut terms = Vec::new();

                while stream.advance() {
                    let token = stream.token().text.clone();
                    let term = value_to_term(
                        field,
                        &OwnedValue::Str(token),
                        &field_type,
                        path.as_deref(),
                        false,
                    )?;
                    let term_query: Box<dyn Query> = match (distance, prefix) {
                        (0, _) => Box::new(TermQuery::new(
                            term,
                            IndexRecordOption::WithFreqsAndPositions.into(),
                        )),
                        (distance, true) => Box::new(FuzzyTermQuery::new_prefix(
                            term,
                            distance,
                            transposition_cost_one,
                        )),
                        (distance, false) => {
                            Box::new(FuzzyTermQuery::new(term, distance, transposition_cost_one))
                        }
                    };

                    let occur = if conjunction_mode {
                        Occur::Must
                    } else {
                        Occur::Should
                    };

                    terms.push((occur, term_query));
                }

                Ok(Box::new(BooleanQuery::new(terms)))
            }
            Self::MoreLikeThis {
                min_doc_frequency,
                max_doc_frequency,
                min_term_frequency,
                max_query_terms,
                min_word_length,
                max_word_length,
                boost_factor,
                stop_words,
                document_fields,
                document_id,
            } => {
                let mut builder = MoreLikeThisQuery::builder();

                if let Some(min_doc_frequency) = min_doc_frequency {
                    builder = builder.with_min_doc_frequency(min_doc_frequency);
                }
                if let Some(max_doc_frequency) = max_doc_frequency {
                    builder = builder.with_max_doc_frequency(max_doc_frequency);
                }
                if let Some(min_term_frequency) = min_term_frequency {
                    builder = builder.with_min_term_frequency(min_term_frequency);
                }
                if let Some(max_query_terms) = max_query_terms {
                    builder = builder.with_max_query_terms(max_query_terms);
                }
                if let Some(min_work_length) = min_word_length {
                    builder = builder.with_min_word_length(min_work_length);
                }
                if let Some(max_work_length) = max_word_length {
                    builder = builder.with_max_word_length(max_work_length);
                }
                if let Some(boost_factor) = boost_factor {
                    builder = builder.with_boost_factor(boost_factor);
                }
                if let Some(stop_words) = stop_words {
                    builder = builder.with_stop_words(stop_words);
                }

                match (document_id, document_fields) {
                    (Some(key_value), None) => {
                        let (field_type, _, field) = field_lookup.key_field();
                        let term = value_to_term(field, &key_value, &field_type, None, false)?;
                        let query: Box<dyn Query> =
                            Box::new(TermQuery::new(term, IndexRecordOption::Basic.into()));
                        let addresses = searcher.search(&query, &DocSetCollector)?;
                        let disjuncts: Vec<Box<dyn Query>> = addresses
                            .into_iter()
                            .map(|address| builder.clone().with_document(address))
                            .map(|query| Box::new(query) as Box<dyn Query>)
                            .collect();
                        Ok(Box::new(DisjunctionMaxQuery::new(disjuncts)))
                    }
                    (None, Some(doc_fields)) => {
                        let mut fields_map = HashMap::new();
                        for (field_name, value) in doc_fields {
                            if !field_lookup.is_field_type(&field_name, &value) {
                                return Err(Box::new(QueryError::WrongFieldType(field_name)));
                            }

                            let (_, _, field) = field_lookup
                                .as_field_type(&field_name)
                                .ok_or_else(|| QueryError::WrongFieldType(field_name.clone()))?;

                            fields_map.entry(field).or_insert_with(std::vec::Vec::new);

                            if let Some(vec) = fields_map.get_mut(&field) {
                                vec.push(value)
                            }
                        }
                        Ok(Box::new(
                            builder.with_document_fields(fields_map.into_iter().collect()),
                        ))
                    }
                    (Some(_), Some(_)) => {
                        panic!("more_like_this must be called with only one of document_id or document_fields")
                    }
                    (None, None) => {
                        panic!("more_like_this must be called with either document_id or document_fields");
                    }
                }
            }
            Self::PhrasePrefix {
                field,
                phrases,
                max_expansions,
            } => {
                let (field, path) = split_field_and_path(&field);
                let (field_type, _, field) = field_lookup
                    .as_field_type(&field)
                    .ok_or(QueryError::NonIndexedField(field))?;
                let terms = phrases.clone().into_iter().map(|phrase| {
                    value_to_term(
                        field,
                        &OwnedValue::Str(phrase),
                        &field_type,
                        path.as_deref(),
                        false,
                    )
                    .unwrap()
                });
                let mut query = PhrasePrefixQuery::new(terms.collect());
                if let Some(max_expansions) = max_expansions {
                    query.set_max_expansions(max_expansions)
                }
                Ok(Box::new(query))
            }
            Self::Nested {
                query,
                score_mode,
                path,
                ignore_unmapped,
            } => {
                let child_query = query.into_tantivy_query(field_lookup, parser, searcher)?;
                let actual_score_mode = score_mode.unwrap_or_default().into();
                let nested_query =
                    NestedQuery::new(path, child_query, actual_score_mode, ignore_unmapped);

                Ok(Box::new(nested_query))
            }
            Self::Parse {
                query_string,
                lenient,
                conjunction_mode,
            } => {
                if let Some(true) = conjunction_mode {
                    parser.set_conjunction_by_default();
                }

                match lenient {
                    Some(true) => {
                        let (parsed_query, _) = parser.parse_query_lenient(&query_string);
                        Ok(Box::new(parsed_query))
                    }
                    _ => {
                        Ok(Box::new(parser.parse_query(&query_string).map_err(
                            |err| QueryError::ParseError(err, query_string),
                        )?))
                    }
                }
            }
            Self::ParseWithField {
                field,
                query_string,
                lenient,
                conjunction_mode,
            } => {
                let query_string = format!("{field}:({query_string})");
                Self::Parse {
                    query_string,
                    lenient,
                    conjunction_mode,
                }
                .into_tantivy_query(field_lookup, parser, searcher)
            }
            Self::Phrase {
                field,
                phrases,
                slop,
            } => {
                let (field, path) = split_field_and_path(&field);
                let (field_type, _, field) = field_lookup
                    .as_field_type(&field)
                    .ok_or(QueryError::NonIndexedField(field))?;
                let terms = phrases.clone().into_iter().map(|phrase| {
                    value_to_term(
                        field,
                        &OwnedValue::Str(phrase),
                        &field_type,
                        path.as_deref(),
                        false,
                    )
                    .unwrap()
                });
                let mut query = PhraseQuery::new(terms.collect());
                if let Some(slop) = slop {
                    query.set_slop(slop)
                }
                Ok(Box::new(query))
            }
            Self::Range {
                field,
                lower_bound,
                upper_bound,
                is_datetime,
            } => {
                let (field, path) = split_field_and_path(&field);
                let field_name = field;
                let (field_type, typeoid, field) = field_lookup
                    .as_field_type(&field_name)
                    .ok_or_else(|| QueryError::WrongFieldType(field_name.clone()))?;

                let is_datetime = is_datetime_typeoid(typeoid) || is_datetime;
                let (lower_bound, upper_bound) =
                    check_range_bounds(typeoid, lower_bound, upper_bound)?;

                let lower_bound = match lower_bound {
                    Bound::Included(value) => Bound::Included(value_to_term(
                        field,
                        &value,
                        &field_type,
                        path.as_deref(),
                        is_datetime,
                    )?),
                    Bound::Excluded(value) => Bound::Excluded(value_to_term(
                        field,
                        &value,
                        &field_type,
                        path.as_deref(),
                        is_datetime,
                    )?),
                    Bound::Unbounded => Bound::Unbounded,
                };

                let upper_bound = match upper_bound {
                    Bound::Included(value) => Bound::Included(value_to_term(
                        field,
                        &value,
                        &field_type,
                        path.as_deref(),
                        is_datetime,
                    )?),
                    Bound::Excluded(value) => Bound::Excluded(value_to_term(
                        field,
                        &value,
                        &field_type,
                        path.as_deref(),
                        is_datetime,
                    )?),
                    Bound::Unbounded => Bound::Unbounded,
                };

                Ok(Box::new(RangeQuery::new(lower_bound, upper_bound)))
            }
            Self::RangeContains {
                field,
                lower_bound,
                upper_bound,
                is_datetime,
            } => {
                let (_, typeoid, _) = field_lookup
                    .as_field_type(&field)
                    .ok_or_else(|| QueryError::NonIndexedField(field.clone()))?;

                let is_datetime = is_datetime_typeoid(typeoid) || is_datetime;
                let (lower_bound, upper_bound) =
                    check_range_bounds(typeoid, lower_bound, upper_bound)?;

                let range_field = RangeField::new(
                    field_lookup
                        .as_json_object(&field)
                        .ok_or_else(|| QueryError::WrongFieldType(field.clone()))?,
                    is_datetime,
                );

                let mut satisfies_lower_bound: Vec<(Occur, Box<dyn Query>)> = vec![];
                let mut satisfies_upper_bound: Vec<(Occur, Box<dyn Query>)> = vec![];

                match lower_bound {
                    Bound::Included(lower) => {
                        satisfies_lower_bound.push((
                            Occur::Must,
                            Box::new(BooleanQuery::new(vec![(
                                Occur::Must,
                                Box::new(
                                    range_field
                                        .compare_lower_bound(&lower, Comparison::LessThanOrEqual)?,
                                ),
                            )])),
                        ));
                    }
                    Bound::Excluded(lower) => {
                        satisfies_lower_bound.push((
                            Occur::Must,
                            (Box::new(BooleanQuery::new(vec![
                                (
                                    Occur::Should,
                                    Box::new(BooleanQuery::new(vec![
                                        (
                                            Occur::Must,
                                            Box::new(range_field.compare_lower_bound(
                                                &lower,
                                                Comparison::LessThan,
                                            )?),
                                        ),
                                        (
                                            Occur::Must,
                                            Box::new(range_field.lower_bound_inclusive(true)?),
                                        ),
                                    ])),
                                ),
                                (
                                    Occur::Should,
                                    Box::new(BooleanQuery::new(vec![
                                        (
                                            Occur::Must,
                                            Box::new(range_field.compare_lower_bound(
                                                &lower,
                                                Comparison::LessThanOrEqual,
                                            )?),
                                        ),
                                        (
                                            Occur::Must,
                                            Box::new(range_field.lower_bound_inclusive(false)?),
                                        ),
                                    ])),
                                ),
                            ]))),
                        ))
                    }
                    _ => {
                        satisfies_lower_bound.push((Occur::Should, Box::new(range_field.exists()?)))
                    }
                }

                match upper_bound {
                    Bound::Included(upper) => {
                        satisfies_upper_bound.push((
                            Occur::Must,
                            Box::new(BooleanQuery::new(vec![(
                                Occur::Must,
                                Box::new(
                                    range_field.compare_upper_bound(
                                        &upper,
                                        Comparison::GreaterThanOrEqual,
                                    )?,
                                ),
                            )])),
                        ));
                    }
                    Bound::Excluded(upper) => satisfies_upper_bound.push((
                        Occur::Must,
                        (Box::new(BooleanQuery::new(vec![
                            (
                                Occur::Should,
                                Box::new(BooleanQuery::new(vec![
                                    (
                                        Occur::Must,
                                        Box::new(range_field.compare_upper_bound(
                                            &upper,
                                            Comparison::GreaterThan,
                                        )?),
                                    ),
                                    (
                                        Occur::Must,
                                        Box::new(range_field.upper_bound_inclusive(true)?),
                                    ),
                                ])),
                            ),
                            (
                                Occur::Should,
                                Box::new(BooleanQuery::new(vec![
                                    (
                                        Occur::Must,
                                        Box::new(range_field.compare_upper_bound(
                                            &upper,
                                            Comparison::GreaterThanOrEqual,
                                        )?),
                                    ),
                                    (
                                        Occur::Must,
                                        Box::new(range_field.upper_bound_inclusive(false)?),
                                    ),
                                ])),
                            ),
                        ]))),
                    )),
                    _ => {
                        satisfies_upper_bound.push((Occur::Should, Box::new(range_field.exists()?)))
                    }
                }

                let satisfies_lower_bound = BooleanQuery::new(vec![
                    (Occur::Should, Box::new(range_field.empty(true)?)),
                    (
                        Occur::Should,
                        Box::new(BooleanQuery::new(satisfies_lower_bound)),
                    ),
                ]);

                let satisfies_upper_bound = BooleanQuery::new(vec![
                    (Occur::Should, Box::new(range_field.empty(true)?)),
                    (
                        Occur::Should,
                        Box::new(BooleanQuery::new(satisfies_upper_bound)),
                    ),
                ]);

                Ok(Box::new(BooleanQuery::new(vec![
                    (Occur::Must, Box::new(satisfies_lower_bound)),
                    (Occur::Must, Box::new(satisfies_upper_bound)),
                ])))
            }
            Self::RangeIntersects {
                field,
                lower_bound,
                upper_bound,
                is_datetime,
                ..
            } => {
                let (_, typeoid, _) = field_lookup
                    .as_field_type(&field)
                    .ok_or_else(|| QueryError::NonIndexedField(field.clone()))?;

                let is_datetime = is_datetime_typeoid(typeoid) || is_datetime;
                let (lower_bound, upper_bound) =
                    check_range_bounds(typeoid, lower_bound, upper_bound)?;

                let range_field = RangeField::new(
                    field_lookup
                        .as_json_object(&field)
                        .ok_or_else(|| QueryError::WrongFieldType(field.clone()))?,
                    is_datetime,
                );

                let mut satisfies_lower_bound: Vec<(Occur, Box<dyn Query>)> = vec![];
                let mut satisfies_upper_bound: Vec<(Occur, Box<dyn Query>)> = vec![];

                match lower_bound {
                    Bound::Excluded(ref lower) => {
                        satisfies_lower_bound.push((
                            Occur::Must,
                            Box::new(BooleanQuery::new(vec![(
                                Occur::Must,
                                Box::new(
                                    range_field.compare_upper_bound(lower, Comparison::LessThan)?,
                                ),
                            )])),
                        ));
                    }
                    Bound::Included(ref lower) => satisfies_lower_bound.push((
                        Occur::Must,
                        (Box::new(BooleanQuery::new(vec![
                            (
                                Occur::Should,
                                Box::new(BooleanQuery::new(vec![
                                    (
                                        Occur::Must,
                                        Box::new(range_field.compare_upper_bound(
                                            lower,
                                            Comparison::LessThanOrEqual,
                                        )?),
                                    ),
                                    (
                                        Occur::Must,
                                        Box::new(range_field.upper_bound_inclusive(true)?),
                                    ),
                                ])),
                            ),
                            (
                                Occur::Should,
                                Box::new(BooleanQuery::new(vec![
                                    (
                                        Occur::Must,
                                        Box::new(
                                            range_field
                                                .compare_upper_bound(lower, Comparison::LessThan)?,
                                        ),
                                    ),
                                    (
                                        Occur::Must,
                                        Box::new(range_field.upper_bound_inclusive(false)?),
                                    ),
                                ])),
                            ),
                        ]))),
                    )),
                    Bound::Unbounded => {
                        satisfies_lower_bound.push((Occur::Should, Box::new(range_field.exists()?)))
                    }
                }

                match upper_bound {
                    Bound::Excluded(ref upper) => {
                        satisfies_upper_bound.push((
                            Occur::Must,
                            Box::new(BooleanQuery::new(vec![(
                                Occur::Must,
                                Box::new(
                                    range_field
                                        .compare_lower_bound(upper, Comparison::GreaterThan)?,
                                ),
                            )])),
                        ));
                    }
                    Bound::Included(ref upper) => {
                        satisfies_upper_bound.push((
                            Occur::Must,
                            (Box::new(BooleanQuery::new(vec![
                                (
                                    Occur::Should,
                                    Box::new(BooleanQuery::new(vec![
                                        (
                                            Occur::Must,
                                            Box::new(range_field.compare_lower_bound(
                                                upper,
                                                Comparison::GreaterThanOrEqual,
                                            )?),
                                        ),
                                        (
                                            Occur::Must,
                                            Box::new(range_field.lower_bound_inclusive(true)?),
                                        ),
                                    ])),
                                ),
                                (
                                    Occur::Should,
                                    Box::new(BooleanQuery::new(vec![
                                        (
                                            Occur::Must,
                                            Box::new(range_field.compare_lower_bound(
                                                upper,
                                                Comparison::GreaterThan,
                                            )?),
                                        ),
                                        (
                                            Occur::Must,
                                            Box::new(range_field.lower_bound_inclusive(false)?),
                                        ),
                                    ])),
                                ),
                            ]))),
                        ))
                    }
                    Bound::Unbounded => {
                        satisfies_upper_bound.push((Occur::Should, Box::new(range_field.exists()?)))
                    }
                }

                let satisfies_lower_bound = BooleanQuery::new(vec![
                    (
                        Occur::Should,
                        Box::new(range_field.upper_bound_unbounded(true)?),
                    ),
                    (
                        Occur::Should,
                        Box::new(BooleanQuery::new(satisfies_lower_bound)),
                    ),
                ]);

                let satisfies_upper_bound = BooleanQuery::new(vec![
                    (
                        Occur::Should,
                        Box::new(range_field.lower_bound_unbounded(true)?),
                    ),
                    (
                        Occur::Should,
                        Box::new(BooleanQuery::new(satisfies_upper_bound)),
                    ),
                ]);

                let is_empty = match (lower_bound, upper_bound) {
                    (Bound::Included(lower), Bound::Excluded(upper)) => lower == upper,
                    _ => false,
                };

                if is_empty {
                    Ok(Box::new(EmptyQuery))
                } else {
                    Ok(Box::new(BooleanQuery::new(vec![
                        (Occur::Must, Box::new(satisfies_lower_bound)),
                        (Occur::Must, Box::new(satisfies_upper_bound)),
                        (Occur::Must, Box::new(range_field.empty(false)?)),
                    ])))
                }
            }
            Self::RangeTerm {
                field,
                value,
                is_datetime,
            } => {
                let range_field = RangeField::new(
                    field_lookup
                        .as_json_object(&field)
                        .ok_or_else(|| QueryError::WrongFieldType(field.clone()))?,
                    is_datetime,
                );

                let satisfies_lower_bound = BooleanQuery::new(vec![
                    (
                        Occur::Should,
                        Box::new(range_field.lower_bound_unbounded(true)?),
                    ),
                    (
                        Occur::Should,
                        Box::new(BooleanQuery::new(vec![
                            (
                                Occur::Should,
                                Box::new(BooleanQuery::new(vec![
                                    (
                                        Occur::Must,
                                        Box::new(range_field.lower_bound_inclusive(true)?),
                                    ),
                                    (
                                        Occur::Must,
                                        Box::new(range_field.compare_lower_bound(
                                            &value,
                                            Comparison::GreaterThanOrEqual,
                                        )?),
                                    ),
                                ])),
                            ),
                            (
                                Occur::Should,
                                Box::new(BooleanQuery::new(vec![
                                    (
                                        Occur::Must,
                                        Box::new(range_field.lower_bound_inclusive(false)?),
                                    ),
                                    (
                                        Occur::Must,
                                        Box::new(range_field.compare_lower_bound(
                                            &value,
                                            Comparison::GreaterThan,
                                        )?),
                                    ),
                                ])),
                            ),
                        ])),
                    ),
                ]);

                let satisfies_upper_bound =
                    BooleanQuery::new(vec![
                        (
                            Occur::Should,
                            Box::new(range_field.upper_bound_unbounded(true)?),
                        ),
                        (
                            Occur::Should,
                            Box::new(BooleanQuery::new(vec![
                                (
                                    Occur::Should,
                                    Box::new(BooleanQuery::new(vec![
                                        (
                                            Occur::Must,
                                            Box::new(range_field.upper_bound_inclusive(true)?),
                                        ),
                                        (
                                            Occur::Must,
                                            Box::new(range_field.compare_upper_bound(
                                                &value,
                                                Comparison::LessThanOrEqual,
                                            )?),
                                        ),
                                    ])),
                                ),
                                (
                                    Occur::Should,
                                    Box::new(BooleanQuery::new(vec![
                                        (
                                            Occur::Must,
                                            Box::new(range_field.upper_bound_inclusive(false)?),
                                        ),
                                        (
                                            Occur::Must,
                                            Box::new(range_field.compare_upper_bound(
                                                &value,
                                                Comparison::LessThan,
                                            )?),
                                        ),
                                    ])),
                                ),
                            ])),
                        ),
                    ]);

                Ok(Box::new(BooleanQuery::new(vec![
                    (Occur::Must, Box::new(satisfies_lower_bound)),
                    (Occur::Must, Box::new(satisfies_upper_bound)),
                ])))
            }
            Self::RangeWithin {
                field,
                lower_bound,
                upper_bound,
                is_datetime,
            } => {
                let (_, typeoid, _) = field_lookup
                    .as_field_type(&field)
                    .ok_or_else(|| QueryError::NonIndexedField(field.clone()))?;

                let is_datetime = is_datetime_typeoid(typeoid) || is_datetime;
                let (lower_bound, upper_bound) =
                    check_range_bounds(typeoid, lower_bound, upper_bound)?;

                let range_field = RangeField::new(
                    field_lookup
                        .as_json_object(&field)
                        .ok_or_else(|| QueryError::WrongFieldType(field.clone()))?,
                    is_datetime,
                );

                let mut satisfies_lower_bound: Vec<(Occur, Box<dyn Query>)> = vec![];
                let mut satisfies_upper_bound: Vec<(Occur, Box<dyn Query>)> = vec![];

                match lower_bound {
                    Bound::Excluded(ref lower) => {
                        satisfies_lower_bound.push((
                            Occur::Must,
                            Box::new(BooleanQuery::new(vec![(
                                Occur::Must,
                                Box::new(
                                    range_field.compare_lower_bound(
                                        lower,
                                        Comparison::GreaterThanOrEqual,
                                    )?,
                                ),
                            )])),
                        ));
                    }
                    Bound::Included(ref lower) => {
                        satisfies_lower_bound.push((
                            Occur::Must,
                            (Box::new(BooleanQuery::new(vec![
                                (
                                    Occur::Should,
                                    Box::new(BooleanQuery::new(vec![
                                        (
                                            Occur::Must,
                                            Box::new(range_field.compare_lower_bound(
                                                lower,
                                                Comparison::GreaterThan,
                                            )?),
                                        ),
                                        (
                                            Occur::Must,
                                            Box::new(range_field.lower_bound_inclusive(false)?),
                                        ),
                                    ])),
                                ),
                                (
                                    Occur::Should,
                                    Box::new(BooleanQuery::new(vec![
                                        (
                                            Occur::Must,
                                            Box::new(range_field.compare_lower_bound(
                                                lower,
                                                Comparison::GreaterThanOrEqual,
                                            )?),
                                        ),
                                        (
                                            Occur::Must,
                                            Box::new(range_field.lower_bound_inclusive(true)?),
                                        ),
                                    ])),
                                ),
                            ]))),
                        ))
                    }
                    _ => {}
                }

                match upper_bound {
                    Bound::Excluded(ref upper) => {
                        satisfies_upper_bound.push((
                            Occur::Must,
                            Box::new(BooleanQuery::new(vec![(
                                Occur::Must,
                                Box::new(
                                    range_field
                                        .compare_upper_bound(upper, Comparison::LessThanOrEqual)?,
                                ),
                            )])),
                        ));
                    }
                    Bound::Included(ref upper) => satisfies_upper_bound.push((
                        Occur::Must,
                        (Box::new(BooleanQuery::new(vec![
                            (
                                Occur::Should,
                                Box::new(BooleanQuery::new(vec![
                                    (
                                        Occur::Must,
                                        Box::new(
                                            range_field
                                                .compare_upper_bound(upper, Comparison::LessThan)?,
                                        ),
                                    ),
                                    (
                                        Occur::Must,
                                        Box::new(range_field.upper_bound_inclusive(false)?),
                                    ),
                                ])),
                            ),
                            (
                                Occur::Should,
                                Box::new(BooleanQuery::new(vec![
                                    (
                                        Occur::Must,
                                        Box::new(range_field.compare_upper_bound(
                                            upper,
                                            Comparison::LessThanOrEqual,
                                        )?),
                                    ),
                                    (
                                        Occur::Must,
                                        Box::new(range_field.upper_bound_inclusive(true)?),
                                    ),
                                ])),
                            ),
                        ]))),
                    )),
                    _ => {}
                }

                let satisfies_lower_bound = BooleanQuery::new(vec![
                    (
                        Occur::Should,
                        Box::new(range_field.lower_bound_unbounded(true)?),
                    ),
                    (
                        Occur::Should,
                        Box::new(BooleanQuery::new(satisfies_lower_bound)),
                    ),
                ]);

                let satisfies_upper_bound = BooleanQuery::new(vec![
                    (
                        Occur::Should,
                        Box::new(range_field.upper_bound_unbounded(true)?),
                    ),
                    (
                        Occur::Should,
                        Box::new(BooleanQuery::new(satisfies_upper_bound)),
                    ),
                ]);

                let is_empty = match (lower_bound, upper_bound) {
                    (Bound::Included(lower), Bound::Excluded(upper)) => lower == upper,
                    _ => false,
                };

                if is_empty {
                    Ok(Box::new(range_field.exists()?))
                } else {
                    Ok(Box::new(BooleanQuery::new(vec![
                        (Occur::Must, Box::new(satisfies_lower_bound)),
                        (Occur::Must, Box::new(satisfies_upper_bound)),
                    ])))
                }
            }
            Self::Regex { field, pattern } => Ok(Box::new(
                RegexQuery::from_pattern(
                    &pattern,
                    field_lookup
                        .as_str(&field)
                        .ok_or_else(|| QueryError::WrongFieldType(field.clone()))?,
                )
                .map_err(|err| QueryError::RegexError(err, pattern.clone()))?,
            )),
            Self::RegexPhrase {
                field,
                regexes,
                slop,
                max_expansions,
            } => {
                let (field, _) = split_field_and_path(&field);
                let (_, _, field) = field_lookup
                    .as_field_type(&field)
                    .ok_or(QueryError::NonIndexedField(field))?;

                let mut query = RegexPhraseQuery::new(field, regexes);

                if let Some(slop) = slop {
                    query.set_slop(slop)
                }
                if let Some(max_expansions) = max_expansions {
                    query.set_max_expansions(max_expansions)
                }
                Ok(Box::new(query))
            }

            Self::Term {
                field,
                value,
                is_datetime,
            } => {
                let record_option = IndexRecordOption::WithFreqsAndPositions;
                if let Some(field) = field {
                    let (field, path) = split_field_and_path(&field);
                    let (field_type, typeoid, field) = field_lookup
                        .as_field_type(&field)
                        .ok_or(QueryError::NonIndexedField(field))?;

                    let is_datetime = is_datetime_typeoid(typeoid) || is_datetime;
                    let term =
                        value_to_term(field, &value, &field_type, path.as_deref(), is_datetime)?;

                    Ok(Box::new(TermQuery::new(term, record_option.into())))
                } else {
                    // If no field is passed, then search all fields.
                    let all_fields = field_lookup.fields();
                    let mut terms = vec![];
                    for (field_type, _, field) in all_fields {
                        if let Ok(term) =
                            value_to_term(field, &value, &field_type, None, is_datetime)
                        {
                            terms.push(term);
                        }
                    }

                    Ok(Box::new(TermSetQuery::new(terms)))
                }
            }
            Self::TermSet { terms: fields } => {
                let mut terms = vec![];
                for TermInput {
                    field,
                    value,
                    is_datetime,
                } in fields
                {
                    let (field, path) = split_field_and_path(&field);
                    let (field_type, typeoid, field) = field_lookup
                        .as_field_type(&field)
                        .ok_or(QueryError::NonIndexedField(field))?;

                    let is_datetime = is_datetime_typeoid(typeoid) || is_datetime;
                    terms.push(value_to_term(
                        field,
                        &value,
                        &field_type,
                        path.as_deref(),
                        is_datetime,
                    )?);
                }

                Ok(Box::new(TermSetQuery::new(terms)))
            }
            Self::WithIndex { query, .. } => {
                query.into_tantivy_query(field_lookup, parser, searcher)
            }
        }
    }
}

fn value_to_json_term(
    field: Field,
    value: &OwnedValue,
    path: Option<&str>,
    expand_dots: bool,
    is_datetime: bool,
) -> Result<Term, Box<dyn std::error::Error>> {
    let mut term = Term::from_field_json_path(field, path.unwrap_or_default(), expand_dots);
    match value {
        OwnedValue::Str(text) => {
            if is_datetime {
                let TantivyDateTime(date) = TantivyDateTime::try_from(text.as_str())?;
                // https://github.com/quickwit-oss/tantivy/pull/2456
                // It's a footgun that date needs to truncated when creating the Term
                term.append_type_and_fast_value(date.truncate(DATE_TIME_PRECISION_INDEXED));
            } else {
                term.append_type_and_str(text);
            }
        }
        OwnedValue::U64(value) => {
            if let Ok(i64_val) = (*value).try_into() {
                term.append_type_and_fast_value::<i64>(i64_val);
            } else {
                term.append_type_and_fast_value(*value);
            }
        }
        OwnedValue::I64(value) => {
            term.append_type_and_fast_value(*value);
        }
        OwnedValue::F64(value) => {
            term.append_type_and_fast_value(*value);
        }
        OwnedValue::Bool(value) => {
            term.append_type_and_fast_value(*value);
        }
        OwnedValue::Date(value) => {
            term.append_type_and_fast_value(*value);
        }
        unsupported => panic!(
            "Tantivy OwnedValue type {:?} not supported for JSON term",
            unsupported
        ),
    };

    Ok(term)
}

pub fn value_to_term(
    field: Field,
    value: &OwnedValue,
    field_type: &FieldType,
    path: Option<&str>,
    is_datetime: bool,
) -> Result<Term, Box<dyn std::error::Error>> {
    let json_options = match field_type {
        FieldType::JsonObject(ref options) => Some(options),
        _ => None,
    };

    if let Some(json_options) = json_options {
        return value_to_json_term(
            field,
            value,
            path,
            json_options.is_expand_dots_enabled(),
            is_datetime,
        );
    }

    if is_datetime {
        if let OwnedValue::Str(text) = value {
            let TantivyDateTime(date) = TantivyDateTime::try_from(text.as_str())?;
            // https://github.com/quickwit-oss/tantivy/pull/2456
            // It's a footgun that date needs to truncated when creating the Term
            return Ok(Term::from_field_date(
                field,
                date.truncate(DATE_TIME_PRECISION_INDEXED),
            ));
        }
    }

    Ok(match value {
        OwnedValue::Str(text) => Term::from_field_text(field, text),
        OwnedValue::PreTokStr(_) => panic!("pre-tokenized text cannot be converted to term"),
        OwnedValue::U64(u64) => {
            // Positive numbers seem to be automatically turned into u64s even if they are i64s,
            // so we should use the field type to assign the term type
            match field_type {
                FieldType::I64(_) => Term::from_field_i64(field, *u64 as i64),
                FieldType::U64(_) => Term::from_field_u64(field, *u64),
                _ => panic!("invalid field type for u64 value"),
            }
        }
        OwnedValue::I64(i64) => Term::from_field_i64(field, *i64),
        OwnedValue::F64(f64) => Term::from_field_f64(field, *f64),
        OwnedValue::Bool(bool) => Term::from_field_bool(field, *bool),
        OwnedValue::Date(date) => {
            Term::from_field_date(field, date.truncate(DATE_TIME_PRECISION_INDEXED))
        }
        OwnedValue::Facet(facet) => Term::from_facet(field, facet),
        OwnedValue::Bytes(bytes) => Term::from_field_bytes(field, bytes),
        OwnedValue::Object(_) => panic!("json cannot be converted to term"),
        OwnedValue::IpAddr(ip) => Term::from_field_ip_addr(field, *ip),
        _ => panic!("Tantivy OwnedValue type not supported"),
    })
}

struct TantivyDateTime(pub tantivy::DateTime);
impl TryFrom<&str> for TantivyDateTime {
    type Error = QueryError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        let datetime = match chrono::NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%SZ") {
            Ok(dt) => dt,
            Err(_) => chrono::NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%S%.fZ")
                .map_err(|_| QueryError::FieldTypeMismatch)?,
        };
        Ok(TantivyDateTime(tantivy::DateTime::from_timestamp_micros(
            datetime.and_utc().timestamp_micros(),
        )))
    }
}

pub fn split_field_and_path(field: &str) -> (String, Option<String>) {
    let json_path = split_json_path(field);
    if json_path.len() == 1 {
        (field.to_string(), None)
    } else {
        (json_path[0].clone(), Some(json_path[1..].join(".")))
    }
}

#[derive(Debug, PostgresType, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NestedScoreMode {
    Avg,
    Max,
    Total,
    None,
}

impl Default for NestedScoreMode {
    fn default() -> Self {
        NestedScoreMode::Avg
    }
}

impl From<NestedScoreMode> for ScoreMode {
    fn from(mode: NestedScoreMode) -> Self {
        match mode {
            NestedScoreMode::Avg => Self::Avg,
            NestedScoreMode::Max => Self::Max,
            NestedScoreMode::Total => Self::Total,
            NestedScoreMode::None => Self::None,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Error)]
enum QueryError {
    #[error("wrong field type for field: {0}")]
    WrongFieldType(String),
    #[error("invalid field map json: {0}")]
    FieldMapJsonValue(#[source] serde_json::Error),
    #[error("field map json must be an object")]
    FieldMapJsonObject,
    #[error("invalid tokenizer setting, expected paradedb.tokenizer()")]
    InvalidTokenizer,
    #[error("field '{0}' is not part of the pg_search index")]
    NonIndexedField(String),
    #[error("wrong type given for field")]
    FieldTypeMismatch,
    #[error("could not build regex with pattern '{1}': {0}")]
    RegexError(#[source] tantivy::TantivyError, String),
    #[error(
        r#"could not parse query string '{1}'.
           make sure to use column:term pairs, and to capitalize AND/OR."#
    )]
    ParseError(#[source] tantivy::query::QueryParserError, String),
}
