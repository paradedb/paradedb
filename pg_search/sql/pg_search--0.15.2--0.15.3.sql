/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/searchqueryinput.rs:55
-- pg_search::api::operator::searchqueryinput::with_index
CREATE  FUNCTION "with_index"(
    "index" regclass, /* pgrx::rel::PgRelation */
    "query" SearchQueryInput /* pg_search::query::SearchQueryInput */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'with_index_wrapper';