/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/bootstrap/create_bm25.rs:424
-- pg_search::bootstrap::create_bm25::force_merge
CREATE  FUNCTION "force_merge"(
    "index" regclass, /* pgrx::rel::PgRelation */
    "oversized_layer_size_pretty" TEXT /* alloc::string::String */
) RETURNS TABLE (
                    "new_segments" bigint,  /* i64 */
                    "merged_segments" bigint  /* i64 */
                )
    STRICT
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'force_merge_pretty_bytes_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/bootstrap/create_bm25.rs:441
-- pg_search::bootstrap::create_bm25::force_merge
CREATE  FUNCTION "force_merge"(
    "index" regclass, /* pgrx::rel::PgRelation */
    "oversized_layer_size_bytes" bigint /* i64 */
) RETURNS TABLE (
                    "new_segments" bigint,  /* i64 */
                    "merged_segments" bigint  /* i64 */
                )
    STRICT
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'force_merge_raw_bytes_wrapper';