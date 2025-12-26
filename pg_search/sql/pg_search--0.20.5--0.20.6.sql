\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.20.6'" to load this file. \quit

-- Override the output function for pdb.alias
CREATE OR REPLACE FUNCTION pdb.alias_out(pdb.alias) RETURNS cstring
AS 'MODULE_PATHNAME', 'alias_out_safe_wrapper'
LANGUAGE c IMMUTABLE STRICT PARALLEL SAFE;
