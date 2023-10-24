use memoffset::*;
use pgrx::pg_sys::AsPgCStr;
use pgrx::*;
use serde_json::from_str;
use std::collections::HashMap;
use std::ffi::CStr;

/* ADDING OPTIONS
 * in init(), call pg_sys::add_{type}_reloption (check postgres docs for what args you need)
 * add the corresponding entries to SparseOptions struct definition
 * in amoptions(), add a relopt_parse_elt entry to the options array and change NUM_REL_OPTS
 * Note that for string options, postgres will give you the offset of the string, and you have to read the string
 * yourself (see get_tokenizer)
*/

/* READING OPTIONS
 * options are placed in relation.rd_options
 * As in ambuild(), cast relation.rd_options into SparseOptions using PgBox (because SparseOptions
 * is a postgres-allocated object) and use getters and setters
*/

static mut RELOPT_KIND_PDB: pg_sys::relopt_kind = 0;

// Option defaults
pub const DEFAULT_M: i32 = 16;
pub const DEFAULT_EF_SEARCH: i32 = 40;
pub const DEFAULT_EF_SEARCH_CONSTRUCTION: i32 = 64;
pub const DEFAULT_RANDOM_SEED: i32 = 1;

// Option ranges
const MIN_M: i32 = 2;
const MAX_M: i32 = 100;
const MIN_EF_SEARCH: i32 = 1;
const MAX_EF_SEARCH: i32 = 100;
const MIN_EF_SEARCH_CONSTRUCTION: i32 = 4;
const MAX_EF_SEARCH_CONSTRUCTION: i32 = 1000;
const MIN_RANDOM_SEED: i32 = 0;
const MAX_RANDOM_SEED: i32 = 32678;

// Number of options
const NUM_REL_OPTS: usize = 4;

// Postgres handles string options by placing each option offset bytes from the start of rdopts and
// plops the offset in the struct
#[derive(Debug)]
#[repr(C)]
pub struct SparseOptions {
    // varlena header (needed bc postgres treats this as bytea)
    vl_len_: i32,
    pub m: i32,
    pub ef_search: i32,
    pub ef_construction: i32,
    pub random_seed: i32,
}

#[pg_guard]
pub unsafe extern "C" fn amoptions(
    reloptions: pg_sys::Datum,
    validate: bool,
) -> *mut pg_sys::bytea {
    let options: [pg_sys::relopt_parse_elt; NUM_REL_OPTS] = [
        pg_sys::relopt_parse_elt {
            optname: "m".as_pg_cstr(),
            opttype: pg_sys::relopt_type_RELOPT_TYPE_STRING,
            offset: offset_of!(SparseOptions, m) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "ef_search".as_pg_cstr(),
            opttype: pg_sys::relopt_type_RELOPT_TYPE_STRING,
            offset: offset_of!(SparseOptions, ef_search) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "ef_construction".as_pg_cstr(),
            opttype: pg_sys::relopt_type_RELOPT_TYPE_STRING,
            offset: offset_of!(SparseOptions, ef_construction) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "random_seed".as_pg_cstr(),
            opttype: pg_sys::relopt_type_RELOPT_TYPE_STRING,
            offset: offset_of!(SparseOptions, random_seed) as i32,
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
        std::mem::size_of::<SparseOptions>(), // TODO: proper size calculator
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
        pg_sys::allocateReloptStruct(std::mem::size_of::<SparseOptions>(), p_options, n_options);
    pg_sys::fillRelOptions(
        rdopts,
        std::mem::size_of::<SparseOptions>(),
        p_options,
        n_options,
        validate,
        options.as_ptr(),
        options.len() as i32,
    );
    pg_sys::pfree(p_options as void_mut_ptr);

    rdopts as *mut pg_sys::bytea
}

impl SparseOptions {}

// it adds the tokenizer option to the list of relation options so we can parse it in amoptions
pub unsafe fn init() {
    RELOPT_KIND_PDB = pg_sys::add_reloption_kind();
    pg_sys::add_int_reloption(
        RELOPT_KIND_PDB,
        "m".as_pg_cstr(),
        "Maximum numbers of connections per layer (16 by default)".as_pg_cstr(),
        DEFAULT_M,
        MIN_M,
        MAX_M,
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15"))]
        {
            pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE
        },
    );
    pg_sys::add_int_reloption(
        RELOPT_KIND_PDB,
        "ef_search".as_pg_cstr(),
        "The size of the dynamic candidate list for search (40 by default)".as_pg_cstr(),
        DEFAULT_EF_SEARCH,
        MIN_EF_SEARCH,
        MAX_EF_SEARCH_CONSTRUCTION,
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15"))]
        {
            pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE
        },
    );
    pg_sys::add_int_reloption(
        RELOPT_KIND_PDB,
        "ef_construction".as_pg_cstr(),
        "The size of the dynamic candidate list for constructing the graph (64 by default)"
            .as_pg_cstr(),
        DEFAULT_EF_SEARCH_CONSTRUCTION,
        MIN_EF_SEARCH_CONSTRUCTION,
        MAX_EF_SEARCH_CONSTRUCTION,
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15"))]
        {
            pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE
        },
    );
    pg_sys::add_int_reloption(
        RELOPT_KIND_PDB,
        "random_seed".as_pg_cstr(),
        "Random seed for level generation (1 by default)".as_pg_cstr(),
        DEFAULT_RANDOM_SEED,
        MIN_RANDOM_SEED,
        MAX_RANDOM_SEED,
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15"))]
        {
            pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE
        },
    );
}
