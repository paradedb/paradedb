use crate::api::tokenizers::typmod;
use crate::api::tokenizers::typmod::{load_typmod, save_typmod};
use pgrx::{extension_sql, pg_extern, Array};
use std::ffi::{CStr, CString};
use tantivy::tokenizer::Language;
use tokenizers::manager::SearchTokenizerFilters;

#[pg_extern(immutable, parallel_safe)]
fn stemmed_typmod_in(typmod_parts: Array<&CStr>) -> i32 {
    save_typmod(typmod_parts.iter()).expect("should not fail to save typmod")
}

#[pg_extern(immutable, parallel_safe)]
fn stemmed_typmod_out(typmod: i32) -> CString {
    let parsed = load_typmod(typmod).expect("should not fail to load typmod");
    CString::new(format!("({parsed})")).unwrap()
}

pub struct StemmedTypmod {
    pub language: Language,
    pub filters: SearchTokenizerFilters,
}

pub fn lookup_stemmed_typmod(typmod: i32) -> typmod::Result<StemmedTypmod> {
    let parsed = load_typmod(typmod)?;
    let filters = SearchTokenizerFilters::from(&parsed);
    let language = parsed
        .try_get("language", 0)
        .map(|p| p.as_language())
        .ok_or(typmod::Error::MissingKey("language"))??;
    Ok(StemmedTypmod { language, filters })
}

extension_sql!(
    r#"
    ALTER TYPE stemmed SET (TYPMOD_IN = stemmed_typmod_in, TYPMOD_OUT = stemmed_typmod_out);
"#,
    name = "stemmed_typmod",
    requires = [stemmed_typmod_in, stemmed_typmod_out, "stemmed_definition"]
);
