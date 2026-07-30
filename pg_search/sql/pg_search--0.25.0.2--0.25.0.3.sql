-- pg_search 0.25.0.2 -> 0.25.0.3 (dev): per-arm rrf candidate windows.
--
-- Both pdb.rrf overloads gain bm25_window_size / vector_window_size named
-- arguments (0 = inherit window_size = auto).

DROP FUNCTION IF EXISTS pdb.rrf(double precision, double precision, integer, integer);
CREATE FUNCTION pdb."rrf"(
    "bm25_score" double precision,
    "vector_distance" double precision,
    "k" integer DEFAULT 60,
    "window_size" integer DEFAULT 0,
    "bm25_window_size" integer DEFAULT 0,
    "vector_window_size" integer DEFAULT 0
) RETURNS double precision
STABLE PARALLEL SAFE COST 1
LANGUAGE c
AS 'MODULE_PATHNAME', 'rrf_placeholder_wrapper';

DROP FUNCTION IF EXISTS pdb.rrf(anyelement, integer, integer);
CREATE FUNCTION pdb."rrf"(
    "relation_reference" anyelement,
    "k" integer DEFAULT 60,
    "window_size" integer DEFAULT 0,
    "bm25_window_size" integer DEFAULT 0,
    "vector_window_size" integer DEFAULT 0
) RETURNS double precision
STRICT STABLE PARALLEL SAFE COST 1
LANGUAGE c
AS 'MODULE_PATHNAME', 'rrf_from_relation_wrapper';
