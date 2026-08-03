-- pg_search/src/api/operator/eqeqeq.rs:38
-- pg_search::api::operator::eqeqeq::term_search_query_input
CREATE  FUNCTION "term_search_query_input"(
	"field" FieldName, /* FieldName */
	"query" pdb.Query /* pdb :: Query */
) RETURNS SearchQueryInput /* SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_search_query_input_wrapper';
