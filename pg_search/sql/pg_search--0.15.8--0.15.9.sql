DROP FUNCTION IF EXISTS is_merging(index regclass);
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/bootstrap/create_bm25.rs:123
-- pg_search::bootstrap::create_bm25::merge_info
CREATE  FUNCTION "merge_info"(
    "index" regclass /* pgrx::rel::PgRelation */
) RETURNS TABLE (
                    "pid" INT,  /* i32 */
                    "xmin" NUMERIC,  /* pgrx::datum::numeric::AnyNumeric */
                    "xmax" NUMERIC,  /* pgrx::datum::numeric::AnyNumeric */
                    "segno" TEXT  /* alloc::string::String */
                )
    STRICT
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'merge_info_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/lib.rs:103
-- pg_search::random_words
CREATE  FUNCTION "random_words"(
    "num_words" INT /* i32 */
) RETURNS TEXT /* alloc::string::String */
    STRICT
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'random_words_wrapper';