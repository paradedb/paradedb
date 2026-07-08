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
use std::ffi::{CStr, CString};

#[pgrx::pg_schema]
pub(crate) mod pdb {
    use crate::api::operator::{boost, const_score, fuzzy, slop};
    use crate::api::tokenizers::{
        tokenize, DatumWrapper, GenericTypeWrapper, JsonMarker, JsonbMarker, SqlNameMarker,
        TextArrayMarker, UuidMarker, VarcharArrayMarker,
    };
    use macros::generate_tokenizer_sql;
    use paste::paste;
    use pgrx::callconv::{Arg, ArgAbi, BoxRet, FcInfo};
    use pgrx::nullable::Nullable;
    use pgrx::pgrx_sql_entity_graph::metadata::{
        ArgumentError, ReturnsError, ReturnsRef, SqlMappingRef, SqlTranslatable, TypeOrigin,
    };
    use pgrx::{extension_sql, pg_extern, pg_sys, FromDatum, IntoDatum};
    use std::ffi::CString;
    use tokenizers::manager::{LinderaLanguage, SearchTokenizerFilters};
    use tokenizers::SearchTokenizer;

    pub trait TokenizerCtor {
        fn make_search_tokenizer() -> SearchTokenizer;
    }

    /// Internal structure to store bytes from any type along with the info
    /// needed to recover what type it is.
    #[repr(C)]
    pub struct DatumWithType {
        // varlena header
        vl_len_: pg_sys::varlena,
        // contents, separate for convenience with vardata functions.
        contents: DatumWithTypeContents,
    }

    #[repr(C)]
    struct DatumWithTypeContents {
        // magic number to identify wrapped datums
        magic: u32,
        // Metadata needed when extracting the wrapped data
        elem_length: i16,
        elem_by_val: bool,
        // Reserved padding; always written as 0.
        _padding: u8,
        // original type OID
        typoid: pg_sys::Oid,
        // underlying types data goes here
    }

    // Check that there's no extra padding
    const _: () = assert!(
        std::mem::size_of::<DatumWithType>()
            == std::mem::size_of::<i32>()
                + std::mem::size_of::<u32>()
                + std::mem::size_of::<i16>()
                + std::mem::size_of::<bool>()
                + std::mem::size_of::<u8>()
                + std::mem::size_of::<pg_sys::Oid>()
    );

    // Magic number: "err\0" - includes null bytes to ensure no valid text string can match
    // (PostgreSQL text values cannot contain embedded nulls). Prints as the string "err" if
    // interpreted as TEXT, making it a bit easier to catch.
    const ALIAS_MAGIC: u32 = u32::from_ne_bytes([b'e', b'r', b'r', b'\0']);

    impl DatumWithType {
        unsafe fn new(mut datum: pg_sys::Datum, typoid: pg_sys::Oid) -> *mut Self {
            use pgrx::varlena::varsize_any;
            use std::mem::size_of;

            // load metadata about the underlying type.
            let mut elem_length = 0;
            let mut elem_by_val = false;
            // Check that the data section is sufficiently aligned to store any
            // type. So we don't need to use `elem_align`
            const _: () = assert!(std::mem::size_of::<DatumWithType>().is_multiple_of(8));
            let mut _elem_align = 0;
            pg_sys::get_typlenbyvalalign(
                typoid,
                &mut elem_length,
                &mut elem_by_val,
                &mut _elem_align,
            );

            // Ensure that the data isn't TOASTed.
            if elem_length == -1 {
                datum = pg_sys::pg_detoast_datum(datum.cast_mut_ptr()).into();
            }

            // based on Postgres's `att_addlength_datum`
            // (specifically `att_addlength_pointer` which it calls)
            let data_len = if elem_length > 0 {
                elem_length as usize
            } else if elem_length == -1 {
                varsize_any(datum.cast_mut_ptr::<pg_sys::varlena>())
            } else {
                unreachable!("tried to pass a C string as a tokenizer")
            };

            // DatumWithType is a multiple of 8bytes (the largest alignment
            // postgres uses) so we don't need any additional aligning.
            let size = size_of::<DatumWithType>() + data_len;
            let ptr = pg_sys::palloc(size).cast::<DatumWithType>();

            *ptr = DatumWithType {
                vl_len_: pg_sys::varlena::default(),
                contents: DatumWithTypeContents {
                    // Set the magic number to identify this as a wrapped datum
                    // This prevents false positives when text happens to be the same size
                    magic: ALIAS_MAGIC,
                    elem_length,
                    elem_by_val,
                    _padding: 0,
                    // set the original type OID and the actual datum value
                    typoid,
                },
            };

            // Set the varlena header so Postgres knows the length.
            pgrx::set_varsize_4b(&mut (*ptr).vl_len_, size as i32);

            // Copy in the data for the underlying type
            // (based on Postgres's `ArrayCastAndSet()`).
            let data_ptr = ptr.add(1).cast::<u8>();
            if elem_by_val {
                crate::postgres::utils::store_att_byval(
                    data_ptr.cast(),
                    datum,
                    data_len.try_into().unwrap(),
                );
            } else {
                data_ptr.copy_from(datum.cast_mut_ptr(), data_len);
            }

            ptr
        }

        /// Returns the a Datum for the underlying type,
        /// along with its type Oid if it isn't a text type
        /// SAFETY: datum must be a pointer to a valid `DatumWithType`
        pub unsafe fn get_underlying_type(
            this: pg_sys::Datum,
        ) -> (pg_sys::Datum, Option<pg_sys::Oid>) {
            use std::mem::offset_of;

            // It would be nice to use pg_detoast_datum_packed(), but then if the
            // stored type was using the 4B header reading it could be unaligned.
            let ptr = pg_sys::pg_detoast_datum(this.cast_mut_ptr());
            debug_assert!(!ptr.is_null());

            let vl_len = pgrx::varlena::varsize_any(ptr.cast::<pg_sys::varlena>());
            let minimum_size = std::mem::size_of::<DatumWithTypeContents>();

            // Check both size AND magic number to avoid false positives.
            // NOTE: False negatives are less risky than false positives since
            //       any element of this type is prefixed with the valid C string
            //       `err`
            if vl_len < minimum_size
                || pgrx::varlena::vardata_any(ptr)
                    .byte_add(offset_of!(DatumWithTypeContents, magic))
                    .cast::<u32>()
                    .read_unaligned()
                    != ALIAS_MAGIC
            {
                // Must have been a text type, return the detoasted value as-is
                return (pg_sys::Datum::from(ptr), None);
            }

            let contents_ptr = pgrx::varlena::vardata_any(ptr).cast::<DatumWithTypeContents>();
            let contents = contents_ptr.read();

            let data_ptr = contents_ptr.add(1).cast();
            let underlying = crate::postgres::utils::fetch_att(
                data_ptr,
                contents.elem_by_val,
                contents.elem_length.into(),
            );
            (underlying, Some(contents.typoid))
        }
    }

    unsafe fn wrap_generic_type<InTy, InMarker, OutTy, OutMarker>(
        input: GenericTypeWrapper<InTy, InMarker>,
    ) -> GenericTypeWrapper<OutTy, OutMarker>
    where
        InTy: DatumWrapper,
        OutTy: DatumWrapper,
        InMarker: SqlNameMarker,
        OutMarker: SqlNameMarker,
    {
        // Wrap datum and original typoid in a custom structure
        let wrapper_ptr = unsafe { DatumWithType::new(input.datum, input.typoid) };

        // Return the wrapper with the original typoid preserved
        // This allows PostgreSQL to track array vs scalar types correctly
        GenericTypeWrapper::new(pg_sys::Datum::from(wrapper_ptr), input.typoid)
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
                    mut arr: GenericTypeWrapper<$rust_ty, [<$marker Marker>]>,
                ) -> GenericTypeWrapper<Alias, AliasMarker> {
                    // TODO probably not needed but maintains old behavior
                    arr.typoid = $typoid;
                    wrap_generic_type(arr)
                }
            }
        };
    }

    macro_rules! define_tokenizer_type {
        ($def_name:literal, $rust_name:ident, $tokenizer_conf:expr, $cast_name:ident, $json_cast_name:ident, $jsonb_cast_name:ident, $uuid_cast_name:ident, $text_array_cast_name:ident, $varchar_array_cast_name:ident, $sql_name:literal, preferred = $preferred:literal, custom_typmod = $custom_typmod:literal) => {
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
                const TYPE_IDENT: &'static str = pgrx::pgrx_resolved_type!($rust_name);
                const TYPE_ORIGIN: TypeOrigin = TypeOrigin::External;
                const ARGUMENT_SQL: Result<SqlMappingRef, ArgumentError> =
                    Ok(SqlMappingRef::literal(concat!("pdb.", $sql_name)));
                const RETURN_SQL: Result<ReturnsRef, ReturnsError> =
                    Ok(ReturnsRef::One(SqlMappingRef::literal(concat!("pdb.", $sql_name))));
            }

            #[pg_extern(immutable, parallel_safe, requires = [$def_name])]
            fn $cast_name(s: $rust_name, fcinfo: pg_sys::FunctionCallInfo) -> Vec<String> {
                let mut tokenizer = $rust_name::make_search_tokenizer();

                unsafe {
                    let func_expr = (*(*fcinfo).flinfo).fn_expr.cast::<pg_sys::FuncExpr>();
                    let args = pgrx::PgList::<pg_sys::Node>::from_pg((*func_expr).args.cast());
                    let first_arg = args.get_ptr(0).unwrap();
                    let typmod = pg_sys::exprTypmod(first_arg);

                    super::super::apply_typmod(&mut tokenizer, typmod);
                }

                unsafe { tokenize(s, tokenizer) }
            }

            paste! {

                struct [<$rust_name TextMarker>];
                impl SqlNameMarker for [<$rust_name TextMarker>] {
                    const SQL_NAME: &'static str = concat!("pdb.", $sql_name);
                }

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

                #[pg_extern(immutable, strict, parallel_safe, requires = [$def_name])] // TODO handle typmod
                fn [<$sql_name _in>](s: &std::ffi::CStr) -> GenericTypeWrapper<$rust_name, [<$rust_name TextMarker>]> {
                    // TEXT and bytea share the same underlying layout,
                    // so the bytea functions provide a convenient way to create
                    // TEXT without UTF-8 conversion
                    let text = pgrx::rust_byte_slice_to_bytea(s.to_bytes());
                    let wrapper_ptr = unsafe { DatumWithType::new(text.into_datum().unwrap(), pg_sys::TEXTOID) };

                    GenericTypeWrapper::new(pg_sys::Datum::from(wrapper_ptr), pg_sys::TEXTOID)
                }

                #[pg_extern(immutable, strict, parallel_safe, requires = [$def_name])]
                fn [<$sql_name _out>](s: $rust_name) -> &'static std::ffi::CStr  {
                    let wrapper_datum = s.as_datum();
                    unsafe {
                        let (underlying, typ) = DatumWithType::get_underlying_type(wrapper_datum);
                        match typ {
                            None => {
                                    let ptr = underlying.cast_mut_ptr();
                                    let cstr = pgrx::pg_sys::text_to_cstring(ptr);
                                    std::ffi::CStr::from_ptr(cstr)
                            }
                            Some(typoid) => {
                                let mut typoutput = 0.into();
                                let mut typ_is_varlena = true;
                                pg_sys::getTypeOutputInfo(
                                    typoid,
                                    &mut typoutput,
                                    &mut typ_is_varlena);
                                let cstr = pg_sys::OidOutputFunctionCall(typoutput, underlying);
                                std::ffi::CStr::from_ptr(cstr)
                            },
                        }
                    }
                }

                #[pg_extern(immutable, parallel_safe, requires = [ $cast_name ])]
                fn $json_cast_name(
                    json: GenericTypeWrapper<pgrx::Json, JsonMarker>,
                ) -> GenericTypeWrapper<$rust_name, [<$rust_name JsonMarker>]> {
                    unsafe { wrap_generic_type(json) }
                }

                #[pg_extern(immutable, parallel_safe, requires = [ $cast_name ])]
                unsafe fn $jsonb_cast_name(
                    jsonb: GenericTypeWrapper<pgrx::JsonB, JsonbMarker>,
                ) -> GenericTypeWrapper<$rust_name, [<$rust_name JsonbMarker>]> {
                    unsafe { wrap_generic_type(jsonb) }
                }

                #[pg_extern(immutable, parallel_safe, requires = [ $cast_name ])]
                unsafe fn $uuid_cast_name(
                    uuid: GenericTypeWrapper<pgrx::datum::Uuid, UuidMarker>,
                ) -> GenericTypeWrapper<$rust_name, [<$rust_name UuidMarker>]> {
                    unsafe { wrap_generic_type(uuid) }
                }

                #[pg_extern(immutable, parallel_safe, requires = [ $cast_name ])]
                unsafe fn $text_array_cast_name(
                    arr: GenericTypeWrapper<Vec<String>, TextArrayMarker>,
                ) -> GenericTypeWrapper<$rust_name, [<$rust_name TextArrayMarker>]> {
                    unsafe { wrap_generic_type(arr) }
                }

                #[pg_extern(immutable, parallel_safe, requires = [ $cast_name ])]
                unsafe fn $varchar_array_cast_name(
                    arr: GenericTypeWrapper<Vec<String>, VarcharArrayMarker>,
                ) -> GenericTypeWrapper<$rust_name, [<$rust_name VarcharArrayMarker>]> {
                    unsafe { wrap_generic_type(arr) }
                }

                #[pg_extern(immutable, parallel_safe, requires = [ $cast_name ])]
                fn [<$sql_name _to_boost>](input: $rust_name, typmod: i32, is_explicit: bool, fcinfo: pg_sys::FunctionCallInfo) -> boost::BoostType {
                    let tokens = $cast_name(input, fcinfo);
                    boost::text_array_to_boost(tokens, typmod, is_explicit)
                }

                #[pg_extern(immutable, parallel_safe, requires = [ $cast_name ])]
                fn [<$sql_name _to_const>](input: $rust_name, typmod: i32, is_explicit: bool, fcinfo: pg_sys::FunctionCallInfo) -> const_score::ConstType {
                    let tokens = $cast_name(input, fcinfo);
                    const_score::text_array_to_const(tokens, typmod, is_explicit)
                }

                #[pg_extern(immutable, parallel_safe, requires = [ $cast_name ])]
                fn [<$sql_name _to_fuzzy>](input: $rust_name, typmod: i32, is_explicit: bool, fcinfo: pg_sys::FunctionCallInfo) -> fuzzy::FuzzyType {
                    let tokens = $cast_name(input, fcinfo);
                    fuzzy::text_array_to_fuzzy(tokens, typmod, is_explicit)
                }

                #[pg_extern(immutable, parallel_safe, requires = [ $cast_name ])]
                fn [<$sql_name _to_slop>](input: $rust_name, typmod: i32, is_explicit: bool, fcinfo: pg_sys::FunctionCallInfo) -> slop::SlopType {
                    let tokens = $cast_name(input, fcinfo);
                    slop::text_array_to_slop(tokens, typmod, is_explicit)
                }
            }

            generate_tokenizer_sql!(
                def_name = $def_name,
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
        "AliasDef",
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
        custom_typmod = true
    );

    define_tokenizer_type!(
        "SimpleDef",
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
        "WhitespaceDef",
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
        "LiteralDef",
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
        "LiteralNormalizedDef",
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
        "ChineseCompatibleDef",
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
        "LinderaDef",
        Lindera,
        SearchTokenizer::Lindera {
            language: LinderaLanguage::Chinese,
            filters: SearchTokenizerFilters::default(),
            keep_whitespace: false,
            nfkc: false,
            reading_form: false,
        },
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
        "JiebaDef",
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
        "SourceCodeDef",
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
        "IcuDef",
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
        "NgramDef",
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
        "EdgeNgramDef",
        EdgeNgram,
        SearchTokenizer::EdgeNgram {
            min_gram: 1,
            max_gram: 2,
            token_chars: vec!["letter".to_string(), "digit".to_string()],
            filters: SearchTokenizerFilters::default(),
        },
        tokenize_edge_ngram,
        json_to_edge_ngram,
        jsonb_to_edge_ngram,
        uuid_to_edge_ngram,
        text_array_to_edge_ngram,
        varchar_array_to_edge_ngram,
        "edge_ngram",
        preferred = false,
        custom_typmod = false
    );

    define_tokenizer_type!(
        "RegexDef",
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
        "UnicodeWordsDef",
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
        ALTER TYPE pdb.literal SET (TYPMOD_IN = literal_typmod_in, TYPMOD_OUT = generic_typmod_out);
    "#,
    name = "literal_typmod",
    requires = [literal_typmod_in, generic_typmod_out, "literal_definition"]
);

#[pg_extern(immutable, parallel_safe)]
fn alias_typmod_in<'a>(typmod_parts: Array<'a, &'a CStr>) -> i32 {
    let parts: Vec<_> = typmod_parts.iter().collect();

    if parts.len() != 1 {
        ErrorReport::new(
            PgSqlErrorCode::ERRCODE_SYNTAX_ERROR,
            "pdb.alias requires exactly one argument",
            function_name!(),
        )
        .report(PgLogLevel::ERROR);
        unreachable!()
    }

    let raw = parts[0]
        .expect("pdb.alias requires a name argument")
        .to_str()
        .expect("alias name must be valid utf-8");

    let normalized = if raw.starts_with("alias=") {
        CString::new(raw).unwrap()
    } else {
        CString::new(format!("alias={raw}")).unwrap()
    };

    save_typmod(std::iter::once(Some(normalized.as_c_str())))
        .expect("should not fail to save typmod")
}

extension_sql!(
    r#"
        ALTER TYPE pdb.alias SET (TYPMOD_IN = alias_typmod_in, TYPMOD_OUT = generic_typmod_out);
    "#,
    name = "alias_typmod",
    requires = [alias_typmod_in, generic_typmod_out, "alias_definition"]
);
