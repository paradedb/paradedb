// Copyright (c) 2023-2026 ParadeDB, Inc.
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
use crate::query::numeric::{
    convert_value_for_field, convert_value_for_range_field, map_bound, numeric_bound_to_bytes,
    scale_numeric_bound, string_to_f64, string_to_i64, string_to_json_numeric, string_to_u64,
};
use crate::query::pdb_query::pdb::{FuzzyData, ScoreAdjustStyle, SlopData};
use crate::query::proximity::query::ProximityQuery;
use crate::query::proximity::{ProximityClause, ProximityDistance};
use crate::query::range::{Comparison, RangeField};
use crate::query::{
    check_range_bounds, coerce_bound_to_field_type, value_to_term, QueryError, SearchQueryInput,
};
use crate::schema::{IndexRecordOption, SearchFieldType, SearchIndexSchema};
use pgrx::{pg_extern, pg_schema, InOutFuncs, StringInfo};
use serde_json::Value;
use std::collections::Bound;
use std::ffi::CStr;
use tantivy::query::{
    AllQuery, BooleanQuery, BoostQuery, ConstScoreQuery, EmptyQuery, ExistsQuery,
    FastFieldRangeQuery, FuzzyTermQuery, Occur, PhrasePrefixQuery, PhraseQuery,
    Query as TantivyQuery, Query, QueryParser, RangeQuery, RegexPhraseQuery, RegexQuery, TermQuery,
    TermSetQuery,
};
use tantivy::schema::OwnedValue;
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
                tokenized_phrase(&field, schema, searcher, &phrase, slop)?
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

    /// Returns `true` if constructing a Tantivy Scorer for this query type is expensive.
    /// Fuzzy term and regex queries require building DFAs/automata and scanning the term
    /// dictionary during scorer construction, which can be too costly for planner selectivity
    /// estimation.
    ///
    /// Range queries and Match-with-distance are intentionally excluded: range queries on
    /// numeric fast fields are cheap to score, and their selectivity is highly data-dependent
    /// making heuristics unreliable. Inaccurate heuristics for these types cause plan
    /// regressions (e.g. wrong join strategies, wrong append methods).
    pub fn is_expensive_to_estimate(&self) -> bool {
        match self {
            pdb::Query::FuzzyTerm { .. }
            | pdb::Query::Regex { .. }
            | pdb::Query::RegexPhrase { .. } => true,

            pdb::Query::ParseWithField { fuzzy_data, .. } => fuzzy_data.is_some(),

            pdb::Query::ScoreAdjusted { query, .. } => query.is_expensive_to_estimate(),

            _ => false,
        }
    }

    /// Returns a heuristic selectivity for this query type, avoiding expensive scorer construction.
    pub fn selectivity_heuristic(&self) -> f64 {
        use crate::{FUZZY_HIGH_SELECTIVITY, FUZZY_LOW_SELECTIVITY, REGEX_SELECTIVITY};

        match self {
            pdb::Query::FuzzyTerm { distance, .. } => {
                let dist = distance.unwrap_or(1);
                if dist <= 1 {
                    FUZZY_LOW_SELECTIVITY
                } else {
                    FUZZY_HIGH_SELECTIVITY
                }
            }

            pdb::Query::ParseWithField { .. } => FUZZY_LOW_SELECTIVITY,

            pdb::Query::Regex { .. } | pdb::Query::RegexPhrase { .. } => REGEX_SELECTIVITY,

            pdb::Query::ScoreAdjusted { query, .. } => query.selectivity_heuristic(),

            _ => crate::UNKNOWN_SELECTIVITY,
        }
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
        .ok_or(QueryError::NonIndexedField(field.clone()))?
        .with_positions()?;

    let prox = ProximityQuery::new(search_field.field(), left, distance, right);
    Ok(Box::new(prox))
}

fn term_set(
    field: FieldName,
    schema: &SearchIndexSchema,
    terms: Vec<OwnedValue>,
) -> anyhow::Result<Box<dyn TantivyQuery>> {
    let search_field = schema
        .search_field(&field)
        .expect("field should exist in schema");
    let field_type = search_field.field_entry().field_type();
    let tantivy_field = search_field.field();
    let is_date_time = search_field.is_datetime();
    let search_field_type = search_field.field_type();

    // Convert terms based on field type (uses same logic as term())
    let converted_terms: Vec<OwnedValue> = terms
        .into_iter()
        .map(|term| convert_value_for_field(term, &search_field_type).unwrap_or(OwnedValue::Null))
        .collect();

    Ok(Box::new(TermSetQuery::new(
        converted_terms.into_iter().map(|term| {
            value_to_term(
                tantivy_field,
                &term,
                field_type,
                field.path().as_deref(),
                is_date_time,
            )
            .expect("could not convert argument to search term")
        }),
    )))
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
    let search_field_type = search_field.field_type();

    // Convert value based on field type (handles NUMERIC scaling, JSON types, etc.)
    let value = convert_value_for_field(value.clone(), &search_field_type)?;

    let term = value_to_term(
        search_field.field(),
        &value,
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
        .ok_or(QueryError::NonIndexedField(field.clone()))?
        .with_positions()?;

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

/// Note: For JSON numeric fast field limitations, see documentation on [`range`].
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

    // Convert numeric values to appropriate types based on range field type.
    // Range fields are indexed as JSON with specific element types:
    // - INT4RANGEOID, INT8RANGEOID: indexed as i32/i64 → convert to I64
    // - NUMRANGEOID: indexed as hex-encoded sortable bytes (see SortableDecimal) → convert to hex string
    // - Date/time ranges: handled by is_datetime flag
    let value = convert_value_for_range_field(value.clone(), &search_field.field_type());

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
                                    .compare_lower_bound(&value, Comparison::GreaterThanOrEqual)?,
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
                                range_field.compare_lower_bound(&value, Comparison::GreaterThan)?,
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
                                    .compare_upper_bound(&value, Comparison::LessThanOrEqual)?,
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
                            Box::new(
                                range_field.compare_upper_bound(&value, Comparison::LessThan)?,
                            ),
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

/// Creates a range query for the given field and bounds.
///
/// # JSON Numeric Range Queries and Fast Fields
///
/// For JSON fields, Tantivy requires fast fields for range queries (returns error otherwise).
/// Fast field storage has important limitations for JSON numeric values:
///
/// - Each JSON path gets ONE fast field column with ONE numeric type (I64, U64, or F64)
/// - Column type is determined at index time based on values stored:
///   - All integers that fit in i64 → I64 column
///   - All non-negative integers, some exceeding i64::MAX → U64 column
///   - Any float value OR mix of negative + large positive (≥ 2^63) → F64 column
/// - When column is F64, integers > 2^53 lose precision (e.g., 9007199254740993 → 9007199254740992.0)
///
/// At query time, Tantivy discovers the actual column type and converts query bounds accordingly
/// (see `search_on_json_numerical_field` in tantivy's range_query_fastfield.rs).
///
/// References:
/// - Fast field column type selection: tantivy columnar/src/columnar/writer/column_writers.rs
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
    let search_field_type = search_field.field_type();

    // Handle NUMERIC field types with special storage strategies
    // Also handle string-encoded numeric values for JSON and other numeric fields
    let (lower_bound, upper_bound) = match search_field_type {
        SearchFieldType::Numeric64(_, scale) => {
            // Scale bounds for I64 fixed-point storage
            let lower = scale_numeric_bound(lower_bound, scale)?;
            let upper = scale_numeric_bound(upper_bound, scale)?;
            (lower, upper)
        }
        SearchFieldType::NumericBytes(..) => {
            // Convert bounds to lexicographically sortable bytes
            let lower = numeric_bound_to_bytes(lower_bound)?;
            let upper = numeric_bound_to_bytes(upper_bound)?;
            (lower, upper)
        }
        SearchFieldType::Json(_) => {
            // For JSON fields, convert string numeric values to appropriate JSON types
            let lower = map_bound(lower_bound, string_to_json_numeric);
            let upper = map_bound(upper_bound, string_to_json_numeric);
            check_range_bounds(typeoid, lower, upper)?
        }
        SearchFieldType::I64(_) => {
            let lower = map_bound(lower_bound, string_to_i64);
            let upper = map_bound(upper_bound, string_to_i64);
            check_range_bounds(typeoid, lower, upper)?
        }
        SearchFieldType::U64(_) => {
            let lower = map_bound(lower_bound, string_to_u64);
            let upper = map_bound(upper_bound, string_to_u64);
            check_range_bounds(typeoid, lower, upper)?
        }
        SearchFieldType::F64(_) => {
            let lower = map_bound(lower_bound, string_to_f64);
            let upper = map_bound(upper_bound, string_to_f64);
            check_range_bounds(typeoid, lower, upper)?
        }
        _ => {
            // Standard path for other field types
            let lower_bound = coerce_bound_to_field_type(lower_bound, field_type);
            let upper_bound = coerce_bound_to_field_type(upper_bound, field_type);
            check_range_bounds(typeoid, lower_bound, upper_bound)?
        }
    };

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
) -> anyhow::Result<Box<dyn TantivyQuery>> {
    let search_field = schema
        .search_field(field.root())
        .ok_or(QueryError::NonIndexedField(field.clone()))?
        .with_positions()?;

    let field_type = search_field.field_entry().field_type();
    let mut tokenizer = match search_field.field_config().search_tokenizer() {
        Some(st) => st
            .to_tantivy_tokenizer()
            .expect("search_tokenizer should be a valid tantivy tokenizer"),
        None => searcher.index().tokenizer_for_field(search_field.field())?,
    };
    let mut stream = tokenizer.token_stream(phrase);
    let path = field.path();

    let mut tokens = Vec::new();
    while let Some(token) = stream.next() {
        let value = OwnedValue::Str(token.text.clone());
        let term = value_to_term(
            search_field.field(),
            &value,
            field_type,
            path.as_deref(),
            false,
        )?;
        tokens.push(term);
    }
    Ok(if tokens.is_empty() {
        Box::new(EmptyQuery)
    } else if tokens.len() == 1 {
        Box::new(TermQuery::new(
            tokens.remove(0),
            IndexRecordOption::WithFreqs.into(),
        ))
    } else {
        let mut query = PhraseQuery::new(tokens);
        query.set_slop(slop.unwrap_or(0));
        Box::new(query)
    })
}

fn phrase_prefix(
    field: &FieldName,
    schema: &SearchIndexSchema,
    phrases: Vec<String>,
    max_expansions: Option<u32>,
) -> anyhow::Result<Box<dyn TantivyQuery>> {
    let search_field = schema
        .search_field(field.root())
        .ok_or(QueryError::NonIndexedField(field.clone()))?
        .with_positions()?;

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
        .ok_or(QueryError::NonIndexedField(field.clone()))?
        .with_positions()?;
    let field_type = search_field.field_entry().field_type();

    let mut terms = Vec::new();
    let mut analyzer = match search_field.field_config().search_tokenizer() {
        Some(st) => st
            .to_tantivy_tokenizer()
            .expect("search_tokenizer should be a valid tantivy tokenizer"),
        None => searcher.index().tokenizer_for_field(search_field.field())?,
    };
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
        .ok_or(QueryError::NonIndexedField(field.clone()))?
        .with_positions()?;
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
    let search_field = schema
        .search_field(field)
        .ok_or(QueryError::NonIndexedField(field.clone()))?;
    let field_type = search_field.field_type();

    // Handle Numeric64 and NumericBytes fields specially
    // Tantivy's QueryParser can't parse decimal strings directly for these types
    if matches!(
        field_type,
        SearchFieldType::Numeric64(_, _) | SearchFieldType::NumericBytes(..)
    ) {
        // Convert the query string to the appropriate numeric format
        let value = OwnedValue::Str(query_string.trim().to_string());
        if let Ok(converted) = convert_value_for_field(value, &field_type) {
            let tantivy_field_type = search_field.field_entry().field_type();
            let term = value_to_term(
                search_field.field(),
                &converted,
                tantivy_field_type,
                field.path().as_deref(),
                false,
            )?;
            return Ok(Box::new(TermQuery::new(
                term,
                IndexRecordOption::WithFreqsAndPositions.into(),
            )));
        }
        // If conversion fails, fall through to standard parsing (will likely error)
    }

    let mut parser = parser();
    let query_string = format!("{field}:({query_string})");
    if let Some(true) = conjunction_mode {
        parser.set_conjunction_by_default();
    }
    let tantivy_field = search_field.field();

    if let Some(fuzzy_data) = fuzzy_data {
        parser.set_field_fuzzy(
            tantivy_field,
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
        Some(ref tokenizer) => SearchTokenizer::from_json_value(tokenizer)?
            .to_tantivy_tokenizer()
            .expect("tantivy should support tokenizer {tokenizer:?}"),
        None => match search_field.field_config().search_tokenizer() {
            Some(st) => st
                .to_tantivy_tokenizer()
                .expect("search_tokenizer should be a valid tantivy tokenizer"),
            None => searcher.index().tokenizer_for_field(search_field.field())?,
        },
    };
    let mut stream = analyzer.token_stream(value);
    let mut terms = Vec::new();

    while stream.advance() {
        let token = stream.token().text.clone();
        terms.push(value_to_term(
            search_field.field(),
            &OwnedValue::Str(token),
            field_type,
            field.path().as_deref(),
            false,
        )?);
    }

    // For conjunction mode, duplicate terms produce redundant Must clauses.
    // This can happen with ngram tokenizers on strings with repeated substrings.
    if conjunction_mode {
        let mut seen = crate::api::HashSet::default();
        terms.retain(|term| seen.insert(term.clone()));
    }

    let occur = if conjunction_mode {
        Occur::Must
    } else {
        Occur::Should
    };

    let clauses: Vec<_> = terms
        .into_iter()
        .map(|term| {
            let query: Box<dyn TantivyQuery> = match (distance, prefix) {
                (0, _) => Box::new(TermQuery::new(term, IndexRecordOption::WithFreqs.into())),
                (d, true) => Box::new(FuzzyTermQuery::new_prefix(term, d, transposition_cost_one)),
                (d, false) => Box::new(FuzzyTermQuery::new(term, d, transposition_cost_one)),
            };
            (occur, query)
        })
        .collect();

    Ok(Box::new(BooleanQuery::new(clauses)))
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
