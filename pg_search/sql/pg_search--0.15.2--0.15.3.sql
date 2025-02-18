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

-- pg_search/src/bootstrap/create_bm25.rs:545
-- pg_search::bootstrap::create_bm25::reset_num_segments
CREATE  FUNCTION "reset_num_segments"(
    "index" regclass, /* pgrx::rel::PgRelation */
    "num_segments" INT /* i32 */
) RETURNS INT /* core::result::Result<i32, anyhow::Error> */
    STRICT
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'reset_num_segments_wrapper';