-- pg_search 0.25.0.4 -> 0.25.0.5 (dev): measure-named fusion-arm
-- annotations.
--
-- The measure-less pdb.top(n) is replaced by pdb.top_bm25(n) and
-- pdb.top_knn(n): the type name declares what ranks the arm, so every other
-- predicate inside the arm is a filter:
--
--   WHERE (category === 'shoes' AND description ||| 'running shoes')::pdb.top_bm25(100)
--      OR (category === 'shoes' AND embedding ~~~ '[...]')::pdb.top_knn(200)
--   ORDER BY pdb.rrf(id) LIMIT 10;

-- Drop the measure-less pdb.top; CASCADE takes its casts and the functions
-- whose signatures mention the type (top_in/top_out and the cast functions).
-- On chains that skipped 0.25.0.4's creation (its symbols are gone) this is
-- a no-op, which is why the typmod functions below are CREATE OR REPLACE.
DROP TYPE IF EXISTS pdb.top CASCADE;

CREATE TYPE pdb.top_bm25;
CREATE TYPE pdb.top_knn;

CREATE OR REPLACE FUNCTION top_typmod_in(cstring[]) RETURNS integer
IMMUTABLE STRICT PARALLEL SAFE LANGUAGE c AS 'MODULE_PATHNAME', 'top_typmod_in_wrapper';
CREATE OR REPLACE FUNCTION top_typmod_out(integer) RETURNS cstring
IMMUTABLE STRICT PARALLEL SAFE LANGUAGE c AS 'MODULE_PATHNAME', 'top_typmod_out_wrapper';

CREATE FUNCTION top_bm25_in(cstring, oid, integer) RETURNS pdb.top_bm25
IMMUTABLE STRICT PARALLEL SAFE LANGUAGE c AS 'MODULE_PATHNAME', 'top_bm25_in_wrapper';
CREATE FUNCTION top_bm25_out(pdb.top_bm25) RETURNS cstring
IMMUTABLE STRICT PARALLEL SAFE LANGUAGE c AS 'MODULE_PATHNAME', 'top_bm25_out_wrapper';
CREATE FUNCTION top_knn_in(cstring, oid, integer) RETURNS pdb.top_knn
IMMUTABLE STRICT PARALLEL SAFE LANGUAGE c AS 'MODULE_PATHNAME', 'top_knn_in_wrapper';
CREATE FUNCTION top_knn_out(pdb.top_knn) RETURNS cstring
IMMUTABLE STRICT PARALLEL SAFE LANGUAGE c AS 'MODULE_PATHNAME', 'top_knn_out_wrapper';

CREATE TYPE pdb.top_bm25 (
    INPUT = top_bm25_in,
    OUTPUT = top_bm25_out,
    LIKE = bool,
    TYPMOD_IN = top_typmod_in,
    TYPMOD_OUT = top_typmod_out
);
CREATE TYPE pdb.top_knn (
    INPUT = top_knn_in,
    OUTPUT = top_knn_out,
    LIKE = bool,
    TYPMOD_IN = top_typmod_in,
    TYPMOD_OUT = top_typmod_out
);

CREATE FUNCTION bool_to_top_bm25(boolean, integer, boolean) RETURNS pdb.top_bm25
IMMUTABLE STRICT PARALLEL SAFE LANGUAGE c AS 'MODULE_PATHNAME', 'bool_to_top_bm25_wrapper';
CREATE FUNCTION top_bm25_to_top_bm25(pdb.top_bm25, integer, boolean) RETURNS pdb.top_bm25
IMMUTABLE STRICT PARALLEL SAFE LANGUAGE c AS 'MODULE_PATHNAME', 'top_bm25_to_top_bm25_wrapper';
CREATE FUNCTION top_bm25_to_bool(pdb.top_bm25) RETURNS boolean
IMMUTABLE STRICT PARALLEL SAFE LANGUAGE c AS 'MODULE_PATHNAME', 'top_bm25_to_bool_wrapper';
CREATE FUNCTION bool_to_top_knn(boolean, integer, boolean) RETURNS pdb.top_knn
IMMUTABLE STRICT PARALLEL SAFE LANGUAGE c AS 'MODULE_PATHNAME', 'bool_to_top_knn_wrapper';
CREATE FUNCTION top_knn_to_top_knn(pdb.top_knn, integer, boolean) RETURNS pdb.top_knn
IMMUTABLE STRICT PARALLEL SAFE LANGUAGE c AS 'MODULE_PATHNAME', 'top_knn_to_top_knn_wrapper';
CREATE FUNCTION top_knn_to_bool(pdb.top_knn) RETURNS boolean
IMMUTABLE STRICT PARALLEL SAFE LANGUAGE c AS 'MODULE_PATHNAME', 'top_knn_to_bool_wrapper';

CREATE CAST (boolean AS pdb.top_bm25) WITH FUNCTION bool_to_top_bm25(boolean, integer, boolean) AS ASSIGNMENT;
CREATE CAST (pdb.top_bm25 AS pdb.top_bm25) WITH FUNCTION top_bm25_to_top_bm25(pdb.top_bm25, integer, boolean) AS IMPLICIT;
CREATE CAST (pdb.top_bm25 AS boolean) WITH FUNCTION top_bm25_to_bool(pdb.top_bm25) AS IMPLICIT;
CREATE CAST (boolean AS pdb.top_knn) WITH FUNCTION bool_to_top_knn(boolean, integer, boolean) AS ASSIGNMENT;
CREATE CAST (pdb.top_knn AS pdb.top_knn) WITH FUNCTION top_knn_to_top_knn(pdb.top_knn, integer, boolean) AS IMPLICIT;
CREATE CAST (pdb.top_knn AS boolean) WITH FUNCTION top_knn_to_bool(pdb.top_knn) AS IMPLICIT;
