-- pg_search 0.25.0 -> 0.25.0.1 (dev): hybrid search via built-in
-- Reciprocal Rank Fusion.
--
--   * pdb.score(relation, "type") — typed score projection
--     ('bm25' | 'vector' | 'hybrid' | 'rank')
--   * pdb.rrf(bm25_score, vector_distance, k, window_size) — rank-fusion
--     ORDER BY expression
--   * ~~~ (vector, vector) — knn candidacy predicate for union-style hybrid
--
-- The DROPs are defensive: dev databases may have created these functions
-- manually before this upgrade path existed.

DROP FUNCTION IF EXISTS pdb.score(anyelement, text);
CREATE FUNCTION pdb."score"(
    "relation_reference" anyelement,
    "type" text
) RETURNS real
STRICT STABLE PARALLEL SAFE COST 1
LANGUAGE c
AS 'MODULE_PATHNAME', 'score_from_relation_typed_wrapper';
ALTER FUNCTION pdb.score(anyelement, text) SUPPORT paradedb.placeholder_support;

DROP FUNCTION IF EXISTS pdb.rrf(real, double precision, integer);
DROP FUNCTION IF EXISTS pdb.rrf(double precision, double precision, integer);
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

-- the fully-implied relation form: both legs come from the WHERE clause
-- (text query + ~~~ knn predicate) of the relation the reference points at
DROP FUNCTION IF EXISTS pdb.rrf(anyelement, integer, integer);
CREATE FUNCTION pdb."rrf"(
    "relation_reference" anyelement,
    "k" integer DEFAULT 60,
    "window_size" integer DEFAULT 0
) RETURNS double precision
STRICT STABLE PARALLEL SAFE COST 1
LANGUAGE c
AS 'MODULE_PATHNAME', 'rrf_from_relation_wrapper';

DROP OPERATOR IF EXISTS pg_catalog.~~~ (anyelement, anyelement);
DROP FUNCTION IF EXISTS paradedb.search_with_knn(anyelement, anyelement);
DROP FUNCTION IF EXISTS paradedb.search_with_knn_support(internal);

CREATE FUNCTION paradedb."search_with_knn"(
    "_field" anyelement,
    "_vector" anyelement
) RETURNS bool
IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
LANGUAGE c
AS 'MODULE_PATHNAME', 'search_with_knn_wrapper';

CREATE OPERATOR pg_catalog.~~~ (
    PROCEDURE = paradedb."search_with_knn",
    LEFTARG = anyelement,
    RIGHTARG = anyelement
);

CREATE FUNCTION paradedb."search_with_knn_support"(
    "arg" internal
) RETURNS internal
IMMUTABLE PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'search_with_knn_support_wrapper';

ALTER FUNCTION paradedb.search_with_knn SUPPORT paradedb.search_with_knn_support;
