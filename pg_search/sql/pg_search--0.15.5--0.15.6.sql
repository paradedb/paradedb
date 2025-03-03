-- pg_search/src/bootstrap/create_bm25.rs:235
-- pg_search::bootstrap::create_bm25::is_merging
CREATE  FUNCTION "is_merging"(
    "index" regclass /* pgrx::rel::PgRelation */
) RETURNS bool /* bool */
    STRICT
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'is_merging_wrapper';