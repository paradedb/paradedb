CREATE FUNCTION pdb."explain_score"(
    "relation_reference" anyelement
) RETURNS jsonb
STRICT STABLE PARALLEL SAFE COST 1
LANGUAGE c
AS 'MODULE_PATHNAME', 'explain_score_from_relation_wrapper';

ALTER FUNCTION pdb.explain_score SUPPORT paradedb.placeholder_support;
