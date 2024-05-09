use memoffset::*;
use pgrx::pg_sys::AsPgCStr;
use pgrx::*;
use serde_json::json;
use std::collections::HashMap;
use std::ffi::CStr;

use crate::schema::{SearchFieldConfig, SearchFieldName};

/* ADDING OPTIONS
 * in init(), call pg_sys::add_{type}_reloption (check postgres docs for what args you need)
 * add the corresponding entries to SearchIndexCreateOptions struct definition
 * in amoptions(), add a relopt_parse_elt entry to the options array and change NUM_REL_OPTS
 * Note that for string options, postgres will give you the offset of the string, and you have to read the string
 * yourself (see get_tokenizer)
*/

/* READING OPTIONS
 * options are placed in relation.rd_options
 * As in ambuild(), cast relation.rd_options into SearchIndexCreateOptions using PgBox
 * (because SearchIndexCreateOptions is a postgres-allocated object) and use getters and setters
*/

static mut RELOPT_KIND_PDB: pg_sys::relopt_kind = 0;

// Postgres handles string options by placing each option offset bytes from the start of rdopts and
// plops the offset in the struct
#[repr(C)]
pub struct SearchIndexCreateOptions {
    // varlena header (needed bc postgres treats this as bytea)
    vl_len_: i32,
    text_fields_offset: i32,
    numeric_fields_offset: i32,
    boolean_fields_offset: i32,
    json_fields_offset: i32,
    date_fields_offset: i32,
    key_field_offset: i32,
}

#[pg_guard]
extern "C" fn validate_text_fields(value: *const std::os::raw::c_char) {
    let json_str = cstr_to_rust_str(value);
    if json_str.is_empty() {
        return;
    }
    SearchIndexCreateOptions::deserialize_config_fields("Text".into(), json_str);
}

#[pg_guard]
extern "C" fn validate_numeric_fields(value: *const std::os::raw::c_char) {
    let json_str = cstr_to_rust_str(value);
    if json_str.is_empty() {
        return;
    }
    SearchIndexCreateOptions::deserialize_config_fields("Numeric".into(), json_str);
}

#[pg_guard]
extern "C" fn validate_boolean_fields(value: *const std::os::raw::c_char) {
    let json_str = cstr_to_rust_str(value);
    if json_str.is_empty() {
        return;
    }
    SearchIndexCreateOptions::deserialize_config_fields("Boolean".into(), json_str);
}

#[pg_guard]
extern "C" fn validate_json_fields(value: *const std::os::raw::c_char) {
    let json_str = cstr_to_rust_str(value);
    if json_str.is_empty() {
        return;
    }
    SearchIndexCreateOptions::deserialize_config_fields("Json".into(), json_str);
}

#[pg_guard]
extern "C" fn validate_date_fields(value: *const std::os::raw::c_char) {
    let json_str = cstr_to_rust_str(value);
    if json_str.is_empty() {
        return;
    }
    SearchIndexCreateOptions::deserialize_config_fields("Date".into(), json_str);
}

#[pg_guard]
extern "C" fn validate_key_field(value: *const std::os::raw::c_char) {
    cstr_to_rust_str(value);
}

#[inline]
fn cstr_to_rust_str(value: *const std::os::raw::c_char) -> String {
    if value.is_null() {
        return "".to_string();
    }

    unsafe { CStr::from_ptr(value) }
        .to_str()
        .expect("failed to parse fields as utf-8")
        .to_string()
}

// For now, we support changing the tokenizer between default, raw, and en_stem
const NUM_REL_OPTS: usize = 6;
#[pg_guard]
pub unsafe extern "C" fn amoptions(
    reloptions: pg_sys::Datum,
    validate: bool,
) -> *mut pg_sys::bytea {
    let options: [pg_sys::relopt_parse_elt; NUM_REL_OPTS] = [
        pg_sys::relopt_parse_elt {
            optname: "text_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type_RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexCreateOptions, text_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "numeric_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type_RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexCreateOptions, numeric_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "boolean_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type_RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexCreateOptions, boolean_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "json_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type_RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexCreateOptions, json_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "date_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type_RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexCreateOptions, date_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "key_field".as_pg_cstr(),
            opttype: pg_sys::relopt_type_RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexCreateOptions, key_field_offset) as i32,
        },
    ];
    build_relopts(reloptions, validate, options)
}

#[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
unsafe fn build_relopts(
    reloptions: pg_sys::Datum,
    validate: bool,
    options: [pg_sys::relopt_parse_elt; NUM_REL_OPTS],
) -> *mut pg_sys::bytea {
    let rdopts = pg_sys::build_reloptions(
        reloptions,
        validate,
        RELOPT_KIND_PDB,
        std::mem::size_of::<SearchIndexCreateOptions>(), // TODO: proper size calculator
        options.as_ptr(),
        NUM_REL_OPTS as i32,
    );

    rdopts as *mut pg_sys::bytea
}

// build_reloptions is not available when pg<13, so we need our own
#[cfg(feature = "pg12")]
unsafe fn build_relopts(
    reloptions: pg_sys::Datum,
    validate: bool,
    options: [pg_sys::relopt_parse_elt; NUM_REL_OPTS],
) -> *mut pg_sys::bytea {
    let mut n_options = 0;
    let p_options = pg_sys::parseRelOptions(reloptions, validate, RELOPT_KIND_PDB, &mut n_options);
    if n_options == 0 {
        return std::ptr::null_mut();
    }

    for relopt in std::slice::from_raw_parts_mut(p_options, n_options as usize) {
        relopt.gen.as_mut().unwrap().lockmode = pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE;
    }

    let rdopts = pg_sys::allocateReloptStruct(
        std::mem::size_of::<SearchIndexCreateOptions>(),
        p_options,
        n_options,
    );
    pg_sys::fillRelOptions(
        rdopts,
        std::mem::size_of::<SearchIndexCreateOptions>(),
        p_options,
        n_options,
        validate,
        options.as_ptr(),
        options.len() as i32,
    );
    pg_sys::pfree(p_options as void_mut_ptr);

    rdopts as *mut pg_sys::bytea
}

impl SearchIndexCreateOptions {
    /// As a SearchFieldConfig is an enum, for it to be correctly serialized the variant needs
    /// to be present on the json object. This helper method will "wrap" the json object in
    /// another object with the variant key, which is passed into the function. For example:
    ///
    /// {"Text": { <actual_config> }}
    ///
    /// This way, serde will know to deserialize the config as SearchFieldConfig::Text.
    fn deserialize_config_fields(
        variant: String,
        serialized: String,
    ) -> Vec<(SearchFieldName, SearchFieldConfig)> {
        let config_map: HashMap<String, serde_json::Value> = json5::from_str(&serialized)
            .unwrap_or_else(|err| panic!("failed to deserialize {variant} field config: {err:?}"));

        config_map
            .into_iter()
            .map(|(field_name, field_config)| {
                (
                    field_name.into(),
                    serde_json::from_value(json!({variant.clone(): field_config})).unwrap(),
                )
            })
            .collect()
    }

    pub fn get_text_fields(&self) -> Vec<(SearchFieldName, SearchFieldConfig)> {
        let config = self.get_str(self.text_fields_offset, "".to_string());
        if config.is_empty() {
            return Vec::new();
        }
        Self::deserialize_config_fields("Text".into(), config)
    }

    pub fn get_numeric_fields(&self) -> Vec<(SearchFieldName, SearchFieldConfig)> {
        let config = self.get_str(self.numeric_fields_offset, "".to_string());
        if config.is_empty() {
            return Vec::new();
        }
        Self::deserialize_config_fields("Numeric".into(), config)
    }

    pub fn get_boolean_fields(&self) -> Vec<(SearchFieldName, SearchFieldConfig)> {
        let config = self.get_str(self.boolean_fields_offset, "".to_string());
        if config.is_empty() {
            return Vec::new();
        }
        Self::deserialize_config_fields("Boolean".into(), config)
    }

    pub fn get_json_fields(&self) -> Vec<(SearchFieldName, SearchFieldConfig)> {
        let config = self.get_str(self.json_fields_offset, "".to_string());
        if config.is_empty() {
            return Vec::new();
        }
        Self::deserialize_config_fields("Json".into(), config)
    }

    pub fn get_date_fields(&self) -> Vec<(SearchFieldName, SearchFieldConfig)> {
        let config = self.get_str(self.date_fields_offset, "".to_string());
        if config.is_empty() {
            return Vec::new();
        }
        Self::deserialize_config_fields("Date".into(), config)
    }

    pub fn get_key_field(&self) -> Option<SearchFieldName> {
        let key_field = self.get_str(self.key_field_offset, "".to_string());
        if key_field.is_empty() {
            None
        } else {
            Some(key_field.into())
        }
    }

    fn get_str(&self, offset: i32, default: String) -> String {
        if offset == 0 {
            default
        } else {
            let opts = self as *const _ as void_ptr as usize;
            let value =
                unsafe { CStr::from_ptr((opts + offset as usize) as *const std::os::raw::c_char) };

            value.to_str().unwrap().to_owned()
        }
    }
}

// it adds the tokenizer option to the list of relation options so we can parse it in amoptions
pub unsafe fn init() {
    // adding our own relopt type because zombodb does, but one of the built-in Postgres ones might be more appropriate
    RELOPT_KIND_PDB = pg_sys::add_reloption_kind();
    pg_sys::add_string_reloption(
        RELOPT_KIND_PDB,
        "text_fields".as_pg_cstr(),
        "JSON string specifying how text fields should be indexed".as_pg_cstr(),
        std::ptr::null(),
        Some(validate_text_fields),
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
        {
            pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE
        },
    );
    pg_sys::add_string_reloption(
        RELOPT_KIND_PDB,
        "numeric_fields".as_pg_cstr(),
        "JSON string specifying how numeric fields should be indexed".as_pg_cstr(),
        std::ptr::null(),
        Some(validate_numeric_fields),
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
        {
            pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE
        },
    );
    pg_sys::add_string_reloption(
        RELOPT_KIND_PDB,
        "boolean_fields".as_pg_cstr(),
        "JSON string specifying how boolean fields should be indexed".as_pg_cstr(),
        std::ptr::null(),
        Some(validate_boolean_fields),
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
        {
            pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE
        },
    );
    pg_sys::add_string_reloption(
        RELOPT_KIND_PDB,
        "json_fields".as_pg_cstr(),
        "JSON string specifying how JSON fields should be indexed".as_pg_cstr(),
        std::ptr::null(),
        Some(validate_json_fields),
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
        {
            pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE
        },
    );
    pg_sys::add_string_reloption(
        RELOPT_KIND_PDB,
        "date_fields".as_pg_cstr(),
        "JSON string specifying how date fields should be indexed".as_pg_cstr(),
        std::ptr::null(),
        Some(validate_date_fields),
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
        {
            pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE
        },
    );
    pg_sys::add_string_reloption(
        RELOPT_KIND_PDB,
        "key_field".as_pg_cstr(),
        "Column name as a string specify the unique identifier for a row".as_pg_cstr(),
        std::ptr::null(),
        Some(validate_key_field),
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
        {
            pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE
        },
    );
}
