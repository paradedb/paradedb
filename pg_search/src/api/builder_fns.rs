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

use pgrx::*;

use crate::api::{FieldName, HashMap};
use crate::postgres::types::{TantivyValue, TantivyValueError};
use crate::query::pdb_query::pdb;
use crate::query::{SearchQueryInput, TermInput};
use pgrx::nullable::IntoNullableIterator;
use std::ops::Bound;

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
pub fn boost(factor: f32, query: SearchQueryInput) -> SearchQueryInput {
    SearchQueryInput::Boost {
        query: Box::new(query),
        factor,
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

#[pg_extern(name = "more_like_this", immutable, parallel_safe)]
pub fn more_like_this_empty() -> SearchQueryInput {
    panic!("more_like_this must be called with either document_id or document_fields");
}

#[allow(clippy::too_many_arguments)]
#[pg_extern(name = "more_like_this", immutable, parallel_safe)]
pub fn more_like_this_fields(
    document_fields: String,
    min_doc_frequency: default!(Option<i32>, "NULL"),
    max_doc_frequency: default!(Option<i32>, "NULL"),
    min_term_frequency: default!(Option<i32>, "NULL"),
    max_query_terms: default!(Option<i32>, "NULL"),
    min_word_length: default!(Option<i32>, "NULL"),
    max_word_length: default!(Option<i32>, "NULL"),
    boost_factor: default!(Option<f32>, "NULL"),
    stop_words: default!(Option<Vec<String>>, "NULL"),
) -> SearchQueryInput {
    let document_fields: HashMap<String, tantivy::schema::OwnedValue> =
        json5::from_str(&document_fields).expect("could not parse document_fields");

    SearchQueryInput::MoreLikeThis {
        min_doc_frequency: min_doc_frequency.map(|n| n as u64),
        max_doc_frequency: max_doc_frequency.map(|n| n as u64),
        min_term_frequency: min_term_frequency.map(|n| n as usize),
        max_query_terms: max_query_terms.map(|n| n as usize),
        min_word_length: min_word_length.map(|n| n as usize),
        max_word_length: max_word_length.map(|n| n as usize),
        boost_factor,
        stop_words,
        document_fields: Some(document_fields.into_iter().collect()),
        document_id: None,
    }
}

#[allow(clippy::too_many_arguments)]
#[pg_extern(name = "more_like_this", immutable, parallel_safe)]
pub fn more_like_this_id(
    document_id: AnyElement,
    min_doc_frequency: default!(Option<i32>, "NULL"),
    max_doc_frequency: default!(Option<i32>, "NULL"),
    min_term_frequency: default!(Option<i32>, "NULL"),
    max_query_terms: default!(Option<i32>, "NULL"),
    min_word_length: default!(Option<i32>, "NULL"),
    max_word_length: default!(Option<i32>, "NULL"),
    boost_factor: default!(Option<f32>, "NULL"),
    stop_words: default!(Option<Vec<String>>, "NULL"),
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
        document_fields: None,
        document_id: unsafe {
            Some(
                TantivyValue::try_from_datum(
                    document_id.datum(),
                    PgOid::from_untagged(document_id.oid()),
                )
                .unwrap_or_else(|err| panic!("could not read more_like_this document_id: {err}"))
                .0,
            )
        },
    }
}

#[pg_extern(immutable, parallel_safe)]
pub fn parse(
    query_string: String,
    lenient: default!(Option<bool>, "NULL"),
    conjunction_mode: default!(Option<bool>, "NULL"),
) -> SearchQueryInput {
    SearchQueryInput::Parse {
        query_string,
        lenient,
        conjunction_mode,
    }
}

#[rustfmt::skip]
#[pg_extern(immutable, parallel_safe)]
pub unsafe fn term_with_operator(
    field: FieldName,
    operator: String,
    value: AnyElement,
) -> anyhow::Result<SearchQueryInput> {
    macro_rules! make_query {
        ($operator:expr, $field:expr, $eq_func_name:ident, $value_type:ty, $anyelement:expr, $is_datetime:literal) => {
            match $operator.as_str() {
                "=" => Ok(SearchQueryInput::FieldedQuery { field: $field, query: $eq_func_name(<$value_type>::from_datum($anyelement.datum(), false).unwrap())}),
                "<>" => Ok(
                    SearchQueryInput::Boolean {
                        // ensure that we don't match NULL nulls as not being equal to whatever the user specified
                        must: vec![SearchQueryInput::FieldedQuery {field: $field.clone(), query: pdb::Query::Exists}],
                        should: vec![],
                        must_not: vec![SearchQueryInput::FieldedQuery { field: $field, query: $eq_func_name(<$value_type>::from_datum($anyelement.datum(), false).unwrap())}]
                    }
                ),
                ">" => generic_range_query(Bound::Excluded($anyelement), Bound::Unbounded, $is_datetime).map(|query| SearchQueryInput::FieldedQuery { field: $field, query }),
                ">=" => generic_range_query(Bound::Included($anyelement), Bound::Unbounded, $is_datetime).map(|query| SearchQueryInput::FieldedQuery { field: $field, query }),
                "<" => generic_range_query(Bound::Unbounded, Bound::Excluded($anyelement), $is_datetime).map(|query| SearchQueryInput::FieldedQuery { field: $field, query }),
                "<=" => generic_range_query(Bound::Unbounded, Bound::Included($anyelement), $is_datetime).map(|query| SearchQueryInput::FieldedQuery { field: $field, query }),
                other => panic!("unsupported operator: {other}"),
            }
        };
    }

    use crate::api::pdb_builder_fns::*;
    match value.oid() {
        pg_sys::CHAROID => make_query!(operator, field, term_i8, i8, value, false),
        pg_sys::INT2OID => make_query!(operator, field, term_i16, i16, value, false),
        pg_sys::INT4OID => make_query!(operator, field, term_i32, i32, value, false),
        pg_sys::INT8OID => make_query!(operator, field, term_i64, i64, value, false),
        pg_sys::FLOAT4OID => make_query!(operator, field, term_f32, f32, value, false),
        pg_sys::FLOAT8OID => make_query!(operator, field, term_f64, f64, value, false),
        pg_sys::BOOLOID => make_query!(operator, field, term_bool, bool, value, false),
        pg_sys::NUMERICOID => make_query!(operator, field, numeric, AnyNumeric, value, false),

        pg_sys::TEXTOID => make_query!(operator, field, term_str, String, value, false),
        pg_sys::VARCHAROID => make_query!(operator, field, term_str, String, value, false),
        pg_sys::UUIDOID => make_query!(operator, field, uuid, pgrx::datum::Uuid, value, false),

        pg_sys::DATEOID => make_query!(operator, field, date, pgrx::datum::Date, value, true),
        pg_sys::TIMEOID => make_query!(operator, field, time, pgrx::datum::Time, value, true),
        pg_sys::TIMETZOID => make_query!(operator, field, time_with_time_zone, pgrx::datum::TimeWithTimeZone, value, true),
        pg_sys::TIMESTAMPOID => make_query!(operator, field, timestamp, pgrx::datum::Timestamp, value, true),
        pg_sys::TIMESTAMPTZOID => make_query!(operator, field, timestamp_with_time_zone, pgrx::datum::TimestampWithTimeZone, value, true),

        other => panic!("unsupported type: {other:?}"),
    }
}

/// An internal function, not intended to be used directly by end users
/// but instead used by our pushdown code.
///
/// Our pushdown code will rewrite the former into the latter.
///
/// ```sql
/// SELECT * FROM mock_items WHERE rating = ANY(ARRAY[1, 2, 3]);
/// ```
///
/// is equivalent to:
///
/// ```sql
/// SELECT * FROM mock_items WHERE id @@@ paradedb.terms_with_operator('rating', '=', ARRAY[1, 2, 3], false);
/// ```
///
#[pg_extern(immutable, parallel_safe)]
pub unsafe fn terms_with_operator(
    field: FieldName,
    operator: String,
    value: AnyElement,
    conjunction_mode: bool,
) -> anyhow::Result<SearchQueryInput> {
    let array_type = unsafe { pg_sys::get_element_type(value.oid()) };
    if array_type == pg_sys::InvalidOid {
        panic!("terms_with_operator: value is not an array");
    }

    let array = Array::<pg_sys::Datum>::from_datum(value.datum(), false).unwrap();
    if !conjunction_mode && operator.trim() == "=" {
        // represents:
        //    - field IN (x, y, z)
        //    - = ANY(ARRAY[x, y, z])
        //
        // this case can be optimized to use the tantivy TermSet
        let is_datetime = matches!(
            array_type,
            pg_sys::DATEOID
                | pg_sys::TIMEOID
                | pg_sys::TIMETZOID
                | pg_sys::TIMESTAMPOID
                | pg_sys::TIMESTAMPTZOID
        );
        let array_type = PgOid::from_untagged(array_type);
        let terms = array
            .into_nullable_iter()
            // skip NULL values in the array -- SQL boolean logic says we can't match them
            // and since this is the disjunction case ("OR"), we don't need to anyway
            .filter(|datum| datum.is_valid())
            .map(|datum| {
                let value = TantivyValue::try_from_datum(datum.unwrap(), array_type)?;

                Ok(TermInput {
                    field: field.clone(),
                    value: value.into(),
                    is_datetime,
                })
            })
            .collect::<Result<Vec<_>, TantivyValueError>>()?;

        Ok(SearchQueryInput::TermSet { terms })
    } else {
        // this case must form a tantivy Boolean expression of individual terms
        // which are OR'd or AND'd together
        //
        // represents the general forms of:
        //      - field $op ANY|ALL(ARRAY[x, y, z])
        //
        // and specifically the cases of:
        //      - field NOT IN (x, y, z), which is actually rewritten as...
        //      - field <> ALL(ARRAY[x, y, z])
        //

        if conjunction_mode && array.contains_nulls() {
            // if the array contains a null, it cannot be matched in a conjunction because in SQL a NULL doesn't
            // compare in any way to another NULL
            return Ok(SearchQueryInput::Empty);
        }

        let quals = array
            .into_nullable_iter()
            .filter(|datum| datum.is_valid())
            .map(|datum| {
                let anyelement =
                    AnyElement::from_polymorphic_datum(datum.unwrap(), false, array_type)
                        .expect("should be a valid AnyElement");
                term_with_operator(field.clone(), operator.clone(), anyelement)
                    .expect("should return a valid SearchQueryInput")
            })
            .collect::<Vec<_>>();

        if conjunction_mode {
            // AND the queries together
            Ok(SearchQueryInput::Boolean {
                must: quals,
                should: vec![],
                must_not: vec![],
            })
        } else {
            // OR the queries together
            Ok(SearchQueryInput::Boolean {
                must: vec![],
                should: quals,
                must_not: vec![],
            })
        }
    }
}

macro_rules! term_fn_unsupported {
    ($func_name:ident, $value_type:ty, $term_type:literal) => {
        #[pg_extern(name = "term", immutable, parallel_safe)]
        #[allow(unused)]
        pub fn $func_name(
            field: FieldName,
            value: default!(Option<$value_type>, "NULL"),
        ) -> SearchQueryInput {
            unimplemented!("{} in term query not implemented", $term_type)
        }
    };
}

// Generate functions for each type
// NOTE: We cannot use AnyElement for `term` because it sullies the user experience.
//       For example, searching for a string value is an ambiguous type, so the user
//       would have to search for 'string'::text or 'string'::varchar in the `value`
//       argument.

term_fn_unsupported!(json, pgrx::Json, "json");
term_fn_unsupported!(jsonb, pgrx::JsonB, "jsonb");
term_fn_unsupported!(anyarray, pgrx::AnyArray, "array");
term_fn_unsupported!(pg_box, pgrx::pg_sys::BOX, "box");
term_fn_unsupported!(point, pgrx::pg_sys::Point, "point");
term_fn_unsupported!(tid, pgrx::pg_sys::ItemPointerData, "tid");
term_fn_unsupported!(int4range, pgrx::Range<i32>, "int4 range");
term_fn_unsupported!(int8range, pgrx::Range<i64>, "int8 range");
term_fn_unsupported!(numrange, pgrx::Range<pgrx::AnyNumeric>, "numeric range");
term_fn_unsupported!(daterange, pgrx::Range<pgrx::datum::Date>, "date range");
term_fn_unsupported!(
    tsrange,
    pgrx::Range<pgrx::datum::Timestamp>,
    "timestamp range"
);
term_fn_unsupported!(
    tstzrange,
    pgrx::Range<pgrx::datum::TimestampWithTimeZone>,
    "timestamp ranges with time zone"
);

#[pg_extern(immutable, parallel_safe)]
pub fn term_set(terms: Vec<SearchQueryInput>) -> SearchQueryInput {
    let terms: Vec<_> = terms
        .into_iter()
        .map(|input| match input {
            SearchQueryInput::FieldedQuery {
                field,
                query: pdb::Query::Term { value, is_datetime },
            } => TermInput {
                field,
                value,
                is_datetime,
            },
            _ => panic!("only term queries can be passed to term_set"),
        })
        .collect();

    SearchQueryInput::TermSet { terms }
}

#[pg_cast(implicit)]
fn jsonb_to_searchqueryinput(query: JsonB) -> SearchQueryInput {
    serde_path_to_error::deserialize(query.0).unwrap_or_else(|err| {
        panic!(
            r#"error parsing search query input json at "{}": {}"#,
            err.path(),
            match err.inner().to_string() {
                msg if msg.contains("expected unit") => {
                    format!(
                        r#"invalid type: map, pass null as value for "{}""#,
                        err.path()
                    )
                }
                msg => msg,
            }
        )
    })
}

extension_sql!(
    "ALTER FUNCTION jsonb_to_searchqueryinput IMMUTABLE STRICT PARALLEL SAFE;",
    name = "jsonb_to_searchqueryinput",
    requires = [jsonb_to_searchqueryinput]
);
