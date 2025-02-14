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