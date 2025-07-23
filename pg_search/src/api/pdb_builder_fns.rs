pub use pdb::*;

#[pgrx::pg_schema]
mod pdb {
    use crate::postgres::types::TantivyValue;
    use crate::query::pdb_query::pdb;
    use crate::schema::AnyEnum;
    use macros::builder_fn;
    use pgrx::datum::RangeBound;
    use pgrx::{default, pg_extern, AnyElement, AnyNumeric, PostgresEnum, Range};
    use serde::Serialize;
    use std::collections::Bound;
    use std::fmt::{Display, Formatter};
    use tantivy::schema::{OwnedValue, Value};

    #[builder_fn]
    #[pg_extern(immutable, parallel_safe)]
    pub fn match_conjunction(terms_to_tokenize: String) -> pdb::Query {
        pdb::Query::Match {
            value: terms_to_tokenize,
            conjunction_mode: Some(true),
            tokenizer: None,
            distance: None,
            transposition_cost_one: None,
            prefix: None,
        }
    }

    #[builder_fn]
    #[pg_extern(immutable, parallel_safe)]
    pub fn match_disjunction(terms_to_tokenize: String) -> pdb::Query {
        pdb::Query::Match {
            value: terms_to_tokenize,
            conjunction_mode: Some(false),
            tokenizer: None,
            distance: None,
            transposition_cost_one: None,
            prefix: None,
        }
    }

    #[builder_fn]
    #[pg_extern(immutable, parallel_safe, name = "phrase")]
    pub fn phrase_string(phrase: String) -> pdb::Query {
        pdb::Query::TokenizedPhrase { phrase, slop: None }
    }

    #[builder_fn]
    #[pg_extern(immutable, parallel_safe, name = "exists")]
    pub fn exists() -> pdb::Query {
        pdb::Query::Exists
    }

    #[builder_fn]
    #[pg_extern(immutable, parallel_safe, name = "fuzzy_term")]
    pub fn fuzzy_term(
        value: default!(Option<String>, "NULL"),
        distance: default!(Option<i32>, "NULL"),
        transposition_cost_one: default!(Option<bool>, "NULL"),
        prefix: default!(Option<bool>, "NULL"),
    ) -> pdb::Query {
        pdb::Query::FuzzyTerm {
            value: value.expect("`value` argument is required"),
            distance: distance.map(|n| n as u8),
            transposition_cost_one,
            prefix,
        }
    }

    #[builder_fn]
    #[pg_extern(immutable, parallel_safe, name = "match")]
    pub fn match_query(
        value: String,
        tokenizer: default!(Option<pgrx::JsonB>, "NULL"),
        distance: default!(Option<i32>, "NULL"),
        transposition_cost_one: default!(Option<bool>, "NULL"),
        prefix: default!(Option<bool>, "NULL"),
        conjunction_mode: default!(Option<bool>, "NULL"),
    ) -> pdb::Query {
        pdb::Query::Match {
            value,
            tokenizer: tokenizer.map(|t| t.0),
            distance: distance.map(|n| n as u8),
            transposition_cost_one,
            prefix,
            conjunction_mode,
        }
    }

    #[builder_fn]
    #[pg_extern(immutable, parallel_safe, name = "parse_with_field")]
    pub fn parse_with_field(
        query_string: String,
        lenient: default!(Option<bool>, "NULL"),
        conjunction_mode: default!(Option<bool>, "NULL"),
    ) -> pdb::Query {
        pdb::Query::ParseWithField {
            query_string,
            lenient,
            conjunction_mode,
        }
    }

    #[builder_fn]
    #[pg_extern(immutable, parallel_safe, name = "phrase")]
    pub fn phrase(phrases: Vec<String>, slop: default!(Option<i32>, "NULL")) -> pdb::Query {
        pdb::Query::Phrase {
            phrases,
            slop: slop.map(|n| n as u32),
        }
    }

    #[builder_fn]
    #[pg_extern(immutable, parallel_safe, name = "phrase_prefix")]
    pub fn phrase_prefix(
        phrases: Vec<String>,
        max_expansion: default!(Option<i32>, "NULL"),
    ) -> pdb::Query {
        pdb::Query::PhrasePrefix {
            phrases,
            max_expansions: max_expansion.map(|n| n as u32),
        }
    }

    #[builder_fn]
    #[pg_extern(immutable, parallel_safe, name = "range")]
    pub fn range_i32(range: Range<i32>) -> pdb::Query {
        match range.into_inner() {
            None => pdb::Query::Range {
                lower_bound: Bound::Included(OwnedValue::I64(0)),
                upper_bound: Bound::Excluded(OwnedValue::I64(0)),
                is_datetime: false,
            },

            Some((lower, upper)) => pdb::Query::Range {
                lower_bound: match lower {
                    RangeBound::Infinite => Bound::Unbounded,
                    RangeBound::Inclusive(n) => Bound::Included(OwnedValue::I64(n as i64)),
                    RangeBound::Exclusive(n) => Bound::Excluded(OwnedValue::I64(n as i64)),
                },
                upper_bound: match upper {
                    RangeBound::Infinite => Bound::Unbounded,
                    RangeBound::Inclusive(n) => Bound::Included(OwnedValue::I64(n as i64)),
                    RangeBound::Exclusive(n) => Bound::Excluded(OwnedValue::I64(n as i64)),
                },
                is_datetime: false,
            },
        }
    }

    #[builder_fn]
    #[pg_extern(immutable, parallel_safe, name = "range")]
    pub fn range_i64(range: Range<i64>) -> pdb::Query {
        match range.into_inner() {
            None => pdb::Query::Range {
                lower_bound: Bound::Included(OwnedValue::I64(0)),
                upper_bound: Bound::Excluded(OwnedValue::I64(0)),
                is_datetime: false,
            },
            Some((lower, upper)) => pdb::Query::Range {
                lower_bound: match lower {
                    RangeBound::Infinite => Bound::Unbounded,
                    RangeBound::Inclusive(n) => Bound::Included(OwnedValue::I64(n)),
                    RangeBound::Exclusive(n) => Bound::Excluded(OwnedValue::I64(n)),
                },
                upper_bound: match upper {
                    RangeBound::Infinite => Bound::Unbounded,
                    RangeBound::Inclusive(n) => Bound::Included(OwnedValue::I64(n)),
                    RangeBound::Exclusive(n) => Bound::Excluded(OwnedValue::I64(n)),
                },
                is_datetime: false,
            },
        }
    }

    #[builder_fn]
    #[pg_extern(immutable, parallel_safe, name = "range")]
    pub fn range_numeric(range: Range<AnyNumeric>) -> pdb::Query {
        match range.into_inner() {
            None => pdb::Query::Range {
                lower_bound: Bound::Included(OwnedValue::F64(0.0)),
                upper_bound: Bound::Excluded(OwnedValue::F64(0.0)),
                is_datetime: false,
            },
            Some((lower, upper)) => pdb::Query::Range {
                lower_bound: match lower {
                    RangeBound::Infinite => Bound::Unbounded,
                    RangeBound::Inclusive(n) => Bound::Included(OwnedValue::F64(
                        n.try_into().expect("numeric should be a valid f64"),
                    )),
                    RangeBound::Exclusive(n) => Bound::Excluded(OwnedValue::F64(
                        n.try_into().expect("numeric should be a valid f64"),
                    )),
                },
                upper_bound: match upper {
                    RangeBound::Infinite => Bound::Unbounded,
                    RangeBound::Inclusive(n) => Bound::Included(OwnedValue::F64(
                        n.try_into().expect("numeric should be a valid f64"),
                    )),
                    RangeBound::Exclusive(n) => Bound::Excluded(OwnedValue::F64(
                        n.try_into().expect("numeric should be a valid f64"),
                    )),
                },
                is_datetime: false,
            },
        }
    }

    macro_rules! datetime_range_fn {
        ($func_name:ident, $value_type:ty) => {
            #[builder_fn]
            #[pg_extern(immutable, parallel_safe, name = "range")]
            pub fn $func_name(range: Range<$value_type>) -> pdb::Query {
                match range.into_inner() {
                    None => pdb::Query::Range {
                        lower_bound: Bound::Included(tantivy::schema::OwnedValue::Date(
                            tantivy::DateTime::from_timestamp_micros(0),
                        )),
                        upper_bound: Bound::Excluded(tantivy::schema::OwnedValue::Date(
                            tantivy::DateTime::from_timestamp_micros(0),
                        )),
                        is_datetime: true,
                    },
                    Some((lower, upper)) => pdb::Query::Range {
                        lower_bound: match lower {
                            RangeBound::Infinite => Bound::Unbounded,
                            RangeBound::Inclusive(n) => Bound::Included(
                                (&TantivyValue::try_from(n)
                                    .expect("n should be a valid TantivyValue representation")
                                    .tantivy_schema_value())
                                    .as_datetime()
                                    .expect("OwnedValue should be a valid datetime value")
                                    .into(),
                            ),
                            RangeBound::Exclusive(n) => Bound::Excluded(
                                (&TantivyValue::try_from(n)
                                    .expect("n should be a valid TantivyValue representation")
                                    .tantivy_schema_value())
                                    .as_datetime()
                                    .expect("OwnedValue should be a valid datetime value")
                                    .into(),
                            ),
                        },
                        upper_bound: match upper {
                            RangeBound::Infinite => Bound::Unbounded,
                            RangeBound::Inclusive(n) => Bound::Included(
                                (&TantivyValue::try_from(n)
                                    .expect("n should be a valid TantivyValue representation")
                                    .tantivy_schema_value())
                                    .as_datetime()
                                    .expect("OwnedValue should be a valid datetime value")
                                    .into(),
                            ),
                            RangeBound::Exclusive(n) => Bound::Excluded(
                                (&TantivyValue::try_from(n)
                                    .expect("n should be a valid TantivyValue representation")
                                    .tantivy_schema_value())
                                    .as_datetime()
                                    .expect("OwnedValue should be a valid datetime value")
                                    .into(),
                            ),
                        },
                        is_datetime: true,
                    },
                }
            }
        };
    }

    datetime_range_fn!(range_date, pgrx::datum::Date);
    datetime_range_fn!(range_timestamp, pgrx::datum::Timestamp);
    datetime_range_fn!(range_timestamptz, pgrx::datum::TimestampWithTimeZone);

    pub unsafe fn generic_range_query(
        lower: Bound<AnyElement>,
        upper: Bound<AnyElement>,
        is_datetime: bool,
    ) -> anyhow::Result<pdb::Query> {
        let query = match (lower, upper) {
            (Bound::Included(s), Bound::Included(e)) => pdb::Query::Range {
                lower_bound: Bound::Included(OwnedValue::from(TantivyValue::try_from_anyelement(
                    s,
                )?)),
                upper_bound: Bound::Included(OwnedValue::from(TantivyValue::try_from_anyelement(
                    e,
                )?)),
                is_datetime,
            },
            (Bound::Included(s), Bound::Excluded(e)) => pdb::Query::Range {
                lower_bound: Bound::Included(OwnedValue::from(TantivyValue::try_from_anyelement(
                    s,
                )?)),
                upper_bound: Bound::Excluded(OwnedValue::from(TantivyValue::try_from_anyelement(
                    e,
                )?)),
                is_datetime,
            },
            (Bound::Included(s), Bound::Unbounded) => pdb::Query::Range {
                lower_bound: Bound::Included(OwnedValue::from(TantivyValue::try_from_anyelement(
                    s,
                )?)),
                upper_bound: Bound::Unbounded,
                is_datetime,
            },
            (Bound::Excluded(s), Bound::Excluded(e)) => pdb::Query::Range {
                lower_bound: Bound::Excluded(OwnedValue::from(TantivyValue::try_from_anyelement(
                    s,
                )?)),
                upper_bound: Bound::Excluded(OwnedValue::from(TantivyValue::try_from_anyelement(
                    e,
                )?)),
                is_datetime,
            },
            (Bound::Excluded(s), Bound::Included(e)) => pdb::Query::Range {
                lower_bound: Bound::Excluded(OwnedValue::from(TantivyValue::try_from_anyelement(
                    s,
                )?)),
                upper_bound: Bound::Included(OwnedValue::from(TantivyValue::try_from_anyelement(
                    e,
                )?)),
                is_datetime,
            },
            (Bound::Excluded(s), Bound::Unbounded) => pdb::Query::Range {
                lower_bound: Bound::Excluded(OwnedValue::from(TantivyValue::try_from_anyelement(
                    s,
                )?)),
                upper_bound: Bound::Unbounded,
                is_datetime,
            },
            (Bound::Unbounded, Bound::Unbounded) => pdb::Query::Range {
                lower_bound: Bound::Unbounded,
                upper_bound: Bound::Unbounded,
                is_datetime,
            },
            (Bound::Unbounded, Bound::Included(e)) => pdb::Query::Range {
                lower_bound: Bound::Unbounded,
                upper_bound: Bound::Included(OwnedValue::from(TantivyValue::try_from_anyelement(
                    e,
                )?)),
                is_datetime,
            },
            (Bound::Unbounded, Bound::Excluded(e)) => pdb::Query::Range {
                lower_bound: Bound::Unbounded,
                upper_bound: Bound::Excluded(OwnedValue::from(TantivyValue::try_from_anyelement(
                    e,
                )?)),
                is_datetime,
            },
        };

        Ok(query)
    }

    #[builder_fn]
    #[pg_extern(immutable, parallel_safe, name = "regex")]
    pub fn regex(pattern: String) -> pdb::Query {
        pdb::Query::Regex { pattern }
    }

    #[builder_fn]
    #[pg_extern(immutable, parallel_safe, name = "regex_phrase")]
    pub fn regex_phrase(
        regexes: Vec<String>,
        slop: default!(Option<i32>, "NULL"),
        max_expansions: default!(Option<i32>, "NULL"),
    ) -> pdb::Query {
        pdb::Query::RegexPhrase {
            regexes,
            slop: slop.map(|n| n as u32),
            max_expansions: max_expansions.map(|n| n as u32),
        }
    }

    macro_rules! term_fn {
        ($func_name:ident, $value_type:ty) => {
            #[builder_fn]
            #[pg_extern(immutable, parallel_safe, name = "term")]
            pub fn $func_name(value: $value_type) -> pdb::Query {
                let tantivy_value = TantivyValue::try_from(value)
                    .expect("value should be a valid TantivyValue representation")
                    .tantivy_schema_value();
                let is_datetime = match tantivy_value {
                    OwnedValue::Date(_) => true,
                    _ => false,
                };

                pdb::Query::Term {
                    value: tantivy_value,
                    is_datetime,
                }
            }
        };
    }

    #[builder_fn]
    #[pg_extern(immutable, parallel_safe, name = "term")]
    pub fn term_anyenum(value: AnyEnum) -> pdb::Query {
        let tantivy_value = TantivyValue::try_from(value)
            .expect("value should be a valid TantivyValue representation")
            .tantivy_schema_value();
        let is_datetime = matches!(tantivy_value, OwnedValue::Date(_));

        pdb::Query::Term {
            value: tantivy_value,
            is_datetime,
        }
    }

    term_fn!(term_bytes, Vec<u8>);
    term_fn!(term_str, String);
    term_fn!(term_i8, i8);
    term_fn!(term_i16, i16);
    term_fn!(term_i32, i32);
    term_fn!(term_i64, i64);
    term_fn!(term_f32, f32);
    term_fn!(term_f64, f64);
    term_fn!(term_bool, bool);
    term_fn!(date, pgrx::datum::Date);
    term_fn!(time, pgrx::datum::Time);
    term_fn!(timestamp, pgrx::datum::Timestamp);
    term_fn!(time_with_time_zone, pgrx::datum::TimeWithTimeZone);
    term_fn!(timestamp_with_time_zone, pgrx::datum::TimestampWithTimeZone);
    term_fn!(numeric, pgrx::AnyNumeric);
    term_fn!(uuid, pgrx::Uuid);
    term_fn!(inet, pgrx::Inet);

    macro_rules! term_set_fn {
        ($func_name:ident, $value_type:ty) => {
            #[builder_fn]
            #[pg_extern(immutable, parallel_safe, name = "term_set")]
            pub fn $func_name(terms: Vec<$value_type>) -> pdb::Query {
                let terms = terms
                    .into_iter()
                    .map(|term| {
                        TantivyValue::try_from(term)
                            .expect("value should be a valid TantivyValue representation")
                            .tantivy_schema_value()
                    })
                    .collect::<Vec<_>>();
                pdb::Query::TermSet { terms }
            }
        };
    }

    term_set_fn!(term_set_str, String);
    term_set_fn!(term_set_i8, i8);
    term_set_fn!(term_set_i16, i16);
    term_set_fn!(term_set_i32, i32);
    term_set_fn!(term_set_i64, i64);
    term_set_fn!(term_set_f32, f32);
    term_set_fn!(term_set_f64, f64);
    term_set_fn!(term_set_bool, bool);
    term_set_fn!(term_set_date, pgrx::datum::Date);
    term_set_fn!(term_set_time, pgrx::datum::Time);
    term_set_fn!(term_set_timestamp, pgrx::datum::Timestamp);
    term_set_fn!(term_set_time_with_time_zone, pgrx::datum::TimeWithTimeZone);
    term_set_fn!(
        term_set_timestamp_with_time_zone,
        pgrx::datum::TimestampWithTimeZone
    );
    term_set_fn!(term_set_numeric, pgrx::AnyNumeric);
    term_set_fn!(term_set_uuid, pgrx::Uuid);

    // TODO:  this requires `impl UnboxDatum for Inet` in pgrx
    // term_set_fn!(term_set_inet, pgrx::Inet, false);

    macro_rules! range_term_fn {
        ($func_name:ident, $value_type:ty, $is_datetime:expr) => {
            #[builder_fn]
            #[pg_extern(immutable, parallel_safe, name = "range_term")]
            pub fn $func_name(term: $value_type) -> pdb::Query {
                pdb::Query::RangeTerm {
                    value: TantivyValue::try_from(term)
                        .expect("term should be a valid TantivyValue representation")
                        .tantivy_schema_value(),
                    is_datetime: $is_datetime,
                }
            }
        };
    }

    range_term_fn!(range_term_i8, i8, false);
    range_term_fn!(range_term_i16, i16, false);
    range_term_fn!(range_term_i32, i32, false);
    range_term_fn!(range_term_i64, i64, false);
    range_term_fn!(range_term_f32, f32, false);
    range_term_fn!(range_term_f64, f64, false);
    range_term_fn!(range_term_numeric, pgrx::AnyNumeric, false);
    range_term_fn!(range_term_date, pgrx::datum::Date, true);
    range_term_fn!(range_term_timestamp, pgrx::datum::Timestamp, true);
    range_term_fn!(
        range_term_timestamp_with_time_zone,
        pgrx::datum::TimestampWithTimeZone,
        true
    );

    #[derive(PostgresEnum, Serialize)]
    pub enum RangeRelation {
        Intersects,
        Contains,
        Within,
    }

    impl Display for RangeRelation {
        fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
            match self {
                RangeRelation::Intersects => write!(f, "Intersects"),
                RangeRelation::Contains => write!(f, "Contains"),
                RangeRelation::Within => write!(f, "Within"),
            }
        }
    }

    macro_rules! range_term_range_fn {
        ($func_name:ident, $value_type:ty, $is_datetime:expr, $default:expr) => {
            #[builder_fn]
            #[pg_extern(immutable, parallel_safe, name = "range_term")]
            pub fn $func_name(range: $value_type, relation: RangeRelation) -> pdb::Query {
                let (lower_bound, upper_bound) = match range.into_inner() {
                    None => (Bound::Included($default), Bound::Excluded($default)),
                    Some((lower, upper)) => {
                        let lower_bound = match lower {
                            RangeBound::Infinite => Bound::Unbounded,
                            RangeBound::Inclusive(n) => Bound::Included(
                                TantivyValue::try_from(n)
                                    .expect("value should be a valid TantivyValue representation")
                                    .tantivy_schema_value(),
                            ),
                            RangeBound::Exclusive(n) => Bound::Excluded(
                                TantivyValue::try_from(n)
                                    .expect("value should be a valid TantivyValue representation")
                                    .tantivy_schema_value(),
                            ),
                        };

                        let upper_bound = match upper {
                            RangeBound::Infinite => Bound::Unbounded,
                            RangeBound::Inclusive(n) => Bound::Included(
                                TantivyValue::try_from(n)
                                    .expect("value should be a valid TantivyValue representation")
                                    .tantivy_schema_value(),
                            ),
                            RangeBound::Exclusive(n) => Bound::Excluded(
                                TantivyValue::try_from(n)
                                    .expect("value should be a valid TantivyValue representation")
                                    .tantivy_schema_value(),
                            ),
                        };

                        (lower_bound, upper_bound)
                    }
                };

                match relation {
                    RangeRelation::Intersects => pdb::Query::RangeIntersects {
                        lower_bound,
                        upper_bound,
                        is_datetime: $is_datetime,
                    },
                    RangeRelation::Contains => pdb::Query::RangeContains {
                        lower_bound,
                        upper_bound,
                        is_datetime: $is_datetime,
                    },
                    RangeRelation::Within => pdb::Query::RangeWithin {
                        lower_bound,
                        upper_bound,
                        is_datetime: $is_datetime,
                    },
                }
            }
        };
    }

    range_term_range_fn!(
        range_term_range_int4range,
        pgrx::Range<i32>,
        false,
        OwnedValue::I64(0)
    );
    range_term_range_fn!(
        range_term_range_int8range,
        pgrx::Range<i64>,
        false,
        OwnedValue::I64(0)
    );
    range_term_range_fn!(
        range_term_range_numrange,
        pgrx::Range<pgrx::AnyNumeric>,
        false,
        OwnedValue::F64(0.0)
    );
    range_term_range_fn!(
        range_term_range_daterange,
        pgrx::Range<pgrx::datum::Date>,
        true,
        OwnedValue::Date(tantivy::DateTime::from_timestamp_micros(0))
    );
    range_term_range_fn!(
        range_term_range_tsrange,
        pgrx::Range<pgrx::datum::Timestamp>,
        true,
        OwnedValue::Date(tantivy::DateTime::from_timestamp_micros(0))
    );
    range_term_range_fn!(
        range_term_range_tstzrange,
        pgrx::Range<pgrx::datum::TimestampWithTimeZone>,
        true,
        OwnedValue::Date(tantivy::DateTime::from_timestamp_micros(0))
    );
}
