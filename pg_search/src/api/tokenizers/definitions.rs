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

use crate::api::tokenizers::typmod::{save_typmod, ParsedTypmod};
use pgrx::pg_sys::panic::ErrorReport;
use pgrx::{extension_sql, function_name, pg_extern, Array, PgLogLevel, PgSqlErrorCode};
use std::ffi::CStr;

#[pgrx::pg_schema]
pub(crate) mod pdb {
    use crate::api::tokenizers::{
        CowString, DatumWrapper, GenericTypeWrapper, JsonMarker, JsonbMarker, SqlNameMarker,
        TextArrayMarker, VarcharArrayMarker,
    };
    use macros::generate_tokenizer_sql;
    use paste::paste;
    use pgrx::callconv::{Arg, ArgAbi, BoxRet, FcInfo};
    use pgrx::nullable::Nullable;
    use pgrx::pgrx_sql_entity_graph::metadata::{
        ArgumentError, Returns, ReturnsError, SqlMapping, SqlTranslatable,
    };
    use pgrx::{extension_sql, pg_extern, pg_sys, FromDatum, IntoDatum};
    use std::ffi::CString;
    use tokenizers::manager::{LinderaLanguage, SearchTokenizerFilters};
    use tokenizers::SearchTokenizer;

    pub trait TokenizerCtor {
        fn make_search_tokenizer() -> SearchTokenizer;
    }

    macro_rules! define_tokenizer_type {
        ($rust_name:ident, $tokenizer_conf:expr, $cast_name:ident, $json_cast_name:ident, $jsonb_cast_name:ident, $text_array_cast_name:ident, $varchar_array_cast_name:ident, $sql_name:literal, preferred = $preferred:literal, custom_typmod = $custom_typmod:literal) => {
            pub struct $rust_name(pg_sys::Datum);

            impl TokenizerCtor for $rust_name {
                #[inline(always)]
                fn make_search_tokenizer() -> SearchTokenizer {
                    $tokenizer_conf
                }
            }

            impl DatumWrapper for $rust_name {
                fn from_datum(datum: pg_sys::Datum) -> Self {
                    $rust_name(datum)
                }

                fn as_datum(&self) -> pg_sys::Datum {
                    self.0
                }
            }

            impl FromDatum for $rust_name {
                unsafe fn from_polymorphic_datum(
                    datum: pg_sys::Datum,
                    is_null: bool,
                    _typoid: pg_sys::Oid,
                ) -> Option<Self> {
                    (!is_null).then_some($rust_name(datum))
                }
            }

            impl IntoDatum for $rust_name {
                fn into_datum(self) -> Option<pg_sys::Datum> {
                    Some(self.0)
                }

                fn type_oid() -> pg_sys::Oid {
                    use crate::postgres::catalog::*;
                    let name = CString::new(stringify!($rust_name))
                        .expect("type name should be valid utf8");
                    lookup_typoid(c"paradedb", name.as_c_str())
                        .expect("should not fail to lookup type oid")
                }
            }

            unsafe impl<'fcx> ArgAbi<'fcx> for $rust_name {
                unsafe fn unbox_arg_unchecked(arg: Arg<'_, 'fcx>) -> Self {
                    let index = arg.index();
                    unsafe {
                        arg.unbox_arg_using_from_datum()
                            .unwrap_or_else(|| panic!("argument {index} must not be null"))
                    }
                }

                unsafe fn unbox_nullable_arg(arg: Arg<'_, 'fcx>) -> Nullable<Self> {
                    unsafe { arg.unbox_arg_using_from_datum().into() }
                }
            }

            unsafe impl BoxRet for $rust_name {
                unsafe fn box_into<'fcx>(
                    self,
                    fcinfo: &mut FcInfo<'fcx>,
                ) -> pgrx::datum::Datum<'fcx> {
                    match self.into_datum() {
                        Some(datum) => unsafe { fcinfo.return_raw_datum(datum) },
                        None => fcinfo.return_null(),
                    }
                }
            }

            unsafe impl SqlTranslatable for $rust_name {
                fn argument_sql() -> Result<SqlMapping, ArgumentError> {
                    Ok(SqlMapping::As(format!("pdb.{}", $sql_name)))
                }

                fn return_sql() -> Result<Returns, ReturnsError> {
                    Ok(Returns::One(SqlMapping::As(format!("pdb.{}", $sql_name))))
                }
            }

            #[pg_extern(immutable, parallel_safe)]
            fn $cast_name(s: $rust_name, fcinfo: pg_sys::FunctionCallInfo) -> Vec<String> {
                let mut tokenizer = $rust_name::make_search_tokenizer();

                unsafe {
                    let func_expr = (*(*fcinfo).flinfo).fn_expr.cast::<pg_sys::FuncExpr>();
                    let args = pgrx::PgList::<pg_sys::Node>::from_pg((*func_expr).args.cast());
                    let first_arg = args.get_ptr(0).unwrap();
                    let typmod = pg_sys::exprTypmod(first_arg);

                    super::super::apply_typmod(&mut tokenizer, typmod);
                }

                let mut analyzer = tokenizer
                    .to_tantivy_tokenizer()
                    .expect("failed to convert tokenizer to tantivy tokenizer");

                let s = s.to_str();
                let mut stream = analyzer.token_stream(&s);

                let mut tokens = Vec::new();
                while stream.advance() {
                    let token = stream.token();
                    tokens.push(token.text.to_string());
                }
                tokens
            }

            paste! {
                struct [<$rust_name JsonMarker>];
                impl SqlNameMarker for [<$rust_name JsonMarker>] {
                    const SQL_NAME: &'static str = concat!("pdb.", $sql_name);
                }

                struct [<$rust_name JsonbMarker>];
                impl SqlNameMarker for [<$rust_name JsonbMarker>] {
                    const SQL_NAME: &'static str = concat!("pdb.", $sql_name);
                }

                struct [<$rust_name TextArrayMarker>];
                impl SqlNameMarker for [<$rust_name TextArrayMarker>] {
                    const SQL_NAME: &'static str = concat!("pdb.", $sql_name);
                }

                struct [<$rust_name VarcharArrayMarker>];
                impl SqlNameMarker for [<$rust_name VarcharArrayMarker>] {
                    const SQL_NAME: &'static str = concat!("pdb.", $sql_name);
                }

                #[pg_extern(immutable, parallel_safe, requires = [ $cast_name ])]
                fn $json_cast_name(
                    json: GenericTypeWrapper<pgrx::Json, JsonMarker>,
                ) -> GenericTypeWrapper<$rust_name, [<$rust_name JsonMarker>]> {
                    GenericTypeWrapper::new(json.datum)
                }

                #[pg_extern(immutable, parallel_safe, requires = [ $cast_name ])]
                unsafe fn $jsonb_cast_name(
                    jsonb: GenericTypeWrapper<pgrx::JsonB, JsonbMarker>,
                ) -> GenericTypeWrapper<$rust_name, [<$rust_name JsonbMarker>]> {
                    GenericTypeWrapper::new(jsonb.datum)
                }

                #[pg_extern(immutable, parallel_safe, requires = [ $cast_name ])]
                unsafe fn $text_array_cast_name(
                    arr: GenericTypeWrapper<Vec<String>, TextArrayMarker>,
                ) -> GenericTypeWrapper<$rust_name, [<$rust_name TextArrayMarker>]> {
                    GenericTypeWrapper::new(arr.datum)
                }

                #[pg_extern(immutable, parallel_safe, requires = [ $cast_name ])]
                unsafe fn $varchar_array_cast_name(
                    arr: GenericTypeWrapper<Vec<String>, VarcharArrayMarker>,
                ) -> GenericTypeWrapper<$rust_name, [<$rust_name VarcharArrayMarker>]> {
                    GenericTypeWrapper::new(arr.datum)
                }
            }

            generate_tokenizer_sql!(
                rust_name = $rust_name,
                sql_name = $sql_name,
                cast_name = $cast_name,
                preferred = $preferred,
                custom_typmod = $custom_typmod,
                json_cast_name = $json_cast_name,
                jsonb_cast_name = $jsonb_cast_name,
                text_array_cast_name = $text_array_cast_name,
                varchar_array_cast_name = $varchar_array_cast_name,
                schema = pdb
            );
        };
    }

    define_tokenizer_type!(
        Alias,
        SearchTokenizer::Simple(SearchTokenizerFilters::default()),
        tokenize_alias,
        json_to_alias,
        jsonb_to_alias,
        text_array_to_alias,
        varchar_array_to_alias,
        "alias",
        preferred = false,
        custom_typmod = false
    );

    define_tokenizer_type!(
        Simple,
        SearchTokenizer::Simple(SearchTokenizerFilters::default()),
        tokenize_simple,
        json_to_simple,
        jsonb_to_simple,
        text_array_to_simple,
        varchar_array_to_simple,
        "simple",
        preferred = true,
        custom_typmod = false
    );

    define_tokenizer_type!(
        Whitespace,
        SearchTokenizer::WhiteSpace(SearchTokenizerFilters::default()),
        tokenize_whitespace,
        json_to_whitespace,
        jsonb_to_whitespace,
        text_array_to_whitespace,
        varchar_array_to_whitespace,
        "whitespace",
        preferred = false,
        custom_typmod = false
    );

    define_tokenizer_type!(
        Literal,
        SearchTokenizer::Keyword,
        tokenize_literal,
        json_to_literal,
        jsonb_to_literal,
        text_array_to_literal,
        varchar_array_to_literal,
        "literal",
        preferred = false,
        custom_typmod = true
    );

    define_tokenizer_type!(
        LiteralNormalized,
        SearchTokenizer::LiteralNormalized(SearchTokenizerFilters::default()),
        tokenize_literal_normalized,
        json_to_literal_normalized,
        jsonb_to_literal_normalized,
        text_array_to_literal_normalized,
        varchar_array_to_literal_normalized,
        "literal_normalized",
        preferred = false,
        custom_typmod = false
    );

    define_tokenizer_type!(
        ChineseCompatible,
        SearchTokenizer::ChineseCompatible(SearchTokenizerFilters::default()),
        tokenize_chinese_compatible,
        json_to_chinese_compatible,
        jsonb_to_chinese_compatible,
        text_array_to_chinese_compatible,
        varchar_array_to_chinese_compatible,
        "chinese_compatible",
        preferred = false,
        custom_typmod = false
    );

    define_tokenizer_type!(
        Lindera,
        SearchTokenizer::Lindera(LinderaLanguage::Chinese, SearchTokenizerFilters::default()),
        tokenize_lindera,
        json_to_lindera,
        jsonb_to_lindera,
        text_array_to_lindera,
        varchar_array_to_lindera,
        "lindera",
        preferred = false,
        custom_typmod = false
    );

    define_tokenizer_type!(
        Jieba,
        SearchTokenizer::Jieba(SearchTokenizerFilters::default()),
        tokenize_jieba,
        json_to_jieba,
        jsonb_to_jieba,
        text_array_to_jieba,
        varchar_array_to_jieba,
        "jieba",
        preferred = false,
        custom_typmod = false
    );

    define_tokenizer_type!(
        SourceCode,
        SearchTokenizer::SourceCode(SearchTokenizerFilters::default()),
        tokenize_source_code,
        json_to_source_code,
        jsonb_to_source_code,
        text_array_to_source_code,
        varchar_array_to_source_code,
        "source_code",
        preferred = false,
        custom_typmod = false
    );

    #[cfg(feature = "icu")]
    define_tokenizer_type!(
        ICU,
        SearchTokenizer::ICUTokenizer(SearchTokenizerFilters::default()),
        tokenize_icu,
        json_to_icu,
        jsonb_to_icu,
        text_array_to_icu,
        varchar_array_to_icu,
        "icu",
        preferred = false,
        custom_typmod = false
    );

    define_tokenizer_type!(
        Ngram,
        SearchTokenizer::Ngram {
            min_gram: 1,
            max_gram: 3,
            prefix_only: false,
            filters: SearchTokenizerFilters::default(),
        },
        tokenize_ngram,
        json_to_ngram,
        jsonb_to_ngram,
        text_array_to_ngram,
        varchar_array_to_ngram,
        "ngram",
        preferred = false,
        custom_typmod = false
    );

    define_tokenizer_type!(
        Regex,
        SearchTokenizer::RegexTokenizer {
            pattern: ".*".to_string(),
            filters: SearchTokenizerFilters::default(),
        },
        tokenize_regex,
        json_to_regex,
        jsonb_to_regex,
        text_array_to_regex_pattern,
        varchar_array_to_regex_pattern,
        "regex_pattern",
        preferred = false,
        custom_typmod = false
    );

    define_tokenizer_type!(
        UnicodeWords,
        SearchTokenizer::UnicodeWords {
            remove_emojis: false,
            filters: SearchTokenizerFilters::default()
        },
        tokenize_unicode_words,
        json_to_unicode_words,
        jsonb_to_unicode_words,
        text_array_to_unicode_words,
        varchar_array_to_unicode_words,
        "unicode_words",
        preferred = false,
        custom_typmod = false
    );

    struct AliasMarker;
    impl SqlNameMarker for AliasMarker {
        const SQL_NAME: &'static str = "pdb.alias";
    }

    // allow <smallint>::pdb.alias
    struct SmallIntMarker;
    impl SqlNameMarker for SmallIntMarker {
        const SQL_NAME: &'static str = "smallint";
    }

    #[pg_extern(immutable, parallel_safe, requires = [tokenize_alias])]
    unsafe fn smallint_to_alias(
        arr: GenericTypeWrapper<i16, SmallIntMarker>,
    ) -> GenericTypeWrapper<Alias, AliasMarker> {
        GenericTypeWrapper::new(arr.datum)
    }

    // allow <integer>::pdb.alias
    struct IntegerMarker;
    impl SqlNameMarker for IntegerMarker {
        const SQL_NAME: &'static str = "integer";
    }

    #[pg_extern(immutable, parallel_safe, requires = [tokenize_alias])]
    unsafe fn integer_to_alias(
        arr: GenericTypeWrapper<i32, IntegerMarker>,
    ) -> GenericTypeWrapper<Alias, AliasMarker> {
        GenericTypeWrapper::new(arr.datum)
    }

    // allow <bigint>::pdb.alias
    struct BigIntMarker;
    impl SqlNameMarker for BigIntMarker {
        const SQL_NAME: &'static str = "bigint";
    }

    #[pg_extern(immutable, parallel_safe, requires = [tokenize_alias])]
    unsafe fn bigint_to_alias(
        arr: GenericTypeWrapper<i64, BigIntMarker>,
    ) -> GenericTypeWrapper<Alias, AliasMarker> {
        GenericTypeWrapper::new(arr.datum)
    }

    // allow <oid>::pdb.alias
    struct OidMarker;
    impl SqlNameMarker for OidMarker {
        const SQL_NAME: &'static str = "oid";
    }

    #[pg_extern(immutable, parallel_safe, requires = [tokenize_alias])]
    unsafe fn oid_to_alias(
        arr: GenericTypeWrapper<u32, OidMarker>,
    ) -> GenericTypeWrapper<Alias, AliasMarker> {
        GenericTypeWrapper::new(arr.datum)
    }

    // allow <float4>::pdb.alias
    struct Float4Marker;
    impl SqlNameMarker for Float4Marker {
        const SQL_NAME: &'static str = "float4";
    }

    #[pg_extern(immutable, parallel_safe, requires = [tokenize_alias])]
    unsafe fn float4_to_alias(
        arr: GenericTypeWrapper<f32, Float4Marker>,
    ) -> GenericTypeWrapper<Alias, AliasMarker> {
        GenericTypeWrapper::new(arr.datum)
    }

    // allow <float8>::pdb.alias
    struct Float8Marker;
    impl SqlNameMarker for Float8Marker {
        const SQL_NAME: &'static str = "float8";
    }

    #[pg_extern(immutable, parallel_safe, requires = [tokenize_alias])]
    unsafe fn float8_to_alias(
        arr: GenericTypeWrapper<f64, Float8Marker>,
    ) -> GenericTypeWrapper<Alias, AliasMarker> {
        GenericTypeWrapper::new(arr.datum)
    }

    // // allow <numeric>::pdb.alias
    // struct NumericMarker;
    // impl SqlNameMarker for NumericMarker {
    //     const SQL_NAME: &'static str = "numeric";
    // }

    // #[pg_extern(immutable, parallel_safe, requires = [tokenize_alias])]
    // unsafe fn numeric_to_alias(
    //     arr: GenericTypeWrapper<AnyNumeric, NumericMarker>,
    // ) -> GenericTypeWrapper<Alias, AliasMarker> {
    //     GenericTypeWrapper::new(arr.datum)
    // }

    // allow <boolean>::pdb.alias
    struct BooleanMarker;
    impl SqlNameMarker for BooleanMarker {
        const SQL_NAME: &'static str = "boolean";
    }

    #[pg_extern(immutable, parallel_safe, requires = [tokenize_alias])]
    unsafe fn boolean_to_alias(
        arr: GenericTypeWrapper<bool, BooleanMarker>,
    ) -> GenericTypeWrapper<Alias, AliasMarker> {
        GenericTypeWrapper::new(arr.datum)
    }

    // allow <date>::pdb.alias
    struct DateMarker;
    impl SqlNameMarker for DateMarker {
        const SQL_NAME: &'static str = "date";
    }

    #[pg_extern(immutable, parallel_safe, requires = [tokenize_alias])]
    unsafe fn date_to_alias(
        arr: GenericTypeWrapper<pgrx::datum::Date, DateMarker>,
    ) -> GenericTypeWrapper<Alias, AliasMarker> {
        GenericTypeWrapper::new(arr.datum)
    }

    // allow <time>::pdb.alias
    struct TimeMarker;
    impl SqlNameMarker for TimeMarker {
        const SQL_NAME: &'static str = "time";
    }

    #[pg_extern(immutable, parallel_safe, requires = [tokenize_alias])]
    unsafe fn time_to_alias(
        arr: GenericTypeWrapper<pgrx::datum::Time, TimeMarker>,
    ) -> GenericTypeWrapper<Alias, AliasMarker> {
        GenericTypeWrapper::new(arr.datum)
    }

    // allow <timestamp>::pdb.alias
    struct TimestampMarker;
    impl SqlNameMarker for TimestampMarker {
        const SQL_NAME: &'static str = "timestamp";
    }

    #[pg_extern(immutable, parallel_safe, requires = [tokenize_alias])]
    unsafe fn timestamp_to_alias(
        arr: GenericTypeWrapper<pgrx::datum::Timestamp, TimestampMarker>,
    ) -> GenericTypeWrapper<Alias, AliasMarker> {
        GenericTypeWrapper::new(arr.datum)
    }

    // allow <timestamp with time zone>::pdb.alias
    struct TimestampWithTimeZoneMarker;
    impl SqlNameMarker for TimestampWithTimeZoneMarker {
        const SQL_NAME: &'static str = "timestamp with time zone";
    }

    #[pg_extern(immutable, parallel_safe, requires = [tokenize_alias])]
    unsafe fn timestamp_with_time_zone_to_alias(
        arr: GenericTypeWrapper<pgrx::datum::TimestampWithTimeZone, TimestampWithTimeZoneMarker>,
    ) -> GenericTypeWrapper<Alias, AliasMarker> {
        GenericTypeWrapper::new(arr.datum)
    }

    // allow <time with time zone>::pdb.alias
    struct TimeWithTimeZoneMarker;
    impl SqlNameMarker for TimeWithTimeZoneMarker {
        const SQL_NAME: &'static str = "time with time zone";
    }

    #[pg_extern(immutable, parallel_safe, requires = [tokenize_alias])]
    unsafe fn time_with_time_zone_to_alias(
        arr: GenericTypeWrapper<pgrx::datum::TimeWithTimeZone, TimeWithTimeZoneMarker>,
    ) -> GenericTypeWrapper<Alias, AliasMarker> {
        GenericTypeWrapper::new(arr.datum)
    }

    // allow <inet>::pdb.alias
    struct InetMarker;
    impl SqlNameMarker for InetMarker {
        const SQL_NAME: &'static str = "inet";
    }

    #[pg_extern(immutable, parallel_safe, requires = [tokenize_alias])]
    unsafe fn inet_to_alias(
        arr: GenericTypeWrapper<pgrx::datum::Inet, InetMarker>,
    ) -> GenericTypeWrapper<Alias, AliasMarker> {
        GenericTypeWrapper::new(arr.datum)
    }

    // allow <int4range>::pdb.alias
    struct Int4RangeMarker;
    impl SqlNameMarker for Int4RangeMarker {
        const SQL_NAME: &'static str = "int4range";
    }

    #[pg_extern(immutable, parallel_safe, requires = [tokenize_alias])]
    unsafe fn int4range_to_alias(
        arr: GenericTypeWrapper<pgrx::datum::Range<i32>, Int4RangeMarker>,
    ) -> GenericTypeWrapper<Alias, AliasMarker> {
        GenericTypeWrapper::new(arr.datum)
    }

    // allow <int8range>::pdb.alias
    struct Int8RangeMarker;
    impl SqlNameMarker for Int8RangeMarker {
        const SQL_NAME: &'static str = "int8range";
    }

    #[pg_extern(immutable, parallel_safe, requires = [tokenize_alias])]
    unsafe fn int8range_to_alias(
        arr: GenericTypeWrapper<pgrx::datum::Range<i64>, Int8RangeMarker>,
    ) -> GenericTypeWrapper<Alias, AliasMarker> {
        GenericTypeWrapper::new(arr.datum)
    }

    // // allow <numrange>::pdb.alias
    // struct NumRangeMarker;
    // impl SqlNameMarker for NumRangeMarker {
    //     const SQL_NAME: &'static str = "numrange";
    // }

    // #[pg_extern(immutable, parallel_safe, requires = [tokenize_alias])]
    // unsafe fn numrange_to_alias(
    //     arr: GenericTypeWrapper<NumRange, NumRangeMarker>,
    // ) -> GenericTypeWrapper<Alias, AliasMarker> {
    //     GenericTypeWrapper::new(arr.datum)
    // }

    // // allow <daterange>::pdb.alias
    // struct DateRangeMarker;
    // impl SqlNameMarker for DateRangeMarker {
    //     const SQL_NAME: &'static str = "daterange";
    // }

    // #[pg_extern(immutable, parallel_safe, requires = [tokenize_alias])]
    // unsafe fn daterange_to_alias(
    //     arr: GenericTypeWrapper<DateRange, DateRangeMarker>,
    // ) -> GenericTypeWrapper<Alias, AliasMarker> {
    //     GenericTypeWrapper::new(arr.datum)
    // }

    // // allow <tsrange>::pdb.alias
    // struct TsRangeMarker;
    // impl SqlNameMarker for TsRangeMarker {
    //     const SQL_NAME: &'static str = "tsrange";
    // }

    // #[pg_extern(immutable, parallel_safe, requires = [tokenize_alias])]
    // unsafe fn tsrange_to_alias(
    //     arr: GenericTypeWrapper<TsRange, TsRangeMarker>,
    // ) -> GenericTypeWrapper<Alias, AliasMarker> {
    //     GenericTypeWrapper::new(arr.datum)
    // }

    // // allow <tstzrange>::pdb.alias
    // struct TstzRangeMarker;
    // impl SqlNameMarker for TstzRangeMarker {
    //     const SQL_NAME: &'static str = "tstzrange";
    // }

    // #[pg_extern(immutable, parallel_safe, requires = [tokenize_alias])]
    // unsafe fn tstzrange_to_alias(
    //     arr: GenericTypeWrapper<TstzRange, TstzRangeMarker>,
    // ) -> GenericTypeWrapper<Alias, AliasMarker> {
    //     GenericTypeWrapper::new(arr.datum)
    // }

    extension_sql!(
        r#"
            CREATE CAST (smallint AS pdb.alias) WITH FUNCTION pdb.smallint_to_alias AS ASSIGNMENT;
            CREATE CAST (integer AS pdb.alias) WITH FUNCTION pdb.integer_to_alias AS ASSIGNMENT;
            CREATE CAST (bigint AS pdb.alias) WITH FUNCTION pdb.bigint_to_alias AS ASSIGNMENT;
            CREATE CAST (oid AS pdb.alias) WITH FUNCTION pdb.oid_to_alias AS ASSIGNMENT;
            CREATE CAST (float4 AS pdb.alias) WITH FUNCTION pdb.float4_to_alias AS ASSIGNMENT;
            CREATE CAST (float8 AS pdb.alias) WITH FUNCTION pdb.float8_to_alias AS ASSIGNMENT;
            CREATE CAST (boolean AS pdb.alias) WITH FUNCTION pdb.boolean_to_alias AS ASSIGNMENT;
            CREATE CAST (date AS pdb.alias) WITH FUNCTION pdb.date_to_alias AS ASSIGNMENT;
            CREATE CAST (time AS pdb.alias) WITH FUNCTION pdb.time_to_alias AS ASSIGNMENT;
            CREATE CAST (timestamp AS pdb.alias) WITH FUNCTION pdb.timestamp_to_alias AS ASSIGNMENT;
            CREATE CAST (timestamp with time zone AS pdb.alias) WITH FUNCTION pdb.timestamp_with_time_zone_to_alias AS ASSIGNMENT;
            CREATE CAST (time with time zone AS pdb.alias) WITH FUNCTION pdb.time_with_time_zone_to_alias AS ASSIGNMENT;
            CREATE CAST (inet AS pdb.alias) WITH FUNCTION pdb.inet_to_alias AS ASSIGNMENT;
            CREATE CAST (int4range AS pdb.alias) WITH FUNCTION pdb.int4range_to_alias AS ASSIGNMENT;
            CREATE CAST (int8range AS pdb.alias) WITH FUNCTION pdb.int8range_to_alias AS ASSIGNMENT;
        "#,
        name = "alias_cast",
        requires = [
            tokenize_alias,
            "alias_definition",
            smallint_to_alias,
            integer_to_alias,
            bigint_to_alias,
            oid_to_alias,
            float4_to_alias,
            float8_to_alias,
            boolean_to_alias,
            date_to_alias,
            time_to_alias,
            timestamp_to_alias,
            timestamp_with_time_zone_to_alias,
            time_with_time_zone_to_alias,
            inet_to_alias,
            int4range_to_alias,
            int8range_to_alias,
        ]
    );
}

#[pg_extern(immutable, parallel_safe)]
fn literal_typmod_in<'a>(typmod_parts: Array<'a, &'a CStr>) -> i32 {
    let parsed_typmod = ParsedTypmod::try_from(&typmod_parts).unwrap();
    if parsed_typmod.len() == 1 && matches!(parsed_typmod[0].key(), Some("alias")) {
        drop(parsed_typmod);
        return save_typmod(typmod_parts.iter()).expect("should not fail to save typmod");
    }

    ErrorReport::new(
        PgSqlErrorCode::ERRCODE_SYNTAX_ERROR,
        "type modifier is not allowed for type \"literal\"",
        function_name!(),
    )
    .report(PgLogLevel::ERROR);
    unreachable!()
}

extension_sql!(
    r#"
        ALTER TYPE pdb.literal SET (TYPMOD_IN = literal_typmod_in);
    "#,
    name = "literal_typmod",
    requires = [literal_typmod_in, "literal_definition"]
);
