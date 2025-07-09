--
-- setup the new ProximityClause type
--
/* <begin connected objects> */
-- pg_search/src/query/proximity/mod.rs:28
-- pg_search::query::proximity::ProximityClause
CREATE TYPE ProximityClause;

-- pg_search/src/query/proximity/mod.rs:28
-- pg_search::query::proximity::proximityclause_in
CREATE  FUNCTION "proximityclause_in"(
    "input" cstring /* core::option::Option<&core::ffi::c_str::CStr> */
) RETURNS ProximityClause /* core::option::Option<pg_search::query::proximity::ProximityClause> */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'proximityclause_in_wrapper';

-- pg_search/src/query/proximity/mod.rs:28
-- pg_search::query::proximity::proximityclause_out
CREATE  FUNCTION "proximityclause_out"(
    "input" ProximityClause /* pg_search::query::proximity::ProximityClause */
) RETURNS cstring /* alloc::ffi::c_str::CString */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'proximityclause_out_wrapper';

-- pg_search/src/query/proximity/mod.rs:28
-- pg_search::query::proximity::ProximityClause
CREATE TYPE ProximityClause (
    INTERNALLENGTH = variable,
    INPUT = proximityclause_in, /* pg_search::query::proximity::proximityclause_in */
    OUTPUT = proximityclause_out, /* pg_search::query::proximity::proximityclause_out */
    STORAGE = extended
);
/* </end connected objects> */

--
-- all the various UDFs for proximity searching
---

-- pg_search/src/api/proximity.rs:23
-- pg_search::api::proximity::prox_array
CREATE  FUNCTION "prox_array"(
    "clauses" VARIADIC ProximityClause[] /* pgrx::datum::array::VariadicArray<pg_search::query::proximity::ProximityClause> */
) RETURNS ProximityClause /* pg_search::query::proximity::ProximityClause */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'prox_array_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/proximity.rs:28
-- pg_search::api::proximity::prox_clause
CREATE  FUNCTION "prox_clause"(
    "left" ProximityClause, /* pg_search::query::proximity::ProximityClause */
    "distance" INT, /* i32 */
    "right" ProximityClause /* pg_search::query::proximity::ProximityClause */
) RETURNS ProximityClause /* core::result::Result<pg_search::query::proximity::ProximityClause, anyhow::Error> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'prox_clause_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/proximity.rs:41
-- pg_search::api::proximity::prox_clause_in_order
CREATE  FUNCTION "prox_clause_in_order"(
    "left" ProximityClause, /* pg_search::query::proximity::ProximityClause */
    "distance" INT, /* i32 */
    "right" ProximityClause /* pg_search::query::proximity::ProximityClause */
) RETURNS ProximityClause /* core::result::Result<pg_search::query::proximity::ProximityClause, anyhow::Error> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'prox_clause_in_order_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/proximity.rs:12
-- pg_search::api::proximity::prox_regex
CREATE  FUNCTION "prox_regex"(
    "regex" TEXT, /* alloc::string::String */
    "max_expansions" INT DEFAULT 50 /* i32 */
) RETURNS ProximityClause /* core::result::Result<pg_search::query::proximity::ProximityClause, anyhow::Error> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'prox_regex_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/proximity.rs:7
-- pg_search::api::proximity::prox_term
CREATE  FUNCTION "prox_term"(
    "term" TEXT /* alloc::string::String */
) RETURNS ProximityClause /* pg_search::query::proximity::ProximityClause */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'prox_term_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/proximity.rs:54
-- pg_search::api::proximity::proximity
CREATE  FUNCTION "proximity"(
    "field" FieldName, /* pg_search::api::FieldName */
    "left" ProximityClause, /* pg_search::query::proximity::ProximityClause */
    "distance" INT, /* i32 */
    "right" ProximityClause /* pg_search::query::proximity::ProximityClause */
) RETURNS SearchQueryInput /* core::result::Result<pg_search::query::SearchQueryInput, anyhow::Error> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'proximity_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/proximity.rs:70
-- pg_search::api::proximity::proximity_in_order
CREATE  FUNCTION "proximity_in_order"(
    "field" FieldName, /* pg_search::api::FieldName */
    "left" ProximityClause, /* pg_search::query::proximity::ProximityClause */
    "distance" INT, /* i32 */
    "right" ProximityClause /* pg_search::query::proximity::ProximityClause */
) RETURNS SearchQueryInput /* core::result::Result<pg_search::query::SearchQueryInput, anyhow::Error> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'proximity_in_order_wrapper';
/* </end connected objects> */

--
-- convenience CASTs
--

/* <begin connected objects> */
-- pg_search/src/api/proximity.rs:91
-- pg_search::api::proximity::text_array_to_prox_clause
CREATE  FUNCTION "text_array_to_prox_clause"(
    "t" TEXT[] /* alloc::vec::Vec<alloc::string::String> */
) RETURNS ProximityClause /* pg_search::query::proximity::ProximityClause */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'text_array_to_prox_clause_wrapper';
-- pg_search/src/api/proximity.rs:91
-- pg_search::api::proximity::text_array_to_prox_clause
CREATE CAST (
    TEXT[] /* alloc::vec::Vec<alloc::string::String> */
    AS
    ProximityClause /* pg_search::query::proximity::ProximityClause */
    )
    WITH FUNCTION text_array_to_prox_clause AS IMPLICIT;
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/proximity.rs:86
-- pg_search::api::proximity::text_to_prox_clause
CREATE  FUNCTION "text_to_prox_clause"(
    "t" TEXT /* alloc::string::String */
) RETURNS ProximityClause /* pg_search::query::proximity::ProximityClause */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'text_to_prox_clause_wrapper';
-- pg_search/src/api/proximity.rs:86
-- pg_search::api::proximity::text_to_prox_clause
CREATE CAST (
    TEXT /* alloc::string::String */
    AS
    ProximityClause /* pg_search::query::proximity::ProximityClause */
)
WITH FUNCTION text_to_prox_clause AS IMPLICIT;
