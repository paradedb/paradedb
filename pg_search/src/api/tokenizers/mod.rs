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

use crate::api::tokenizers::typmod::{
    lookup_lindera_typmod, lookup_ngram_typmod, lookup_regex_typmod, lookup_unicode_words_typmod,
};
use crate::postgres::catalog::{lookup_type_category, lookup_type_name, lookup_typoid};
use once_cell::sync::Lazy;
use pgrx::callconv::{Arg, ArgAbi, BoxRet, FcInfo};
use pgrx::pgrx_sql_entity_graph::metadata::{
    ArgumentError, Returns, ReturnsError, SqlMapping, SqlTranslatable,
};
use pgrx::{pg_sys, set_varsize_4b, FromDatum, IntoDatum};
use std::borrow::Cow;
use std::marker::PhantomData;
use std::ptr::addr_of_mut;
use tokenizers::manager::{LinderaLanguage, SearchTokenizerFilters};
use tokenizers::SearchTokenizer;

pub(crate) mod definitions;
mod typmod;

use crate::schema::{IndexRecordOption, SearchFieldConfig};
pub use typmod::{lookup_alias_typmod, lookup_generic_typmod, Typmod};

#[inline]
pub fn type_is_tokenizer(oid: pg_sys::Oid) -> bool {
    // TODO:  could this benefit from a local cache?
    lookup_type_category(oid)
        .map(|c| c == b't')
        .unwrap_or(false)
}
#[inline]
pub fn type_is_alias(oid: pg_sys::Oid) -> bool {
    // TODO:  could this benefit from a local cache?
    Some(oid) == lookup_typoid(c"pdb", c"alias")
}

pub fn search_field_config_from_type(
    oid: pg_sys::Oid,
    typmod: Typmod,
    inner_typoid: pg_sys::Oid,
) -> Option<SearchFieldConfig> {
    let type_name = lookup_type_name(oid)?;

    let mut tokenizer = match type_name.as_str() {
        "alias" => panic!("`pdb.alias` is not allowed in index definitions"),
        "simple" => SearchTokenizer::Default(SearchTokenizerFilters::default()),
        "lindera" => SearchTokenizer::Lindera(
            LinderaLanguage::default(),
            SearchTokenizerFilters::default(),
        ),
        #[cfg(feature = "icu")]
        "icu" => SearchTokenizer::ICUTokenizer(SearchTokenizerFilters::default()),
        "jieba" => SearchTokenizer::Jieba(SearchTokenizerFilters::default()),
        "ngram" => SearchTokenizer::Ngram {
            min_gram: 0,
            max_gram: 0,
            prefix_only: false,
            filters: SearchTokenizerFilters::default(),
        },
        "whitespace" => SearchTokenizer::WhiteSpace(SearchTokenizerFilters::default()),
        "literal" => SearchTokenizer::Keyword,
        "literal_normalized" => {
            SearchTokenizer::LiteralNormalized(SearchTokenizerFilters::default())
        }
        "chinese_compatible" => {
            SearchTokenizer::ChineseCompatible(SearchTokenizerFilters::default())
        }
        "regex" => SearchTokenizer::RegexTokenizer {
            pattern: "".to_string(),
            filters: Default::default(),
        },
        "source_code" => SearchTokenizer::SourceCode(SearchTokenizerFilters::default()),
        "unicode_words" => SearchTokenizer::UnicodeWords {
            remove_emojis: false,
            filters: SearchTokenizerFilters::default(),
        },
        _ => return None,
    };

    apply_typmod(&mut tokenizer, typmod);

    let normalizer = tokenizer.normalizer().unwrap_or_default();

    let (fast, fieldnorms, record) = if type_name == "literal" {
        // non-tokenized fields get to be fast
        (true, false, IndexRecordOption::Basic)
    } else {
        // all others do not
        (false, true, IndexRecordOption::WithFreqsAndPositions)
    };

    if inner_typoid == pg_sys::JSONOID || inner_typoid == pg_sys::JSONBOID {
        Some(SearchFieldConfig::Json {
            indexed: true,
            fast,
            fieldnorms,
            tokenizer,
            record,
            normalizer,
            column: None,
            expand_dots: true,
        })
    } else {
        Some(SearchFieldConfig::Text {
            indexed: true,
            fast,
            fieldnorms,
            tokenizer,
            record,
            normalizer,
            column: None,
        })
    }
}

pub fn apply_typmod(tokenizer: &mut SearchTokenizer, typmod: Typmod) {
    match tokenizer {
        SearchTokenizer::Ngram {
            min_gram,
            max_gram,
            prefix_only,
            filters,
        } => {
            let ngram_typmod = lookup_ngram_typmod(typmod).expect("typmod lookup should not fail");
            *min_gram = ngram_typmod.min_gram;
            *max_gram = ngram_typmod.max_gram;
            *prefix_only = ngram_typmod.prefix_only;
            *filters = ngram_typmod.filters;
        }
        SearchTokenizer::RegexTokenizer { pattern, filters } => {
            let regex_typmod = lookup_regex_typmod(typmod).expect("typmod lookup should not fail");
            *pattern = regex_typmod.pattern.to_string();
            *filters = regex_typmod.filters;
        }

        SearchTokenizer::Lindera(style, filters) => {
            let lindera_typmod =
                lookup_lindera_typmod(typmod).expect("typmod lookup should not fail");
            *style = lindera_typmod.language;
            *filters = lindera_typmod.filters;
        }

        #[allow(deprecated)]
        SearchTokenizer::Raw(filters)
        | SearchTokenizer::LiteralNormalized(filters)
        | SearchTokenizer::Default(filters)
        | SearchTokenizer::SourceCode(filters)
        | SearchTokenizer::WhiteSpace(filters)
        | SearchTokenizer::ChineseCompatible(filters)
        | SearchTokenizer::ChineseLindera(filters)
        | SearchTokenizer::JapaneseLindera(filters)
        | SearchTokenizer::KoreanLindera(filters)
        | SearchTokenizer::Jieba(filters) => {
            let generic_typmod =
                lookup_generic_typmod(typmod).expect("typmod lookup should not fail");
            *filters = generic_typmod.filters;
        }

        #[cfg(feature = "icu")]
        SearchTokenizer::ICUTokenizer(filters) => {
            let generic_typmod =
                lookup_generic_typmod(typmod).expect("typmod lookup should not fail");
            *filters = generic_typmod.filters;
        }

        SearchTokenizer::UnicodeWords {
            remove_emojis,
            filters,
        } => {
            let unicode_typmod =
                lookup_unicode_words_typmod(typmod).expect("typmod lookup should not fail");
            *remove_emojis = unicode_typmod.remove_emojis;
            *filters = unicode_typmod.filters;
        }

        SearchTokenizer::Keyword => {}
    }
}

pub trait CowString {
    fn to_str(&self) -> Cow<'_, str>;
}

pub trait DatumWrapper {
    fn sql_name() -> &'static str;

    #[allow(dead_code)]
    fn from_datum(datum: pg_sys::Datum) -> Self;

    #[allow(dead_code)]
    fn as_datum(&self) -> pg_sys::Datum;

    #[allow(dead_code)]
    fn from_str<S: AsRef<str>>(value: S) -> Self
    where
        Self: Sized,
    {
        let s = value.as_ref();
        let len = s.len().saturating_add(pg_sys::VARHDRSZ);
        assert!(len < (u32::MAX as usize >> 2));
        unsafe {
            // SAFETY:  palloc gives us a valid pointer and if there's not enough memory it'll raise an error
            let varlena = pg_sys::palloc(len) as *mut pg_sys::varlena;

            // SAFETY: `varlena` can properly cast into a `varattrib_4b` and all of what it contains is properly
            // allocated thanks to our call to `palloc` above
            let varattrib_4b: *mut _ = &mut varlena
                .cast::<pg_sys::varattrib_4b>()
                .as_mut()
                .unwrap_unchecked()
                .va_4byte;

            // This is the same as Postgres' `#define SET_VARSIZE_4B` (which have over in
            // `pgrx/src/varlena.rs`), however we're asserting above that the input string
            // isn't too big for a Postgres varlena, since it's limited to 32 bits and,
            // in reality, it's a quarter that length, but this is good enough
            set_varsize_4b(varlena, len as i32);

            // SAFETY: src and dest pointers are valid, exactly `self.len()` bytes long,
            // and the `dest` was freshly allocated, thus non-overlapping
            std::ptr::copy_nonoverlapping(
                s.as_ptr(),
                addr_of_mut!((&mut *varattrib_4b).va_data).cast::<u8>(),
                s.len(),
            );

            Self::from_datum(pg_sys::Datum::from(varlena))
        }
    }
}

impl<T: DatumWrapper> CowString for T {
    fn to_str(&self) -> Cow<'_, str> {
        unsafe {
            let varlena = self.as_datum().cast_mut_ptr::<pg_sys::varlena>();
            let detoasted = pg_sys::pg_detoast_datum(varlena);

            let s = convert_varlena_to_str_memoized(varlena);
            if std::ptr::eq(detoasted, varlena) {
                // wasn't toasted, can do zero-copy
                Cow::Borrowed(s)
            } else {
                // was toasted, so copy to owned Rust string and free the detoasted memory
                let s = s.to_string();
                pg_sys::pfree(detoasted.cast());
                Cow::Owned(s)
            }
        }
    }
}

struct GenericTypeWrapper<Type: DatumWrapper> {
    pub datum: pg_sys::Datum,
    __marker: PhantomData<Type>,
}

unsafe impl<Type: DatumWrapper> SqlTranslatable for GenericTypeWrapper<Type> {
    fn argument_sql() -> Result<SqlMapping, ArgumentError> {
        Ok(SqlMapping::As(Type::sql_name().into()))
    }

    fn return_sql() -> Result<Returns, ReturnsError> {
        Ok(Returns::One(SqlMapping::As(Type::sql_name().into())))
    }
}

impl<Type: DatumWrapper> IntoDatum for GenericTypeWrapper<Type> {
    fn into_datum(self) -> Option<pg_sys::Datum> {
        Some(self.datum)
    }

    fn type_oid() -> pg_sys::Oid {
        todo!("lookup type name?")
    }
}

impl<Type: DatumWrapper> FromDatum for GenericTypeWrapper<Type> {
    unsafe fn from_polymorphic_datum(
        datum: pg_sys::Datum,
        is_null: bool,
        _typoid: pg_sys::Oid,
    ) -> Option<Self> {
        if is_null {
            None
        } else {
            Some(Self {
                datum,
                __marker: PhantomData,
            })
        }
    }
}

unsafe impl<'mct, Type: DatumWrapper> ArgAbi<'mct> for GenericTypeWrapper<Type> {
    unsafe fn unbox_arg_unchecked(arg: Arg<'_, 'mct>) -> Self {
        let index = arg.index();
        unsafe {
            arg.unbox_arg_using_from_datum()
                .unwrap_or_else(|| panic!("argument {index} must not be null"))
        }
    }
}

unsafe impl<Type: DatumWrapper> BoxRet for GenericTypeWrapper<Type> {
    unsafe fn box_into<'fcx>(self, fcinfo: &mut FcInfo<'fcx>) -> pgrx::datum::Datum<'fcx> {
        fcinfo.return_raw_datum(self.datum)
    }
}

impl<Type: DatumWrapper> GenericTypeWrapper<Type> {
    fn new(datum: pg_sys::Datum) -> Self {
        Self {
            datum,
            __marker: PhantomData,
        }
    }
}

impl DatumWrapper for pgrx::Json {
    fn sql_name() -> &'static str {
        "json"
    }

    fn from_datum(datum: pg_sys::Datum) -> Self {
        unsafe { <pgrx::Json as FromDatum>::from_datum(datum, datum.is_null()).unwrap() }
    }

    fn as_datum(&self) -> pg_sys::Datum {
        unreachable!("this is not supported")
    }
}

impl DatumWrapper for pgrx::JsonB {
    fn sql_name() -> &'static str {
        "jsonb"
    }

    fn from_datum(datum: pg_sys::Datum) -> Self {
        unsafe { <pgrx::JsonB as FromDatum>::from_datum(datum, datum.is_null()).unwrap() }
    }

    fn as_datum(&self) -> pg_sys::Datum {
        unreachable!("this is not supported")
    }
}

//
// taken from pgrx
//

static UTF8DATABASE: Lazy<Utf8Compat> = Lazy::new(|| {
    use pg_sys::pg_enc::*;
    let encoding_int = unsafe { pg_sys::GetDatabaseEncoding() };
    match encoding_int as _ {
        PG_UTF8 => Utf8Compat::Yes,
        // The 0 encoding. It... may be UTF-8
        PG_SQL_ASCII => Utf8Compat::Maybe,
        // Modifies ASCII, and should never be seen as PG doesn't support it as server encoding
        PG_SJIS | PG_SHIFT_JIS_2004
        // Not specified as an ASCII extension, also not a server encoding
        | PG_BIG5
        // Wild vendor differences including non-ASCII are possible, also not a server encoding
        | PG_JOHAB => unreachable!("impossible? unsupported non-ASCII-compatible database encoding is not a server encoding"),
        // Other Postgres encodings either extend US-ASCII or CP437 (which includes US-ASCII)
        // There may be a subtlety that requires us to revisit this later
        1..=41=> Utf8Compat::Ascii,
        // Unfamiliar encoding? Run UTF-8 validation like normal and hope for the best
        _ => Utf8Compat::Maybe,
    }
});

enum Utf8Compat {
    /// It's UTF-8, so... obviously
    Yes,
    /// This is what is assumed about "SQL_ASCII"
    Maybe,
    /// An "extended ASCII" encoding, so we're fine if we only touch ASCII
    Ascii,
}

// This is not marked inline on purpose, to allow it to be in a single code section
// which is then branch-predicted on every time by the CPU.
unsafe fn convert_varlena_to_str_memoized<'a>(varlena: *const pg_sys::varlena) -> &'a str {
    match *UTF8DATABASE {
        Utf8Compat::Yes => pgrx::varlena::text_to_rust_str_unchecked(varlena),
        Utf8Compat::Maybe => pgrx::varlena::text_to_rust_str(varlena)
            .expect("datums converted to &str should be valid UTF-8"),
        Utf8Compat::Ascii => {
            let bytes = pgrx::varlena_to_byte_slice(varlena);
            if bytes.is_ascii() {
                core::str::from_utf8_unchecked(bytes)
            } else {
                panic!("datums converted to &str should be valid UTF-8, database encoding is only UTF-8 compatible for ASCII")
            }
        }
    }
}
