\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.24.1'" to load this file. \quit

-- `boost_to_fuzzy` and its cast are also defined in `fuzzy.rs`, so they live in
-- the base install schema. Depending on which `v0.24.0` an install was built
-- from, they may or may not already exist (community's `v0.24.0` predates them;
-- enterprise's folds them into the base schema). Drop-then-create keeps this
-- upgrade idempotent in both cases so it never fails with "already exists".
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

-- The amcheck-style index verification functions (PR #3907) were originally
-- emitted into `pg_search--0.21.4--0.21.5.sql` -- a migration for a release that
-- had already shipped, so it sits off the upgrade path for anyone already past
-- 0.21.5. Fresh installs got these from the generated base schema, but
-- `ALTER EXTENSION pg_search UPDATE` from e.g. 0.23.x never created them. Re-emit
-- them here on the current upgrade path. DROP-then-CREATE keeps it idempotent for
-- installs that already have them (fresh 0.24.0, or a drop+create workaround).
DROP FUNCTION IF EXISTS pdb."indexes"();
CREATE  FUNCTION pdb."indexes"() RETURNS TABLE (
	"schemaname" TEXT,  /* alloc::string::String */
	"tablename" TEXT,  /* alloc::string::String */
	"indexname" TEXT,  /* alloc::string::String */
	"indexrelid" oid,  /* pgrx_pg_sys::submodules::oids::Oid */
	"num_segments" INT,  /* i32 */
	"total_docs" bigint  /* i64 */
)
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'indexes_wrapper';

DROP FUNCTION IF EXISTS pdb."index_segments"(regclass);
CREATE  FUNCTION pdb."index_segments"(
	"index" regclass /* pgrx::rel::PgRelation */
) RETURNS TABLE (
	"partition_name" TEXT,  /* alloc::string::String */
	"segment_idx" INT,  /* i32 */
	"segment_id" TEXT,  /* alloc::string::String */
	"num_docs" bigint,  /* i64 */
	"num_deleted" bigint,  /* i64 */
	"max_doc" bigint  /* i64 */
)
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'index_segments_wrapper';

DROP FUNCTION IF EXISTS pdb."verify_all_indexes"(TEXT, TEXT, bool, double precision, bool, bool);
CREATE  FUNCTION pdb."verify_all_indexes"(
	"schema_pattern" TEXT DEFAULT NULL, /* core::option::Option<alloc::string::String> */
	"index_pattern" TEXT DEFAULT NULL, /* core::option::Option<alloc::string::String> */
	"heapallindexed" bool DEFAULT false, /* bool */
	"sample_rate" double precision DEFAULT NULL, /* core::option::Option<f64> */
	"report_progress" bool DEFAULT false, /* bool */
	"on_error_stop" bool DEFAULT false /* bool */
) RETURNS TABLE (
	"schemaname" TEXT,  /* alloc::string::String */
	"indexname" TEXT,  /* alloc::string::String */
	"check_name" TEXT,  /* alloc::string::String */
	"passed" bool,  /* bool */
	"details" TEXT  /* core::option::Option<alloc::string::String> */
)
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'verify_all_indexes_wrapper';

DROP FUNCTION IF EXISTS pdb."verify_index"(regclass, bool, double precision, bool, bool, bool, INT[]);
CREATE  FUNCTION pdb."verify_index"(
	"index" regclass, /* pgrx::rel::PgRelation */
	"heapallindexed" bool DEFAULT false, /* bool */
	"sample_rate" double precision DEFAULT NULL, /* core::option::Option<f64> */
	"report_progress" bool DEFAULT false, /* bool */
	"verbose" bool DEFAULT false, /* bool */
	"on_error_stop" bool DEFAULT false, /* bool */
	"segment_ids" INT[] DEFAULT NULL /* core::option::Option<alloc::vec::Vec<i32>> */
) RETURNS TABLE (
	"check_name" TEXT,  /* alloc::string::String */
	"passed" bool,  /* bool */
	"details" TEXT  /* core::option::Option<alloc::string::String> */
)
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'verify_index_wrapper';
