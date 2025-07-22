use crate::api::FieldName;
use crate::query::range::{deserialize_bound, serialize_bound, Comparison, RangeField};
use crate::query::{
    check_range_bounds, coerce_bound_to_field_type, value_to_term, QueryError, SearchQueryInput,
};
use crate::schema::{IndexRecordOption, SearchIndexSchema};
use pgrx::{pg_extern, PostgresType};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::Bound;
use std::error::Error;
use tantivy::query::{
    BooleanQuery, EmptyQuery, ExistsQuery, FastFieldRangeQuery, FuzzyTermQuery, Occur,
    PhrasePrefixQuery, PhraseQuery, Query, QueryParser, RangeQuery, RegexPhraseQuery, RegexQuery,
    TermQuery,
};
use tantivy::schema::OwnedValue;
use tantivy::{Searcher, Term};
use tokenizers::SearchTokenizer;

#[pg_extern(immutable, parallel_safe)]
pub fn to_search_query_input(field: FieldName, query: FieldedQueryInput) -> SearchQueryInput {
    SearchQueryInput::FieldedQuery { field, query }
}

#[derive(Debug, PostgresType, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FieldedQueryInput {
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
    Parse {
        query_string: String,
        lenient: Option<bool>,
        conjunction_mode: Option<bool>,
    },
    Phrase {
        phrases: Vec<String>,
        slop: Option<u32>,
    },
    PhrasePrefix {
        phrases: Vec<String>,
        max_expansions: Option<u32>,
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
}

impl FieldedQueryInput {
    pub fn into_tantivy_query(
        self,
        field: FieldName,
        schema: &SearchIndexSchema,
        parser: &mut QueryParser,
        searcher: &Searcher,
    ) -> anyhow::Result<Box<dyn Query>, Box<dyn Error>> {
        let query: Box<dyn Query> = match self {
            FieldedQueryInput::Exists => exists(field, searcher),
            FieldedQueryInput::FastFieldRangeWeight {
                lower_bound,
                upper_bound,
            } => fast_field_range_weight(&field, schema, lower_bound, upper_bound),
            FieldedQueryInput::FuzzyTerm {
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
            FieldedQueryInput::Match {
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
            FieldedQueryInput::Parse {
                query_string,
                lenient,
                conjunction_mode,
            } => parse(&field, parser, query_string, lenient, conjunction_mode)?,

            FieldedQueryInput::Phrase { phrases, slop } => {
                phrase(&field, schema, searcher, phrases, slop)?
            }

            FieldedQueryInput::PhrasePrefix {
                phrases,
                max_expansions,
            } => phrase_prefix(&field, schema, phrases, max_expansions)?,
            FieldedQueryInput::TokenizedPhrase { phrase, slop } => {
                tokenized_phrase(&field, schema, searcher, &phrase, slop)
            }
            FieldedQueryInput::Range {
                lower_bound,
                upper_bound,
                is_datetime,
            } => range(&field, schema, lower_bound, upper_bound, is_datetime)?,
            FieldedQueryInput::RangeContains {
                lower_bound,
                upper_bound,
                is_datetime,
            } => range_contains(&field, schema, lower_bound, upper_bound, is_datetime)?,
            FieldedQueryInput::RangeIntersects {
                lower_bound,
                upper_bound,
                is_datetime,
            } => range_intersects(&field, schema, lower_bound, upper_bound, is_datetime)?,
            FieldedQueryInput::RangeTerm { value, is_datetime } => {
                range_term(&field, schema, &value, is_datetime)?
            }
            FieldedQueryInput::RangeWithin {
                lower_bound,
                upper_bound,
                is_datetime,
            } => range_within(&field, schema, lower_bound, upper_bound, is_datetime)?,
            FieldedQueryInput::Regex { pattern } => regex(&field, schema, &pattern)?,
            FieldedQueryInput::RegexPhrase {
                regexes,
                slop,
                max_expansions,
            } => regex_phrase(&field, schema, regexes, slop, max_expansions)?,
            FieldedQueryInput::Term { value, is_datetime } => {
                term(field, schema, &value, is_datetime)?
            }
        };

        Ok(query)
    }
}

fn term(
    field: FieldName,
    schema: &SearchIndexSchema,
    value: &OwnedValue,
    is_datetime: bool,
) -> Result<Box<dyn Query>, Box<dyn Error>> {
    let record_option = IndexRecordOption::WithFreqsAndPositions;
    let search_field = schema
        .search_field(field.root())
        .ok_or(QueryError::NonIndexedField(field.clone()))?;
    let field_type = search_field.field_entry().field_type();
    let is_datetime = search_field.is_datetime() || is_datetime;
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
) -> Result<Box<RegexPhraseQuery>, Box<dyn Error>> {
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
    pattern: &String,
) -> Result<Box<RegexQuery>, Box<dyn Error>> {
    let search_field = schema
        .search_field(field.root())
        .ok_or(QueryError::NonIndexedField(field.clone()))?;

    Ok(Box::new(
        RegexQuery::from_pattern(&pattern, search_field.field())
            .map_err(|err| QueryError::RegexError(err, pattern.clone()))?,
    ))
}

fn range_within(
    field: &FieldName,
    schema: &SearchIndexSchema,
    lower_bound: Bound<OwnedValue>,
    upper_bound: Bound<OwnedValue>,
    is_datetime: bool,
) -> Result<Box<dyn Query>, Box<dyn Error>> {
    let search_field = schema
        .search_field(field.root())
        .ok_or(QueryError::NonIndexedField(field.clone()))?;
    let typeoid = search_field.field_type().typeoid();
    let is_datetime = search_field.is_datetime() || is_datetime;
    let (lower_bound, upper_bound) = check_range_bounds(typeoid, lower_bound, upper_bound)?;

    let range_field = RangeField::new(search_field.field(), is_datetime);

    let mut satisfies_lower_bound: Vec<(Occur, Box<dyn Query>)> = vec![];
    let mut satisfies_upper_bound: Vec<(Occur, Box<dyn Query>)> = vec![];

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
) -> Result<Box<BooleanQuery>, Box<dyn Error>> {
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
) -> Result<Box<dyn Query>, Box<dyn Error>> {
    let search_field = schema
        .search_field(field.root())
        .ok_or(QueryError::NonIndexedField(field.clone()))?;
    let typeoid = search_field.field_type().typeoid();
    let is_datetime = search_field.is_datetime() || is_datetime;

    let (lower_bound, upper_bound) = check_range_bounds(typeoid, lower_bound, upper_bound)?;
    let range_field = RangeField::new(search_field.field(), is_datetime);

    let mut satisfies_lower_bound: Vec<(Occur, Box<dyn Query>)> = vec![];
    let mut satisfies_upper_bound: Vec<(Occur, Box<dyn Query>)> = vec![];

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
) -> Result<Box<BooleanQuery>, Box<dyn Error>> {
    let search_field = schema
        .search_field(field.root())
        .ok_or(QueryError::NonIndexedField(field.clone()))?;
    let typeoid = search_field.field_type().typeoid();
    let is_datetime = search_field.is_datetime() || is_datetime;
    let (lower_bound, upper_bound) = check_range_bounds(typeoid, lower_bound, upper_bound)?;
    let range_field = RangeField::new(search_field.field(), is_datetime);

    let mut satisfies_lower_bound: Vec<(Occur, Box<dyn Query>)> = vec![];
    let mut satisfies_upper_bound: Vec<(Occur, Box<dyn Query>)> = vec![];

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
) -> Result<Box<RangeQuery>, Box<dyn Error>> {
    let search_field = schema
        .search_field(field.root())
        .ok_or(QueryError::NonIndexedField(field.clone()))?;
    let field_type = search_field.field_entry().field_type();
    let typeoid = search_field.field_type().typeoid();
    let is_datetime = search_field.is_datetime() || is_datetime;

    let lower_bound = coerce_bound_to_field_type(lower_bound, field_type);
    let upper_bound = coerce_bound_to_field_type(upper_bound, field_type);
    let (lower_bound, upper_bound) = check_range_bounds(typeoid, lower_bound, upper_bound)?;

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
    phrase: &String,
    slop: Option<u32>,
) -> Box<dyn Query> {
    let tantivy_field = schema
        .search_field(&field)
        .unwrap_or_else(|| core::panic!("Field `{field}` not found in tantivy schema"))
        .field();
    let mut tokenizer = searcher
        .index()
        .tokenizer_for_field(tantivy_field)
        .unwrap_or_else(|e| core::panic!("{e}"));
    let mut stream = tokenizer.token_stream(&phrase);

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
) -> Result<Box<PhrasePrefixQuery>, Box<dyn Error>> {
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
) -> Result<Box<PhraseQuery>, Box<dyn Error>> {
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

    // When tokeniser produce more than one token per phrase, their position may not
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

fn parse(
    field: &FieldName,
    parser: &mut QueryParser,
    query_string: String,
    lenient: Option<bool>,
    conjunction_mode: Option<bool>,
) -> Result<Box<dyn Query>, Box<dyn Error>> {
    let query_string = format!("{field}:({query_string})");
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

fn match_query(
    field: &FieldName,
    schema: &SearchIndexSchema,
    searcher: &Searcher,
    value: &String,
    tokenizer: Option<Value>,
    distance: Option<u8>,
    transposition_cost_one: Option<bool>,
    prefix: Option<bool>,
    conjunction_mode: Option<bool>,
) -> Result<Box<BooleanQuery>, Box<dyn Error>> {
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
    let mut stream = analyzer.token_stream(&value);
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

fn fuzzy_term(
    field: &FieldName,
    schema: &SearchIndexSchema,
    value: String,
    distance: Option<u8>,
    transposition_cost_one: Option<bool>,
    prefix: Option<bool>,
) -> Result<Box<dyn Query>, Box<dyn Error>> {
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
