use memoffset::*;
use pgrx::pg_sys::AsPgCStr;
use pgrx::*;
use serde_json::from_str;
use std::collections::HashMap;
use std::ffi::CStr;

use crate::sparse_index::fields::{
    ParadeBooleanOptions, ParadeJsonOptions, ParadeNumericOptions, ParadeTextOptions,
};

/* ADDING OPTIONS
 * in init(), call pg_sys::add_{type}_reloption (check postgres docs for what args you need)
 * add the corresponding entries to ParadeOptions struct definition
 * in amoptions(), add a relopt_parse_elt entry to the options array and change NUM_REL_OPTS
 * Note that for string options, postgres will give you the offset of the string, and you have to read the string
 * yourself (see get_tokenizer)
*/

/* READING OPTIONS
 * options are placed in relation.rd_options
 * As in ambuild(), cast relation.rd_options into ParadeOptions using PgBox (because ParadeOptions
 * is a postgres-allocated object) and use getters and setters
*/

static mut RELOPT_KIND_PDB: pg_sys::relopt_kind = 0;

// Postgres handles string options by placing each option offset bytes from the start of rdopts and
// plops the offset in the struct
#[repr(C)]
pub struct ParadeOptions {
    // varlena header (needed bc postgres treats this as bytea)
    vl_len_: i32,
    text_fields_offset: i32,
    numeric_fields_offset: i32,
    boolean_fields_offset: i32,
    json_fields_offset: i32,
}

#[pg_guard]
extern "C" fn validate_text_fields(value: *const std::os::raw::c_char) {
    let json_str = cstr_to_rust_str(value);

    if json_str.is_empty() {
        return;
    }

    let _options: HashMap<String, ParadeTextOptions> =
        from_str(&json_str).expect("failed to validate text_fields");
}

#[pg_guard]
extern "C" fn validate_numeric_fields(value: *const std::os::raw::c_char) {
    let json_str = cstr_to_rust_str(value);

    if json_str.is_empty() {
        return;
    }

    let _options: HashMap<String, ParadeNumericOptions> =
        from_str(&json_str).expect("failed to validate numeric_fields");
}

#[pg_guard]
extern "C" fn validate_boolean_fields(value: *const std::os::raw::c_char) {
    let json_str = cstr_to_rust_str(value);

    if json_str.is_empty() {
        return;
    }

    let _options: HashMap<String, ParadeBooleanOptions> =
        from_str(&json_str).expect("failed to validate boolean_fields");
}

#[pg_guard]
extern "C" fn validate_json_fields(value: *const std::os::raw::c_char) {
    let json_str = cstr_to_rust_str(value);

    if json_str.is_empty() {
        return;
    }

    let _options: HashMap<String, ParadeJsonOptions> =
        from_str(&json_str).expect("failed to validate boolean_fields");
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
const NUM_REL_OPTS: usize = 4;
#[pg_guard]
pub unsafe extern "C" fn amoptions(
    reloptions: pg_sys::Datum,
    validate: bool,
) -> *mut pg_sys::bytea {
    let options: [pg_sys::relopt_parse_elt; NUM_REL_OPTS] = [
        pg_sys::relopt_parse_elt {
            optname: "text_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type_RELOPT_TYPE_STRING,
            offset: offset_of!(ParadeOptions, text_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "numeric_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type_RELOPT_TYPE_STRING,
            offset: offset_of!(ParadeOptions, numeric_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "boolean_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type_RELOPT_TYPE_STRING,
            offset: offset_of!(ParadeOptions, boolean_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "json_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type_RELOPT_TYPE_STRING,
            offset: offset_of!(ParadeOptions, json_fields_offset) as i32,
        },
    ];
    build_relopts(reloptions, validate, options)
}

#[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15"))]
unsafe fn build_relopts(
    reloptions: pg_sys::Datum,
    validate: bool,
    options: [pg_sys::relopt_parse_elt; NUM_REL_OPTS],
) -> *mut pg_sys::bytea {
    let rdopts = pg_sys::build_reloptions(
        reloptions,
        validate,
        RELOPT_KIND_PDB,
        std::mem::size_of::<ParadeOptions>(), // TODO: proper size calculator
        options.as_ptr(),
        NUM_REL_OPTS as i32,
    );

    rdopts as *mut pg_sys::bytea
}

// build_reloptions is not available when pg<13, so we need our own
#[cfg(any(feature = "pg10", feature = "pg11", feature = "pg12"))]
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

    let rdopts =
        pg_sys::allocateReloptStruct(std::mem::size_of::<ParadeOptions>(), p_options, n_options);
    pg_sys::fillRelOptions(
        rdopts,
        std::mem::size_of::<ParadeOptions>(),
        p_options,
        n_options,
        validate,
        options.as_ptr(),
        options.len() as i32,
    );
    pg_sys::pfree(p_options as void_mut_ptr);

    rdopts as *mut pg_sys::bytea
}

impl ParadeOptions {
    pub fn get_text_fields(&self) -> HashMap<String, ParadeTextOptions> {
        let fields = self.get_str(self.text_fields_offset, "".to_string());

        if fields.is_empty() {
            return HashMap::new();
        }

        from_str::<HashMap<String, ParadeTextOptions>>(&fields).expect("failed to get text_fields")
    }

    pub fn get_numeric_fields(&self) -> HashMap<String, ParadeNumericOptions> {
        let fields = self.get_str(self.numeric_fields_offset, "".to_string());

        if fields.is_empty() {
            return HashMap::new();
        }

        from_str::<HashMap<String, ParadeNumericOptions>>(&fields)
            .expect("failed to parse numeric_fields")
    }

    pub fn get_boolean_fields(&self) -> HashMap<String, ParadeBooleanOptions> {
        let fields = self.get_str(self.boolean_fields_offset, "".to_string());

        if fields.is_empty() {
            return HashMap::new();
        }

        from_str::<HashMap<String, ParadeBooleanOptions>>(&fields)
            .expect("failed to parse boolean_fields")
    }

    pub fn get_json_fields(&self) -> HashMap<String, ParadeJsonOptions> {
        let fields = self.get_str(self.json_fields_offset, "".to_string());

        if fields.is_empty() {
            return HashMap::new();
        }

        from_str::<HashMap<String, ParadeJsonOptions>>(&fields)
            .expect("failed to parse json_fields")
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
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15"))]
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
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15"))]
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
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15"))]
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
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15"))]
        {
            pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE
        },
    );
}
