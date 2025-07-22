pub use pdb::*;

#[pgrx::pg_schema]
mod pdb {
    use crate::postgres::types::TantivyValue;
    use crate::query::fielded_query::FieldedQueryInput;
    use crate::schema::AnyEnum;
    use macros::builder_fn;
    use pgrx::datum::RangeBound;
    use pgrx::{default, pg_extern, AnyElement, AnyNumeric, PostgresEnum, Range};
    use serde::Serialize;
    use std::collections::Bound;
    use std::fmt::{Display, Formatter};
    use tantivy::schema::{OwnedValue, Value};

    #[builder_fn]
    #[pg_extern(immutable, parallel_safe, name = "exists")]
    pub fn exists() -> FieldedQueryInput {
        FieldedQueryInput::Exists
    }

    #[builder_fn]
    #[pg_extern(immutable, parallel_safe, name = "fuzzy_term")]
    pub fn fuzzy_term(
        value: default!(Option<String>, "NULL"),
        distance: default!(Option<i32>, "NULL"),
        transposition_cost_one: default!(Option<bool>, "NULL"),
        prefix: default!(Option<bool>, "NULL"),
    ) -> FieldedQueryInput {
        FieldedQueryInput::FuzzyTerm {
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
    ) -> FieldedQueryInput {
        FieldedQueryInput::Match {
            value,
            tokenizer: tokenizer.map(|t| t.0),
            distance: distance.map(|n| n as u8),
            transposition_cost_one,
            prefix,
            conjunction_mode,
        }
    }

    #[builder_fn]
    #[pg_extern(immutable, parallel_safe, name = "parse")]
    pub fn query_string(
        query_string: String,
        lenient: default!(Option<bool>, "NULL"),
        conjunction_mode: default!(Option<bool>, "NULL"),
    ) -> FieldedQueryInput {
        FieldedQueryInput::Parse {
            query_string,
            lenient,
            conjunction_mode,
        }
    }

    #[builder_fn]
    #[pg_extern(immutable, parallel_safe, name = "phrase")]
    pub fn phrase(phrases: Vec<String>, slop: default!(Option<i32>, "NULL")) -> FieldedQueryInput {
        FieldedQueryInput::Phrase {
            phrases,
            slop: slop.map(|n| n as u32),
        }
    }

    #[builder_fn]
    #[pg_extern(immutable, parallel_safe, name = "phrase_prefix")]
    pub fn phrase_prefix(
        phrases: Vec<String>,
        max_expansion: default!(Option<i32>, "NULL"),
    ) -> FieldedQueryInput {
        FieldedQueryInput::PhrasePrefix {
            phrases,
            max_expansions: max_expansion.map(|n| n as u32),
        }
    }

    #[builder_fn]
    #[pg_extern(immutable, parallel_safe, name = "range")]
    pub fn range_i32(range: Range<i32>) -> FieldedQueryInput {
        match range.into_inner() {
            None => FieldedQueryInput::Range {
                lower_bound: Bound::Included(OwnedValue::I64(0)),
                upper_bound: Bound::Excluded(OwnedValue::I64(0)),
                is_datetime: false,
            },

            Some((lower, upper)) => FieldedQueryInput::Range {
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
    pub fn range_i64(range: Range<i64>) -> FieldedQueryInput {
        match range.into_inner() {
            None => FieldedQueryInput::Range {
                lower_bound: Bound::Included(OwnedValue::I64(0)),
                upper_bound: Bound::Excluded(OwnedValue::I64(0)),
                is_datetime: false,
            },
            Some((lower, upper)) => FieldedQueryInput::Range {
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
    pub fn range_numeric(range: Range<AnyNumeric>) -> FieldedQueryInput {
        match range.into_inner() {
            None => FieldedQueryInput::Range {
                lower_bound: Bound::Included(OwnedValue::F64(0.0)),
                upper_bound: Bound::Excluded(OwnedValue::F64(0.0)),
                is_datetime: false,
            },
            Some((lower, upper)) => FieldedQueryInput::Range {
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
            pub fn $func_name(range: Range<$value_type>) -> FieldedQueryInput {
                match range.into_inner() {
                    None => FieldedQueryInput::Range {
                        lower_bound: Bound::Included(tantivy::schema::OwnedValue::Date(
                            tantivy::DateTime::from_timestamp_micros(0),
                        )),
                        upper_bound: Bound::Excluded(tantivy::schema::OwnedValue::Date(
                            tantivy::DateTime::from_timestamp_micros(0),
                        )),
                        is_datetime: true,
                    },
                    Some((lower, upper)) => FieldedQueryInput::Range {
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
    ) -> anyhow::Result<FieldedQueryInput> {
        let query = match (lower, upper) {
            (Bound::Included(s), Bound::Included(e)) => FieldedQueryInput::Range {
                lower_bound: Bound::Included(OwnedValue::from(TantivyValue::try_from_anyelement(
                    s,
                )?)),
                upper_bound: Bound::Included(OwnedValue::from(TantivyValue::try_from_anyelement(
                    e,
                )?)),
                is_datetime,
            },
            (Bound::Included(s), Bound::Excluded(e)) => FieldedQueryInput::Range {
                lower_bound: Bound::Included(OwnedValue::from(TantivyValue::try_from_anyelement(
                    s,
                )?)),
                upper_bound: Bound::Excluded(OwnedValue::from(TantivyValue::try_from_anyelement(
                    e,
                )?)),
                is_datetime,
            },
            (Bound::Included(s), Bound::Unbounded) => FieldedQueryInput::Range {
                lower_bound: Bound::Included(OwnedValue::from(TantivyValue::try_from_anyelement(
                    s,
                )?)),
                upper_bound: Bound::Unbounded,
                is_datetime,
            },
            (Bound::Excluded(s), Bound::Excluded(e)) => FieldedQueryInput::Range {
                lower_bound: Bound::Excluded(OwnedValue::from(TantivyValue::try_from_anyelement(
                    s,
                )?)),
                upper_bound: Bound::Excluded(OwnedValue::from(TantivyValue::try_from_anyelement(
                    e,
                )?)),
                is_datetime,
            },
            (Bound::Excluded(s), Bound::Included(e)) => FieldedQueryInput::Range {
                lower_bound: Bound::Excluded(OwnedValue::from(TantivyValue::try_from_anyelement(
                    s,
                )?)),
                upper_bound: Bound::Included(OwnedValue::from(TantivyValue::try_from_anyelement(
                    e,
                )?)),
                is_datetime,
            },
            (Bound::Excluded(s), Bound::Unbounded) => FieldedQueryInput::Range {
                lower_bound: Bound::Excluded(OwnedValue::from(TantivyValue::try_from_anyelement(
                    s,
                )?)),
                upper_bound: Bound::Unbounded,
                is_datetime,
            },
            (Bound::Unbounded, Bound::Unbounded) => FieldedQueryInput::Range {
                lower_bound: Bound::Unbounded,
                upper_bound: Bound::Unbounded,
                is_datetime,
            },
            (Bound::Unbounded, Bound::Included(e)) => FieldedQueryInput::Range {
                lower_bound: Bound::Unbounded,
                upper_bound: Bound::Included(OwnedValue::from(TantivyValue::try_from_anyelement(
                    e,
                )?)),
                is_datetime,
            },
            (Bound::Unbounded, Bound::Excluded(e)) => FieldedQueryInput::Range {
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
    pub fn regex(pattern: String) -> FieldedQueryInput {
        FieldedQueryInput::Regex { pattern }
    }

    #[builder_fn]
    #[pg_extern(immutable, parallel_safe, name = "regex_phrase")]
    pub fn regex_phrase(
        regexes: Vec<String>,
        slop: default!(Option<i32>, "NULL"),
        max_expansions: default!(Option<i32>, "NULL"),
    ) -> FieldedQueryInput {
        FieldedQueryInput::RegexPhrase {
            regexes,
            slop: slop.map(|n| n as u32),
            max_expansions: max_expansions.map(|n| n as u32),
        }
    }

    macro_rules! term_fn {
        ($func_name:ident, $value_type:ty) => {
            #[builder_fn]
            #[pg_extern(immutable, parallel_safe, name = "term")]
            pub fn $func_name(value: default!(Option<$value_type>, "NULL")) -> FieldedQueryInput {
                if let Some(value) = value {
                    let tantivy_value = TantivyValue::try_from(value)
                        .expect("value should be a valid TantivyValue representation")
                        .tantivy_schema_value();
                    let is_datetime = match tantivy_value {
                        OwnedValue::Date(_) => true,
                        _ => false,
                    };

                    FieldedQueryInput::Term {
                        value: tantivy_value,
                        is_datetime,
                    }
                } else {
                    panic!("no value provided to term query")
                }
            }
        };
    }

    #[builder_fn]
    #[pg_extern(immutable, parallel_safe, name = "term")]
    pub fn term_anyenum(value: AnyEnum) -> FieldedQueryInput {
        let tantivy_value = TantivyValue::try_from(value)
            .expect("value should be a valid TantivyValue representation")
            .tantivy_schema_value();
        let is_datetime = matches!(tantivy_value, OwnedValue::Date(_));

        FieldedQueryInput::Term {
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

    macro_rules! range_term_fn {
        ($func_name:ident, $value_type:ty, $is_datetime:expr) => {
            #[pg_extern(name = "range_term", immutable, parallel_safe)]
            pub fn $func_name(term: $value_type) -> FieldedQueryInput {
                FieldedQueryInput::RangeTerm {
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
            pub fn $func_name(range: $value_type, relation: RangeRelation) -> FieldedQueryInput {
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
                    RangeRelation::Intersects => FieldedQueryInput::RangeIntersects {
                        lower_bound,
                        upper_bound,
                        is_datetime: $is_datetime,
                    },
                    RangeRelation::Contains => FieldedQueryInput::RangeContains {
                        lower_bound,
                        upper_bound,
                        is_datetime: $is_datetime,
                    },
                    RangeRelation::Within => FieldedQueryInput::RangeWithin {
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
