/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/index.rs:856
-- pg_search::api::index::jsonb_to_searchqueryinput
CREATE  FUNCTION "jsonb_to_searchqueryinput"(
	"query" jsonb /* pgrx::datum::json::JsonB */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'jsonb_to_searchqueryinput_wrapper';
-- pg_search/src/api/index.rs:856
-- pg_search::api::index::jsonb_to_searchqueryinput
CREATE CAST (
	jsonb /* pgrx::datum::json::JsonB */
	AS
	SearchQueryInput /* pg_search::query::SearchQueryInput */
)
WITH FUNCTION jsonb_to_searchqueryinput AS IMPLICIT;
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/index.rs:875
-- requires:
--   jsonb_to_searchqueryinput
ALTER FUNCTION jsonb_to_searchqueryinput IMMUTABLE;
DROP FUNCTION IF EXISTS more_like_this(with_document_fields text, with_min_doc_frequency pg_catalog.int4, with_max_doc_frequency pg_catalog.int4, with_min_term_frequency pg_catalog.int4, with_max_query_terms pg_catalog.int4, with_min_word_length pg_catalog.int4, with_max_word_length pg_catalog.int4, with_boost_factor pg_catalog.float4, with_stop_words text[]);
CREATE OR REPLACE FUNCTION more_like_this(document_fields text, min_doc_frequency pg_catalog.int4 DEFAULT NULL, max_doc_frequency pg_catalog.int4 DEFAULT NULL, min_term_frequency pg_catalog.int4 DEFAULT NULL, max_query_terms pg_catalog.int4 DEFAULT NULL, min_word_length pg_catalog.int4 DEFAULT NULL, max_word_length pg_catalog.int4 DEFAULT NULL, boost_factor pg_catalog.float4 DEFAULT NULL, stop_words text[] DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'more_like_this_fields_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS more_like_this(with_document_id anyelement, with_min_doc_frequency pg_catalog.int4, with_max_doc_frequency pg_catalog.int4, with_min_term_frequency pg_catalog.int4, with_max_query_terms pg_catalog.int4, with_min_word_length pg_catalog.int4, with_max_word_length pg_catalog.int4, with_boost_factor pg_catalog.float4, with_stop_words text[]);
CREATE OR REPLACE FUNCTION more_like_this(document_id anyelement, min_doc_frequency pg_catalog.int4 DEFAULT NULL, max_doc_frequency pg_catalog.int4 DEFAULT NULL, min_term_frequency pg_catalog.int4 DEFAULT NULL, max_query_terms pg_catalog.int4 DEFAULT NULL, min_word_length pg_catalog.int4 DEFAULT NULL, max_word_length pg_catalog.int4 DEFAULT NULL, boost_factor pg_catalog.float4 DEFAULT NULL, stop_words text[] DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'more_like_this_id_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
