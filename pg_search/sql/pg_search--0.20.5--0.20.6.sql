\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.20.6'" to load this file. \quit

/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:326
-- pg_search::api::tokenizers::definitions::pdb::alias_out_safe
CREATE  FUNCTION pdb."alias_out_safe"(
	"input" pdb.alias /* pg_search::api::tokenizers::definitions::pdb::Alias */
) RETURNS cstring /* alloc::ffi::c_str::CString */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'alias_out_safe_wrapper';

-- Override the output function for pdb.alias
CREATE OR REPLACE FUNCTION pdb.alias_out(pdb.alias) RETURNS cstring
AS 'MODULE_PATHNAME', 'alias_out_safe_wrapper'
LANGUAGE c IMMUTABLE STRICT PARALLEL SAFE;
