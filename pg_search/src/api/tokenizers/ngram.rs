use pgrx::{extension_sql, pg_extern, Array};
use std::ffi::{CStr, CString};

#[pg_extern(immutable, parallel_safe)]
fn ngram_typmod_in(typmod_parts: Array<&CStr>) -> i32 {
    let min_gram = typmod_parts
        .get(0)
        .expect("malformed ngram typmod")
        .unwrap()
        .to_str()
        .expect("typmod must be valid utf8")
        .parse::<u8>()
        .expect("`min_gram` must be a `u8`");

    let max_gram = typmod_parts
        .get(1)
        .expect("malformed ngram typmod")
        .unwrap()
        .to_str()
        .expect("typmod must be valid utf8")
        .parse::<u8>()
        .expect("`max_gram` must be a `u8`");

    let prefix_only = typmod_parts
        .get(2)
        .unwrap_or_else(|| Some(c""))
        .unwrap()
        .to_str()
        .expect("typmod must be valid utf8")
        .to_ascii_lowercase()
        .starts_with('t');

    (prefix_only as i32) << 8 | (max_gram as i32) << 4 | (min_gram as i32)
}

#[pg_extern(immutable, parallel_safe)]
fn ngram_typmod_out(typmod: i32) -> CString {
    let prefix_only = ((typmod >> 8) & 0xFF) == 1;
    let max_gram = (typmod >> 4) & 0x0F;
    let min_gram = typmod & 0x0F;
    CString::new(format!(
        "({},{},{})",
        min_gram,
        max_gram,
        prefix_only.then_some("t").unwrap_or("f")
    ))
    .unwrap()
}

extension_sql!(
    r#"
    ALTER TYPE ngram SET (TYPMOD_IN = ngram_typmod_in, TYPMOD_OUT = ngram_typmod_out);
"#,
    name = "ngram_typmod",
    requires = [ngram_typmod_in, ngram_typmod_out, "ngram_definition"]
);
