-- Vector indexes use one index-level centroid index. Flushed segments cluster
-- against it while mutable segments stay flat, so vector_info no longer
-- reports the removed vector_format column.
DROP FUNCTION IF EXISTS vector_info(regclass, text);
CREATE  FUNCTION "vector_info"(
	"index" regclass, /* PgRelation */
	"field" TEXT /* String */
) RETURNS TABLE (
	"segno" TEXT,  /* String */
	"vector_field" TEXT,  /* String */
	"vector_num_vectors" NUMERIC,  /* AnyNumeric */
	"vector_num_centroids" NUMERIC,  /* AnyNumeric */
	"vector_min_cluster_size" NUMERIC,  /* AnyNumeric */
	"vector_max_cluster_size" NUMERIC,  /* AnyNumeric */
	"vector_avg_cluster_size" double precision,  /* f64 */
	"vector_empty_clusters" NUMERIC,  /* AnyNumeric */
	"vector_total_memberships" NUMERIC  /* AnyNumeric */
)
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'vector_info_wrapper';
