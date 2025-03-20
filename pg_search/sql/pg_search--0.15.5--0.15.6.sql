-- pg_search/src/bootstrap/create_bm25.rs:235
-- pg_search::bootstrap::create_bm25::is_merging
CREATE  FUNCTION "is_merging"(
    "index" regclass /* pgrx::rel::PgRelation */
) RETURNS bool /* bool */
    STRICT
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'is_merging_wrapper';

--
-- b/c of predicate pushdown work (#2197)
--
/* <begin connected objects> */
-- pg_search/src/api/index.rs:549
-- pg_search::api::index::term_with_operator
CREATE  FUNCTION "term_with_operator"(
    "field" FieldName, /* pg_search::api::index::FieldName */
    "operator" TEXT, /* alloc::string::String */
    "value" anyelement /* pgrx::datum::anyelement::AnyElement */
) RETURNS SearchQueryInput /* core::result::Result<pg_search::query::SearchQueryInput, anyhow::Error> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_with_operator_wrapper';
/* </end connected objects> */
DROP FUNCTION IF EXISTS term(field fieldname, value pg_catalog.timestamptz);
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/index.rs:762
-- pg_search::api::index::term
CREATE  FUNCTION "term"(
    "field" FieldName DEFAULT NULL, /* core::option::Option<pg_search::api::index::FieldName> */
    "value" timestamp with time zone DEFAULT NULL /* core::option::Option<pgrx::datum::time_stamp_with_timezone::TimestampWithTimeZone> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'timestamp_with_time_zone_wrapper';
