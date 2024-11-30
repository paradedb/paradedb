/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/index.rs:552
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