-- pg_search/src/api/operator/eqeqeq.rs:38
-- pg_search::api::operator::eqeqeq::term_search_query_input
CREATE  FUNCTION "term_search_query_input"(
	"field" FieldName, /* FieldName */
	"query" pdb.Query /* pdb :: Query */
) RETURNS SearchQueryInput /* SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_search_query_input_wrapper';

-- pg_search/src/api/operator/ororor.rs:38
-- pg_search::api::operator::ororor::match_disjunction_search_query_input
CREATE  FUNCTION "match_disjunction_search_query_input"(
	"field" FieldName, /* FieldName */
	"query" pdb.Query /* pdb :: Query */
) RETURNS SearchQueryInput /* SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'match_disjunction_search_query_input_wrapper';

-- pg_search/src/api/operator/andandand.rs:38
-- pg_search::api::operator::andandand::match_conjunction_search_query_input
CREATE  FUNCTION "match_conjunction_search_query_input"(
	"field" FieldName, /* FieldName */
	"query" pdb.Query /* pdb :: Query */
) RETURNS SearchQueryInput /* SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'match_conjunction_search_query_input_wrapper';

-- pg_search/src/api/operator/hashhashhash.rs:38
-- pg_search::api::operator::hashhashhash::phrase_search_query_input
CREATE  FUNCTION "phrase_search_query_input"(
	"field" FieldName, /* FieldName */
	"query" pdb.Query /* pdb :: Query */
) RETURNS SearchQueryInput /* SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'phrase_search_query_input_wrapper';
