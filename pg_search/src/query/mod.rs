// Copyright (c) 2023-2025 ParadeDB, Inc.
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

pub mod heap_field_filter;
pub mod iter_mut;
mod more_like_this;
pub mod pdb_query;
mod range;
mod score;

use heap_field_filter::HeapFieldFilter;

use crate::api::operator::searchqueryinput_typoid;
use crate::api::FieldName;
use crate::api::HashMap;
use crate::postgres::utils::convert_pg_date_string;
use crate::query::more_like_this::MoreLikeThisQuery;
pub use crate::query::pdb_query::PdbQuery;
use crate::query::score::ScoreFilter;
use crate::schema::SearchIndexSchema;
use anyhow::Result;
use core::panic;
use pgrx::{pg_sys, IntoDatum, PgBuiltInOids, PgOid, PostgresType};
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Debug, Formatter};
use std::ops::Bound;
use tantivy::query::{
    AllQuery, BooleanQuery, BoostQuery, ConstScoreQuery, DisjunctionMaxQuery, EmptyQuery, Query,
    QueryParser, TermSetQuery,
};
use tantivy::DateTime;
use tantivy::{
    query_grammar::Occur,
    schema::{Field, FieldType, OwnedValue, DATE_TIME_PRECISION_INDEXED},
    Searcher, Term,
};
use thiserror::Error;

#[derive(Debug, PostgresType, Deserialize, Serialize, Clone, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SearchQueryInput {
    #[default]
    Uninitialized,
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
    ScoreFilter {
        bounds: Vec<(Bound<f32>, Bound<f32>)>,
        query: Option<Box<SearchQueryInput>>,
    },
    DisjunctionMax {
        disjuncts: Vec<SearchQueryInput>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tie_breaker: Option<f32>,
    },
    Empty,
    MoreLikeThis {
        #[serde(skip_serializing_if = "Option::is_none")]
        min_doc_frequency: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_doc_frequency: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        min_term_frequency: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_query_terms: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        min_word_length: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_word_length: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        boost_factor: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        stop_words: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        document_fields: Option<Vec<(String, OwnedValue)>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        document_id: Option<OwnedValue>,
    },
    Parse {
        query_string: String,
        lenient: Option<bool>,
        conjunction_mode: Option<bool>,
    },
    TermSet {
        terms: Vec<TermInput>,
    },
    WithIndex {
        oid: pg_sys::Oid,
        query: Box<SearchQueryInput>,
    },
    PostgresExpression {
        expr: PostgresExpression,
    },
    /// Mixed query with indexed search and heap field filters
    HeapFilter {
        indexed_query: Box<SearchQueryInput>,
        field_filters: Vec<HeapFieldFilter>,
    },

    #[serde(serialize_with = "serialize_fielded_query")]
    #[serde(deserialize_with = "deserialize_fielded_query")]
    #[serde(untagged)]
    FieldedQuery {
        field: FieldName,
        query: PdbQuery,
    },
}

fn serialize_fielded_query<S>(
    field: &FieldName,
    query: &PdbQuery,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut query_json = serde_json::to_value(query).unwrap();

    if let Some(map) = query_json.as_object_mut() {
        let fielded_query_input_entry = map.values_mut().next().unwrap();
        fielded_query_input_entry
            .as_object_mut()
            .unwrap()
            .shift_insert(0, "field".into(), serde_json::to_value(field).unwrap());

        query_json.serialize(serializer)
    } else if let Some(variant_name) = query_json.as_str() {
        let mut map = serde_json::Map::new();
        map.insert("field".into(), serde_json::to_value(field).unwrap());

        let mut object = serde_json::Map::new();
        object.insert(variant_name.to_string(), serde_json::Value::Object(map));
        object.serialize(serializer)
    } else {
        Err(<S::Error as serde::ser::Error>::custom(
            "this does not appear to be a FieldedQueryInput",
        ))
    }
}

fn deserialize_fielded_query<'de, D>(deserializer: D) -> Result<(FieldName, PdbQuery), D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = (FieldName, PdbQuery);

        fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
            formatter.write_str("a map")
        }

        fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let Some((key, mut value)) = map.next_entry::<String, serde_json::Value>()? else {
                return Err(<A::Error as serde::de::Error>::custom(
                    "this does not appear to be a FieldedQueryInput",
                ));
            };

            if let Some(field_entry) = value.as_object_mut().unwrap().remove_entry("field") {
                // pull the field out of the object that also contains the FieldedQueryInput
                let field = field_entry.1;
                let field = serde_json::from_value::<FieldName>(field).unwrap();

                if value.as_object_mut().unwrap().is_empty() {
                    let field_query_input =
                        serde_json::from_value::<PdbQuery>(serde_json::Value::String(key)).unwrap();
                    Ok((field, field_query_input))
                } else {
                    let mut reconstructed = serde_json::Map::new();
                    reconstructed.insert(key, value);

                    let field_query_input = serde_json::from_value::<PdbQuery>(
                        serde_json::Value::Object(reconstructed),
                    )
                    .unwrap();
                    Ok((field, field_query_input))
                }
            } else {
                Err(<A::Error as serde::de::Error>::custom(
                    "this does not appear to be a FieldedQueryInput",
                ))
            }
        }
    }
    deserializer.deserialize_map(Visitor)
}

impl SearchQueryInput {
    pub fn postgres_expression(node: *mut pg_sys::Node) -> Self {
        SearchQueryInput::PostgresExpression {
            expr: PostgresExpression {
                node: PostgresPointer(node.cast()),
                expr_state: PostgresPointer::default(),
            },
        }
    }

    pub fn need_scores(&self) -> bool {
        match self {
            SearchQueryInput::Boolean {
                must,
                should,
                must_not,
            } => must
                .iter()
                .chain(should.iter())
                .chain(must_not.iter())
                .any(Self::need_scores),
            SearchQueryInput::Boost { query, .. } => Self::need_scores(query),
            SearchQueryInput::ConstScore { query, .. } => Self::need_scores(query),
            SearchQueryInput::DisjunctionMax { disjuncts, .. } => {
                disjuncts.iter().any(Self::need_scores)
            }
            SearchQueryInput::WithIndex { query, .. } => Self::need_scores(query),
            SearchQueryInput::HeapFilter { indexed_query, .. } => Self::need_scores(indexed_query),
            SearchQueryInput::MoreLikeThis { .. } => true,
            SearchQueryInput::ScoreFilter { .. } => true,
            _ => false,
        }
    }

    pub fn index_oid(&self) -> Option<pg_sys::Oid> {
        match self {
            SearchQueryInput::WithIndex { oid, .. } => Some(*oid),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TermInput {
    pub field: FieldName,
    pub value: OwnedValue,
    #[serde(default)]
    pub is_datetime: bool,
}

/// Serialize a [`SearchQueryInput`] node to a Postgres [`pg_sys::Const`] node, palloc'd
/// in the current memory context.
impl From<SearchQueryInput> for *mut pg_sys::Const {
    fn from(value: SearchQueryInput) -> Self {
        unsafe {
            pg_sys::makeConst(
                searchqueryinput_typoid(),
                -1,
                pg_sys::Oid::INVALID,
                -1,
                value.into_datum().unwrap(),
                false,
                false,
            )
        }
    }
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

fn coerce_bound_to_field_type(
    bound: Bound<OwnedValue>,
    field_type: &FieldType,
) -> Bound<OwnedValue> {
    match bound {
        Bound::Included(OwnedValue::U64(n)) if matches!(field_type, FieldType::F64(_)) => {
            Bound::Included(OwnedValue::F64(n as f64))
        }
        Bound::Included(OwnedValue::I64(n)) if matches!(field_type, FieldType::F64(_)) => {
            Bound::Included(OwnedValue::F64(n as f64))
        }
        Bound::Excluded(OwnedValue::U64(n)) if matches!(field_type, FieldType::F64(_)) => {
            Bound::Excluded(OwnedValue::F64(n as f64))
        }
        Bound::Excluded(OwnedValue::I64(n)) if matches!(field_type, FieldType::F64(_)) => {
            Bound::Excluded(OwnedValue::F64(n as f64))
        }
        bound => bound,
    }
}

impl SearchQueryInput {
    pub fn into_tantivy_query(
        self,
        schema: &SearchIndexSchema,
        parser: &mut QueryParser,
        searcher: &Searcher,
        index_oid: pg_sys::Oid,
        relation_oid: Option<pg_sys::Oid>,
    ) -> Result<Box<dyn Query>, Box<dyn std::error::Error>> {
        match self {
            SearchQueryInput::Uninitialized => {
                panic!("this `SearchQueryInput` instance is uninitialized")
            }
            SearchQueryInput::All => Ok(Box::new(ConstScoreQuery::new(Box::new(AllQuery), 0.0))),
            SearchQueryInput::Boolean {
                must,
                should,
                must_not,
            } => {
                let mut subqueries = vec![];
                for input in must {
                    subqueries.push((
                        Occur::Must,
                        input.into_tantivy_query(
                            schema,
                            parser,
                            searcher,
                            index_oid,
                            relation_oid,
                        )?,
                    ));
                }
                for input in should {
                    subqueries.push((
                        Occur::Should,
                        input.into_tantivy_query(
                            schema,
                            parser,
                            searcher,
                            index_oid,
                            relation_oid,
                        )?,
                    ));
                }
                for input in must_not {
                    subqueries.push((
                        Occur::MustNot,
                        input.into_tantivy_query(
                            schema,
                            parser,
                            searcher,
                            index_oid,
                            relation_oid,
                        )?,
                    ));
                }
                Ok(Box::new(BooleanQuery::new(subqueries)))
            }
            SearchQueryInput::Boost { query, factor } => Ok(Box::new(BoostQuery::new(
                query.into_tantivy_query(schema, parser, searcher, index_oid, relation_oid)?,
                factor,
            ))),
            SearchQueryInput::ConstScore { query, score } => Ok(Box::new(ConstScoreQuery::new(
                query.into_tantivy_query(schema, parser, searcher, index_oid, relation_oid)?,
                score,
            ))),
            SearchQueryInput::ScoreFilter { bounds, query } => Ok(Box::new(ScoreFilter::new(
                bounds,
                query
                    .expect("ScoreFilter's query should have been set")
                    .into_tantivy_query(schema, parser, searcher, index_oid, relation_oid)?,
            ))),
            SearchQueryInput::DisjunctionMax {
                disjuncts,
                tie_breaker,
            } => {
                let disjuncts = disjuncts
                    .into_iter()
                    .map(|query| {
                        query.into_tantivy_query(schema, parser, searcher, index_oid, relation_oid)
                    })
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
            SearchQueryInput::Empty => Ok(Box::new(EmptyQuery)),
            SearchQueryInput::MoreLikeThis {
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
                        Ok(Box::new(builder.with_document(key_value, index_oid)))
                    }
                    (None, Some(doc_fields)) => {
                        let mut fields_map = HashMap::default();
                        for (field, mut value) in doc_fields {
                            let search_field = schema
                                .search_field(&field)
                                .ok_or(QueryError::NonIndexedField(field.into()))?;
                            search_field.try_coerce(&mut value)?;
                            fields_map
                                .entry(search_field.field())
                                .or_insert_with(Vec::new);

                            if let Some(vec) = fields_map.get_mut(&search_field.field()) {
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
            SearchQueryInput::Parse {
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

            SearchQueryInput::TermSet { terms: fields } => {
                let mut terms = vec![];
                for TermInput {
                    field,
                    value,
                    is_datetime,
                } in fields
                {
                    let search_field = schema
                        .search_field(field.root())
                        .ok_or(QueryError::NonIndexedField(field.clone()))?;
                    let field_type = search_field.field_entry().field_type();
                    let is_datetime = search_field.is_datetime() || is_datetime;
                    terms.push(value_to_term(
                        search_field.field(),
                        &value,
                        field_type,
                        field.path().as_deref(),
                        is_datetime,
                    )?);
                }

                Ok(Box::new(TermSetQuery::new(terms)))
            }
            SearchQueryInput::WithIndex { query, .. } => {
                query.into_tantivy_query(schema, parser, searcher, index_oid, relation_oid)
            }
            SearchQueryInput::HeapFilter {
                indexed_query,
                field_filters,
            } => {
                // Convert indexed query first
                let indexed_tantivy_query = indexed_query.into_tantivy_query(
                    schema,
                    parser,
                    searcher,
                    index_oid,
                    relation_oid,
                )?;

                // Create combined query with heap field filters
                Ok(Box::new(heap_field_filter::HeapFilterQuery::new(
                    indexed_tantivy_query,
                    field_filters,
                    relation_oid.expect("relation_oid is required for HeapFilter queries"),
                )))
            }
            SearchQueryInput::PostgresExpression { .. } => {
                panic!("postgres expressions have not been solved")
            }
            SearchQueryInput::FieldedQuery { field, query } => {
                query.into_tantivy_query(field, schema, parser, searcher)
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
) -> Result<Term> {
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
) -> Result<Term> {
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

struct TantivyDateTime(pub DateTime);
impl TryFrom<&str> for TantivyDateTime {
    type Error = QueryError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        let datetime = match chrono::NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%SZ") {
            Ok(dt) => dt,
            Err(_) => chrono::NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%S%.fZ")
                .map_err(|_| QueryError::FieldTypeMismatch)?,
        };
        Ok(TantivyDateTime(DateTime::from_timestamp_micros(
            datetime.and_utc().timestamp_micros(),
        )))
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
    NonIndexedField(FieldName),
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

#[derive(Debug, Clone, PartialEq)]
struct PostgresPointer(*mut std::os::raw::c_void);

// SAFETY: PostgresPointer is only used within PostgreSQL's single-threaded context
// during query execution. The PostgresPointer serialization/deserialization handles
// the cross-thread boundary properly via nodeToString/stringToNode.
unsafe impl Send for PostgresPointer {}
unsafe impl Sync for PostgresPointer {}

impl Default for PostgresPointer {
    fn default() -> Self {
        PostgresPointer(std::ptr::null_mut())
    }
}

impl Serialize for PostgresPointer {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.0.is_null() {
            serializer.serialize_none()
        } else {
            unsafe {
                let s = pg_sys::nodeToString(self.0.cast());
                let cstr = core::ffi::CStr::from_ptr(s)
                    .to_str()
                    .map_err(serde::ser::Error::custom)?;
                let string = cstr.to_owned();
                pg_sys::pfree(s.cast());
                serializer.serialize_some(&string)
            }
        }
    }
}

impl<'de> Deserialize<'de> for PostgresPointer {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct NodeVisitor;
        impl<'de2> Visitor<'de2> for NodeVisitor {
            type Value = PostgresPointer;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                write!(formatter, "a string representing a Postgres node")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                unsafe {
                    let cstr = std::ffi::CString::new(v).map_err(E::custom)?;
                    let node = pg_sys::stringToNode(cstr.as_ptr());
                    Ok(PostgresPointer(node.cast()))
                }
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de2>,
            {
                deserializer.deserialize_str(self)
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(PostgresPointer::default())
            }
        }

        deserializer.deserialize_option(NodeVisitor)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostgresExpression {
    node: PostgresPointer,
    #[serde(skip)]
    expr_state: PostgresPointer,
}

impl PostgresExpression {
    pub fn new(node: *mut pg_sys::Node) -> Self {
        Self {
            node: PostgresPointer(node.cast()),
            expr_state: PostgresPointer::default(),
        }
    }

    pub fn set_expr_state(&mut self, expr_state: *mut pg_sys::ExprState) {
        self.expr_state = PostgresPointer(expr_state.cast())
    }

    #[inline]
    pub fn node(&self) -> *mut pg_sys::Node {
        self.node.0.cast()
    }

    #[inline]
    pub fn expr_state(&self) -> *mut pg_sys::ExprState {
        assert!(
            !self.expr_state.0.is_null(),
            "ExprState has not been initialized"
        );
        self.expr_state.0.cast()
    }
}
