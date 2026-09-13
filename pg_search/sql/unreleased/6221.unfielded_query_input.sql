CREATE FUNCTION "to_search_query_input"(
    "query" pdb.Query
) RETURNS SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'to_search_query_input_unfielded_wrapper';
