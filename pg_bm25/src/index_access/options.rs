use memoffset::*;
use pgrx::pg_sys::AsPgCStr;
use pgrx::*;
use serde_json::Value;
use std::ffi::CStr;
use std::str::FromStr;

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

// postgres handles string options by placing each option offset bytes from the start of rdopts and
// plops the offset in the struct
#[repr(C)]
pub struct ParadeOptions {
    // varlena header (needed bc postgres treats this as bytea)
    vl_len_: i32,
    fields_offset: i32,
}

#[derive(Debug, PartialEq, Eq)]
enum OptionKey {
    Tokenizer,
    Fast,
    IndexOption,
}

#[derive(Debug, PartialEq, Eq)]
enum TokenizerOption {
    Default,
    Raw,
    EnStem,
    Whitespace,
}

impl FromStr for OptionKey {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "tokenizer" => Ok(OptionKey::Tokenizer),
            "fast" => Ok(OptionKey::Fast),
            "index_option" => Ok(OptionKey::IndexOption),
            _ => Err(()),
        }
    }
}

impl FromStr for TokenizerOption {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "default" => Ok(TokenizerOption::Default),
            "raw" => Ok(TokenizerOption::Raw),
            "en_stem" => Ok(TokenizerOption::EnStem),
            "whitespace" => Ok(TokenizerOption::Whitespace),
            _ => Err(()),
        }
    }
}

#[pg_guard]
extern "C" fn validate_fields(value: *const std::os::raw::c_char) {
    if value.is_null() {
        return;
    }

    let rust_str = unsafe { CStr::from_ptr(value) }
        .to_str()
        .expect("failed to parse fields as utf-8");

    let json = serde_json::from_str::<Value>(rust_str).expect("fields is not valid JSON");

    if let Value::Object(columns) = &json {
        for options in columns.values() {
            if let Value::Object(options_map) = options {
                for (key, value) in options_map {
                    let key_enum = key.parse::<OptionKey>().expect("Invalid key in options");
                    match key_enum {
                        OptionKey::Tokenizer => {
                            value
                                .as_str()
                                .expect("Tokenizer should be a string")
                                .parse::<TokenizerOption>()
                                .expect("Invalid tokenizer");
                        }
                        _ => {}
                    }
                }
            } else {
                panic!("Options should be a JSON object");
            }
        }
    } else {
        panic!("The JSON should be an object with columns as keys");
    }
}

// For now, we support changing the tokenizer between default, raw, and en_stem
const NUM_REL_OPTS: usize = 1;
#[pg_guard]
pub unsafe extern "C" fn amoptions(
    reloptions: pg_sys::Datum,
    validate: bool,
) -> *mut pg_sys::bytea {
    let options: [pg_sys::relopt_parse_elt; NUM_REL_OPTS] = [pg_sys::relopt_parse_elt {
        optname: "fields".as_pg_cstr(),
        opttype: pg_sys::relopt_type_RELOPT_TYPE_STRING,
        offset: offset_of!(ParadeOptions, fields_offset) as i32,
    }];
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
    pub fn get_fields(&self) -> Value {
        let fields = self.get_str(self.fields_offset, "".to_string());
        serde_json::from_str::<Value>(&fields).expect("fields is not valid JSON")
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
        "fields".as_pg_cstr(),
        "JSON specifying how fields should be indexed".as_pg_cstr(),
        std::ptr::null(),
        Some(validate_fields),
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15"))]
        {
            pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE
        },
    );
}
