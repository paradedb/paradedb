\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.25.4'" to load this file. \quit

-- 0.25.3 -> 0.25.4: add vector_clusters(index regclass, field text), the
-- per-segment per-cluster posting-list sizes and ball-bound radii, in cluster
-- order.
-- The CREATE below is the SchemaBot/pgrx canonical text verbatim (the schema
-- checker compares statements textually); the DROP keeps the script re-runnable.
DROP FUNCTION IF EXISTS vector_clusters(regclass, text);
CREATE  FUNCTION "vector_clusters"(
	"index" regclass, /* PgRelation */
	"field" TEXT /* String */
) RETURNS TABLE (
	"segno" TEXT,  /* String */
	"cluster_sizes" bigint[],  /* :: std :: option :: Option < Vec < i64 > > */
	"cluster_radii" real[]  /* :: std :: option :: Option < Vec < f32 > > */
)
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'vector_clusters_wrapper';

