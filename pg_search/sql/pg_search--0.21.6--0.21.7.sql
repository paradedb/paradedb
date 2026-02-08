
DROP FUNCTION IF EXISTS score(_relation_reference anyelement);
CREATE OR REPLACE FUNCTION score(relation_reference anyelement) RETURNS pg_catalog.float4 AS 'MODULE_PATHNAME', 'paradedb_score_from_relation_wrapper' COST 1 LANGUAGE c PARALLEL SAFE STABLE STRICT;
DROP FUNCTION IF EXISTS pdb.score(_relation_reference anyelement);
CREATE OR REPLACE FUNCTION pdb.score(relation_reference anyelement) RETURNS pg_catalog.float4 AS 'MODULE_PATHNAME', 'score_from_relation_wrapper' COST 1 LANGUAGE c PARALLEL SAFE STABLE STRICT;
