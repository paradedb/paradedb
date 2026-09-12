CREATE FUNCTION "ctid_is_valid"(
    "ctid" tid
) RETURNS bool
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'ctid_is_valid_wrapper';

DROP FUNCTION IF EXISTS xmin_is_visible(xid);

CREATE FUNCTION "xmin_is_visible"(
    "xmin" xid,
    "tableoid" oid,
    "ctid" tid
) RETURNS bool
STRICT STABLE PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'xmin_is_visible_wrapper';

CREATE FUNCTION "search_with_query_input_ctid_or_row_strict"(
    "element" anyelement,
    "query" SearchQueryInput,
    "ctid" tid,
    "fallback_row" record[],
    "original_lhs" record DEFAULT ROW()
) RETURNS bool
IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
LANGUAGE c
AS 'MODULE_PATHNAME', 'search_with_query_input_ctid_or_row_strict_wrapper';

CREATE FUNCTION "search_with_query_input_ctid_or_row"(
    "element" anyelement,
    "query" SearchQueryInput,
    "ctid" tid,
    "fallback_row" record[],
    "original_lhs" record DEFAULT ROW()
) RETURNS bool
IMMUTABLE PARALLEL SAFE COST 1000000000
LANGUAGE c
AS 'MODULE_PATHNAME', 'search_with_query_input_ctid_or_row_wrapper';

ALTER FUNCTION paradedb.search_with_query_input_ctid_or_row SUPPORT paradedb.query_input_support;
ALTER FUNCTION paradedb.search_with_query_input_ctid_or_row_strict SUPPORT paradedb.query_input_support;
