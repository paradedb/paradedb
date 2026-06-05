\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.24.1'" to load this file. \quit

-- `boost_to_fuzzy` and its cast are also defined in `fuzzy.rs`, so they live in
-- the base install schema. Any 0.24.0 install built from a tree that already
-- includes that change ships them, while installs from the v0.24.0 release tag
-- do not. Drop-then-create keeps this upgrade idempotent across both cases so it
-- never fails with "function already exists".
DROP CAST IF EXISTS (pdb.boost AS pdb.fuzzy);
DROP FUNCTION IF EXISTS "boost_to_fuzzy"(pdb.boost, integer, boolean);

CREATE FUNCTION "boost_to_fuzzy"(
	"input" pdb.boost,
	"typmod" INT,
	"is_explicit" bool
) RETURNS pdb.fuzzy
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'boost_to_fuzzy_wrapper';
CREATE CAST (pdb.boost AS pdb.fuzzy) WITH FUNCTION boost_to_fuzzy(pdb.boost, integer, boolean) AS ASSIGNMENT;
