\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.25.0'" to load this file. \quit

-- Upgrading to 0.25.0 requires the pgvector extension: the control file
-- declares requires='vector' (the opclasses below are FOR TYPE public.vector).
-- Postgres enforces this at ALTER EXTENSION time, before this script runs, with
-- ERROR: required extension "vector" is not installed
-- so run CREATE EXTENSION vector first; no mid-script failure is possible.

-- index_info gains vector_* columns, changing its RETURNS TABLE shape, so it must be
-- dropped and recreated. Both index_layer_info views depend on it, so drop them first
-- and recreate them (verbatim from their base definitions) afterward.
DROP VIEW IF EXISTS pdb.index_layer_info;
DROP VIEW IF EXISTS paradedb.index_layer_info;

DROP FUNCTION IF EXISTS index_info(index regclass, show_invisible bool);
CREATE OR REPLACE FUNCTION index_info(index regclass, show_invisible bool DEFAULT 'false') RETURNS TABLE(index_name text, visible bool, recyclable bool, xmax xid, segno text, mutable bool, byte_size pg_catalog."numeric", num_docs pg_catalog."numeric", num_deleted pg_catalog."numeric", termdict_bytes pg_catalog."numeric", postings_bytes pg_catalog."numeric", positions_bytes pg_catalog."numeric", fast_fields_bytes pg_catalog."numeric", fieldnorms_bytes pg_catalog."numeric", store_bytes pg_catalog."numeric", deletes_bytes pg_catalog."numeric", vector_field text, vector_format text, vector_num_vectors pg_catalog."numeric", vector_num_centroids pg_catalog."numeric", vector_min_cluster_size pg_catalog."numeric", vector_max_cluster_size pg_catalog."numeric", vector_avg_cluster_size pg_catalog.float8, vector_empty_clusters pg_catalog."numeric") AS 'MODULE_PATHNAME', 'index_info_wrapper' LANGUAGE c STRICT;

create view paradedb.index_layer_info as
select relname::text,
       layer_size,
       low,
       high,
       byte_size,
       case when segments = ARRAY [NULL] then 0 else count end       as count,
       case when segments = ARRAY [NULL] then NULL else segments end as segments
from (select relname,
             coalesce(pg_size_pretty(case when low = 0 then null else low end), '') || '..' ||
             coalesce(pg_size_pretty(case when high = 9223372036854775807 then null else high end), '') as layer_size,
             count(*),
             coalesce(sum(byte_size), 0)                                                                as byte_size,
             min(low)                                                                                   as low,
             max(high)                                                                                  as high,
             array_agg(segno)                                                                           as segments
      from (with indexes as (select oid::regclass as relname
                             from pg_class
                             where relam = (select oid from pg_am where amname = 'bm25')),
                 segments as (select relname, index_info.*
                              from indexes
                                       inner join paradedb.index_info(indexes.relname, true) on true),
                 layer_sizes as (select relname, coalesce(lead(unnest) over (), 0) low, unnest as high
                                 from indexes
                                          inner join lateral (select unnest(0 || paradedb.layer_sizes(indexes.relname) || 9223372036854775807)
                                                              order by 1 desc) x on true)
            select layer_sizes.relname, layer_sizes.low, layer_sizes.high, segments.segno, segments.byte_size
            from layer_sizes
                     left join segments on layer_sizes.relname = segments.relname and
                                           (byte_size * 1.33)::bigint between low and high) x
      where low < high
      group by relname, low, high
      order by relname, low desc) x;

GRANT SELECT ON paradedb.index_layer_info TO PUBLIC;

create view pdb.index_layer_info as
select relname::text,
       layer_size,
       low,
       high,
       byte_size,
       case when segments = ARRAY [NULL] then 0 else count end       as count,
       case when segments = ARRAY [NULL] then NULL else segments end as segments
from (select relname,
             coalesce(pg_size_pretty(case when low = 0 then null else low end), '') || '..' ||
             coalesce(pg_size_pretty(case when high = 9223372036854775807 then null else high end), '') as layer_size,
             count(*),
             coalesce(sum(byte_size), 0)                                                                as byte_size,
             min(low)                                                                                   as low,
             max(high)                                                                                   as high,
             array_agg(segno)                                                                           as segments
      from (with indexes as (select oid::regclass as relname
                             from pg_class
                             where relam = (select oid from pg_am where amname = 'bm25')),
                 segments as (select relname, index_info.*
                              from indexes
                                       inner join paradedb.index_info(indexes.relname, true) on true),
                 layer_sizes as (select relname, coalesce(lead(unnest) over (), 0) low, unnest as high
                                 from indexes
                                          inner join lateral (select unnest(0 || paradedb.combined_layer_sizes(indexes.relname) || 9223372036854775807)
                                                              order by 1 desc) x on true)
            select layer_sizes.relname, layer_sizes.low, layer_sizes.high, segments.segno, segments.byte_size
            from layer_sizes
                     left join segments on layer_sizes.relname = segments.relname and
                                           (byte_size * 1.33)::bigint between low and high) x
      where low < high
      group by relname, low, high
      order by relname, low desc) x;

GRANT SELECT ON pdb.index_layer_info TO PUBLIC;

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

-- Adds ivf_cluster_sizes(index regclass): a read-only set-returning function
-- that surfaces the raw per-cluster IVF posting-list sizes, one row per cluster
-- per segment. This is the un-collapsed distribution behind index_info's
-- vector_min/max/avg_cluster_size columns; it computes nothing new and adds no
-- on-disk state.
-- The CREATE below is the SchemaBot/pgrx canonical text verbatim (the schema
-- checker compares statements textually); the DROP keeps the script re-runnable.
DROP FUNCTION IF EXISTS ivf_cluster_sizes(regclass);
CREATE  FUNCTION "ivf_cluster_sizes"(
	"index" regclass /* PgRelation */
) RETURNS TABLE (
	"segno" TEXT,  /* String */
	"field" TEXT,  /* String */
	"cluster_ord" INT,  /* i32 */
	"size" bigint  /* i64 */
)
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'ivf_cluster_sizes_wrapper';

-- Vector opclasses (pgvector convention). Pure metric tags: STORAGE only,
-- no strategy operators or support functions. bm25 reads the metric back at
-- build time; vector_l2_ops is DEFAULT so a bare `(embedding)` resolves to L2.
CREATE OPERATOR CLASS public.vector_l2_ops DEFAULT FOR TYPE public.vector USING bm25 AS
    STORAGE public.vector;
CREATE OPERATOR CLASS public.vector_cosine_ops FOR TYPE public.vector USING bm25 AS
    STORAGE public.vector;
CREATE OPERATOR CLASS public.vector_ip_ops FOR TYPE public.vector USING bm25 AS
    STORAGE public.vector;
