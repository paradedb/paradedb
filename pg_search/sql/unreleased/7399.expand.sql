DROP FUNCTION IF EXISTS pdb.with_synonyms(pdb.query, regclass);
DROP FUNCTION IF EXISTS pdb.with_synonyms(text, regclass);

CREATE FUNCTION pdb."expand_agg_expand_agg_state"(
    "this" internal,
    "query" TEXT,
    "rule" TEXT
) RETURNS internal
LANGUAGE c
AS 'MODULE_PATHNAME', 'expand_agg_expand_agg_state_wrapper';

CREATE FUNCTION pdb."expand_agg_expand_agg_finalize"(
    "this" internal
) RETURNS pdb.Query
LANGUAGE c
AS 'MODULE_PATHNAME', 'expand_agg_expand_agg_finalize_wrapper';

CREATE AGGREGATE pdb.expand (
    "query" TEXT,
    "rule" TEXT
)
(
    SFUNC = pdb."expand_agg_expand_agg_state",
    STYPE = internal,
    FINALFUNC = pdb."expand_agg_expand_agg_finalize",
    PARALLEL = SAFE
);
