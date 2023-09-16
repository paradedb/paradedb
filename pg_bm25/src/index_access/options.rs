use pgrx::*;
use pgrx::pg_sys::AsPgCStr;
use std::ffi::CStr; // TODO: remove this later

static mut RELOPT_KIND_PDB: pg_sys::relopt_kind = 0;

// postgres handles string options by placing each option offset bytes from the start of rdopts and
// plops the offset in the struct
pub struct ParadeOptions {
    // varlena header (needed bc postgres treats this as bytea)
    vl_len_: i32,

    tokenizer_offset: i32,
}

// pg_guard the validators so the panic only exits the query
#[pg_guard]
extern "C" fn validate_tokenizer(value: *const std::os::raw::c_char) {
    if value.is_null() {
        return;
    }

    let value = unsafe { CStr::from_ptr(value) }.to_str()
        .expect("failed to convert tokenizer to utf-8");

    info!("tokenizer: {}", value);

    // TODO: not hardcode this
    if value != "default" && value != "raw" && value != "en_stem" {
        panic!("invalid tokenizer: {}", value);
    }
}
// For now, we support changing the tokenizer between default, raw, and en_stem
#[pg_guard]
pub unsafe extern "C" fn amoptions(
    reloptions: pg_sys::Datum,
    validate: bool,
) -> *mut pg_sys::bytea {
    // TODO: not hardcode offset
    let options: [pg_sys::relopt_parse_elt; 1] = [
    pg_sys::relopt_parse_elt {
        optname: "tokenizer".as_pg_cstr(),
        opttype: pg_sys::relopt_type_RELOPT_TYPE_STRING,
        offset: 4,
    },
    ];
    let rdopts = pg_sys::build_reloptions(
        reloptions,
        validate,
        RELOPT_KIND_PDB,
        std::mem::size_of::<ParadeOptions>(), // TODO: proper size calculator
        options.as_ptr(),
        1
    );
    rdopts as *mut pg_sys::bytea
}

impl ParadeOptions {
    pub fn get_tokenizer(&self) -> String {
        info!("tokenizer offset is {}", self.tokenizer_offset);
        if self.tokenizer_offset == 0 {
            return "default".to_string();
        }
        let opts = self as *const _ as void_ptr as usize;
        let value = unsafe {CStr::from_ptr((opts + self.tokenizer_offset as usize) as *const std::os::raw::c_char) };
        value.to_str().unwrap().to_owned()
    }
}

// this is modeled after ZomboDB's function
// it adds the tokenizer option to the list of relation options so we can parse it in amoptions
pub unsafe fn init() {
    // following ZomboDB, I'm adding our own relopt type
    // but one of the built-in Postgres ones might be more appropriate
    RELOPT_KIND_PDB = pg_sys::add_reloption_kind();
    pg_sys::add_string_reloption(
        RELOPT_KIND_PDB,
        "tokenizer".as_pg_cstr(),
        "Tantivy tokenizer used".as_pg_cstr(),
        "default".as_pg_cstr(),
        Some(validate_tokenizer),
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15"))]
        {
            // "The default choice for any new option should be AccessExclusiveLock." - postgres
            pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE
        },
    );
}

