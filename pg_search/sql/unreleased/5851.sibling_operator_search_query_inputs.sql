-- Runtime *_search_query_input functions for the &&&, ||| and ### operators.
-- Each mirrors paradedb.term_search_query_input and is called from the
-- operator's exec rewrite when the RHS is not foldable at plan time.

CREATE FUNCTION "match_conjunction_search_query_input"(
    "field" FieldName,
    "query" pdb.Query
) RETURNS SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'match_conjunction_search_query_input_wrapper';

CREATE FUNCTION "match_disjunction_search_query_input"(
    "field" FieldName,
    "query" pdb.Query
) RETURNS SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'match_disjunction_search_query_input_wrapper';

CREATE FUNCTION "phrase_search_query_input"(
    "field" FieldName,
    "query" pdb.Query
) RETURNS SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'phrase_search_query_input_wrapper';
