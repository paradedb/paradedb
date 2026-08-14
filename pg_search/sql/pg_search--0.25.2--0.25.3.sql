\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.25.3'" to load this file. \quit

-- 0.25.2 -> 0.25.3: add vector_cluster_sizes(index regclass, field text), the
-- per-segment raw cluster-size arrays behind vector_info's min/avg/max
-- aggregates, for callers that want percentiles or histograms of the cluster
-- size distribution. cluster_sizes is NULL for flat segments; sizes count
-- posting rows, so under replication their sum exceeds the segment's
-- distinct-doc count.
-- The CREATE below is the SchemaBot/pgrx canonical text verbatim (the schema
-- checker compares statements textually); the DROP keeps the script re-runnable.
DROP FUNCTION IF EXISTS vector_cluster_sizes(regclass, text);
CREATE  FUNCTION "vector_cluster_sizes"(
	"index" regclass, /* PgRelation */
	"field" TEXT /* String */
) RETURNS TABLE (
	"segno" TEXT,  /* String */
	"cluster_sizes" bigint[]  /* :: std :: option :: Option < Vec < i64 > > */
)
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'vector_cluster_sizes_wrapper';
