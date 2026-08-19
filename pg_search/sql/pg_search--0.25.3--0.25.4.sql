-- Vector indexes moved to tantivy's index-level centroid set format: every
-- segment is clustered (the flat layout is gone), so vector_info's
-- vector_format column is replaced by vector_centroid_set_version — the
-- set version the segment assigned against — and the cluster columns are
-- no longer nullable. The CREATE below is the SchemaBot/pgrx canonical
-- text verbatim; the DROP keeps the script re-runnable.
DROP FUNCTION IF EXISTS vector_info(regclass, text);
CREATE  FUNCTION "vector_info"(
	"index" regclass, /* PgRelation */
	"field" TEXT /* String */
) RETURNS TABLE (
	"segno" TEXT,  /* String */
	"vector_field" TEXT,  /* String */
	"vector_centroid_set_version" NUMERIC,  /* AnyNumeric */
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
