\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.25.0'" to load this file. \quit

-- Upgrading to 0.25.0 requires the pgvector extension: the control file
-- declares requires='vector' (the opclasses below are FOR TYPE public.vector).
-- Postgres enforces this at ALTER EXTENSION time, before this script runs, with
-- ERROR: required extension "vector" is not installed
-- so run CREATE EXTENSION vector first; no mid-script failure is possible.

-- Re-emit index_created_by on the current upgrade path (same pattern as the
-- pdb.indexes()/index_segments() re-emit in pg_search--0.24.0--0.24.1.sql).
-- The v0.24.3 release tag was cut before #5533 landed, so a fresh install of
-- released 0.24.3 lacks this function even though main's
-- pg_search--0.24.2--0.24.3.sql creates it for upgraders passing through --
-- an installation that STARTED at 0.24.3 would otherwise never receive it.
-- DROP-then-CREATE keeps it idempotent for those that did come through 0.24.2.
DROP FUNCTION IF EXISTS "index_created_by"(regclass);
CREATE FUNCTION "index_created_by"(
	"index" regclass /* pgrx::rel::PgRelation */
) RETURNS TEXT /* core::option::Option<alloc::string::String> */
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'index_created_by_wrapper';

-- Adds vector_info(index regclass, field text): the per-segment vector statistics
-- for a single vector field, split out of index_info so each vector field can be
-- inspected explicitly. The cluster columns are IVF-only and count posting rows,
-- so under replication their totals exceed vector_num_vectors (distinct docs).
-- The CREATE below is the SchemaBot/pgrx canonical text verbatim (the schema
-- checker compares statements textually); the DROP keeps the script re-runnable.
DROP FUNCTION IF EXISTS vector_info(regclass, text);
CREATE  FUNCTION "vector_info"(
	"index" regclass, /* PgRelation */
	"field" TEXT /* String */
) RETURNS TABLE (
	"segno" TEXT,  /* String */
	"vector_field" TEXT,  /* String */
	"vector_format" TEXT,  /* String */
	"vector_num_vectors" NUMERIC,  /* AnyNumeric */
	"vector_num_centroids" NUMERIC,  /* Option < AnyNumeric > */
	"vector_min_cluster_size" NUMERIC,  /* Option < AnyNumeric > */
	"vector_max_cluster_size" NUMERIC,  /* Option < AnyNumeric > */
	"vector_avg_cluster_size" double precision,  /* Option < f64 > */
	"vector_empty_clusters" NUMERIC,  /* Option < AnyNumeric > */
	"vector_total_memberships" NUMERIC  /* Option < AnyNumeric > */
)
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'vector_info_wrapper';

-- Introduce `paradedb` as the primary name for the index access method (sharing the
-- existing `bm25_handler`), a default operator class for it, and refresh the
-- `index_layer_info` views so they recognize any index whose access method uses
-- `paradedb.bm25_handler`, independent of the access method's name.
-- Statements below are emitted verbatim by SchemaBot (pg-schema-diff).
CREATE ACCESS METHOD paradedb TYPE INDEX HANDLER bm25_handler;
DROP VIEW pdb.index_layer_info;
CREATE VIEW pdb.index_layer_info AS SELECT ((relname)::text), layer_size, low, high, byte_size, CASE WHEN (segments = ARRAY[NULL]) THEN 0 ELSE count END AS count, CASE WHEN (segments = ARRAY[NULL]) THEN NULL ELSE segments END AS segments FROM (SELECT relname, ((COALESCE (pg_size_pretty(CASE WHEN (low = 0) THEN NULL ELSE low END), '') || '..') || COALESCE (pg_size_pretty(CASE WHEN (high = 9223372036854775807) THEN NULL ELSE high END), '')) AS layer_size, count(*), COALESCE (sum(byte_size), 0) AS byte_size, min(low) AS low, max(high) AS high, array_agg(segno) AS segments FROM (WITH indexes AS (SELECT ((c.oid)::regclass) AS relname FROM pg_class AS c INNER JOIN pg_index AS i ON (i.indexrelid = c.oid) WHERE (c.relam IN (SELECT oid FROM pg_am WHERE (amhandler = (('paradedb.bm25_handler')::regproc))) AND i.indisvalid AND i.indisready AND i.indislive)) , segments AS (SELECT relname, index_info.* FROM indexes INNER JOIN paradedb.index_info(indexes.relname, (('t')::pg_catalog.bool)) ON (('t')::pg_catalog.bool)) , layer_sizes AS (SELECT relname, COALESCE (lead(unnest) OVER(), 0) AS low, unnest AS high FROM indexes INNER JOIN LATERAL (SELECT unnest(((0 || paradedb.combined_layer_sizes(indexes.relname)) || 9223372036854775807)) ORDER BY 1 DESC ) AS x ON (('t')::pg_catalog.bool)) SELECT layer_sizes.relname, layer_sizes.low, layer_sizes.high, segments.segno, segments.byte_size FROM layer_sizes LEFT JOIN segments ON ((layer_sizes.relname = segments.relname) AND ((((byte_size * 1.33))::pg_catalog.int8) BETWEEN low AND high))) AS x WHERE (low < high) GROUP BY relname, low, high ORDER BY relname , low DESC ) AS x ;
GRANT SELECT ON pdb.index_layer_info TO PUBLIC;
DROP VIEW paradedb.index_layer_info;
CREATE VIEW paradedb.index_layer_info AS SELECT ((relname)::text), layer_size, low, high, byte_size, CASE WHEN (segments = ARRAY[NULL]) THEN 0 ELSE count END AS count, CASE WHEN (segments = ARRAY[NULL]) THEN NULL ELSE segments END AS segments FROM (SELECT relname, ((COALESCE (pg_size_pretty(CASE WHEN (low = 0) THEN NULL ELSE low END), '') || '..') || COALESCE (pg_size_pretty(CASE WHEN (high = 9223372036854775807) THEN NULL ELSE high END), '')) AS layer_size, count(*), COALESCE (sum(byte_size), 0) AS byte_size, min(low) AS low, max(high) AS high, array_agg(segno) AS segments FROM (WITH indexes AS (SELECT ((c.oid)::regclass) AS relname FROM pg_class AS c INNER JOIN pg_index AS i ON (i.indexrelid = c.oid) WHERE (c.relam IN (SELECT oid FROM pg_am WHERE (amhandler = (('paradedb.bm25_handler')::regproc))) AND i.indisvalid AND i.indisready AND i.indislive)) , segments AS (SELECT relname, index_info.* FROM indexes INNER JOIN paradedb.index_info(indexes.relname, (('t')::pg_catalog.bool)) ON (('t')::pg_catalog.bool)) , layer_sizes AS (SELECT relname, COALESCE (lead(unnest) OVER(), 0) AS low, unnest AS high FROM indexes INNER JOIN LATERAL (SELECT unnest(((0 || paradedb.layer_sizes(indexes.relname)) || 9223372036854775807)) ORDER BY 1 DESC ) AS x ON (('t')::pg_catalog.bool)) SELECT layer_sizes.relname, layer_sizes.low, layer_sizes.high, segments.segno, segments.byte_size FROM layer_sizes LEFT JOIN segments ON ((layer_sizes.relname = segments.relname) AND ((((byte_size * 1.33))::pg_catalog.int8) BETWEEN low AND high))) AS x WHERE (low < high) GROUP BY relname, low, high ORDER BY relname , low DESC ) AS x ;
GRANT SELECT ON paradedb.index_layer_info TO PUBLIC;
CREATE OPERATOR CLASS anyelement_paradedb_ops DEFAULT FOR TYPE anyelement USING paradedb AS
    OPERATOR 1 pg_catalog.@@@(anyelement, text),                         /* for querying with a tantivy-compatible text query */
    OPERATOR 2 pg_catalog.@@@(anyelement, paradedb.searchqueryinput),    /* for querying with a paradedb.searchqueryinput structure */
    STORAGE anyelement;

-- Vector opclasses (pgvector convention). Pure metric tags: STORAGE only,
-- no strategy operators or support functions. bm25 reads the metric back at
-- build time; vector_l2_ops is DEFAULT so a bare `(embedding)` resolves to L2.
CREATE OPERATOR CLASS public.vector_l2_ops DEFAULT FOR TYPE public.vector USING bm25 AS
    STORAGE public.vector;
CREATE OPERATOR CLASS public.vector_cosine_ops FOR TYPE public.vector USING bm25 AS
    STORAGE public.vector;
CREATE OPERATOR CLASS public.vector_ip_ops FOR TYPE public.vector USING bm25 AS
    STORAGE public.vector;
CREATE OPERATOR CLASS public.vector_l2_ops DEFAULT FOR TYPE public.vector USING paradedb AS
    STORAGE public.vector;
CREATE OPERATOR CLASS public.vector_cosine_ops FOR TYPE public.vector USING paradedb AS
    STORAGE public.vector;
CREATE OPERATOR CLASS public.vector_ip_ops FOR TYPE public.vector USING paradedb AS
    STORAGE public.vector;

-- Adds ivf_cluster_radii(index regclass, field text): a read-only
-- set-returning function surfacing the stored per-cluster IVF radii
-- (`.centroids` slot [3]) of one vector field, one row per cluster per
-- segment - the observation instrument for the radius-aware probe gate.
-- Radii are NATIVE-only (rank-0 members; replica spill excluded); segments
-- written before the radius slot report zeros, and segments written by an
-- earlier replica-inclusive fold report those larger, conservative values.
-- Computes nothing new and adds no on-disk state. The CREATE below is the
-- SchemaBot/pgrx canonical text verbatim (the schema checker compares
-- statements textually); the DROP keeps the script re-runnable.
DROP FUNCTION IF EXISTS ivf_cluster_radii(regclass, text);
CREATE  FUNCTION "ivf_cluster_radii"(
	"index" regclass, /* PgRelation */
	"field" TEXT /* String */
) RETURNS TABLE (
	"segno" TEXT,  /* String */
	"vector_field" TEXT,  /* String */
	"cluster_ord" INT,  /* i32 */
	"radius" real  /* f32 */
)
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'ivf_cluster_radii_wrapper';
