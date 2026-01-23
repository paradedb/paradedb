GRANT ALL ON SCHEMA pdb TO PUBLIC;

-- Migration for PR #3911: Support text[], pdb.* parameters in operators with generic plans
-- This adds new query conversion functions for pdb.query, pdb.boost, pdb.fuzzy, pdb.slop types

-- ============================================================================
-- match_conjunction functions (for &&& operator exec_rewrite)
-- ============================================================================

CREATE FUNCTION "match_conjunction"(
    "field" FieldName,
    "query" pdb.Query
) RETURNS SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'match_conjunction_query_wrapper';

CREATE FUNCTION "match_conjunction"(
    "field" FieldName,
    "query" pdb.boost
) RETURNS SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'match_conjunction_boost_wrapper';

CREATE FUNCTION "match_conjunction"(
    "field" FieldName,
    "query" pdb.fuzzy
) RETURNS SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'match_conjunction_fuzzy_wrapper';

-- ============================================================================
-- match_disjunction functions (for ||| operator exec_rewrite)
-- ============================================================================

CREATE FUNCTION "match_disjunction"(
    "field" FieldName,
    "query" pdb.Query
) RETURNS SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'match_disjunction_query_wrapper';

CREATE FUNCTION "match_disjunction"(
    "field" FieldName,
    "query" pdb.boost
) RETURNS SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'match_disjunction_boost_wrapper';

CREATE FUNCTION "match_disjunction"(
    "field" FieldName,
    "query" pdb.fuzzy
) RETURNS SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'match_disjunction_fuzzy_wrapper';

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

CREATE FUNCTION "phrase"(
    "field" FieldName,
    "query" pdb.boost
) RETURNS SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'phrase_boost_wrapper';

CREATE FUNCTION "phrase"(
    "field" FieldName,
    "query" pdb.slop
) RETURNS SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'phrase_slop_wrapper';

-- ============================================================================
-- term functions (for === operator exec_rewrite)
-- ============================================================================

CREATE FUNCTION "term"(
    "field" FieldName,
    "term" pdb.Query
) RETURNS SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'term_query_wrapper';

CREATE FUNCTION "term"(
    "field" FieldName,
    "term" pdb.boost
) RETURNS SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'term_boost_wrapper';

CREATE FUNCTION "term"(
    "field" FieldName,
    "term" pdb.fuzzy
) RETURNS SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'term_fuzzy_wrapper';

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
