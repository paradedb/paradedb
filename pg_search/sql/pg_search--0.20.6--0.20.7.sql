\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.20.7'" to load this file. \quit

DROP FUNCTION IF EXISTS snippet_positions(field anyelement, "limit" pg_catalog.int4, "offset" pg_catalog.int4);
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/postgres/customscan/pdbscan/projections/snippet.rs:341
-- pg_search::postgres::customscan::pdbscan::projections::snippet::paradedb_snippet_positions_from_relation

CREATE OR REPLACE FUNCTION "paradedb"."snippet_positions"(
    "field" anyelement,
    "limit" INT DEFAULT NULL,
    "offset" INT DEFAULT NULL
) RETURNS integer[]
STABLE PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'paradedb_snippet_positions_from_relation_wrapper';
DROP FUNCTION IF EXISTS pdb.snippet_positions(field anyelement, "limit" pg_catalog.int4, "offset" pg_catalog.int4);
CREATE OR REPLACE FUNCTION pdb.snippet_positions(field anyelement, "limit" pg_catalog.int4 DEFAULT NULL, "offset" pg_catalog.int4 DEFAULT NULL) RETURNS pg_catalog.int4[] AS 'MODULE_PATHNAME', 'snippet_positions_from_relation_wrapper' LANGUAGE c PARALLEL SAFE STABLE;
