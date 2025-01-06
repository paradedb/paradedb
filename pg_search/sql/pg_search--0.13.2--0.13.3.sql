/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/bootstrap/create_bm25.rs:389
-- pg_search::bootstrap::create_bm25::page_info
CREATE  FUNCTION "page_info"(
	"index" regclass, /* pgrx::rel::PgRelation */
	"blockno" bigint /* i64 */
) RETURNS TABLE (
	"offsetno" INT,  /* i32 */
	"size" INT,  /* i32 */
	"visible" bool,  /* bool */
	"recyclable" bool,  /* bool */
	"contents" jsonb  /* pgrx::datum::json::JsonB */
)
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'page_info_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/bootstrap/create_bm25.rs:367
-- pg_search::bootstrap::create_bm25::storage_info
CREATE  FUNCTION "storage_info"(
	"index" regclass /* pgrx::rel::PgRelation */
) RETURNS TABLE (
	"block" bigint,  /* i64 */
	"max_offset" INT  /* i32 */
)
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'storage_info_wrapper';
