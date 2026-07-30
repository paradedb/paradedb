-- pg_search 0.25.0.1 -> 0.25.0.2 (dev): pdb.rrf ergonomics.
--
--   * new fully-implied relation form `pdb.rrf(relation, k, window_size)`:
--     the BM25 leg is the relation's WHERE text query and the vector leg its
--     `~~~` knn predicate, so `ORDER BY pdb.rrf(id)` says everything once
--   * the explicit-legs form loses its DEFAULT NULL distance (it would make
--     one-argument calls ambiguous against the relation form)

DROP FUNCTION IF EXISTS pdb.rrf(double precision, double precision, integer, integer);
-- deliberately NOT STRICT so NULL legs still reach the placeholder body and
-- error loudly when the query is not executed by a rank-fusion scan.
CREATE FUNCTION pdb."rrf"(
    "bm25_score" double precision,
    "vector_distance" double precision,
    "k" integer DEFAULT 60,
    "window_size" integer DEFAULT 0
) RETURNS double precision
STABLE PARALLEL SAFE COST 1
LANGUAGE c
AS 'MODULE_PATHNAME', 'rrf_placeholder_wrapper';

DROP FUNCTION IF EXISTS pdb.rrf(anyelement, integer, integer);
CREATE FUNCTION pdb."rrf"(
    "relation_reference" anyelement,
    "k" integer DEFAULT 60,
    "window_size" integer DEFAULT 0
) RETURNS double precision
STRICT STABLE PARALLEL SAFE COST 1
LANGUAGE c
AS 'MODULE_PATHNAME', 'rrf_from_relation_wrapper';
