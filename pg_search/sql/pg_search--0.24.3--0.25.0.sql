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

-- Adds ivf_cluster_sizes(index regclass): a read-only set-returning function
-- that surfaces the raw per-cluster IVF posting-list sizes, one row per cluster
-- per segment. This is the un-collapsed distribution behind vector_info's
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

-- Adds vector_info(index regclass, field text): the per-segment vector statistics
-- for a single vector field, split out of index_info so each vector field can be
-- inspected explicitly. The CREATE below is the SchemaBot/pgrx canonical text
-- verbatim; the DROP keeps the script re-runnable.
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
	"vector_empty_clusters" NUMERIC  /* Option < AnyNumeric > */
)
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'vector_info_wrapper';

-- Vector opclasses (pgvector convention). Pure metric tags: STORAGE only,
-- no strategy operators or support functions. bm25 reads the metric back at
-- build time; vector_l2_ops is DEFAULT so a bare `(embedding)` resolves to L2.
CREATE OPERATOR CLASS public.vector_l2_ops DEFAULT FOR TYPE public.vector USING bm25 AS
    STORAGE public.vector;
CREATE OPERATOR CLASS public.vector_cosine_ops FOR TYPE public.vector USING bm25 AS
    STORAGE public.vector;
CREATE OPERATOR CLASS public.vector_ip_ops FOR TYPE public.vector USING bm25 AS
    STORAGE public.vector;
