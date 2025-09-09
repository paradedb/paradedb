use crate::api::tokenizers::typmod;
use crate::api::tokenizers::typmod::{load_typmod, save_typmod};
use pgrx::{extension_sql, pg_extern, Array, Spi};
use std::ffi::{CStr, CString};
use tokenizers::manager::SearchTokenizerFilters;

#[pg_extern(immutable, parallel_safe)]
fn regex_typmod_in(typmod_parts: Array<&CStr>) -> i32 {
    save_typmod(typmod_parts.iter()).expect("should not fail to save typmod")
}

#[pg_extern(immutable, parallel_safe)]
pub fn regex_typmod_out(typmod: i32) -> CString {
    let parsed = load_typmod(typmod).expect("should not fail to load typmod");
    CString::new(format!("({parsed})")).unwrap()
}

pub struct RegexTypmod {
    pub pattern: regex::Regex,
    pub filters: SearchTokenizerFilters,
}

pub fn lookup_regex_typmod(typmod: i32) -> typmod::Result<RegexTypmod> {
    let parsed = load_typmod(typmod)?;
    let filters = SearchTokenizerFilters::from(&parsed);
    let pattern = parsed
        .try_get("pattern", 0)
        .map(|p| p.as_regex())
        .flatten()
        .ok_or(typmod::Error::MissingKey("pattern"))??;

    Ok(RegexTypmod { pattern, filters })
}

extension_sql!(
    r#"
    ALTER TYPE regex SET (TYPMOD_IN = regex_typmod_in, TYPMOD_OUT = regex_typmod_out);
"#,
    name = "regex_typmod",
    requires = [regex_typmod_in, regex_typmod_out, "regex_definition"]
);
