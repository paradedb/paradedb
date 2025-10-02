\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.16.1'" to load this file. \quit

DROP FUNCTION IF EXISTS term(field fieldname, value inet);

/* <begin connected objects> */
-- pg_search/src/api/builder_fns.rs:761
-- pg_search::api::builder_fns::term
CREATE  FUNCTION "term"(
	"field" FieldName DEFAULT NULL, /* core::option::Option<pg_search::api::FieldName> */
	"value" inet DEFAULT NULL /* core::option::Option<pgrx::datum::inet::Inet> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'inet_wrapper';
/* </end connected objects> */


/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/builder_fns.rs:615
-- pg_search::api::builder_fns::terms_with_operator
CREATE  FUNCTION "terms_with_operator"(
    "field" FieldName, /* pg_search::api::FieldName */
    "operator" TEXT, /* alloc::string::String */
    "value" anyelement, /* pgrx::datum::anyelement::AnyElement */
    "conjunction_mode" bool /* bool */
) RETURNS SearchQueryInput /* core::result::Result<pg_search::query::SearchQueryInput, anyhow::Error> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'terms_with_operator_wrapper';