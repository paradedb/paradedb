/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/slop.rs:120
-- creates:
--   Type(pg_search::api::operator::slop::typedef::SlopType)
CREATE TYPE pg_catalog.slop;
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/slop.rs:128
-- pg_search::api::operator::slop::typedef::slop_in
CREATE  FUNCTION "slop_in"(
    "input" cstring, /* &core::ffi::c_str::CStr */
    "_typoid" oid, /* pgrx_pg_sys::submodules::oids::Oid */
    "typmod" INT /* i32 */
) RETURNS slop /* pg_search::api::operator::slop::SlopType */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'slop_in_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/slop.rs:136
-- pg_search::api::operator::slop::typedef::slop_out
CREATE  FUNCTION "slop_out"(
    "input" slop /* pg_search::api::operator::slop::SlopType */
) RETURNS cstring /* alloc::ffi::c_str::CString */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'slop_out_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/slop.rs:195
-- pg_search::api::operator::slop::slop_to_boost
CREATE  FUNCTION "slop_to_boost"(
    "input" slop, /* pg_search::api::operator::slop::SlopType */
    "typmod" INT, /* i32 */
    "is_explicit" bool /* bool */
) RETURNS boost /* pg_search::api::operator::boost::BoostType */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'slop_to_boost_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/slop.rs:214
-- pg_search::api::operator::slop::slop_to_slop
CREATE  FUNCTION "slop_to_slop"(
    "input" slop, /* pg_search::api::operator::slop::SlopType */
    "typmod" INT, /* i32 */
    "is_explicit" bool /* bool */
) RETURNS slop /* pg_search::api::operator::slop::SlopType */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'slop_to_slop_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/slop.rs:143
-- pg_search::api::operator::slop::typedef::slop_typmod_in
CREATE  FUNCTION "slop_typmod_in"(
    "typmod_parts" cstring[] /* pgrx::datum::array::Array<&core::ffi::c_str::CStr> */
) RETURNS INT /* i32 */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'slop_typmod_in_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/slop.rs:156
-- pg_search::api::operator::slop::typedef::slop_typmod_out
CREATE  FUNCTION "slop_typmod_out"(
    "typmod" INT /* i32 */
) RETURNS cstring /* alloc::ffi::c_str::CString */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'slop_typmod_out_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/slop.rs:162
-- requires:
--   SlopType_shell
--   slop_in
--   slop_out
--   slop_typmod_in
--   slop_typmod_out
CREATE TYPE pg_catalog.slop (
                                INPUT = slop_in,
                                OUTPUT = slop_out,
                                INTERNALLENGTH = VARIABLE,
                                LIKE = text,
                                TYPMOD_IN = slop_typmod_in,
                                TYPMOD_OUT = slop_typmod_out
                            );
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/hashhashhash.rs:53
-- pg_search::api::operator::hashhashhash::search_with_phrase_slop
-- requires:
--   SlopType_final
CREATE  FUNCTION "search_with_phrase_slop"(
    "_field" TEXT, /* &str */
    "terms_to_tokenize" slop /* pg_search::api::operator::slop::SlopType */
) RETURNS bool /* bool */
    IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_phrase_slop_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/slop.rs:190
-- pg_search::api::operator::slop::slop_to_query
CREATE  FUNCTION "slop_to_query"(
    "input" slop /* pg_search::api::operator::slop::SlopType */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'slop_to_query_wrapper';
-- pg_search/src/api/operator/slop.rs:190
-- pg_search::api::operator::slop::slop_to_query
CREATE CAST (
    slop /* pg_search::api::operator::slop::SlopType */
    AS
    pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    )
    WITH FUNCTION slop_to_query AS IMPLICIT;
ALTER FUNCTION paradedb.search_with_phrase_slop SUPPORT paradedb.search_with_phrase_support;
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/slop.rs:184
-- pg_search::api::operator::slop::query_to_slop
CREATE  FUNCTION "query_to_slop"(
    "input" pdb.Query, /* pg_search::query::pdb_query::pdb::Query */
    "typmod" INT, /* i32 */
    "_is_explicit" bool /* bool */
) RETURNS slop /* pg_search::api::operator::slop::SlopType */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'query_to_slop_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/slop.rs:219
-- requires:
--   query_to_slop
--   slop_to_boost
--   slop_to_slop
--   SlopType_final
CREATE CAST (pdb.query AS pg_catalog.slop) WITH FUNCTION query_to_slop(pdb.query, integer, boolean) AS ASSIGNMENT;
CREATE CAST (pg_catalog.slop AS pg_catalog.boost) WITH FUNCTION slop_to_boost(pg_catalog.slop, integer, boolean) AS IMPLICIT;
CREATE CAST (pg_catalog.slop AS pg_catalog.slop) WITH FUNCTION slop_to_slop(pg_catalog.slop, integer, boolean) AS IMPLICIT;