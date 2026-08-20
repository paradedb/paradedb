\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.25.4'" to load this file. \quit

-- Use the public "columnar" terminology for index component sizes. Dropping the
-- function cascades to the two compatibility views, which are recreated below.
DROP FUNCTION IF EXISTS paradedb.index_info(regclass, bool) CASCADE;
CREATE FUNCTION paradedb.index_info(
    "index" regclass,
    "show_invisible" bool DEFAULT false
) RETURNS TABLE (
    "index_name" TEXT,
    "visible" bool,
    "recyclable" bool,
    "xmax" xid,
    "segno" TEXT,
    "mutable" bool,
    "byte_size" NUMERIC,
    "num_docs" NUMERIC,
    "num_deleted" NUMERIC,
    "termdict_bytes" NUMERIC,
    "postings_bytes" NUMERIC,
    "positions_bytes" NUMERIC,
    "columnar_bytes" NUMERIC,
    "fieldnorms_bytes" NUMERIC,
    "store_bytes" NUMERIC,
    "deletes_bytes" NUMERIC
)
STRICT
LANGUAGE c
AS 'MODULE_PATHNAME', 'index_info_wrapper';

CREATE VIEW pdb.index_layer_info AS SELECT ((relname)::text), layer_size, low, high, byte_size, CASE WHEN (segments = ARRAY[NULL]) THEN 0 ELSE count END AS count, CASE WHEN (segments = ARRAY[NULL]) THEN NULL ELSE segments END AS segments FROM (SELECT relname, ((COALESCE (pg_size_pretty(CASE WHEN (low = 0) THEN NULL ELSE low END), '') || '..') || COALESCE (pg_size_pretty(CASE WHEN (high = 9223372036854775807) THEN NULL ELSE high END), '')) AS layer_size, count(*), COALESCE (sum(byte_size), 0) AS byte_size, min(low) AS low, max(high) AS high, array_agg(segno) AS segments FROM (WITH indexes AS (SELECT ((c.oid)::regclass) AS relname FROM pg_class AS c INNER JOIN pg_index AS i ON (i.indexrelid = c.oid) WHERE (c.relam IN (SELECT oid FROM pg_am WHERE (amhandler = (('paradedb.bm25_handler')::regproc))) AND i.indisvalid AND i.indisready AND i.indislive)) , segments AS (SELECT relname, index_info.* FROM indexes INNER JOIN paradedb.index_info(indexes.relname, (('t')::pg_catalog.bool)) ON (('t')::pg_catalog.bool)) , layer_sizes AS (SELECT relname, COALESCE (lead(unnest) OVER(), 0) AS low, unnest AS high FROM indexes INNER JOIN LATERAL (SELECT unnest(((0 || paradedb.combined_layer_sizes(indexes.relname)) || 9223372036854775807)) ORDER BY 1 DESC ) AS x ON (('t')::pg_catalog.bool)) SELECT layer_sizes.relname, layer_sizes.low, layer_sizes.high, segments.segno, segments.byte_size FROM layer_sizes LEFT JOIN segments ON ((layer_sizes.relname = segments.relname) AND ((((byte_size * 1.33))::pg_catalog.int8) BETWEEN low AND high))) AS x WHERE (low < high) GROUP BY relname, low, high ORDER BY relname , low DESC ) AS x;
GRANT SELECT ON pdb.index_layer_info TO PUBLIC;

CREATE VIEW paradedb.index_layer_info AS SELECT ((relname)::text), layer_size, low, high, byte_size, CASE WHEN (segments = ARRAY[NULL]) THEN 0 ELSE count END AS count, CASE WHEN (segments = ARRAY[NULL]) THEN NULL ELSE segments END AS segments FROM (SELECT relname, ((COALESCE (pg_size_pretty(CASE WHEN (low = 0) THEN NULL ELSE low END), '') || '..') || COALESCE (pg_size_pretty(CASE WHEN (high = 9223372036854775807) THEN NULL ELSE high END), '')) AS layer_size, count(*), COALESCE (sum(byte_size), 0) AS byte_size, min(low) AS low, max(high) AS high, array_agg(segno) AS segments FROM (WITH indexes AS (SELECT ((c.oid)::regclass) AS relname FROM pg_class AS c INNER JOIN pg_index AS i ON (i.indexrelid = c.oid) WHERE (c.relam IN (SELECT oid FROM pg_am WHERE (amhandler = (('paradedb.bm25_handler')::regproc))) AND i.indisvalid AND i.indisready AND i.indislive)) , segments AS (SELECT relname, index_info.* FROM indexes INNER JOIN paradedb.index_info(indexes.relname, (('t')::pg_catalog.bool)) ON (('t')::pg_catalog.bool)) , layer_sizes AS (SELECT relname, COALESCE (lead(unnest) OVER(), 0) AS low, unnest AS high FROM indexes INNER JOIN LATERAL (SELECT unnest(((0 || paradedb.layer_sizes(indexes.relname)) || 9223372036854775807)) ORDER BY 1 DESC ) AS x ON (('t')::pg_catalog.bool)) SELECT layer_sizes.relname, layer_sizes.low, layer_sizes.high, segments.segno, segments.byte_size FROM layer_sizes LEFT JOIN segments ON ((layer_sizes.relname = segments.relname) AND ((((byte_size * 1.33))::pg_catalog.int8) BETWEEN low AND high))) AS x WHERE (low < high) GROUP BY relname, low, high ORDER BY relname , low DESC ) AS x;
GRANT SELECT ON paradedb.index_layer_info TO PUBLIC;
