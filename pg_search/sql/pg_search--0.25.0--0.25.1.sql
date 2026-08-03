<<<<<<< HEAD
\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.25.1'" to load this file. \quit

-- 0.25.0 -> 0.25.1: no schema changes (epsilon GUC removal and the
-- bounds_scope reloption are not schema objects).
=======
-- pg_search/src/api/operator/eqeqeq.rs:38
-- pg_search::api::operator::eqeqeq::term_search_query_input
CREATE  FUNCTION "term_search_query_input"(
	"field" FieldName, /* FieldName */
	"query" pdb.Query /* pdb :: Query */
) RETURNS SearchQueryInput /* SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_search_query_input_wrapper';
>>>>>>> 7fd3046b2 (chore: add term_search_query_input to 0.25.0 to 0.25.1 migration)
