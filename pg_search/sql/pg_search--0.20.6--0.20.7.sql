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

DROP FUNCTION IF EXISTS tokenizer(name text, remove_long pg_catalog.int4, lowercase bool, min_gram pg_catalog.int4, max_gram pg_catalog.int4, prefix_only bool, language text, pattern text, stemmer text, stopwords_language text, stopwords text[], ascii_folding bool);
CREATE OR REPLACE FUNCTION tokenizer(name text, remove_long pg_catalog.int4 DEFAULT '255', lowercase bool DEFAULT 'true', min_gram pg_catalog.int4 DEFAULT NULL, max_gram pg_catalog.int4 DEFAULT NULL, prefix_only bool DEFAULT NULL, language text DEFAULT NULL, pattern text DEFAULT NULL, stemmer text DEFAULT NULL, stopwords_language text DEFAULT NULL, stopwords_languages text[] DEFAULT NULL, stopwords text[] DEFAULT NULL, ascii_folding bool DEFAULT NULL) RETURNS jsonb AS 'MODULE_PATHNAME', 'tokenizer_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;

