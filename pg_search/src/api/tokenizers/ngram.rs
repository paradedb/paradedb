use crate::api::tokenizers::typmod;
use crate::api::tokenizers::typmod::{load_typmod, save_typmod};
use pgrx::{extension_sql, pg_extern, Array};
use std::ffi::{CStr, CString};
use tokenizers::manager::SearchTokenizerFilters;

#[pg_extern(immutable, parallel_safe)]
fn ngram_typmod_in(typmod_parts: Array<&CStr>) -> i32 {
    save_typmod(typmod_parts.iter()).expect("should not fail to save typmod")
}

#[pg_extern(immutable, parallel_safe)]
fn ngram_typmod_out(typmod: i32) -> CString {
    let parsed = load_typmod(typmod).expect("should not fail to load typmod");
    CString::new(format!("({parsed})")).unwrap()
}

pub struct NgramTypmod {
    pub min_gram: usize,
    pub max_gram: usize,
    pub prefix_only: bool,
    pub filters: SearchTokenizerFilters,
}

pub fn lookup_ngram_typmod(typmod: i32) -> typmod::Result<NgramTypmod> {
    let parsed = load_typmod(typmod)?;
    let filters = SearchTokenizerFilters::from(&parsed);

    let min_gram = parsed
        .try_get("min", 0)
        .map(|p| p.as_usize())
        .flatten()
        .ok_or(typmod::Error::MissingKey("min"))?;
    let max_gram = parsed
        .try_get("max", 1)
        .map(|p| p.as_usize())
        .flatten()
        .ok_or(typmod::Error::MissingKey("max"))?;
    let prefix_only = parsed
        .try_get("prefix_only", 2)
        .map(|p| p.as_bool())
        .flatten()
        .unwrap_or(false);

    Ok(NgramTypmod {
        min_gram,
        max_gram,
        prefix_only,
        filters,
    })
}

extension_sql!(
    r#"
    ALTER TYPE ngram SET (TYPMOD_IN = ngram_typmod_in, TYPMOD_OUT = ngram_typmod_out);
"#,
    name = "ngram_typmod",
    requires = [ngram_typmod_in, ngram_typmod_out, "ngram_definition"]
);
