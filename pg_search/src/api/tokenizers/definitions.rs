use crate::api::tokenizers::{CowString, DatumWrapper};
use crate::define_tokenizer_type;
use macros::generate_tokenizer_sql;
use pgrx::callconv::{Arg, ArgAbi, BoxRet, FcInfo};
use pgrx::nullable::Nullable;
use pgrx::pgrx_sql_entity_graph::metadata::{
    ArgumentError, Returns, ReturnsError, SqlMapping, SqlTranslatable,
};
use pgrx::{extension_sql, pg_cast, pg_extern, pg_sys, Array, FromDatum, IntoDatum};
use std::ffi::{CStr, CString};
use tantivy::tokenizer::Language;
use tokenizers::manager::SearchTokenizerFilters;
use tokenizers::SearchTokenizer;

define_tokenizer_type!(
    Simple,
    SearchTokenizer::Default(SearchTokenizerFilters::default()),
    tokenize_simple,
    "simple",
    true
);

define_tokenizer_type!(
    Whitespace,
    SearchTokenizer::WhiteSpace(SearchTokenizerFilters::default()),
    tokenize_whitespace,
    "whitespace",
    false
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
    false
);

define_tokenizer_type!(
    Stemmed,
    SearchTokenizer::Stem {
        language: Language::English,
        filters: SearchTokenizerFilters::default(),
    },
    tokenize_stemmed,
    "stemmed",
    false
);

define_tokenizer_type!(
    Regex,
    SearchTokenizer::RegexTokenizer {
        pattern: ".*".to_string(),
        filters: SearchTokenizerFilters::default(),
    },
    tokenize_regex,
    "regex",
    false
);
