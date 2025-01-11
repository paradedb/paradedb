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
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/index.rs:554
-- pg_search::api::index::regex_phrase
CREATE  FUNCTION "regex_phrase"(
    "field" FieldName, /* pg_search::api::index::FieldName */
    "regexes" TEXT[], /* alloc::vec::Vec<alloc::string::String> */
    "slop" INT DEFAULT NULL, /* core::option::Option<i32> */
    "max_expansions" INT DEFAULT NULL /* core::option::Option<i32> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'regex_phrase_wrapper';