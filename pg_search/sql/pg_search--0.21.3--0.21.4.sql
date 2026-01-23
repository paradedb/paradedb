-- Migration for PR #3911: Support text[], pdb.* parameters in operators with generic plans
-- This adds new functions and operators for pdb.boost, pdb.fuzzy, pdb.slop, and pdb.query types

-- ============================================================================
-- match_conjunction functions (for &&& operator)
-- ============================================================================

CREATE FUNCTION paradedb."match_conjunction"(
    "field" paradedb.FieldName,
    "query" pdb.Query
) RETURNS paradedb.SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'match_conjunction_query_wrapper';

CREATE FUNCTION paradedb."match_conjunction"(
    "field" paradedb.FieldName,
    "query" pdb.boost
) RETURNS paradedb.SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'match_conjunction_boost_wrapper';

CREATE FUNCTION paradedb."match_conjunction"(
    "field" paradedb.FieldName,
    "query" pdb.fuzzy
) RETURNS paradedb.SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'match_conjunction_fuzzy_wrapper';

-- Operator stubs for &&&
CREATE FUNCTION paradedb."search_with_match_conjunction_boost"(
    "_field" anyelement,
    "terms_to_tokenize" pdb.boost
) RETURNS bool
IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
LANGUAGE c
AS 'MODULE_PATHNAME', 'search_with_match_conjunction_boost_wrapper';

CREATE OPERATOR pg_catalog.&&& (
    PROCEDURE=paradedb.search_with_match_conjunction_boost,
    LEFTARG=anyelement,
    RIGHTARG=pdb.boost
);

CREATE FUNCTION paradedb."search_with_match_conjunction_fuzzy"(
    "_field" anyelement,
    "terms_to_tokenize" pdb.fuzzy
) RETURNS bool
IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
LANGUAGE c
AS 'MODULE_PATHNAME', 'search_with_match_conjunction_fuzzy_wrapper';

CREATE OPERATOR pg_catalog.&&& (
    PROCEDURE=paradedb.search_with_match_conjunction_fuzzy,
    LEFTARG=anyelement,
    RIGHTARG=pdb.fuzzy
);

ALTER FUNCTION paradedb.search_with_match_conjunction_boost SUPPORT paradedb.search_with_match_conjunction_support;
ALTER FUNCTION paradedb.search_with_match_conjunction_fuzzy SUPPORT paradedb.search_with_match_conjunction_support;

-- ============================================================================
-- match_disjunction functions (for ||| operator)
-- ============================================================================

CREATE FUNCTION paradedb."match_disjunction"(
    "field" paradedb.FieldName,
    "query" pdb.Query
) RETURNS paradedb.SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'match_disjunction_query_wrapper';

CREATE FUNCTION paradedb."match_disjunction"(
    "field" paradedb.FieldName,
    "query" pdb.boost
) RETURNS paradedb.SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'match_disjunction_boost_wrapper';

CREATE FUNCTION paradedb."match_disjunction"(
    "field" paradedb.FieldName,
    "query" pdb.fuzzy
) RETURNS paradedb.SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'match_disjunction_fuzzy_wrapper';

-- Operator stubs for |||
CREATE FUNCTION paradedb."search_with_match_disjunction_boost"(
    "_field" anyelement,
    "terms_to_tokenize" pdb.boost
) RETURNS bool
IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
LANGUAGE c
AS 'MODULE_PATHNAME', 'search_with_match_disjunction_boost_wrapper';

CREATE OPERATOR pg_catalog.||| (
    PROCEDURE=paradedb.search_with_match_disjunction_boost,
    LEFTARG=anyelement,
    RIGHTARG=pdb.boost
);

CREATE FUNCTION paradedb."search_with_match_disjunction_fuzzy"(
    "_field" anyelement,
    "terms_to_tokenize" pdb.fuzzy
) RETURNS bool
IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
LANGUAGE c
AS 'MODULE_PATHNAME', 'search_with_match_disjunction_fuzzy_wrapper';

CREATE OPERATOR pg_catalog.||| (
    PROCEDURE=paradedb.search_with_match_disjunction_fuzzy,
    LEFTARG=anyelement,
    RIGHTARG=pdb.fuzzy
);

ALTER FUNCTION paradedb.search_with_match_disjunction_boost SUPPORT paradedb.search_with_match_disjunction_support;
ALTER FUNCTION paradedb.search_with_match_disjunction_fuzzy SUPPORT paradedb.search_with_match_disjunction_support;

-- ============================================================================
-- phrase functions (for ### operator)
-- ============================================================================

CREATE FUNCTION paradedb."phrase"(
    "field" paradedb.FieldName,
    "query" pdb.Query
) RETURNS paradedb.SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'phrase_query_wrapper';

CREATE FUNCTION paradedb."phrase"(
    "field" paradedb.FieldName,
    "query" pdb.boost
) RETURNS paradedb.SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'phrase_boost_wrapper';

CREATE FUNCTION paradedb."phrase"(
    "field" paradedb.FieldName,
    "query" pdb.slop
) RETURNS paradedb.SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'phrase_slop_wrapper';

-- Operator stubs for ###
CREATE FUNCTION paradedb."search_with_phrase_boost"(
    "_field" anyelement,
    "terms_to_tokenize" pdb.boost
) RETURNS bool
IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
LANGUAGE c
AS 'MODULE_PATHNAME', 'search_with_phrase_boost_wrapper';

CREATE OPERATOR pg_catalog.### (
    PROCEDURE=paradedb.search_with_phrase_boost,
    LEFTARG=anyelement,
    RIGHTARG=pdb.boost
);

CREATE FUNCTION paradedb."search_with_phrase_slop"(
    "_field" anyelement,
    "terms_to_tokenize" pdb.slop
) RETURNS bool
IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
LANGUAGE c
AS 'MODULE_PATHNAME', 'search_with_phrase_slop_wrapper';

CREATE OPERATOR pg_catalog.### (
    PROCEDURE=paradedb.search_with_phrase_slop,
    LEFTARG=anyelement,
    RIGHTARG=pdb.slop
);

ALTER FUNCTION paradedb.search_with_phrase_boost SUPPORT paradedb.search_with_phrase_support;
ALTER FUNCTION paradedb.search_with_phrase_slop SUPPORT paradedb.search_with_phrase_support;

-- ============================================================================
-- term functions (for === operator)
-- ============================================================================

CREATE FUNCTION paradedb."term"(
    "field" paradedb.FieldName,
    "term" pdb.Query
) RETURNS paradedb.SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'term_query_wrapper';

CREATE FUNCTION paradedb."term"(
    "field" paradedb.FieldName,
    "term" pdb.boost
) RETURNS paradedb.SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'term_boost_wrapper';

CREATE FUNCTION paradedb."term"(
    "field" paradedb.FieldName,
    "term" pdb.fuzzy
) RETURNS paradedb.SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'term_fuzzy_wrapper';

-- Operator stubs for ===
CREATE FUNCTION paradedb."search_with_term_boost"(
    "_field" anyelement,
    "term" pdb.boost
) RETURNS bool
IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
LANGUAGE c
AS 'MODULE_PATHNAME', 'search_with_term_boost_wrapper';

CREATE OPERATOR pg_catalog.=== (
    PROCEDURE=paradedb.search_with_term_boost,
    LEFTARG=anyelement,
    RIGHTARG=pdb.boost
);

CREATE FUNCTION paradedb."search_with_term_fuzzy"(
    "_field" anyelement,
    "term" pdb.fuzzy
) RETURNS bool
IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
LANGUAGE c
AS 'MODULE_PATHNAME', 'search_with_term_fuzzy_wrapper';

CREATE OPERATOR pg_catalog.=== (
    PROCEDURE=paradedb.search_with_term_fuzzy,
    LEFTARG=anyelement,
    RIGHTARG=pdb.fuzzy
);

ALTER FUNCTION paradedb.search_with_term_boost SUPPORT paradedb.search_with_term_support;
ALTER FUNCTION paradedb.search_with_term_fuzzy SUPPORT paradedb.search_with_term_support;

-- ============================================================================
-- parse_with_field_query function (for @@@ operator)
-- ============================================================================

CREATE FUNCTION paradedb."parse_with_field_query"(
    "field" paradedb.FieldName,
    "query" pdb.Query
) RETURNS paradedb.SearchQueryInput
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'parse_with_field_query_wrapper';
