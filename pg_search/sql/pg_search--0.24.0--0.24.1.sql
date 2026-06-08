\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.24.1'" to load this file. \quit

-- `boost_to_fuzzy` and its cast are also defined in `fuzzy.rs`, so they live in
-- the base install schema. Depending on which `v0.24.0` an install was built
-- from, they may or may not already exist (community's `v0.24.0` predates them;
-- enterprise's folds them into the base schema). Drop-then-create keeps this
-- upgrade idempotent in both cases so it never fails with "already exists".
-- Update boolean() overloads to add minimum_should_match parameter
DROP FUNCTION IF EXISTS "boolean"(must searchqueryinput, should searchqueryinput, must_not searchqueryinput);
DROP FUNCTION IF EXISTS "boolean"(must searchqueryinput[], should searchqueryinput[], must_not searchqueryinput[]);

CREATE OR REPLACE FUNCTION "boolean"(
	"must" SearchQueryInput DEFAULT NULL,
	"should" SearchQueryInput DEFAULT NULL,
	"must_not" SearchQueryInput DEFAULT NULL,
	"minimum_should_match" bigint DEFAULT NULL
) RETURNS SearchQueryInput
IMMUTABLE PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'boolean_singles_wrapper';

CREATE OR REPLACE FUNCTION "boolean"(
	"must" SearchQueryInput[] DEFAULT ARRAY[]::searchqueryinput[],
	"should" SearchQueryInput[] DEFAULT ARRAY[]::searchqueryinput[],
	"must_not" SearchQueryInput[] DEFAULT ARRAY[]::searchqueryinput[],
	"minimum_should_match" bigint DEFAULT NULL
) RETURNS SearchQueryInput
IMMUTABLE PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'boolean_arrays_wrapper';

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
