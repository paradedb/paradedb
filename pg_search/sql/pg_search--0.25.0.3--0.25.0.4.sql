-- pg_search 0.25.0.3 -> 0.25.0.4 (dev): ::pdb.top(n) fusion-arm annotations.
--
-- Casting a boolean search-predicate subtree to pdb.top(n) marks it as a
-- rank-fusion arm bounded to its top n candidates:
--
--   WHERE (description ||| 'shoes' OR category === 'footwear')::pdb.top(100)
--      OR (embedding ~~~ '[...]')::pdb.top(200)
--   ORDER BY pdb.rrf(id) LIMIT 10;

CREATE TYPE pdb.top;

CREATE FUNCTION top_in(cstring, oid, integer) RETURNS pdb.top
IMMUTABLE STRICT PARALLEL SAFE LANGUAGE c AS 'MODULE_PATHNAME', 'top_in_wrapper';
CREATE FUNCTION top_out(pdb.top) RETURNS cstring
IMMUTABLE STRICT PARALLEL SAFE LANGUAGE c AS 'MODULE_PATHNAME', 'top_out_wrapper';
CREATE FUNCTION top_typmod_in(cstring[]) RETURNS integer
IMMUTABLE STRICT PARALLEL SAFE LANGUAGE c AS 'MODULE_PATHNAME', 'top_typmod_in_wrapper';
CREATE FUNCTION top_typmod_out(integer) RETURNS cstring
IMMUTABLE STRICT PARALLEL SAFE LANGUAGE c AS 'MODULE_PATHNAME', 'top_typmod_out_wrapper';

CREATE TYPE pdb.top (
    INPUT = top_in,
    OUTPUT = top_out,
    LIKE = bool,
    TYPMOD_IN = top_typmod_in,
    TYPMOD_OUT = top_typmod_out
);

CREATE FUNCTION bool_to_top(boolean, integer, boolean) RETURNS pdb.top
IMMUTABLE STRICT PARALLEL SAFE LANGUAGE c AS 'MODULE_PATHNAME', 'bool_to_top_wrapper';
CREATE FUNCTION top_to_top(pdb.top, integer, boolean) RETURNS pdb.top
IMMUTABLE STRICT PARALLEL SAFE LANGUAGE c AS 'MODULE_PATHNAME', 'top_to_top_wrapper';
CREATE FUNCTION top_to_bool(pdb.top) RETURNS boolean
IMMUTABLE STRICT PARALLEL SAFE LANGUAGE c AS 'MODULE_PATHNAME', 'top_to_bool_wrapper';

CREATE CAST (boolean AS pdb.top) WITH FUNCTION bool_to_top(boolean, integer, boolean) AS ASSIGNMENT;
CREATE CAST (pdb.top AS pdb.top) WITH FUNCTION top_to_top(pdb.top, integer, boolean) AS IMPLICIT;
CREATE CAST (pdb.top AS boolean) WITH FUNCTION top_to_bool(pdb.top) AS IMPLICIT;
