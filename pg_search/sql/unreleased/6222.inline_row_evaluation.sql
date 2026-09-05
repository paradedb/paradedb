CREATE FUNCTION "ctid_is_valid"(
    "ctid" tid
) RETURNS bool
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'ctid_is_valid_wrapper';

CREATE FUNCTION "search_with_query_input_ctid_or_row_strict"(
    "element" anyelement,
    "query" SearchQueryInput,
    "ctid" tid,
    "fallback_row" record[]
) RETURNS bool
IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
LANGUAGE c
AS 'MODULE_PATHNAME', 'search_with_query_input_ctid_or_row_strict_wrapper';

CREATE FUNCTION "search_with_query_input_ctid_or_row"(
    "element" anyelement,
    "query" SearchQueryInput,
    "ctid" tid,
    "fallback_row" record[]
) RETURNS bool
IMMUTABLE PARALLEL SAFE COST 1000000000
LANGUAGE c
AS 'MODULE_PATHNAME', 'search_with_query_input_ctid_or_row_wrapper';

ALTER FUNCTION paradedb.search_with_query_input_ctid_or_row SUPPORT paradedb.query_input_support;
ALTER FUNCTION paradedb.search_with_query_input_ctid_or_row_strict SUPPORT paradedb.query_input_support;
