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

use crate::api::FieldName;
use crate::query::pdb_query::pdb::{FuzzyData, ScoreAdjustStyle, SlopData};
use crate::query::proximity::query::ProximityQuery;
use crate::query::proximity::{ProximityClause, ProximityDistance};
use crate::query::range::{Comparison, RangeField};
use crate::query::{
    check_range_bounds, coerce_bound_to_field_type, expand_json_numeric_to_terms,
    value_to_json_term, value_to_term, QueryError, SearchQueryInput, F64_SAFE_INTEGER_MAX,
};
use crate::schema::{IndexRecordOption, SearchIndexSchema};
use pgrx::{pg_extern, pg_schema, InOutFuncs, StringInfo};
use serde_json::Value;
use smallvec::smallvec;
use std::collections::Bound;
use std::ffi::CStr;
use tantivy::query::{
    AllQuery, BooleanQuery, BoostQuery, ConstScoreQuery, EmptyQuery, ExistsQuery,
    FastFieldRangeQuery, FuzzyTermQuery, Occur, PhrasePrefixQuery, PhraseQuery,
    Query as TantivyQuery, Query, QueryParser, RangeQuery, RegexPhraseQuery, RegexQuery, TermQuery,
    TermSetQuery,
};
use tantivy::schema::OwnedValue;
use tantivy::schema::{Field, FieldType};
use tantivy::{Searcher, Term};
use tokenizers::SearchTokenizer;

#[pg_extern(immutable, parallel_safe)]
pub fn to_search_query_input(field: FieldName, query: pdb::Query) -> SearchQueryInput {
    SearchQueryInput::FieldedQuery { field, query }
}

#[pg_schema]
pub mod pdb {
    use crate::query::proximity::{ProximityClause, ProximityDistance};
    use crate::query::range::{deserialize_bound, serialize_bound};
    use pgrx::PostgresType;
    use serde::{Deserialize, Serialize};
    use std::collections::Bound;
    use std::fmt::{Display, Formatter};
    use tantivy::schema::OwnedValue;

    #[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
    #[serde(rename_all = "snake_case")]
    pub struct FuzzyData {
        pub distance: u8,
        pub prefix: bool,
        pub transposition_cost_one: bool,
    }

    impl Display for FuzzyData {
        #[rustfmt::skip]
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{},{},{}",
                self.distance,
                if self.prefix { "t" } else { "f" },
                if self.transposition_cost_one { "t" } else { "f" }
            )
        }
    }

    impl From<i32> for FuzzyData {
        fn from(value: i32) -> Self {
            let distance = (value >> 2) as u8;
            let prefix = (value & 2) != 0;
            let transposition_cost_one = (value & 1) != 0;
            FuzzyData {
                distance,
                prefix,
                transposition_cost_one,
            }
        }
    }

    impl From<FuzzyData> for i32 {
        fn from(data: FuzzyData) -> Self {
            ((data.distance as i32) << 2)
                | ((data.prefix as i32) << 1)
                | (data.transposition_cost_one as i32)
        }
    }

    #[test]
    fn fuzzy_data_roundtrip() {
        proptest::proptest!(|(distance in 0u8..=255u8, prefix in 0..=1, transposition_cost_one in 0..=1)| {
            let original = FuzzyData {
                distance,
                prefix: prefix == 1,
                transposition_cost_one: transposition_cost_one == 1,
            };

            let typmod_repr:i32 = original.clone().into();
            assert!(typmod_repr >= 0);  // can't be negative
            let from_typmod:FuzzyData = typmod_repr.into();
            assert_eq!(original, from_typmod);
        })
    }

    #[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
    #[serde(rename_all = "snake_case")]
    pub struct SlopData {
        pub slop: u32,
    }

    impl Display for SlopData {
        #[rustfmt::skip]
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{}",
                self.slop,
            )
        }
    }

    impl From<i32> for SlopData {
        fn from(value: i32) -> Self {
            SlopData { slop: value as u32 }
        }
    }

    impl From<SlopData> for i32 {
        fn from(data: SlopData) -> Self {
            data.slop as i32
        }
    }

    #[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
    pub enum ScoreAdjustStyle {
        Boost(tantivy::Score),
        Const(tantivy::Score),
    }

    #[derive(Debug, PostgresType, Deserialize, Serialize, Clone, PartialEq)]
    #[inoutfuncs]
    #[serde(rename_all = "snake_case")]
    pub enum Query {
        All,
        Empty,

        /// This is instantiated in places where a string literal is used
        /// as the right-hand-side of one of our operators.  For example, in
        ///
        /// ```sql
        /// SELECT * FROM t WHERE f @@@ 'some string'
        /// ```
        ///
        /// This variant is constructed first, then the "SUPPORT" function for our various operators
        /// will rewrite it to the [`Query`] variant that is correct for its usage.
        ///
        /// For example, the `===` operator will rewrite it to a [`Query::Term`] and `@@@` to
        /// a [`Query::ParseWithField`].
        UnclassifiedString {
            string: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            fuzzy_data: Option<FuzzyData>,
            #[serde(skip_serializing_if = "Option::is_none")]
            slop_data: Option<SlopData>,
        },
        /// This is instantiated in places where a text array is used
        /// as the right-hand-side of one of our operators.  For example, in
        ///
        /// ```sql
        /// SELECT * FROM t WHERE f @@@ ARRAY['some', 'terms']
        /// ```
        ///
        /// This variant is constructed first, then the "SUPPORT" function for our various operators
        /// will rewrite it to the [`Query`] variant that is correct for its usage.
        ///
        /// For example, the `===` operator will rewrite it to a [`Query::TermSet`] and `@@@` to
        /// a [`Query::ParseWithField`].
        UnclassifiedArray {
            array: Vec<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            fuzzy_data: Option<FuzzyData>,
            #[serde(skip_serializing_if = "Option::is_none")]
            slop_data: Option<SlopData>,
        },
        ScoreAdjusted {
            query: Box<Query>,
            score: Option<ScoreAdjustStyle>,
        },
        Exists,
        FastFieldRangeWeight {
            #[serde(
                serialize_with = "serialize_bound",
                deserialize_with = "deserialize_bound"
            )]
            lower_bound: Bound<u64>,
            #[serde(
                serialize_with = "serialize_bound",
                deserialize_with = "deserialize_bound"
            )]
            upper_bound: Bound<u64>,
        },
        FuzzyTerm {
            value: String,
            distance: Option<u8>,
            transposition_cost_one: Option<bool>,
            prefix: Option<bool>,
        },
        Match {
            value: String,
            tokenizer: Option<serde_json::Value>,
            distance: Option<u8>,
            transposition_cost_one: Option<bool>,
            prefix: Option<bool>,
            conjunction_mode: Option<bool>,
        },
        MatchArray {
            tokens: Vec<String>,
            distance: Option<u8>,
            transposition_cost_one: Option<bool>,
            prefix: Option<bool>,
            conjunction_mode: Option<bool>,
        },
        Parse {
            query_string: String,
            lenient: Option<bool>,
            conjunction_mode: Option<bool>,
        },
        ParseWithField {
            query_string: String,
            lenient: Option<bool>,
            conjunction_mode: Option<bool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            fuzzy_data: Option<FuzzyData>,
        },
        Phrase {
            phrases: Vec<String>,
            slop: Option<u32>,
        },
        PhraseArray {
            tokens: Vec<String>,
            slop: Option<u32>,
        },
        PhrasePrefix {
            phrases: Vec<String>,
            max_expansions: Option<u32>,
        },
        Proximity {
            left: ProximityClause,
            distance: ProximityDistance,
            right: ProximityClause,
        },
        TokenizedPhrase {
            phrase: String,
            slop: Option<u32>,
        },
        Range {
            #[serde(
                serialize_with = "serialize_bound",
                deserialize_with = "deserialize_bound"
            )]
            lower_bound: Bound<OwnedValue>,
            #[serde(
                serialize_with = "serialize_bound",
                deserialize_with = "deserialize_bound"
            )]
            upper_bound: Bound<OwnedValue>,
            #[serde(default)]
            is_datetime: bool,
        },
        RangeContains {
            #[serde(
                serialize_with = "serialize_bound",
                deserialize_with = "deserialize_bound"
            )]
            lower_bound: Bound<OwnedValue>,
            #[serde(
                serialize_with = "serialize_bound",
                deserialize_with = "deserialize_bound"
            )]
            upper_bound: Bound<OwnedValue>,
            #[serde(default)]
            is_datetime: bool,
        },
        RangeIntersects {
            #[serde(
                serialize_with = "serialize_bound",
                deserialize_with = "deserialize_bound"
            )]
            lower_bound: Bound<OwnedValue>,
            #[serde(
                serialize_with = "serialize_bound",
                deserialize_with = "deserialize_bound"
            )]
            upper_bound: Bound<OwnedValue>,
            #[serde(default)]
            is_datetime: bool,
        },
        RangeTerm {
            value: OwnedValue,
            #[serde(default)]
            is_datetime: bool,
        },
        RangeWithin {
            #[serde(
                serialize_with = "serialize_bound",
                deserialize_with = "deserialize_bound"
            )]
            lower_bound: Bound<OwnedValue>,
            #[serde(
                serialize_with = "serialize_bound",
                deserialize_with = "deserialize_bound"
            )]
            upper_bound: Bound<OwnedValue>,
            #[serde(default)]
            is_datetime: bool,
        },
        Regex {
            pattern: String,
        },
        RegexPhrase {
            regexes: Vec<String>,
            slop: Option<u32>,
            max_expansions: Option<u32>,
        },
        Term {
            value: OwnedValue,
            #[serde(default)]
            is_datetime: bool,
        },
        TermSet {
            terms: Vec<OwnedValue>,
        },
    }
}

impl pdb::Query {
    pub fn unclassified_string(s: &str) -> pdb::Query {
        pdb::Query::UnclassifiedString {
            string: s.to_string(),
            fuzzy_data: None,
            slop_data: None,
        }
    }

    pub fn unclassified_string_with_fuzz(s: &str, fuzz: FuzzyData) -> pdb::Query {
        pdb::Query::UnclassifiedString {
            string: s.to_string(),
            fuzzy_data: Some(fuzz),
            slop_data: None,
        }
    }

    pub fn unclassified_string_with_slop(s: &str, slop: SlopData) -> pdb::Query {
        pdb::Query::UnclassifiedString {
            string: s.to_string(),
            fuzzy_data: None,
            slop_data: Some(slop),
        }
    }

    pub fn apply_fuzzy_data(&mut self, new_fuzzy_data: Option<FuzzyData>) {
        if new_fuzzy_data.is_none() {
            return;
        }
        let new_fuzzy_data = new_fuzzy_data.unwrap();
        match self {
            pdb::Query::UnclassifiedString { fuzzy_data, .. } => {
                *fuzzy_data = Some(new_fuzzy_data);
            }
            pdb::Query::UnclassifiedArray { fuzzy_data, .. } => {
                *fuzzy_data = Some(new_fuzzy_data);
            }

            pdb::Query::Term {
                value: OwnedValue::Str(value),
                is_datetime,
            } if !*is_datetime => {
                *self = pdb::Query::FuzzyTerm {
                    value: value.to_string(),
                    distance: Some(new_fuzzy_data.distance),
                    transposition_cost_one: Some(new_fuzzy_data.transposition_cost_one),
                    prefix: Some(new_fuzzy_data.prefix),
                }
            }

            pdb::Query::TermSet { terms } => {
                // must convert to an OR'd set of FuzzyTerms
                let mut fuzzy_terms = Vec::with_capacity(terms.len());
                for term in terms {
                    let OwnedValue::Str(term) = term else {
                        continue;
                    };
                    fuzzy_terms.push(term.clone());
                }

                *self = pdb::Query::MatchArray {
                    tokens: fuzzy_terms,
                    distance: Some(new_fuzzy_data.distance),
                    transposition_cost_one: Some(new_fuzzy_data.transposition_cost_one),
                    prefix: Some(new_fuzzy_data.prefix),
                    conjunction_mode: Some(false),
                }
            }

            pdb::Query::FuzzyTerm {
                distance,
                transposition_cost_one,
                prefix,
                ..
            } => {
                *distance = Some(new_fuzzy_data.distance);
                *transposition_cost_one = Some(new_fuzzy_data.transposition_cost_one);
                *prefix = Some(new_fuzzy_data.prefix);
            }

            pdb::Query::Match {
                distance,
                transposition_cost_one,
                prefix,
                ..
            } => {
                *distance = Some(new_fuzzy_data.distance);
                *transposition_cost_one = Some(new_fuzzy_data.transposition_cost_one);
                *prefix = Some(new_fuzzy_data.prefix);
            }

            pdb::Query::ParseWithField { fuzzy_data, .. } => *fuzzy_data = Some(new_fuzzy_data),

            _ => panic!("query type is not compatible with fuzzy"),
        }
    }

    pub fn apply_slop_data(&mut self, new_slop_data: Option<SlopData>) {
        if new_slop_data.is_none() {
            return;
        }
        let new_slop_data = new_slop_data.unwrap();
        match self {
            pdb::Query::UnclassifiedString { slop_data, .. } => {
                *slop_data = Some(new_slop_data);
            }
            pdb::Query::UnclassifiedArray { slop_data, .. } => {
                *slop_data = Some(new_slop_data);
            }
            pdb::Query::Phrase { slop, .. } => *slop = Some(new_slop_data.slop),

            pdb::Query::PhraseArray { slop, .. } => *slop = Some(new_slop_data.slop),

            pdb::Query::TokenizedPhrase { slop, .. } => *slop = Some(new_slop_data.slop),

            _ => panic!("query type is not compatible with slop"),
        }
    }

    pub fn into_tantivy_query<QueryParserCtor: Fn() -> QueryParser>(
        self,
        field: FieldName,
        schema: &SearchIndexSchema,
        parser: &QueryParserCtor,
        searcher: &Searcher,
    ) -> anyhow::Result<Box<dyn TantivyQuery>> {
        let query: Box<dyn TantivyQuery> = match self {
            pdb::Query::All => Box::new(AllQuery),
            pdb::Query::Empty => Box::new(EmptyQuery),

            pdb::Query::UnclassifiedString { .. } => {
                // this would indicate a problem with the various operator SUPPORT functions failing
                // to convert the UnclassifiedString into the pdb::Query variant they require
                unreachable!(
                    "pdb::Query::UnclassifiedString cannot be converted into a TantivyQuery"
                )
            }
            pdb::Query::UnclassifiedArray { .. } => {
                // this would indicate a problem with the various operator SUPPORT functions failing
                // to convert the UnclassifiedArray into the pdb::Query variant they require
                unreachable!(
                    "pdb::Query::UnclassifiedArray cannot be converted into a TantivyQuery"
                )
            }
            pdb::Query::Exists => exists(field, searcher),
            pdb::Query::ScoreAdjusted { query, score } => score_adjust_query(
                field,
                schema,
                parser,
                searcher,
                *query,
                score.expect("score adjustment value should have been set"),
            )?,
            pdb::Query::FastFieldRangeWeight {
                lower_bound,
                upper_bound,
            } => fast_field_range_weight(&field, schema, lower_bound, upper_bound),
            pdb::Query::FuzzyTerm {
                value,
                distance,
                transposition_cost_one,
                prefix,
            } => fuzzy_term(
                &field,
                schema,
                value,
                distance,
                transposition_cost_one,
                prefix,
            )?,
            pdb::Query::Match {
                value,
                tokenizer,
                distance,
                transposition_cost_one,
                prefix,
                conjunction_mode,
            } => match_query(
                &field,
                schema,
                searcher,
                &value,
                tokenizer,
                distance,
                transposition_cost_one,
                prefix,
                conjunction_mode,
            )?,
            pdb::Query::MatchArray {
                tokens: value,
                distance,
                transposition_cost_one,
                prefix,
                conjunction_mode,
            } => match_array_query(
                &field,
                schema,
                value,
                distance,
                transposition_cost_one,
                prefix,
                conjunction_mode,
            )?,
            pdb::Query::Parse {
                query_string,
                lenient,
                conjunction_mode,
            } => parse(parser, query_string, lenient, conjunction_mode)?,
            pdb::Query::ParseWithField {
                query_string,
                lenient,
                conjunction_mode,
                fuzzy_data,
            } => parse_with_field(
                &field,
                parser,
                schema,
                query_string,
                lenient,
                conjunction_mode,
                fuzzy_data,
            )?,

            pdb::Query::Phrase { phrases, slop } => {
                phrase(&field, schema, searcher, phrases, slop)?
            }
            pdb::Query::PhraseArray { tokens, slop } => phrase_array(&field, schema, tokens, slop)?,

            pdb::Query::PhrasePrefix {
                phrases,
                max_expansions,
            } => phrase_prefix(&field, schema, phrases, max_expansions)?,
            pdb::Query::Proximity {
                left,
                distance,
                right,
            } => proximity(&field, schema, left, distance, right)?,
            pdb::Query::TokenizedPhrase { phrase, slop } => {
                tokenized_phrase(&field, schema, searcher, &phrase, slop)
            }
            pdb::Query::Range {
                lower_bound,
                upper_bound,
                is_datetime,
            } => range(&field, schema, lower_bound, upper_bound, is_datetime)?,
            pdb::Query::RangeContains {
                lower_bound,
                upper_bound,
                is_datetime,
            } => range_contains(&field, schema, lower_bound, upper_bound, is_datetime)?,
            pdb::Query::RangeIntersects {
                lower_bound,
                upper_bound,
                is_datetime,
            } => range_intersects(&field, schema, lower_bound, upper_bound, is_datetime)?,
            pdb::Query::RangeTerm { value, is_datetime } => {
                range_term(&field, schema, &value, is_datetime)?
            }
            pdb::Query::RangeWithin {
                lower_bound,
                upper_bound,
                is_datetime,
            } => range_within(&field, schema, lower_bound, upper_bound, is_datetime)?,
            pdb::Query::Regex { pattern } => regex(&field, schema, &pattern)?,
            pdb::Query::RegexPhrase {
                regexes,
                slop,
                max_expansions,
            } => regex_phrase(&field, schema, regexes, slop, max_expansions)?,
            pdb::Query::Term { value, is_datetime } => term(field, schema, &value, is_datetime)?,
            pdb::Query::TermSet { terms } => term_set(field, schema, terms)?,
        };

        Ok(query)
    }
}

impl InOutFuncs for pdb::Query {
    fn input(input: &CStr) -> Self
    where
        Self: Sized,
    {
        if let Ok(from_json) = serde_json::from_slice::<pdb::Query>(input.to_bytes()) {
            from_json
        } else {
            // assume it's just a string and write it as a "match"
            pdb::Query::UnclassifiedString {
                string: input
                    .to_str()
                    .expect("input should be valid UTF8")
                    .to_string(),
                fuzzy_data: None,
                slop_data: None,
            }
        }
    }

    fn output(&self, buffer: &mut StringInfo) {
        serde_json::to_writer(buffer, self).unwrap();
    }
}

fn score_adjust_query<QueryParserCtor: Fn() -> QueryParser>(
    field: FieldName,
    schema: &SearchIndexSchema,
    parser: &QueryParserCtor,
    searcher: &Searcher,
    query: pdb::Query,
    score: ScoreAdjustStyle,
) -> anyhow::Result<Box<dyn TantivyQuery>> {
    let query = query.into_tantivy_query(field, schema, parser, searcher)?;
    match score {
        ScoreAdjustStyle::Boost(boost) => Ok(Box::new(BoostQuery::new(query, boost))),
        ScoreAdjustStyle::Const(score) => Ok(Box::new(ConstScoreQuery::new(query, score))),
    }
}

fn proximity(
    field: &FieldName,
    schema: &SearchIndexSchema,
    left: ProximityClause,
    distance: ProximityDistance,
    right: ProximityClause,
) -> anyhow::Result<Box<dyn Query>> {
    if left.is_empty() || right.is_empty() {
        return Ok(Box::new(EmptyQuery));
    }

    let search_field = schema
        .search_field(field.root())
        .ok_or(QueryError::NonIndexedField(field.clone()))?;
    if !search_field.is_tokenized_with_freqs_and_positions() {
        return Err(QueryError::InvalidTokenizer.into());
    }

    let prox = ProximityQuery::new(search_field.field(), left, distance, right);
    Ok(Box::new(prox))
}

/// Creates TermSetQuery for pdb::Query::TermSet (used by === operator for TEXT, not for numeric IN pushdown).
/// For JSON numeric fields, expands each value into I64/U64/F64 variants for cross-type matching.
/// Numeric IN clauses use query/mod.rs::SearchQueryInput::TermSet instead.
fn term_set(
    field: FieldName,
    schema: &SearchIndexSchema,
    terms: Vec<OwnedValue>,
) -> anyhow::Result<Box<dyn TantivyQuery>> {
    // For JSON paths like "data.amount", we must look up the root field "data" in the schema
    // to correctly identify it as a JSON field type. Using the full path would fail the lookup
    // since only the root column exists in the schema as a JsonObject field.
    let search_field = schema
        .search_field(field.root())
        .expect("field should exist in schema");
    let field_type = search_field.field_entry().field_type();
    let tantivy_field = search_field.field();
    let is_date_time = search_field.is_datetime();

    // Check if this is a JSON numeric field requiring multi-type matching
    let is_json_field = search_field.is_json();
    let has_nested_path = field.path().is_some();
    let is_json_numeric_field = is_json_field && has_nested_path;

    let has_numeric_terms = terms.iter().any(|v| {
        matches!(
            v,
            OwnedValue::F64(_) | OwnedValue::I64(_) | OwnedValue::U64(_)
        )
    });

    if is_json_numeric_field && has_numeric_terms && !is_date_time {
        // For JSON numeric fields, each term may expand to multiple type variants
        let all_terms: Vec<Term> = terms
            .into_iter()
            .flat_map(|term_value| {
                if matches!(
                    term_value,
                    OwnedValue::F64(_) | OwnedValue::I64(_) | OwnedValue::U64(_)
                ) {
                    // Expand numeric value to all possible type variants
                    expand_json_numeric_to_terms(
                        tantivy_field,
                        &term_value,
                        field.path().as_deref(),
                    )
                    .expect("could not expand JSON numeric to terms")
                } else {
                    // Non-numeric values use standard term creation
                    smallvec![value_to_term(
                        tantivy_field,
                        &term_value,
                        field_type,
                        field.path().as_deref(),
                        is_date_time,
                    )
                    .expect("could not convert argument to search term")]
                }
            })
            .collect();

        return Ok(Box::new(TermSetQuery::new(all_terms)));
    }

    // Standard term set for non-JSON or non-numeric fields
    Ok(Box::new(TermSetQuery::new(terms.into_iter().map(|term| {
        value_to_term(
            tantivy_field,
            &term,
            field_type,
            field.path().as_deref(),
            is_date_time,
        )
        .expect("could not convert argument to search term")
    }))))
}

fn term(
    field: FieldName,
    schema: &SearchIndexSchema,
    value: &OwnedValue,
    is_datetime: bool,
) -> anyhow::Result<Box<dyn TantivyQuery>> {
    let record_option = IndexRecordOption::WithFreqsAndPositions;
    let search_field = schema
        .search_field(field.root())
        .ok_or(QueryError::NonIndexedField(field.clone()))?;
    let field_type = search_field.field_entry().field_type();
    let is_datetime = search_field.is_datetime() || is_datetime;

    // For JSON numeric fields, create multi-type query to handle I64/F64 matching
    // Check if the root field is JSON AND if we have a nested path (indicating JSON field access)
    let is_json_field = search_field.is_json();
    let has_nested_path = field.path().is_some(); // If path exists, we're accessing a nested field
    let is_json_numeric_field = is_json_field && has_nested_path;

    let is_numeric_value = matches!(
        value,
        OwnedValue::F64(_) | OwnedValue::I64(_) | OwnedValue::U64(_)
    );

    if is_json_numeric_field && is_numeric_value && !is_datetime {
        // Use the shared helper function to expand numeric values to multiple type variants
        let expanded_terms =
            expand_json_numeric_to_terms(search_field.field(), value, field.path().as_deref())?;

        // Convert record_option once to avoid ownership issues in closure
        let index_record_option = record_option.into();

        // If only one term variant, return a simple TermQuery (optimization)
        if expanded_terms.len() == 1 {
            return Ok(Box::new(TermQuery::new(
                expanded_terms.into_iter().next().unwrap(),
                index_record_option,
            )));
        }

        // Multiple term variants: create BooleanQuery with OR logic (should)
        let term_queries: Vec<(Occur, Box<dyn TantivyQuery>)> = expanded_terms
            .into_iter()
            .map(|term| {
                (
                    Occur::Should,
                    Box::new(TermQuery::new(term, index_record_option)) as Box<dyn TantivyQuery>,
                )
            })
            .collect();

        return Ok(Box::new(BooleanQuery::new(term_queries)));
    }

    // Standard single-term query for non-JSON or non-numeric fields
    let term = value_to_term(
        search_field.field(),
        value,
        field_type,
        field.path().as_deref(),
        is_datetime,
    )?;

    Ok(Box::new(TermQuery::new(term, record_option.into())))
}

fn regex_phrase(
    field: &FieldName,
    schema: &SearchIndexSchema,
    regexes: Vec<String>,
    slop: Option<u32>,
    max_expansions: Option<u32>,
) -> anyhow::Result<Box<dyn TantivyQuery>> {
    let search_field = schema
        .search_field(field.root())
        .ok_or(QueryError::NonIndexedField(field.clone()))?;
    let mut query = RegexPhraseQuery::new(search_field.field(), regexes);

    if let Some(slop) = slop {
        query.set_slop(slop)
    }
    if let Some(max_expansions) = max_expansions {
        query.set_max_expansions(max_expansions)
    }
    Ok(Box::new(query))
}

fn regex(
    field: &FieldName,
    schema: &SearchIndexSchema,
    pattern: &str,
) -> anyhow::Result<Box<dyn TantivyQuery>> {
    let search_field = schema
        .search_field(field.root())
        .ok_or(QueryError::NonIndexedField(field.clone()))?;

    Ok(Box::new(
        RegexQuery::from_pattern(pattern, search_field.field())
            .map_err(|err| QueryError::RegexError(err, pattern.to_string()))?,
    ))
}

/// Creates multi-type range query for JSON numeric fields: generates I64/U64/F64 RangeQuery variants combined with OR logic.
/// Used for BETWEEN, >, <, >=, <= operators on JSON numeric fields to handle type ambiguity (values stored as I64/F64/U64).
/// Helper functions determine_types_for_range() selects types, convert_bound_to_type() converts bounds per type.
fn create_json_numeric_range_query(
    _field_name: &FieldName,
    tantivy_field: Field,
    field_type: &FieldType,
    lower_bound: Bound<OwnedValue>,
    upper_bound: Bound<OwnedValue>,
    path: Option<&str>,
) -> anyhow::Result<Box<dyn TantivyQuery>> {
    // Collect all type-specific range queries
    let mut range_queries: Vec<(Occur, Box<dyn TantivyQuery>)> = Vec::new();

    // Determine which types to generate based on the bound values
    let types_to_generate = determine_types_for_range(&lower_bound, &upper_bound);

    // Generate a RangeQuery for each applicable type
    for value_type in types_to_generate {
        // Try to convert bounds to this type. If conversion fails (e.g., overflow),
        // skip this type variant rather than failing the entire query.
        let lower_term_result =
            convert_bound_to_type(&lower_bound, value_type, tantivy_field, field_type, path);
        let upper_term_result =
            convert_bound_to_type(&upper_bound, value_type, tantivy_field, field_type, path);

        if let (Ok(lower_term), Ok(upper_term)) = (lower_term_result, upper_term_result) {
            // Only add if the bounds are valid (not empty range)
            if !is_empty_range(&lower_term, &upper_term) {
                // Try to create the range query. This can fail with arithmetic overflow
                // for edge cases like Excluded(u64::MAX) which internally tries u64::MAX + 1.
                // Use catch_unwind to handle panics from Tantivy's range normalization.
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    RangeQuery::new(lower_term.clone(), upper_term.clone())
                })) {
                    Ok(range_query) => {
                        range_queries.push((Occur::Should, Box::new(range_query)));
                    }
                    Err(_) => {
                        // Range construction panicked (likely overflow), skip this variant
                    }
                }
            }
        }
        // If conversion failed, silently skip this type variant
    }

    // If no valid range queries could be generated, return an empty query
    if range_queries.is_empty() {
        return Ok(Box::new(EmptyQuery));
    }

    // If only one variant, return it directly (optimization)
    if range_queries.len() == 1 {
        return Ok(range_queries.into_iter().next().unwrap().1);
    }

    // Multiple variants: wrap in BooleanQuery with OR logic (should)
    Ok(Box::new(BooleanQuery::new(range_queries)))
}

/// Determines which numeric types (I64, F64, U64) should be used for range query expansion
/// based on the values in the range bounds.
fn determine_types_for_range(
    lower_bound: &Bound<OwnedValue>,
    upper_bound: &Bound<OwnedValue>,
) -> Vec<NumericType> {
    let mut types = Vec::new();

    // Extract values from bounds to analyze
    let values: Vec<&OwnedValue> = [lower_bound, upper_bound]
        .iter()
        .filter_map(|bound| match bound {
            Bound::Included(v) | Bound::Excluded(v) => Some(v),
            Bound::Unbounded => None,
        })
        .collect();

    if values.is_empty() {
        // Unbounded range, generate all types
        return vec![NumericType::I64, NumericType::F64, NumericType::U64];
    }

    // Analyze each value to determine which types it could be
    let mut needs_i64 = false;
    let mut needs_f64 = false;
    let mut needs_u64 = false;

    for value in &values {
        match value {
            OwnedValue::I64(i64_val) => {
                needs_i64 = true;
                // Only generate F64 if not at boundary values (i64::MAX, i64::MIN)
                // to avoid precision/overflow issues
                if *i64_val != i64::MAX && *i64_val != i64::MIN {
                    needs_f64 = true; // I64 values can also match F64 representation
                }
                if *i64_val >= 0 {
                    needs_u64 = true; // Non-negative I64 can also match U64
                }
            }
            OwnedValue::U64(u64_val) => {
                needs_u64 = true;
                // Only generate F64 if within safe range
                // Values > F64_SAFE_INTEGER_MAX can't be safely round-tripped through F64
                if *u64_val <= F64_SAFE_INTEGER_MAX {
                    needs_f64 = true; // Within F64 safe range
                }
                // Only generate I64 if within range AND not at max boundary
                // i64::MAX conversion through F64 can cause issues
                if *u64_val < i64::MAX as u64 {
                    needs_i64 = true; // Within I64 range
                } else if *u64_val == i64::MAX as u64 {
                    // At exact i64::MAX boundary, include I64 but not F64
                    needs_i64 = true;
                }
            }
            OwnedValue::F64(f64_val) => {
                needs_f64 = true;
                if !f64_val.is_finite() {
                    // NaN, Infinity: only F64
                    return vec![NumericType::F64];
                }
                if f64_val.fract() == 0.0 {
                    // Whole number: could match integer types
                    // Avoid boundaries where F64<->I64/U64 conversion can overflow
                    if *f64_val > i64::MIN as f64 && *f64_val < i64::MAX as f64 {
                        needs_i64 = true;
                    }
                    // For U64, be conservative: u64::MAX as f64 can round to a value > u64::MAX
                    // Use a safe upper bound that's definitely within range
                    if *f64_val >= 0.0 && *f64_val < (u64::MAX as f64) {
                        needs_u64 = true;
                    }
                }
            }
            _ => continue,
        }
    }

    if needs_i64 {
        types.push(NumericType::I64);
    }
    if needs_f64 {
        types.push(NumericType::F64);
    }
    if needs_u64 {
        types.push(NumericType::U64);
    }

    types
}

/// Numeric type enumeration for multi-type expansion
#[derive(Copy, Clone)]
enum NumericType {
    I64,
    F64,
    U64,
}

/// Converts a bound value to the specified numeric type and returns it as a Term bound.
fn convert_bound_to_type(
    bound: &Bound<OwnedValue>,
    target_type: NumericType,
    tantivy_field: Field,
    _field_type: &FieldType,
    path: Option<&str>,
) -> anyhow::Result<Bound<Term>> {
    match bound {
        Bound::Included(value) => {
            let converted_value = convert_value_to_type(value, target_type)?;
            let term = value_to_json_term(tantivy_field, &converted_value, path, true, false)?;
            Ok(Bound::Included(term))
        }
        Bound::Excluded(value) => {
            let converted_value = convert_value_to_type(value, target_type)?;
            let term = value_to_json_term(tantivy_field, &converted_value, path, true, false)?;
            Ok(Bound::Excluded(term))
        }
        Bound::Unbounded => Ok(Bound::Unbounded),
    }
}

/// Converts an OwnedValue to the specified numeric type.
fn convert_value_to_type(
    value: &OwnedValue,
    target_type: NumericType,
) -> anyhow::Result<OwnedValue> {
    Ok(match target_type {
        NumericType::I64 => match value {
            OwnedValue::I64(v) => OwnedValue::I64(*v),
            OwnedValue::U64(v) => {
                if *v <= i64::MAX as u64 {
                    OwnedValue::I64(*v as i64)
                } else {
                    // Value too large for I64, this variant will be skipped
                    return Err(anyhow::anyhow!("Value too large for I64"));
                }
            }
            OwnedValue::F64(v) => {
                if v.fract() == 0.0 && *v >= i64::MIN as f64 && *v <= i64::MAX as f64 {
                    OwnedValue::I64(*v as i64)
                } else {
                    return Err(anyhow::anyhow!("F64 value not convertible to I64"));
                }
            }
            _ => return Err(anyhow::anyhow!("Unsupported value type")),
        },
        NumericType::F64 => match value {
            OwnedValue::I64(v) => OwnedValue::F64(*v as f64),
            OwnedValue::U64(v) => OwnedValue::F64(*v as f64),
            OwnedValue::F64(v) => OwnedValue::F64(*v),
            _ => return Err(anyhow::anyhow!("Unsupported value type")),
        },
        NumericType::U64 => match value {
            OwnedValue::I64(v) => {
                if *v >= 0 {
                    OwnedValue::U64(*v as u64)
                } else {
                    // Negative value cannot be U64
                    return Err(anyhow::anyhow!("Negative value cannot be U64"));
                }
            }
            OwnedValue::U64(v) => OwnedValue::U64(*v),
            OwnedValue::F64(v) => {
                if v.fract() == 0.0 && *v >= 0.0 && *v <= u64::MAX as f64 {
                    OwnedValue::U64(*v as u64)
                } else {
                    return Err(anyhow::anyhow!("F64 value not convertible to U64"));
                }
            }
            _ => return Err(anyhow::anyhow!("Unsupported value type")),
        },
    })
}

/// Checks if a range is empty (lower >= upper for included bounds).
fn is_empty_range(lower: &Bound<Term>, upper: &Bound<Term>) -> bool {
    match (lower, upper) {
        (Bound::Included(l), Bound::Excluded(u)) => l >= u,
        (Bound::Excluded(l), Bound::Included(u)) => l >= u,
        (Bound::Excluded(l), Bound::Excluded(u)) => l >= u,
        _ => false,
    }
}

fn range_within(
    field: &FieldName,
    schema: &SearchIndexSchema,
    lower_bound: Bound<OwnedValue>,
    upper_bound: Bound<OwnedValue>,
    is_datetime: bool,
) -> anyhow::Result<Box<dyn TantivyQuery>> {
    let search_field = schema
        .search_field(field.root())
        .ok_or(QueryError::NonIndexedField(field.clone()))?;
    let field_type = search_field.field_entry().field_type();
    let typeoid = search_field.field_type().typeoid();
    let is_datetime = search_field.is_datetime() || is_datetime;
    let (lower_bound, upper_bound) = check_range_bounds(typeoid, lower_bound, upper_bound)?;

    // For JSON numeric fields, create multi-type range query to handle I64/F64/U64 matching
    let is_json_field = search_field.is_json();
    let has_nested_path = field.path().is_some();
    let is_json_numeric_field = is_json_field && has_nested_path;

    let has_numeric_bounds = [&lower_bound, &upper_bound]
        .iter()
        .any(|bound| match bound {
            Bound::Included(v) | Bound::Excluded(v) => {
                matches!(
                    v,
                    OwnedValue::I64(_) | OwnedValue::U64(_) | OwnedValue::F64(_)
                )
            }
            Bound::Unbounded => false,
        });

    if is_json_numeric_field && has_numeric_bounds && !is_datetime {
        return create_json_numeric_range_query(
            field,
            search_field.field(),
            field_type,
            lower_bound,
            upper_bound,
            field.path().as_deref(),
        );
    }

    let range_field = RangeField::new(search_field.field(), is_datetime);

    let mut satisfies_lower_bound: Vec<(Occur, Box<dyn TantivyQuery>)> = vec![];
    let mut satisfies_upper_bound: Vec<(Occur, Box<dyn TantivyQuery>)> = vec![];

    match lower_bound {
        Bound::Excluded(ref lower) => {
            satisfies_lower_bound.push((
                Occur::Must,
                Box::new(BooleanQuery::new(vec![(
                    Occur::Must,
                    Box::new(
                        range_field.compare_lower_bound(lower, Comparison::GreaterThanOrEqual)?,
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
                            Box::new(
                                range_field.compare_lower_bound(lower, Comparison::GreaterThan)?,
                            ),
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
                            Box::new(
                                range_field
                                    .compare_lower_bound(lower, Comparison::GreaterThanOrEqual)?,
                            ),
                        ),
                        (
                            Occur::Must,
                            Box::new(range_field.lower_bound_inclusive(true)?),
                        ),
                    ])),
                ),
            ]))),
        )),
        _ => {}
    }

    match upper_bound {
        Bound::Excluded(ref upper) => {
            satisfies_upper_bound.push((
                Occur::Must,
                Box::new(BooleanQuery::new(vec![(
                    Occur::Must,
                    Box::new(range_field.compare_upper_bound(upper, Comparison::LessThanOrEqual)?),
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
                            Box::new(range_field.compare_upper_bound(upper, Comparison::LessThan)?),
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
                            Box::new(
                                range_field
                                    .compare_upper_bound(upper, Comparison::LessThanOrEqual)?,
                            ),
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

    Ok(if is_empty {
        Box::new(range_field.exists()?)
    } else {
        Box::new(BooleanQuery::new(vec![
            (Occur::Must, Box::new(satisfies_lower_bound)),
            (Occur::Must, Box::new(satisfies_upper_bound)),
        ]))
    })
}

fn range_term(
    field: &FieldName,
    schema: &SearchIndexSchema,
    value: &OwnedValue,
    is_datetime: bool,
) -> anyhow::Result<Box<dyn TantivyQuery>> {
    let search_field = schema
        .search_field(field.root())
        .ok_or(QueryError::NonIndexedField(field.clone()))?;
    let range_field = RangeField::new(search_field.field(), is_datetime);

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
                            Box::new(
                                range_field
                                    .compare_lower_bound(value, Comparison::GreaterThanOrEqual)?,
                            ),
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
                            Box::new(
                                range_field.compare_lower_bound(value, Comparison::GreaterThan)?,
                            ),
                        ),
                    ])),
                ),
            ])),
        ),
    ]);

    let satisfies_upper_bound = BooleanQuery::new(vec![
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
                            Box::new(
                                range_field
                                    .compare_upper_bound(value, Comparison::LessThanOrEqual)?,
                            ),
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
                            Box::new(range_field.compare_upper_bound(value, Comparison::LessThan)?),
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

fn range_intersects(
    field: &FieldName,
    schema: &SearchIndexSchema,
    lower_bound: Bound<OwnedValue>,
    upper_bound: Bound<OwnedValue>,
    is_datetime: bool,
) -> anyhow::Result<Box<dyn TantivyQuery>> {
    let search_field = schema
        .search_field(field.root())
        .ok_or(QueryError::NonIndexedField(field.clone()))?;
    let typeoid = search_field.field_type().typeoid();
    let is_datetime = search_field.is_datetime() || is_datetime;

    let (lower_bound, upper_bound) = check_range_bounds(typeoid, lower_bound, upper_bound)?;
    let range_field = RangeField::new(search_field.field(), is_datetime);

    let mut satisfies_lower_bound: Vec<(Occur, Box<dyn TantivyQuery>)> = vec![];
    let mut satisfies_upper_bound: Vec<(Occur, Box<dyn TantivyQuery>)> = vec![];

    match lower_bound {
        Bound::Excluded(ref lower) => {
            satisfies_lower_bound.push((
                Occur::Must,
                Box::new(BooleanQuery::new(vec![(
                    Occur::Must,
                    Box::new(range_field.compare_upper_bound(lower, Comparison::LessThan)?),
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
                            Box::new(
                                range_field
                                    .compare_upper_bound(lower, Comparison::LessThanOrEqual)?,
                            ),
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
                            Box::new(range_field.compare_upper_bound(lower, Comparison::LessThan)?),
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
                    Box::new(range_field.compare_lower_bound(upper, Comparison::GreaterThan)?),
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
                                    .compare_lower_bound(upper, Comparison::GreaterThanOrEqual)?,
                            ),
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
                            Box::new(
                                range_field.compare_lower_bound(upper, Comparison::GreaterThan)?,
                            ),
                        ),
                        (
                            Occur::Must,
                            Box::new(range_field.lower_bound_inclusive(false)?),
                        ),
                    ])),
                ),
            ]))),
        )),
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

    Ok(if is_empty {
        Box::new(EmptyQuery)
    } else {
        Box::new(BooleanQuery::new(vec![
            (Occur::Must, Box::new(satisfies_lower_bound)),
            (Occur::Must, Box::new(satisfies_upper_bound)),
            (Occur::Must, Box::new(range_field.empty(false)?)),
        ]))
    })
}

fn range_contains(
    field: &FieldName,
    schema: &SearchIndexSchema,
    lower_bound: Bound<OwnedValue>,
    upper_bound: Bound<OwnedValue>,
    is_datetime: bool,
) -> anyhow::Result<Box<dyn TantivyQuery>> {
    let search_field = schema
        .search_field(field.root())
        .ok_or(QueryError::NonIndexedField(field.clone()))?;
    let typeoid = search_field.field_type().typeoid();
    let is_datetime = search_field.is_datetime() || is_datetime;
    let (lower_bound, upper_bound) = check_range_bounds(typeoid, lower_bound, upper_bound)?;
    let range_field = RangeField::new(search_field.field(), is_datetime);

    let mut satisfies_lower_bound: Vec<(Occur, Box<dyn TantivyQuery>)> = vec![];
    let mut satisfies_upper_bound: Vec<(Occur, Box<dyn TantivyQuery>)> = vec![];

    match lower_bound {
        Bound::Included(lower) => {
            satisfies_lower_bound.push((
                Occur::Must,
                Box::new(BooleanQuery::new(vec![(
                    Occur::Must,
                    Box::new(range_field.compare_lower_bound(&lower, Comparison::LessThanOrEqual)?),
                )])),
            ));
        }
        Bound::Excluded(lower) => satisfies_lower_bound.push((
            Occur::Must,
            (Box::new(BooleanQuery::new(vec![
                (
                    Occur::Should,
                    Box::new(BooleanQuery::new(vec![
                        (
                            Occur::Must,
                            Box::new(
                                range_field.compare_lower_bound(&lower, Comparison::LessThan)?,
                            ),
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
                            Box::new(
                                range_field
                                    .compare_lower_bound(&lower, Comparison::LessThanOrEqual)?,
                            ),
                        ),
                        (
                            Occur::Must,
                            Box::new(range_field.lower_bound_inclusive(false)?),
                        ),
                    ])),
                ),
            ]))),
        )),
        _ => satisfies_lower_bound.push((Occur::Should, Box::new(range_field.exists()?))),
    }

    match upper_bound {
        Bound::Included(upper) => {
            satisfies_upper_bound.push((
                Occur::Must,
                Box::new(BooleanQuery::new(vec![(
                    Occur::Must,
                    Box::new(
                        range_field.compare_upper_bound(&upper, Comparison::GreaterThanOrEqual)?,
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
                            Box::new(
                                range_field.compare_upper_bound(&upper, Comparison::GreaterThan)?,
                            ),
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
                                    .compare_upper_bound(&upper, Comparison::GreaterThanOrEqual)?,
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
        _ => satisfies_upper_bound.push((Occur::Should, Box::new(range_field.exists()?))),
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

fn range(
    field: &FieldName,
    schema: &SearchIndexSchema,
    lower_bound: Bound<OwnedValue>,
    upper_bound: Bound<OwnedValue>,
    is_datetime: bool,
) -> anyhow::Result<Box<dyn TantivyQuery>> {
    let search_field = schema
        .search_field(field.root())
        .ok_or(QueryError::NonIndexedField(field.clone()))?;
    let field_type = search_field.field_entry().field_type();
    let typeoid = search_field.field_type().typeoid();
    let is_datetime = search_field.is_datetime() || is_datetime;

    let lower_bound = coerce_bound_to_field_type(lower_bound, field_type);
    let upper_bound = coerce_bound_to_field_type(upper_bound, field_type);
    let (lower_bound, upper_bound) = check_range_bounds(typeoid, lower_bound, upper_bound)?;

    // For JSON numeric fields, create multi-type range query to handle I64/F64/U64 matching
    // Check if the root field is JSON AND if we have a nested path (indicating JSON field access)
    let is_json_field = search_field.is_json();
    let has_nested_path = field.path().is_some(); // If path exists, we're accessing a nested field
    let is_json_numeric_field = is_json_field && has_nested_path;

    let has_numeric_bounds = [&lower_bound, &upper_bound]
        .iter()
        .any(|bound| match bound {
            Bound::Included(v) | Bound::Excluded(v) => {
                matches!(
                    v,
                    OwnedValue::I64(_) | OwnedValue::U64(_) | OwnedValue::F64(_)
                )
            }
            Bound::Unbounded => false,
        });

    if is_json_numeric_field && has_numeric_bounds && !is_datetime {
        return create_json_numeric_range_query(
            field,
            search_field.field(),
            field_type,
            lower_bound,
            upper_bound,
            field.path().as_deref(),
        );
    }

    // Standard single-type range query for non-JSON fields or non-numeric values
    let lower_bound = match lower_bound {
        Bound::Included(value) => Bound::Included(value_to_term(
            search_field.field(),
            &value,
            field_type,
            field.path().as_deref(),
            is_datetime,
        )?),
        Bound::Excluded(value) => Bound::Excluded(value_to_term(
            search_field.field(),
            &value,
            field_type,
            field.path().as_deref(),
            is_datetime,
        )?),
        Bound::Unbounded => Bound::Unbounded,
    };

    let upper_bound = match upper_bound {
        Bound::Included(value) => Bound::Included(value_to_term(
            search_field.field(),
            &value,
            field_type,
            field.path().as_deref(),
            is_datetime,
        )?),
        Bound::Excluded(value) => Bound::Excluded(value_to_term(
            search_field.field(),
            &value,
            field_type,
            field.path().as_deref(),
            is_datetime,
        )?),
        Bound::Unbounded => Bound::Unbounded,
    };

    Ok(Box::new(RangeQuery::new(lower_bound, upper_bound)))
}

fn tokenized_phrase(
    field: &FieldName,
    schema: &SearchIndexSchema,
    searcher: &Searcher,
    phrase: &str,
    slop: Option<u32>,
) -> Box<dyn TantivyQuery> {
    let tantivy_field = schema
        .search_field(field)
        .unwrap_or_else(|| core::panic!("Field `{field}` not found in tantivy schema"))
        .field();
    let mut tokenizer = searcher
        .index()
        .tokenizer_for_field(tantivy_field)
        .unwrap_or_else(|e| core::panic!("{e}"));
    let mut stream = tokenizer.token_stream(phrase);

    let mut tokens = Vec::new();
    while let Some(token) = stream.next() {
        tokens.push(Term::from_field_text(tantivy_field, &token.text));
    }
    if tokens.is_empty() {
        Box::new(EmptyQuery)
    } else if tokens.len() == 1 {
        let query = TermQuery::new(tokens.remove(0), IndexRecordOption::WithFreqs.into());
        Box::new(query)
    } else {
        let mut query = PhraseQuery::new(tokens);
        query.set_slop(slop.unwrap_or(0));
        Box::new(query)
    }
}

fn phrase_prefix(
    field: &FieldName,
    schema: &SearchIndexSchema,
    phrases: Vec<String>,
    max_expansions: Option<u32>,
) -> anyhow::Result<Box<dyn TantivyQuery>> {
    let search_field = schema
        .search_field(field.root())
        .ok_or(QueryError::NonIndexedField(field.clone()))?;
    let field_type = search_field.field_entry().field_type();
    let terms = phrases.clone().into_iter().map(|phrase| {
        value_to_term(
            search_field.field(),
            &OwnedValue::Str(phrase),
            field_type,
            field.path().as_deref(),
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

fn phrase(
    field: &FieldName,
    schema: &SearchIndexSchema,
    searcher: &Searcher,
    phrases: Vec<String>,
    slop: Option<u32>,
) -> anyhow::Result<Box<dyn TantivyQuery>> {
    let search_field = schema
        .search_field(field.root())
        .ok_or(QueryError::NonIndexedField(field.clone()))?;
    let field_type = search_field.field_entry().field_type();

    let mut terms = Vec::new();
    let mut analyzer = searcher.index().tokenizer_for_field(search_field.field())?;
    let mut should_warn = false;

    for phrase in phrases.into_iter() {
        let mut stream = analyzer.token_stream(&phrase);
        let len_before = terms.len();

        while stream.advance() {
            let token = stream.token().text.clone();
            let term = value_to_term(
                search_field.field(),
                &OwnedValue::Str(token),
                field_type,
                field.path().as_deref(),
                false,
            )?;

            terms.push(term);
        }

        if len_before + 1 < terms.len() {
            should_warn = true;
        }
    }

    // When tokenizers produce more than one token per phrase, their position may not
    // correctly represent the original query.
    // For example, NgramTokenizer can produce many tokens per word and all of them will
    // have position=0 which won't be correctly interpreted when processing slop
    if should_warn {
        pgrx::warning!("Phrase query with multiple tokens per phrase may not be correctly interpreted. Consider using a different tokenizer or switch to parse/match");
    }

    let mut query = PhraseQuery::new(terms);
    if let Some(slop) = slop {
        query.set_slop(slop)
    }
    Ok(Box::new(query))
}

fn phrase_array(
    field: &FieldName,
    schema: &SearchIndexSchema,
    mut tokens: Vec<String>,
    slop: Option<u32>,
) -> anyhow::Result<Box<dyn TantivyQuery>> {
    let search_field = schema
        .search_field(field.root())
        .ok_or(QueryError::NonIndexedField(field.clone()))?;
    let field_type = search_field.field_entry().field_type();

    let mut terms = Vec::with_capacity(tokens.len());

    if tokens.len() == 1 {
        let term = value_to_term(
            search_field.field(),
            &OwnedValue::Str(tokens.pop().unwrap()),
            field_type,
            field.path().as_deref(),
            false,
        )?;
        Ok(Box::new(TermQuery::new(
            term,
            IndexRecordOption::WithFreqsAndPositions.into(),
        )))
    } else {
        for token in tokens {
            let term = value_to_term(
                search_field.field(),
                &OwnedValue::Str(token),
                field_type,
                field.path().as_deref(),
                false,
            )?;

            terms.push(term);
        }

        let mut query = PhraseQuery::new(terms);
        if let Some(slop) = slop {
            query.set_slop(slop)
        }
        Ok(Box::new(query))
    }
}

fn parse<QueryParserCtor: Fn() -> QueryParser>(
    parser: &QueryParserCtor,
    query_string: String,
    lenient: Option<bool>,
    conjunction_mode: Option<bool>,
) -> anyhow::Result<Box<dyn TantivyQuery>> {
    let mut parser = parser();
    if let Some(true) = conjunction_mode {
        parser.set_conjunction_by_default();
    }

    let lenient = lenient.unwrap_or(false);
    Ok(if lenient {
        let (parsed_query, _) = parser.parse_query_lenient(&query_string);
        Box::new(parsed_query)
    } else {
        Box::new(
            parser
                .parse_query(&query_string)
                .map_err(|err| QueryError::ParseError(err, query_string))?,
        )
    })
}

fn parse_with_field<QueryParserCtor: Fn() -> QueryParser>(
    field: &FieldName,
    parser: &QueryParserCtor,
    schema: &SearchIndexSchema,
    query_string: String,
    lenient: Option<bool>,
    conjunction_mode: Option<bool>,
    fuzzy_data: Option<FuzzyData>,
) -> anyhow::Result<Box<dyn TantivyQuery>> {
    let mut parser = parser();
    let query_string = format!("{field}:({query_string})");
    if let Some(true) = conjunction_mode {
        parser.set_conjunction_by_default();
    }
    let field = schema
        .search_field(field)
        .ok_or(QueryError::NonIndexedField(field.clone()))?
        .field();

    if let Some(fuzzy_data) = fuzzy_data {
        parser.set_field_fuzzy(
            field,
            fuzzy_data.prefix,
            fuzzy_data.distance,
            fuzzy_data.transposition_cost_one,
        );
    }

    let lenient = lenient.unwrap_or(false);
    Ok(if lenient {
        let (parsed_query, _) = parser.parse_query_lenient(&query_string);
        Box::new(parsed_query)
    } else {
        Box::new(
            parser
                .parse_query(&query_string)
                .map_err(|err| QueryError::ParseError(err, query_string))?,
        )
    })
}

#[allow(clippy::too_many_arguments)]
fn match_query(
    field: &FieldName,
    schema: &SearchIndexSchema,
    searcher: &Searcher,
    value: &str,
    tokenizer: Option<Value>,
    distance: Option<u8>,
    transposition_cost_one: Option<bool>,
    prefix: Option<bool>,
    conjunction_mode: Option<bool>,
) -> anyhow::Result<Box<dyn TantivyQuery>> {
    let distance = distance.unwrap_or(0);
    let transposition_cost_one = transposition_cost_one.unwrap_or(true);
    let conjunction_mode = conjunction_mode.unwrap_or(false);
    let prefix = prefix.unwrap_or(false);

    let search_field = schema
        .search_field(field.root())
        .ok_or(QueryError::NonIndexedField(field.clone()))?;
    let field_type = search_field.field_entry().field_type();
    let mut analyzer = match tokenizer {
        Some(tokenizer) => {
            let tokenizer = SearchTokenizer::from_json_value(&tokenizer)
                .map_err(|_| QueryError::InvalidTokenizer)?;
            tokenizer
                .to_tantivy_tokenizer()
                .ok_or(QueryError::InvalidTokenizer)?
        }
        None => searcher.index().tokenizer_for_field(search_field.field())?,
    };
    let mut stream = analyzer.token_stream(value);
    let mut terms = Vec::new();

    while stream.advance() {
        let token = stream.token().text.clone();
        let term = value_to_term(
            search_field.field(),
            &OwnedValue::Str(token),
            field_type,
            field.path().as_deref(),
            false,
        )?;
        let term_query: Box<dyn TantivyQuery> = match (distance, prefix) {
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
#[allow(clippy::too_many_arguments)]
fn match_array_query(
    field: &FieldName,
    schema: &SearchIndexSchema,
    tokens: Vec<String>,
    distance: Option<u8>,
    transposition_cost_one: Option<bool>,
    prefix: Option<bool>,
    conjunction_mode: Option<bool>,
) -> anyhow::Result<Box<dyn TantivyQuery>> {
    let distance = distance.unwrap_or(0);
    let transposition_cost_one = transposition_cost_one.unwrap_or(true);
    let conjunction_mode = conjunction_mode.unwrap_or(false);
    let prefix = prefix.unwrap_or(false);

    let search_field = schema
        .search_field(field.root())
        .ok_or(QueryError::NonIndexedField(field.clone()))?;
    let field_type = search_field.field_entry().field_type();
    let mut terms = Vec::with_capacity(tokens.len());

    for token in tokens {
        let term = value_to_term(
            search_field.field(),
            &OwnedValue::Str(token),
            field_type,
            field.path().as_deref(),
            false,
        )?;
        let term_query: Box<dyn TantivyQuery> = match (distance, prefix) {
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

fn fuzzy_term(
    field: &FieldName,
    schema: &SearchIndexSchema,
    value: String,
    distance: Option<u8>,
    transposition_cost_one: Option<bool>,
    prefix: Option<bool>,
) -> anyhow::Result<Box<dyn TantivyQuery>> {
    let search_field = schema
        .search_field(field.root())
        .ok_or(QueryError::NonIndexedField(field.clone()))?;
    let field_type = search_field.field_entry().field_type();
    let term = value_to_term(
        search_field.field(),
        &OwnedValue::Str(value),
        field_type,
        field.path().as_deref(),
        false,
    )?;
    let distance = distance.unwrap_or(2);
    let transposition_cost_one = transposition_cost_one.unwrap_or(true);
    Ok(if prefix.unwrap_or(false) {
        Box::new(FuzzyTermQuery::new_prefix(
            term,
            distance,
            transposition_cost_one,
        ))
    } else {
        Box::new(FuzzyTermQuery::new(term, distance, transposition_cost_one))
    })
}

fn fast_field_range_weight(
    field: &FieldName,
    schema: &SearchIndexSchema,
    lower_bound: Bound<u64>,
    upper_bound: Bound<u64>,
) -> Box<FastFieldRangeQuery> {
    let field = schema.search_field(field.root()).unwrap().field();
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

    Box::new(FastFieldRangeQuery::new(new_lower_bound, new_upper_bound))
}

fn exists(field: FieldName, searcher: &Searcher) -> Box<ExistsQuery> {
    let schema_field = searcher.schema().get_field(&field.root()).unwrap();
    let is_json = searcher
        .schema()
        .get_field_entry(schema_field)
        .field_type()
        .is_json();
    Box::new(ExistsQuery::new(field.into_inner(), is_json))
}
