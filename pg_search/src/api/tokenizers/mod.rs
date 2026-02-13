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

pub use crate::api::tokenizers::typmod::{
    AliasTypmod, GenericTypmod, JiebaTypmod, LinderaTypmod, NgramTypmod, RegexTypmod, Typmod,
    UncheckedTypmod, UnicodeWordsTypmod,
};

// if a ::pdb.<tokenizer> cast is used, ie ::pdb.simple, ::pdb.lindera, etc.
#[inline]
pub fn type_is_tokenizer(oid: pg_sys::Oid) -> bool {
    // TODO:  could this benefit from a local cache?
    lookup_type_category(oid)
        .map(|c| c == b't')
        .unwrap_or(false)
}
// if a ::pdb.alias cast is used
#[inline]
pub fn type_is_alias(oid: pg_sys::Oid) -> bool {
    // TODO:  could this benefit from a local cache?
    Some(oid) == lookup_typoid(c"pdb", c"alias")
}
// only fields that could contain text can be tokenized
#[inline]
pub fn type_can_be_tokenized(oid: pg_sys::Oid) -> bool {
    [
        pg_sys::VARCHAROID,
        pg_sys::TEXTOID,
        pg_sys::JSONOID,
        pg_sys::JSONBOID,
        pg_sys::TEXTARRAYOID,
        pg_sys::VARCHARARRAYOID,
    ]
    .contains(&oid)
}
// given an oid and typmod, return the alias name if it is an alias, otherwise return None
#[inline]
pub fn try_get_alias(oid: pg_sys::Oid, typmod: Typmod) -> Option<String> {
    if type_is_alias(oid) {
        AliasTypmod::try_from(typmod).ok()?.alias()
    } else if type_is_tokenizer(oid) {
        UncheckedTypmod::try_from(typmod).ok()?.alias()
    } else {
        None
    }
}

pub fn search_field_config_from_type(
    oid: pg_sys::Oid,
    typmod: Typmod,
    inner_typoid: pg_sys::Oid,
) -> Option<SearchFieldConfig> {
    let type_name = lookup_type_name(oid)?;

    if type_name.as_str() == "alias" && !type_can_be_tokenized(oid) {
        return None;
    }

    let mut tokenizer = match type_name.as_str() {
        "alias" => panic!("`pdb.alias` is not allowed in index definitions"),
        "simple" => SearchTokenizer::Simple(SearchTokenizerFilters::default()),
        "lindera" => SearchTokenizer::Lindera(
            LinderaLanguage::default(),
            SearchTokenizerFilters::default(),
        ),
        "icu" => SearchTokenizer::ICUTokenizer(SearchTokenizerFilters::default()),
        "jieba" => SearchTokenizer::Jieba {
            chinese_convert: None,
            filters: SearchTokenizerFilters::default(),
        },
        "ngram" => SearchTokenizer::Ngram {
            min_gram: 0,
            max_gram: 0,
            prefix_only: false,
            positions: false,
            filters: SearchTokenizerFilters::default(),
        },
        "whitespace" => SearchTokenizer::WhiteSpace(SearchTokenizerFilters::default()),
        "literal" => SearchTokenizer::Keyword,
        "normalized" | "literal_normalized" => {
            SearchTokenizer::LiteralNormalized(SearchTokenizerFilters::default())
        }
        "chinese_compatible" => {
            SearchTokenizer::ChineseCompatible(SearchTokenizerFilters::default())
        }
        "regex_pattern" => SearchTokenizer::RegexTokenizer {
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

    if type_name == "literal_normalized" {
        pgrx::warning!(
            "`pdb.literal_normalized` is deprecated; use `pdb.normalized` instead"
        );
    }

    apply_typmod(&mut tokenizer, typmod);

    let normalizer = tokenizer.normalizer().unwrap_or_default();

    let parsed_typmod = typmod::load_typmod(typmod).unwrap_or_default();

    let parsed_fieldnorms = parsed_typmod.get("fieldnorms").and_then(|p| p.as_bool());
    // columnar=true/false is our renaming of Tantivy's `fast` option
    // fast is default to true for any field that's not text or JSON
    // if it is text or JSON, it also defaults to true for literal and normalized
    // otherwise the user needs to explicitly set it to true
    let columnar_explicit = parsed_typmod.get("columnar").and_then(|p| p.as_bool());

    let (fast, fieldnorms, record) = if type_name == "literal"
        || type_name == "normalized"
        || type_name == "literal_normalized"
    {
        // literal and normalized default to fast=true (columnar=true)
        let fast = columnar_explicit.unwrap_or(true);

        // literal and normalized default to fieldnorms=false
        let fieldnorms = parsed_fieldnorms.unwrap_or(false);
        (fast, fieldnorms, IndexRecordOption::Basic)
    } else {
        // all others default to fast=false (columnar=false)
        let fast = columnar_explicit.unwrap_or(false);
        // all others default to fieldnorms=true
        let fieldnorms = parsed_fieldnorms.unwrap_or(true);
        (fast, fieldnorms, IndexRecordOption::WithFreqsAndPositions)
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
            positions,
            filters,
        } => {
            let ngram_typmod = NgramTypmod::try_from(typmod).unwrap_or_else(|e| {
                panic!("{}", e);
            });
            *min_gram = ngram_typmod.min_gram;
            *max_gram = ngram_typmod.max_gram;
            *prefix_only = ngram_typmod.prefix_only;
            *positions = ngram_typmod.positions;
            *filters = ngram_typmod.filters;
        }
        SearchTokenizer::RegexTokenizer { pattern, filters } => {
            let regex_typmod = RegexTypmod::try_from(typmod).unwrap_or_else(|e| {
                panic!("{}", e);
            });
            *pattern = regex_typmod.pattern.to_string();
            *filters = regex_typmod.filters;
        }

        SearchTokenizer::Lindera(style, filters) => {
            let lindera_typmod = LinderaTypmod::try_from(typmod).unwrap_or_else(|e| {
                panic!("{}", e);
            });
            *style = lindera_typmod.language;
            *filters = lindera_typmod.filters;
        }

        #[allow(deprecated)]
        SearchTokenizer::Raw(filters)
        | SearchTokenizer::LiteralNormalized(filters)
        | SearchTokenizer::Simple(filters)
        | SearchTokenizer::SourceCode(filters)
        | SearchTokenizer::WhiteSpace(filters)
        | SearchTokenizer::ChineseCompatible(filters)
        | SearchTokenizer::ChineseLindera(filters)
        | SearchTokenizer::JapaneseLindera(filters)
        | SearchTokenizer::KoreanLindera(filters) => {
            // | SearchTokenizer::Jieba(filters) =>  {
            let generic_typmod = GenericTypmod::try_from(typmod).unwrap_or_else(|e| {
                panic!("{}", e);
            });
            *filters = generic_typmod.filters;
        }

        SearchTokenizer::Jieba {
            chinese_convert,
            filters,
        } => {
            let jieba_typmod = JiebaTypmod::try_from(typmod).unwrap_or_else(|e| {
                panic!("{}", e);
            });
            *filters = jieba_typmod.filters;
            *chinese_convert = jieba_typmod.chinese_convert;
        }

        SearchTokenizer::ICUTokenizer(filters) => {
            let generic_typmod = GenericTypmod::try_from(typmod).unwrap_or_else(|e| {
                panic!("{}", e);
            });
            *filters = generic_typmod.filters;
        }

        SearchTokenizer::UnicodeWords {
            remove_emojis,
            filters,
        }
        | SearchTokenizer::UnicodeWordsDeprecated {
            remove_emojis,
            filters,
        } => {
            let unicode_typmod = UnicodeWordsTypmod::try_from(typmod).unwrap_or_else(|e| {
                panic!("{}", e);
            });
            *remove_emojis = unicode_typmod.remove_emojis;
            *filters = unicode_typmod.filters;
        }

        SearchTokenizer::Keyword => {}
        #[allow(deprecated)]
        SearchTokenizer::KeywordDeprecated => {}
    }
}

pub trait CowString {
    fn to_str(&self) -> Cow<'_, str>;
}

pub trait DatumWrapper {
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

            let s = convert_varlena_to_str_memoized(detoasted);
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

struct GenericTypeWrapper<Type: DatumWrapper, SqlName: SqlNameMarker> {
    pub datum: pg_sys::Datum,
    pub typoid: pg_sys::Oid,
    __marker: PhantomData<(Type, SqlName)>,
}

unsafe impl<Type: DatumWrapper, SqlName: SqlNameMarker> SqlTranslatable
    for GenericTypeWrapper<Type, SqlName>
{
    fn argument_sql() -> Result<SqlMapping, ArgumentError> {
        Ok(SqlMapping::As(SqlName::SQL_NAME.into()))
    }

    fn return_sql() -> Result<Returns, ReturnsError> {
        Ok(Returns::One(SqlMapping::As(SqlName::SQL_NAME.into())))
    }
}

impl<Type: DatumWrapper, SqlName: SqlNameMarker> IntoDatum for GenericTypeWrapper<Type, SqlName> {
    fn into_datum(self) -> Option<pg_sys::Datum> {
        Some(self.datum)
    }

    fn type_oid() -> pg_sys::Oid {
        todo!("lookup type name?")
    }
}

impl<Type: DatumWrapper, SqlName: SqlNameMarker> FromDatum for GenericTypeWrapper<Type, SqlName> {
    unsafe fn from_polymorphic_datum(
        datum: pg_sys::Datum,
        is_null: bool,
        typoid: pg_sys::Oid,
    ) -> Option<Self> {
        if is_null {
            None
        } else {
            Some(Self {
                datum,
                typoid,
                __marker: PhantomData,
            })
        }
    }
}

unsafe impl<'mct, Type: DatumWrapper, SqlName: SqlNameMarker> ArgAbi<'mct>
    for GenericTypeWrapper<Type, SqlName>
{
    unsafe fn unbox_arg_unchecked(arg: Arg<'_, 'mct>) -> Self {
        let index = arg.index();
        unsafe {
            arg.unbox_arg_using_from_datum()
                .unwrap_or_else(|| panic!("argument {index} must not be null"))
        }
    }
}

unsafe impl<Type: DatumWrapper, SqlName: SqlNameMarker> BoxRet
    for GenericTypeWrapper<Type, SqlName>
{
    unsafe fn box_into<'fcx>(self, fcinfo: &mut FcInfo<'fcx>) -> pgrx::datum::Datum<'fcx> {
        fcinfo.return_raw_datum(self.datum)
    }
}

impl<Type: DatumWrapper, SqlName: SqlNameMarker> GenericTypeWrapper<Type, SqlName> {
    fn new(datum: pg_sys::Datum, typoid: pg_sys::Oid) -> Self {
        Self {
            datum,
            typoid,
            __marker: PhantomData,
        }
    }
}

macro_rules! datum_wrapper_for {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl DatumWrapper for $ty {
fn from_datum(datum: pg_sys::Datum) -> Self {
    unsafe {
        if datum.is_null() {
            panic!("null datum not allowed in alias cast");
        }
        <$ty as pgrx::datum::FromDatum>::from_datum(datum, false)
            .expect("failed to convert datum")
    }
}

                fn as_datum(&self) -> pg_sys::Datum {
                    unreachable!("this is not supported")
                }
            }
        )+
    };
}

datum_wrapper_for!(
    String,
    pgrx::datum::Uuid,
    pgrx::Json,
    pgrx::JsonB,
    Vec<String>,
    i16,
    i32,
    i64,
    u32,
    f32,
    f64,
    bool,
    pgrx::datum::Date,
    pgrx::datum::Time,
    pgrx::datum::Timestamp,
    pgrx::datum::TimestampWithTimeZone,
    pgrx::datum::TimeWithTimeZone,
    pgrx::datum::Inet,
    pgrx::datum::AnyNumeric,
    pgrx::datum::Range<i32>,
    pgrx::datum::Range<i64>,
    pgrx::datum::Range<pgrx::datum::AnyNumeric>,
    pgrx::datum::Range<pgrx::datum::Date>,
    pgrx::datum::Range<pgrx::datum::Timestamp>,
    pgrx::datum::Range<pgrx::datum::TimestampWithTimeZone>,
    Vec<i16>,
    Vec<i32>,
    Vec<i64>,
    Vec<f32>,
    Vec<f64>,
    Vec<bool>,
    Vec<pgrx::datum::Date>,
    Vec<pgrx::datum::Time>,
    Vec<pgrx::datum::Timestamp>,
    Vec<pgrx::datum::TimestampWithTimeZone>,
    Vec<pgrx::datum::TimeWithTimeZone>,
    Vec<pgrx::datum::AnyNumeric>
);

pub trait SqlNameMarker {
    const SQL_NAME: &'static str;
}

pub struct TextArrayMarker;
impl SqlNameMarker for TextArrayMarker {
    const SQL_NAME: &'static str = "text[]";
}

pub struct VarcharArrayMarker;
impl SqlNameMarker for VarcharArrayMarker {
    const SQL_NAME: &'static str = "varchar[]";
}

pub struct JsonMarker;
impl SqlNameMarker for JsonMarker {
    const SQL_NAME: &'static str = "json";
}

pub struct JsonbMarker;
impl SqlNameMarker for JsonbMarker {
    const SQL_NAME: &'static str = "jsonb";
}

pub struct UuidMarker;
impl SqlNameMarker for UuidMarker {
    const SQL_NAME: &'static str = "uuid";
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
