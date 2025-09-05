use pgrx::datum::DatumWithOid;
use pgrx::{extension_sql, pg_extern, pg_sys, Array, Spi};
use std::ffi::{CStr, CString};

#[pg_extern(immutable, parallel_safe)]
fn regex_typmod_in(typmod_parts: Array<&CStr>) -> i32 {
    let regex = typmod_parts
        .get(0)
        .expect("malformed regex typmod")
        .unwrap()
        .to_str()
        .expect("typmod must be valid utf8");

    Spi::get_one_with_args(
        "SELECT paradedb._typmod($1)",
        &[unsafe { DatumWithOid::new(regex, pg_sys::TEXTOID) }],
    )
    .expect("SPI lookup to paradedb._typmod should not fail")
    .expect("paradedb._typmod should return a single row")
}

#[pg_extern(immutable, parallel_safe)]
pub fn regex_typmod_out(typmod: i32) -> CString {
    let regex = lookup_regex_typmod(typmod).expect("typmod not found by paradedb._typmod()");
    let typmod_str = format!("('{}')", regex.to_str().unwrap().replace("'", "''"));
    CString::new(typmod_str).unwrap()
}

pub fn lookup_regex_typmod(typmod: i32) -> Option<CString> {
    let regex = Spi::get_one_with_args::<String>(
        "SELECT paradedb._typmod($1)",
        &[unsafe { DatumWithOid::new(typmod, pg_sys::INT4OID) }],
    )
    .expect("SPI lookup to paradedb._typmod should not fail");
    regex.map(|regex| CString::new(regex).unwrap())
}

extension_sql!(
    r#"
    ALTER TYPE regex SET (TYPMOD_IN = regex_typmod_in, TYPMOD_OUT = regex_typmod_out);
"#,
    name = "regex_typmod",
    requires = [regex_typmod_in, regex_typmod_out, "regex_definition"]
);
