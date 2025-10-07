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
mod pdb {
    use crate::api::tokenizers::{CowString, DatumWrapper};
    use macros::generate_tokenizer_sql;
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
        ($rust_name:ident, $tokenizer_conf:expr, $cast_name:ident, $sql_name:literal, preferred = $preferred:literal, custom_tymod = $custom_typmod:literal) => {
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

            generate_tokenizer_sql!(
                rust_name = $rust_name,
                sql_name = $sql_name,
                cast_name = $cast_name,
                preferred = $preferred,
                custom_typmod = $custom_typmod,
                schema = pdb
            );
        };
    }

    define_tokenizer_type!(
        Simple,
        SearchTokenizer::Default(SearchTokenizerFilters::default()),
        tokenize_simple,
        "simple",
        preferred = true,
        custom_tymod = false
    );

    define_tokenizer_type!(
        Whitespace,
        SearchTokenizer::WhiteSpace(SearchTokenizerFilters::default()),
        tokenize_whitespace,
        "whitespace",
        preferred = false,
        custom_tymod = false
    );

    define_tokenizer_type!(
        Exact,
        SearchTokenizer::Keyword,
        tokenize_exact,
        "exact",
        preferred = false,
        custom_tymod = true
    );

    define_tokenizer_type!(
        ChineseCompatabile,
        SearchTokenizer::ChineseCompatible(SearchTokenizerFilters::default()),
        tokenize_chinese_compatible,
        "chinese_compatible",
        preferred = false,
        custom_tymod = false
    );

    define_tokenizer_type!(
        Lindera,
        SearchTokenizer::Lindera(LinderaLanguage::Chinese, SearchTokenizerFilters::default()),
        tokenize_lindera,
        "lindera",
        preferred = false,
        custom_tymod = false
    );

    define_tokenizer_type!(
        Jieba,
        SearchTokenizer::Jieba(SearchTokenizerFilters::default()),
        tokenize_jieba,
        "jieba",
        preferred = false,
        custom_tymod = false
    );

    define_tokenizer_type!(
        SourceCode,
        SearchTokenizer::SourceCode(SearchTokenizerFilters::default()),
        tokenize_source_code,
        "source_code",
        preferred = false,
        custom_tymod = false
    );

    #[cfg(feature = "icu")]
    define_tokenizer_type!(
        ICU,
        SearchTokenizer::ICUTokenizer(SearchTokenizerFilters::default()),
        tokenize_icu,
        "icu",
        preferred = false,
        custom_tymod = false
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
        "ngram",
        preferred = false,
        custom_tymod = false
    );

    define_tokenizer_type!(
        Regex,
        SearchTokenizer::RegexTokenizer {
            pattern: ".*".to_string(),
            filters: SearchTokenizerFilters::default(),
        },
        tokenize_regex,
        "regex",
        preferred = false,
        custom_tymod = false
    );
}

#[pg_extern(immutable, parallel_safe)]
fn exact_typmod_in<'a>(typmod_parts: Array<'a, &'a CStr>) -> i32 {
    let parsed_typmod = ParsedTypmod::try_from(&typmod_parts).unwrap();
    if parsed_typmod.len() == 1 && matches!(parsed_typmod[0].key(), Some("alias")) {
        drop(parsed_typmod);
        return save_typmod(typmod_parts.iter()).expect("should not fail to save typmod");
    }

    ErrorReport::new(
        PgSqlErrorCode::ERRCODE_SYNTAX_ERROR,
        "type modifier is not allowed for type \"exact\"",
        function_name!(),
    )
    .report(PgLogLevel::ERROR);
    unreachable!()
}

extension_sql!(
    r#"
        ALTER TYPE pdb.exact SET (TYPMOD_IN = exact_typmod_in);
    "#,
    name = "exact_typmod",
    requires = [exact_typmod_in, "exact_definition"]
);
