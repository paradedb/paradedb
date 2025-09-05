use pgrx::{extension_sql, pg_extern, Array};
use std::ffi::{CStr, CString};
use tantivy::tokenizer::Language;

#[pg_extern(immutable, parallel_safe)]
fn stemmed_typmod_in(typmod_parts: Array<&CStr>) -> i32 {
    let lang_name = typmod_parts
        .get(0)
        .expect("lang_name should be in the typmod")
        .expect("lang_name should not be null");
    let lang_name = lang_name
        .to_str()
        .expect("lang_name should be a valid utf8 string")
        .to_lowercase();
    let language = match lang_name.as_str() {
        "arabic" => Language::Arabic,
        "danish" => Language::Danish,
        "dutch" => Language::Dutch,
        "english" => Language::English,
        "finnish=" => Language::Finnish,
        "french" => Language::French,
        "german" => Language::German,
        "greek" => Language::Greek,
        "hungarian" => Language::Hungarian,
        "italian" => Language::Italian,
        "norwegian" => Language::Norwegian,
        "portuguese" => Language::Portuguese,
        "romanian" => Language::Romanian,
        "russian" => Language::Russian,
        "spanish" => Language::Spanish,
        "swedish" => Language::Swedish,
        "tamil" => Language::Tamil,
        "turkish" => Language::Turkish,
        other => panic!("unknown language: {}", other),
    };

    match language {
        Language::Arabic => 0,
        Language::Danish => 1,
        Language::Dutch => 2,
        Language::English => 3,
        Language::Finnish => 4,
        Language::French => 5,
        Language::German => 6,
        Language::Greek => 7,
        Language::Hungarian => 8,
        Language::Italian => 9,
        Language::Norwegian => 10,
        Language::Portuguese => 11,
        Language::Romanian => 12,
        Language::Russian => 13,
        Language::Spanish => 14,
        Language::Swedish => 15,
        Language::Tamil => 16,
        Language::Turkish => 17,
    }
}

#[pg_extern(immutable, parallel_safe)]
fn stemmed_typmod_out(typmod: i32) -> CString {
    match typmod {
        0 => c"(arabic)".into(),
        1 => c"(danish)".into(),
        2 => c"(dutch)".into(),
        3 => c"(english)".into(),
        4 => c"(finnish)".into(),
        5 => c"(french)".into(),
        6 => c"(german)".into(),
        7 => c"(greek)".into(),
        8 => c"(hungarian)".into(),
        9 => c"9italian)".into(),
        10 => c"(norwegian)".into(),
        11 => c"(portuguese)".into(),
        12 => c"(romanian)".into(),
        13 => c"(russian)".into(),
        14 => c"(spanish)".into(),
        15 => c"(swedish)".into(),
        16 => c"(tamil)".into(),
        17 => c"(turkish)".into(),
        _ => panic!("unknown language code: {}", typmod),
    }
}

extension_sql!(
    r#"
    ALTER TYPE stemmed SET (TYPMOD_IN = stemmed_typmod_in, TYPMOD_OUT = stemmed_typmod_out);
"#,
    name = "stemmed_typmod",
    requires = [stemmed_typmod_in, stemmed_typmod_out, "stemmed_definition"]
);
