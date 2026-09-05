CREATE FUNCTION "search_with_query_input_ctid_strict"(
    "element" anyelement,
    "query" SearchQueryInput,
    "ctid" tid
) RETURNS bool
IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
LANGUAGE c
AS 'MODULE_PATHNAME', 'search_with_query_input_ctid_strict_wrapper';

CREATE FUNCTION "search_with_query_input_ctid"(
    "element" anyelement,
    "query" SearchQueryInput,
    "ctid" tid
) RETURNS bool
IMMUTABLE PARALLEL SAFE COST 1000000000
LANGUAGE c
AS 'MODULE_PATHNAME', 'search_with_query_input_ctid_wrapper';

ALTER FUNCTION paradedb.search_with_query_input_ctid SUPPORT paradedb.query_input_support;
ALTER FUNCTION paradedb.search_with_query_input_ctid_strict SUPPORT paradedb.query_input_support;
