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

use crate::api::tokenizers::typmod::{save_typmod, ParsedTypmod};
use pgrx::pg_sys::panic::ErrorReport;
use pgrx::{extension_sql, function_name, pg_extern, Array, PgLogLevel, PgSqlErrorCode};
use std::ffi::CStr;

#[pgrx::pg_schema]
pub(crate) mod pdb {
    use crate::api::tokenizers::{
        CowString, DatumWrapper, GenericTypeWrapper, JsonMarker, JsonbMarker, SqlNameMarker,
        TextArrayMarker, UuidMarker, VarcharArrayMarker,
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

    /// Internal structure to store both the datum and its original type OID
    /// This allows the output function to properly convert any type to text
    ///
    /// # IMPORTANT: Lifetime and Memory Safety Assumptions
    ///
    /// This structure stores a raw `pg_sys::Datum` value. For pass-by-value types (integers,
    /// booleans, etc.), the datum contains the actual value and is safe to copy. However, for
    /// varlena types (text, arrays, etc.), the datum is a **pointer** to memory that may be:
    ///
    /// - In a temporary memory context that could be freed
    /// - TOASTed data whose detoasted copy could be freed
    /// - Part of a tuple that gets freed after a function returns
    ///
    /// **Current Assumption**: We assume the input datum's lifetime extends beyond the wrapper's
    /// lifetime. This holds true for:
    /// - Index expressions: the datum comes from heap tuples during index build/insert
    /// - Query execution: datums are in memory contexts that outlive the expression evaluation
    ///
    /// **Potential Issue**: If a wrapper is created with a datum from a temporary context and
    /// then used after that context is destroyed, we'll have a dangling pointer leading to:
    /// - Use-after-free bugs
    /// - Segfaults when the output function tries to dereference the datum
    /// - Heap corruption if the freed memory is reused
    ///
    /// **TODO**: Consider using `pg_sys::datumCopy()` for varlena types to ensure we own the
    /// data, along with proper cleanup when the wrapper is freed. This would make the wrapper
    /// fully self-contained and safe regardless of the input datum's lifetime.
    #[repr(C)]
    pub struct AliasDatumWithType {
        vl_len_: i32,               // varlena header
        magic: u32,                 // magic number to identify wrapped datums
        typoid: pg_sys::Oid,        // original type OID
        datum_value: pg_sys::Datum, // the actual datum value (RAW POINTER for varlena types!)
    }

    // Magic number: "AL\0S" - includes a null byte to ensure no valid text string can match
    // PostgreSQL text values cannot contain embedded nulls, making false positives impossible
    const ALIAS_MAGIC: u32 = 0x414C0053; // 'A', 'L', 0x00, 'S'

    impl AliasDatumWithType {
        unsafe fn new(datum: pg_sys::Datum, typoid: pg_sys::Oid) -> *mut Self {
            let size = std::mem::size_of::<AliasDatumWithType>();
            let ptr = pg_sys::palloc(size) as *mut AliasDatumWithType;

            // Since pdb.alias is defined as LIKE = text, PostgreSQL treats it as a varlena
            // (variable-length) type. All varlena types must have a valid size header in the
            // first 4 bytes (vl_len_). This header includes:
            //   1. The total size of the structure (including the header itself)
            //   2. Special bit flags used by PostgreSQL for TOAST and compression
            //
            // set_varsize_4b encodes this information correctly. Without it:
            //   - PostgreSQL wouldn't know where our data ends (memory corruption)
            //   - TOAST (oversized-attribute storage) would fail
            //   - Copying/serializing the datum would cause segfaults
            pgrx::set_varsize_4b(ptr.cast(), size as i32);

            // Set the magic number to identify this as a wrapped datum
            // This prevents false positives when text happens to be the same size
            (*ptr).magic = ALIAS_MAGIC;

            // set the original type OID and the actual datum value
            (*ptr).typoid = typoid;
            (*ptr).datum_value = datum;

            ptr
        }

        /// Check if a datum is a wrapped AliasDatumWithType by verifying size and magic number
        pub unsafe fn is_wrapped(wrapper: pg_sys::Datum) -> bool {
            let ptr = wrapper.cast_mut_ptr::<AliasDatumWithType>();
            if ptr.is_null() {
                return false;
            }

            let vl_len = pgrx::varlena::varsize_any(ptr.cast::<pg_sys::varlena>());
            let expected_size = std::mem::size_of::<AliasDatumWithType>();

            // Check both size AND magic number to avoid false positives
            vl_len == expected_size && (*ptr).magic == ALIAS_MAGIC
        }

        pub unsafe fn extract_datum(wrapper: pg_sys::Datum) -> pg_sys::Datum {
            let ptr = wrapper.cast_mut_ptr::<AliasDatumWithType>();
            (*ptr).datum_value
        }

        unsafe fn extract_typoid(wrapper: pg_sys::Datum) -> pg_sys::Oid {
            let ptr = wrapper.cast_mut_ptr::<AliasDatumWithType>();
            (*ptr).typoid
        }
    }

    macro_rules! cast_alias {
        ($sql_name:literal, $marker:ident, $rust_ty:ty, $fn_prefix:ident, $typoid:expr) => {
            paste::paste! {
                struct [<$marker Marker>];

                impl SqlNameMarker for [<$marker Marker>] {
                    const SQL_NAME: &'static str = $sql_name;
                }

                #[pg_extern(immutable, parallel_safe, requires = [tokenize_alias])]
                unsafe fn [<$fn_prefix _to_alias>](
                    arr: GenericTypeWrapper<$rust_ty, [<$marker Marker>]>,
                ) -> GenericTypeWrapper<Alias, AliasMarker> {
                    let original_typoid: pg_sys::Oid = $typoid;

                    // Wrap datum and original typoid in a custom structure
                    let wrapper_ptr = AliasDatumWithType::new(arr.datum, original_typoid);

                    // Return the wrapper with the original typoid preserved
                    // This allows PostgreSQL to track array vs scalar types correctly
                    GenericTypeWrapper::new(pg_sys::Datum::from(wrapper_ptr), original_typoid)
                }
            }
        };
    }

    macro_rules! define_tokenizer_type {
        ($rust_name:ident, $tokenizer_conf:expr, $cast_name:ident, $json_cast_name:ident, $jsonb_cast_name:ident, $uuid_cast_name:ident, $text_array_cast_name:ident, $varchar_array_cast_name:ident, $sql_name:literal, preferred = $preferred:literal, custom_typmod = $custom_typmod:literal) => {
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

                struct [<$rust_name UuidMarker>];
                impl SqlNameMarker for [<$rust_name UuidMarker>] {
                    const SQL_NAME: &'static str = concat!("pdb.", $sql_name);
                }

                #[pg_extern(immutable, parallel_safe, requires = [ $cast_name ])]
                fn $json_cast_name(
                    json: GenericTypeWrapper<pgrx::Json, JsonMarker>,
                ) -> GenericTypeWrapper<$rust_name, [<$rust_name JsonMarker>]> {
                    GenericTypeWrapper::new(json.datum, json.typoid)
                }

                #[pg_extern(immutable, parallel_safe, requires = [ $cast_name ])]
                unsafe fn $jsonb_cast_name(
                    jsonb: GenericTypeWrapper<pgrx::JsonB, JsonbMarker>,
                ) -> GenericTypeWrapper<$rust_name, [<$rust_name JsonbMarker>]> {
                    GenericTypeWrapper::new(jsonb.datum, jsonb.typoid)
                }

                #[pg_extern(immutable, parallel_safe, requires = [ $cast_name ])]
                unsafe fn $uuid_cast_name(
                    uuid: GenericTypeWrapper<pgrx::datum::Uuid, UuidMarker>,
                ) -> GenericTypeWrapper<$rust_name, [<$rust_name UuidMarker>]> {
                    GenericTypeWrapper::new(uuid.datum, uuid.typoid)
                }

                #[pg_extern(immutable, parallel_safe, requires = [ $cast_name ])]
                unsafe fn $text_array_cast_name(
                    arr: GenericTypeWrapper<Vec<String>, TextArrayMarker>,
                ) -> GenericTypeWrapper<$rust_name, [<$rust_name TextArrayMarker>]> {
                    GenericTypeWrapper::new(arr.datum, arr.typoid)
                }

                #[pg_extern(immutable, parallel_safe, requires = [ $cast_name ])]
                unsafe fn $varchar_array_cast_name(
                    arr: GenericTypeWrapper<Vec<String>, VarcharArrayMarker>,
                ) -> GenericTypeWrapper<$rust_name, [<$rust_name VarcharArrayMarker>]> {
                    GenericTypeWrapper::new(arr.datum, arr.typoid)
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
                uuid_cast_name = $uuid_cast_name,
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
        uuid_to_alias,
        text_array_to_alias,
        varchar_array_to_alias,
        "alias",
        preferred = false,
        custom_typmod = false
    );

    /// Output function for pdb.alias that unwraps the stored datum and converts to text
    #[pg_extern(immutable, strict, parallel_safe)]
    fn alias_out_safe(input: Alias) -> std::ffi::CString {
        unsafe {
            let wrapper_datum = input.as_datum();

            // Check if this is a wrapped AliasDatumWithType using magic number verification
            // For text literals, PostgreSQL might pass them directly without wrapping
            // due to LIKE = text in the type definition
            if AliasDatumWithType::is_wrapped(wrapper_datum) {
                let typoid = AliasDatumWithType::extract_typoid(wrapper_datum);
                let original_datum = AliasDatumWithType::extract_datum(wrapper_datum);

                // Get the output function for the original type
                let mut typoutput: pg_sys::Oid = pg_sys::InvalidOid;
                let mut is_varlena: bool = false;
                pg_sys::getTypeOutputInfo(typoid, &mut typoutput, &mut is_varlena);

                // Call the type's output function to get the C string
                let cstring_ptr = pg_sys::OidOutputFunctionCall(typoutput, original_datum);
                std::ffi::CStr::from_ptr(cstring_ptr).to_owned()
            } else {
                // Not wrapped, it's raw text (or null)
                let ptr = wrapper_datum.cast_mut_ptr::<pg_sys::varlena>();
                if ptr.is_null() {
                    return std::ffi::CString::new("").unwrap();
                }

                // Convert the text varlena to a string using pgrx utilities
                let text_len = pgrx::varlena::varsize_any_exhdr(ptr);
                let text_data = pgrx::varlena::vardata_any(ptr);
                let text_slice = std::slice::from_raw_parts(text_data as *const u8, text_len);
                let text_str = std::str::from_utf8_unchecked(text_slice);
                std::ffi::CString::new(text_str)
                    .unwrap_or_else(|_| std::ffi::CString::new("").unwrap())
            }
        }
    }

    extension_sql!(
        r#"
        -- Override the output function for pdb.alias
        CREATE OR REPLACE FUNCTION pdb.alias_out(pdb.alias) RETURNS cstring
            AS 'MODULE_PATHNAME', 'alias_out_safe_wrapper'
            LANGUAGE c IMMUTABLE STRICT PARALLEL SAFE;
        "#,
        name = "alias_out_safe_override",
        requires = [tokenize_alias, alias_out_safe]
    );

    define_tokenizer_type!(
        Simple,
        SearchTokenizer::Simple(SearchTokenizerFilters::default()),
        tokenize_simple,
        json_to_simple,
        jsonb_to_simple,
        uuid_to_simple,
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
        uuid_to_whitespace,
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
        uuid_to_literal,
        text_array_to_literal,
        varchar_array_to_literal,
        "literal",
        preferred = false,
        custom_typmod = true
    );

    define_tokenizer_type!(
        Normalized,
        SearchTokenizer::LiteralNormalized(SearchTokenizerFilters::default()),
        tokenize_normalized,
        json_to_normalized,
        jsonb_to_normalized,
        uuid_to_normalized,
        text_array_to_normalized,
        varchar_array_to_normalized,
        "normalized",
        preferred = false,
        custom_typmod = false
    );

    // Compatibility alias kept for existing indexes/queries.
    define_tokenizer_type!(
        LiteralNormalized,
        SearchTokenizer::LiteralNormalized(SearchTokenizerFilters::default()),
        tokenize_literal_normalized,
        json_to_literal_normalized,
        jsonb_to_literal_normalized,
        uuid_to_literal_normalized,
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
        uuid_to_chinese_compatible,
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
        uuid_to_lindera,
        text_array_to_lindera,
        varchar_array_to_lindera,
        "lindera",
        preferred = false,
        custom_typmod = false
    );

    define_tokenizer_type!(
        Jieba,
        SearchTokenizer::Jieba {
            chinese_convert: None,
            filters: SearchTokenizerFilters::default(),
        },
        tokenize_jieba,
        json_to_jieba,
        jsonb_to_jieba,
        uuid_to_jieba,
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
        uuid_to_source_code,
        text_array_to_source_code,
        varchar_array_to_source_code,
        "source_code",
        preferred = false,
        custom_typmod = false
    );

    define_tokenizer_type!(
        Icu,
        SearchTokenizer::ICUTokenizer(SearchTokenizerFilters::default()),
        tokenize_icu,
        json_to_icu,
        jsonb_to_icu,
        uuid_to_icu,
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
            positions: false,
            filters: SearchTokenizerFilters::default(),
        },
        tokenize_ngram,
        json_to_ngram,
        jsonb_to_ngram,
        uuid_to_ngram,
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
        uuid_to_regex_pattern,
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
        uuid_to_unicode_words,
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

    // allow the following types to be cast to `pdb.alias` at CREATE INDEX time
    cast_alias!("text", Text, String, text, pg_sys::TEXTOID);
    cast_alias!("varchar", Varchar, String, varchar, pg_sys::VARCHAROID);
    cast_alias!("smallint", SmallInt, i16, smallint, pg_sys::INT2OID);
    cast_alias!("integer", Integer, i32, integer, pg_sys::INT4OID);
    cast_alias!("bigint", BigInt, i64, bigint, pg_sys::INT8OID);
    cast_alias!("oid", Oid, u32, oid, pg_sys::OIDOID);
    cast_alias!("float4", Float4, f32, float4, pg_sys::FLOAT4OID);
    cast_alias!("float8", Float8, f64, float8, pg_sys::FLOAT8OID);
    cast_alias!(
        "numeric",
        Numeric,
        pgrx::datum::AnyNumeric,
        numeric,
        pg_sys::NUMERICOID
    );
    cast_alias!("boolean", Boolean, bool, boolean, pg_sys::BOOLOID);
    cast_alias!("date", Date, pgrx::datum::Date, date, pg_sys::DATEOID);
    cast_alias!("time", Time, pgrx::datum::Time, time, pg_sys::TIMEOID);
    cast_alias!(
        "timestamp",
        Timestamp,
        pgrx::datum::Timestamp,
        timestamp,
        pg_sys::TIMESTAMPOID
    );
    cast_alias!(
        "timestamp with time zone",
        TimestampWithTimeZone,
        pgrx::datum::TimestampWithTimeZone,
        timestamp_with_time_zone,
        pg_sys::TIMESTAMPTZOID
    );
    cast_alias!(
        "time with time zone",
        TimeWithTimeZone,
        pgrx::datum::TimeWithTimeZone,
        time_with_time_zone,
        pg_sys::TIMETZOID
    );

    cast_alias!("inet", Inet, pgrx::datum::Inet, inet, pg_sys::INETOID);
    cast_alias!(
        "int4range",
        Int4Range,
        pgrx::datum::Range<i32>,
        int4range,
        pg_sys::INT4RANGEOID
    );
    cast_alias!(
        "int8range",
        Int8Range,
        pgrx::datum::Range<i64>,
        int8range,
        pg_sys::INT8RANGEOID
    );
    cast_alias!(
        "numrange",
        NumRange,
        pgrx::datum::Range<pgrx::datum::AnyNumeric>,
        numrange,
        pg_sys::NUMRANGEOID
    );
    cast_alias!(
        "daterange",
        DateRange,
        pgrx::datum::Range<pgrx::datum::Date>,
        daterange,
        pg_sys::DATERANGEOID
    );
    cast_alias!(
        "tsrange",
        TsRange,
        pgrx::datum::Range<pgrx::datum::Timestamp>,
        tsrange,
        pg_sys::TSRANGEOID
    );
    cast_alias!(
        "tstzrange",
        TstzRange,
        pgrx::datum::Range<pgrx::datum::TimestampWithTimeZone>,
        tstzrange,
        pg_sys::TSTZRANGEOID
    );
    cast_alias!(
        "smallint[]",
        SmallIntArray,
        Vec<i16>,
        smallint_array,
        pg_sys::INT2ARRAYOID
    );
    cast_alias!(
        "integer[]",
        IntegerArray,
        Vec<i32>,
        integer_array,
        pg_sys::INT4ARRAYOID
    );
    cast_alias!(
        "bigint[]",
        BigIntArray,
        Vec<i64>,
        bigint_array,
        pg_sys::INT8ARRAYOID
    );
    cast_alias!(
        "float4[]",
        Float4Array,
        Vec<f32>,
        float4_array,
        pg_sys::FLOAT4ARRAYOID
    );
    cast_alias!(
        "float8[]",
        Float8Array,
        Vec<f64>,
        float8_array,
        pg_sys::FLOAT8ARRAYOID
    );
    cast_alias!(
        "numeric[]",
        NumericArray,
        Vec<pgrx::datum::AnyNumeric>,
        numeric_array,
        pg_sys::NUMERICARRAYOID
    );
    cast_alias!(
        "boolean[]",
        BooleanArray,
        Vec<bool>,
        boolean_array,
        pg_sys::BOOLARRAYOID
    );
    cast_alias!(
        "date[]",
        DateArray,
        Vec<pgrx::datum::Date>,
        date_array,
        pg_sys::DATEARRAYOID
    );
    cast_alias!(
        "time[]",
        TimeArray,
        Vec<pgrx::datum::Time>,
        time_array,
        pg_sys::TIMEARRAYOID
    );
    cast_alias!(
        "timestamp[]",
        TimestampArray,
        Vec<pgrx::datum::Timestamp>,
        timestamp_array,
        pg_sys::TIMESTAMPARRAYOID
    );
    cast_alias!(
        "timestamp with time zone[]",
        TimestampWithTimeZoneArray,
        Vec<pgrx::datum::TimestampWithTimeZone>,
        timestamp_with_time_zone_array,
        pg_sys::TIMESTAMPTZARRAYOID
    );
    cast_alias!(
        "time with time zone[]",
        TimeWithTimeZoneArray,
        Vec<pgrx::datum::TimeWithTimeZone>,
        time_with_time_zone_array,
        pg_sys::TIMETZARRAYOID
    );

    extension_sql!(
        r#"
        CREATE CAST (text AS pdb.alias) WITH FUNCTION pdb.text_to_alias AS ASSIGNMENT;
        CREATE CAST (varchar AS pdb.alias) WITH FUNCTION pdb.varchar_to_alias AS ASSIGNMENT;
        CREATE CAST (smallint AS pdb.alias) WITH FUNCTION pdb.smallint_to_alias AS ASSIGNMENT;
        CREATE CAST (integer AS pdb.alias) WITH FUNCTION pdb.integer_to_alias AS ASSIGNMENT;
        CREATE CAST (bigint AS pdb.alias) WITH FUNCTION pdb.bigint_to_alias AS ASSIGNMENT;
        CREATE CAST (oid AS pdb.alias) WITH FUNCTION pdb.oid_to_alias AS ASSIGNMENT;
        CREATE CAST (float4 AS pdb.alias) WITH FUNCTION pdb.float4_to_alias AS ASSIGNMENT;
        CREATE CAST (float8 AS pdb.alias) WITH FUNCTION pdb.float8_to_alias AS ASSIGNMENT;
        CREATE CAST (numeric AS pdb.alias) WITH FUNCTION pdb.numeric_to_alias AS ASSIGNMENT;
        CREATE CAST (boolean AS pdb.alias) WITH FUNCTION pdb.boolean_to_alias AS ASSIGNMENT;
        CREATE CAST (date AS pdb.alias) WITH FUNCTION pdb.date_to_alias AS ASSIGNMENT;
        CREATE CAST (time AS pdb.alias) WITH FUNCTION pdb.time_to_alias AS ASSIGNMENT;
        CREATE CAST (timestamp AS pdb.alias) WITH FUNCTION pdb.timestamp_to_alias AS ASSIGNMENT;
        CREATE CAST (timestamp with time zone AS pdb.alias) WITH FUNCTION pdb.timestamp_with_time_zone_to_alias AS ASSIGNMENT;
        CREATE CAST (time with time zone AS pdb.alias) WITH FUNCTION pdb.time_with_time_zone_to_alias AS ASSIGNMENT;
        CREATE CAST (inet AS pdb.alias) WITH FUNCTION pdb.inet_to_alias AS ASSIGNMENT;
        CREATE CAST (int4range AS pdb.alias) WITH FUNCTION pdb.int4range_to_alias AS ASSIGNMENT;
        CREATE CAST (int8range AS pdb.alias) WITH FUNCTION pdb.int8range_to_alias AS ASSIGNMENT;
        CREATE CAST (numrange AS pdb.alias) WITH FUNCTION pdb.numrange_to_alias AS ASSIGNMENT;
        CREATE CAST (daterange AS pdb.alias) WITH FUNCTION pdb.daterange_to_alias AS ASSIGNMENT;
        CREATE CAST (tsrange AS pdb.alias) WITH FUNCTION pdb.tsrange_to_alias AS ASSIGNMENT;
        CREATE CAST (tstzrange AS pdb.alias) WITH FUNCTION pdb.tstzrange_to_alias AS ASSIGNMENT;
        CREATE CAST (smallint[] AS pdb.alias) WITH FUNCTION pdb.smallint_array_to_alias AS ASSIGNMENT;
        CREATE CAST (integer[] AS pdb.alias) WITH FUNCTION pdb.integer_array_to_alias AS ASSIGNMENT;
        CREATE CAST (bigint[] AS pdb.alias) WITH FUNCTION pdb.bigint_array_to_alias AS ASSIGNMENT;
        CREATE CAST (float4[] AS pdb.alias) WITH FUNCTION pdb.float4_array_to_alias AS ASSIGNMENT;
        CREATE CAST (float8[] AS pdb.alias) WITH FUNCTION pdb.float8_array_to_alias AS ASSIGNMENT;
        CREATE CAST (numeric[] AS pdb.alias) WITH FUNCTION pdb.numeric_array_to_alias AS ASSIGNMENT;
        CREATE CAST (boolean[] AS pdb.alias) WITH FUNCTION pdb.boolean_array_to_alias AS ASSIGNMENT;
        CREATE CAST (date[] AS pdb.alias) WITH FUNCTION pdb.date_array_to_alias AS ASSIGNMENT;
        CREATE CAST (time[] AS pdb.alias) WITH FUNCTION pdb.time_array_to_alias AS ASSIGNMENT;
        CREATE CAST (timestamp[] AS pdb.alias) WITH FUNCTION pdb.timestamp_array_to_alias AS ASSIGNMENT;
        CREATE CAST (timestamp with time zone[] AS pdb.alias) WITH FUNCTION pdb.timestamp_with_time_zone_array_to_alias AS ASSIGNMENT;
        CREATE CAST (time with time zone[] AS pdb.alias) WITH FUNCTION pdb.time_with_time_zone_array_to_alias AS ASSIGNMENT;
        "#,
        name = "alias_casts",
        requires = [
            tokenize_alias,
            "alias_definition",
            text_to_alias,
            varchar_to_alias,
            smallint_to_alias,
            integer_to_alias,
            bigint_to_alias,
            oid_to_alias,
            float4_to_alias,
            float8_to_alias,
            numeric_to_alias,
            boolean_to_alias,
            date_to_alias,
            time_to_alias,
            timestamp_to_alias,
            timestamp_with_time_zone_to_alias,
            time_with_time_zone_to_alias,
            inet_to_alias,
            int4range_to_alias,
            int8range_to_alias,
            numrange_to_alias,
            daterange_to_alias,
            tsrange_to_alias,
            tstzrange_to_alias,
            text_array_to_alias,
            varchar_array_to_alias,
            smallint_array_to_alias,
            integer_array_to_alias,
            bigint_array_to_alias,
            float4_array_to_alias,
            float8_array_to_alias,
            numeric_array_to_alias,
            boolean_array_to_alias,
            date_array_to_alias,
            time_array_to_alias,
            timestamp_array_to_alias,
            timestamp_with_time_zone_array_to_alias,
            time_with_time_zone_array_to_alias
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
