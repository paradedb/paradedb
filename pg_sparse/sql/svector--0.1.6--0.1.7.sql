-- complain if script is sourced in psql, rather than via CREATE EXTENSION
\echo Use "ALTER EXTENSION svector UPDATE TO '0.1.7'" to load this file. \quit

CREATE FUNCTION array_to_svector(numeric[], integer, boolean) RETURNS svector
	AS 'MODULE_PATHNAME' LANGUAGE C IMMUTABLE STRICT PARALLEL SAFE;

CREATE CAST (numeric[] AS svector)
	WITH FUNCTION array_to_svector(numeric[], integer, boolean) AS IMPLICIT;
