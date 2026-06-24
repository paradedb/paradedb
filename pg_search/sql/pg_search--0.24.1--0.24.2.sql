\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.24.2'" to load this file. \quit

-- Update boolean() overloads to add minimum_should_match parameter
DROP FUNCTION IF EXISTS "boolean"(must searchqueryinput, should searchqueryinput, must_not searchqueryinput);
CREATE OR REPLACE FUNCTION "boolean"(must searchqueryinput DEFAULT NULL, should searchqueryinput DEFAULT NULL, must_not searchqueryinput DEFAULT NULL, minimum_should_match pg_catalog.int8 DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'boolean_singles_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS "boolean"(must searchqueryinput[], should searchqueryinput[], must_not searchqueryinput[]);
CREATE OR REPLACE FUNCTION "boolean"(must searchqueryinput[] DEFAULT ARRAY[]::searchqueryinput[], should searchqueryinput[] DEFAULT ARRAY[]::searchqueryinput[], must_not searchqueryinput[] DEFAULT ARRAY[]::searchqueryinput[], minimum_should_match pg_catalog.int8 DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'boolean_arrays_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;

-- The `index_layer_info` view re-emissions below were originally added to
-- `pg_search--0.24.0--0.24.1.sql` on `main` *after* v0.24.1 had already shipped,
-- so anyone already running the released 0.24.1 would never have run them. Moved
-- onto the 0.24.1 -> 0.24.2 upgrade path so those upgraders converge on the same
-- schema as a fresh install. DROP-then-CREATE keeps it idempotent.
DROP VIEW pdb.index_layer_info;
CREATE VIEW pdb.index_layer_info AS
SELECT ((relname)::text),
       layer_size,
       low,
       high,
       byte_size,
       CASE WHEN (segments = ARRAY[NULL]) THEN 0 ELSE count END AS count,
       CASE WHEN (segments = ARRAY[NULL]) THEN NULL ELSE segments END AS segments
FROM (SELECT relname,
             ((COALESCE (pg_size_pretty(CASE WHEN (low = 0) THEN NULL ELSE low END), '') || '..') ||
              COALESCE (pg_size_pretty(CASE WHEN (high = 9223372036854775807) THEN NULL ELSE high END), '')) AS layer_size,
             count(*),
             COALESCE (sum(byte_size), 0) AS byte_size,
             min(low) AS low,
             max(high) AS high,
             array_agg(segno) AS segments
      FROM (WITH indexes AS (SELECT ((c.oid)::regclass) AS relname
                             FROM pg_class AS c
                                      INNER JOIN pg_index AS i ON (i.indexrelid = c.oid)
                             WHERE ((c.relam = (SELECT oid FROM pg_am WHERE (amname = 'bm25')))
                                AND i.indisvalid
                                AND i.indisready
                                AND i.indislive)) ,
                 segments AS (SELECT relname, index_info.*
                              FROM indexes
                                       INNER JOIN paradedb.index_info(indexes.relname, (('t')::pg_catalog.bool)) ON (('t')::pg_catalog.bool)) ,
                 layer_sizes AS (SELECT relname,
                                         COALESCE (lead(unnest) OVER(), 0) AS low,
                                         unnest AS high
                                  FROM indexes
                                           INNER JOIN LATERAL (SELECT unnest(((0 || paradedb.combined_layer_sizes(indexes.relname)) || 9223372036854775807))
                                                               ORDER BY 1 DESC ) AS x ON (('t')::pg_catalog.bool))
            SELECT layer_sizes.relname,
                   layer_sizes.low,
                   layer_sizes.high,
                   segments.segno,
                   segments.byte_size
            FROM layer_sizes
                     LEFT JOIN segments ON ((layer_sizes.relname = segments.relname)
                         AND ((((byte_size * 1.33))::pg_catalog.int8) BETWEEN low AND high))) AS x
      WHERE (low < high)
      GROUP BY relname, low, high
      ORDER BY relname , low DESC ) AS x ;

GRANT SELECT ON pdb.index_layer_info TO PUBLIC;

DROP VIEW paradedb.index_layer_info;
CREATE VIEW paradedb.index_layer_info AS
SELECT ((relname)::text),
       layer_size,
       low,
       high,
       byte_size,
       CASE WHEN (segments = ARRAY[NULL]) THEN 0 ELSE count END AS count,
       CASE WHEN (segments = ARRAY[NULL]) THEN NULL ELSE segments END AS segments
FROM (SELECT relname,
             ((COALESCE (pg_size_pretty(CASE WHEN (low = 0) THEN NULL ELSE low END), '') || '..') ||
              COALESCE (pg_size_pretty(CASE WHEN (high = 9223372036854775807) THEN NULL ELSE high END), '')) AS layer_size,
             count(*),
             COALESCE (sum(byte_size), 0) AS byte_size,
             min(low) AS low,
             max(high) AS high,
             array_agg(segno) AS segments
      FROM (WITH indexes AS (SELECT ((c.oid)::regclass) AS relname
                             FROM pg_class AS c
                                      INNER JOIN pg_index AS i ON (i.indexrelid = c.oid)
                             WHERE ((c.relam = (SELECT oid FROM pg_am WHERE (amname = 'bm25')))
                                AND i.indisvalid
                                AND i.indisready
                                AND i.indislive)) ,
                 segments AS (SELECT relname, index_info.*
                              FROM indexes
                                       INNER JOIN paradedb.index_info(indexes.relname, (('t')::pg_catalog.bool)) ON (('t')::pg_catalog.bool)) ,
                 layer_sizes AS (SELECT relname,
                                         COALESCE (lead(unnest) OVER(), 0) AS low,
                                         unnest AS high
                                  FROM indexes
                                           INNER JOIN LATERAL (SELECT unnest(((0 || paradedb.layer_sizes(indexes.relname)) || 9223372036854775807))
                                                               ORDER BY 1 DESC ) AS x ON (('t')::pg_catalog.bool))
            SELECT layer_sizes.relname,
                   layer_sizes.low,
                   layer_sizes.high,
                   segments.segno,
                   segments.byte_size
            FROM layer_sizes
                     LEFT JOIN segments ON ((layer_sizes.relname = segments.relname)
                         AND ((((byte_size * 1.33))::pg_catalog.int8) BETWEEN low AND high))) AS x
      WHERE (low < high)
      GROUP BY relname, low, high
      ORDER BY relname , low DESC ) AS x ;

GRANT SELECT ON paradedb.index_layer_info TO PUBLIC;
