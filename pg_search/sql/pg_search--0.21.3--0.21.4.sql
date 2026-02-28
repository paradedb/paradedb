GRANT ALL ON SCHEMA pdb TO PUBLIC;

-- Migration for PR #3911: Support text[], pdb.* parameters in operators with generic plans
-- This adds new query conversion functions for pdb.query and pdb.slop types

-- ============================================================================
-- match_conjunction function (for &&& operator exec_rewrite)
-- ============================================================================

CREATE FUNCTION "match_conjunction"(
    "field" FieldName,
    "query" pdb.Query
) RETURNS SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'match_conjunction_query_wrapper';

-- ============================================================================
-- match_disjunction function (for ||| operator exec_rewrite)
-- ============================================================================

CREATE FUNCTION "match_disjunction"(
    "field" FieldName,
    "query" pdb.Query
) RETURNS SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'match_disjunction_query_wrapper';

-- ============================================================================
-- phrase functions (for ### operator exec_rewrite)
-- ============================================================================

CREATE FUNCTION "phrase"(
    "field" FieldName,
    "query" pdb.Query
) RETURNS SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'phrase_query_wrapper';

-- pdb.slop is semantically meaningful for phrase queries
CREATE FUNCTION "phrase"(
    "field" FieldName,
    "query" pdb.slop
) RETURNS SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'phrase_slop_wrapper';

-- ============================================================================
-- term function (for === operator exec_rewrite)
-- ============================================================================

CREATE FUNCTION "term"(
    "field" FieldName,
    "term" pdb.Query
) RETURNS SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'term_query_wrapper';

-- ============================================================================
-- parse_with_field_query function (for @@@ operator exec_rewrite)
-- ============================================================================

CREATE FUNCTION "parse_with_field_query"(
    "field" FieldName,
    "query" pdb.Query
) RETURNS SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'parse_with_field_query_wrapper';
