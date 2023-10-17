use pgrx::*;

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
}

const NUM_REL_OPTS: usize = 0;
#[pg_guard]
pub unsafe extern "C" fn amoptions(
    reloptions: pg_sys::Datum,
    validate: bool,
) -> *mut pg_sys::bytea {
    let options: [pg_sys::relopt_parse_elt; NUM_REL_OPTS] = [];
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

impl ParadeOptions {}

// it adds the tokenizer option to the list of relation options so we can parse it in amoptions
pub unsafe fn init() {}
