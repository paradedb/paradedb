use once_cell::sync::Lazy;
use pgrx::{pg_sys, set_varsize_4b};
use std::borrow::{Borrow, Cow};
use std::ptr::addr_of_mut;
use tantivy::tokenizer::Language;
use tokenizers::SearchTokenizer;

mod definitions;
mod ngram;
mod regex;
mod stemmed;
mod typmod;

pub fn apply_typmod(tokenizer: &mut SearchTokenizer, typmod: i32) {
    match tokenizer {
        SearchTokenizer::Ngram {
            min_gram,
            max_gram,
            prefix_only,
            ..
        } => {
            *prefix_only = ((typmod >> 8) & 0xFF) == 1;
            *max_gram = ((typmod >> 4) & 0x0F) as usize;
            *min_gram = (typmod & 0x0F) as usize;
        }
        SearchTokenizer::Stem { language, .. } => {
            *language = match typmod {
                0 => Language::Arabic,
                1 => Language::Danish,
                2 => Language::Dutch,
                3 => Language::English,
                4 => Language::Finnish,
                5 => Language::French,
                6 => Language::German,
                7 => Language::Greek,
                8 => Language::Hungarian,
                9 => Language::Italian,
                10 => Language::Norwegian,
                11 => Language::Portuguese,
                12 => Language::Romanian,
                13 => Language::Russian,
                14 => Language::Spanish,
                15 => Language::Swedish,
                16 => Language::Tamil,
                17 => Language::Turkish,
                _ => panic!("Stem tokenizer requires a language"),
            }
        }
        SearchTokenizer::RegexTokenizer { pattern, .. } => {
            *pattern = regex::lookup_regex_typmod(typmod)
                .expect("Regex tokenizer requires a pattern")
                .into_string()
                .unwrap();
        }
        _ => {}
    }
}

pub trait CowString {
    fn to_str(&self) -> Cow<'_, str>;
}

pub trait DatumWrapper {
    fn from_datum(datum: pg_sys::Datum) -> Self;

    fn as_datum(&self) -> pg_sys::Datum;

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

#[macro_export]
macro_rules! define_tokenizer_type {
    ($rust_name:ident, $tokenizer_conf:expr, $cast_name:ident, $sql_name:literal, $preferred:literal) => {
        pub struct $rust_name(pg_sys::Datum);

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
                todo!("get the type oid for $rust_name")
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
            unsafe fn box_into<'fcx>(self, fcinfo: &mut FcInfo<'fcx>) -> pgrx::datum::Datum<'fcx> {
                match self.into_datum() {
                    Some(datum) => unsafe { fcinfo.return_raw_datum(datum) },
                    None => fcinfo.return_null(),
                }
            }
        }

        unsafe impl SqlTranslatable for $rust_name {
            fn argument_sql() -> Result<SqlMapping, ArgumentError> {
                Ok(SqlMapping::As($sql_name.into()))
            }

            fn return_sql() -> Result<Returns, ReturnsError> {
                Ok(Returns::One(SqlMapping::As($sql_name.into())))
            }
        }

        #[pg_extern(immutable, parallel_safe)]
        fn $cast_name(s: $rust_name, fcinfo: pg_sys::FunctionCallInfo) -> Vec<String> {
            let mut tokenizer = $tokenizer_conf;

            unsafe {
                let func_expr = (*(*fcinfo).flinfo).fn_expr.cast::<pg_sys::FuncExpr>();
                let args = pgrx::PgList::<pg_sys::Node>::from_pg((*func_expr).args.cast());
                let first_arg = args.get_ptr(0).unwrap();
                let typmod = pg_sys::exprTypmod(first_arg);

                super::apply_typmod(&mut tokenizer, typmod);
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
            preferred = $preferred
        );
    };
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
