\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.24.1'" to load this file. \quit

CREATE FUNCTION "boost_to_fuzzy"(
	"input" pdb.boost,
	"typmod" INT,
	"is_explicit" bool
) RETURNS pdb.fuzzy
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'boost_to_fuzzy_wrapper';
CREATE CAST (pdb.boost AS pdb.fuzzy) WITH FUNCTION boost_to_fuzzy(pdb.boost, integer, boolean) AS ASSIGNMENT;
