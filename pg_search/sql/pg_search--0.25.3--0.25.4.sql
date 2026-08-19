\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.25.4'" to load this file. \quit

-- 0.25.3 -> 0.25.4: register the runtime *_search_query_input pg_externs for the
-- &&&, |||, and ### operators. These mirror paradedb.term_search_query_input
-- (added in 0.25.1 -> 0.25.2 for ===) and are called from each operator's
-- exec_rewrite when the RHS is a Param under a generic prepared plan. Fixes
-- issue #5779 for the three sibling operators.

-- pg_search/src/api/operator/andandand.rs
-- pg_search::api::operator::andandand::match_conjunction_search_query_input
CREATE  FUNCTION "match_conjunction_search_query_input"(
	"field" FieldName, /* FieldName */
	"query" pdb.Query /* pdb :: Query */
) RETURNS SearchQueryInput /* SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'match_conjunction_search_query_input_wrapper';

-- pg_search/src/api/operator/ororor.rs
-- pg_search::api::operator::ororor::match_disjunction_search_query_input
CREATE  FUNCTION "match_disjunction_search_query_input"(
	"field" FieldName, /* FieldName */
	"query" pdb.Query /* pdb :: Query */
) RETURNS SearchQueryInput /* SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'match_disjunction_search_query_input_wrapper';

-- pg_search/src/api/operator/hashhashhash.rs
-- pg_search::api::operator::hashhashhash::phrase_search_query_input
CREATE  FUNCTION "phrase_search_query_input"(
	"field" FieldName, /* FieldName */
	"query" pdb.Query /* pdb :: Query */
) RETURNS SearchQueryInput /* SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'phrase_search_query_input_wrapper';
