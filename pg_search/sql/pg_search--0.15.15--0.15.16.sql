\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.15.9'" to load this file. \quit

/* <begin connected objects> */
-- pg_search/src/api/index.rs:771
-- pg_search::api::index::term
CREATE  FUNCTION "term"(
	"field" FieldName, /* pg_search::api::index::FieldName */
	"value" inet DEFAULT NULL /* core::option::Option<pgrx::datum::inet::Inet> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'inet_wrapper';
/* </end connected objects> */
