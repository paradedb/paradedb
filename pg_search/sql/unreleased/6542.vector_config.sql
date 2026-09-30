/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/admin.rs:519
-- pg_search::api::admin::vector_config
CREATE  FUNCTION "vector_config"(
	"index" regclass, /* PgRelation */
	"field" TEXT /* String */
) RETURNS TABLE (
	"quantized" bool,  /* bool */
	"layers" INT[],  /* :: std :: option :: Option < Vec < i32 > > */
	"bytes_per_row" INT,  /* Option < i32 > */
	"settings_version" INT  /* Option < i32 > */
)
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'vector_config_wrapper';
DROP FUNCTION IF EXISTS vector_info(index regclass, field text);
CREATE OR REPLACE FUNCTION vector_info(index regclass, field text) RETURNS TABLE(segno text, vector_field text, vector_format text, vector_num_vectors pg_catalog."numeric", vector_num_centroids pg_catalog."numeric", vector_min_cluster_size pg_catalog."numeric", vector_max_cluster_size pg_catalog."numeric", vector_avg_cluster_size pg_catalog.float8, vector_empty_clusters pg_catalog."numeric", vector_total_rows pg_catalog."numeric", quantized bool, layers pg_catalog.int4[], quantizer_kinds text[], bytes_per_row pg_catalog.int4) AS 'MODULE_PATHNAME', 'vector_info_wrapper' LANGUAGE c STRICT;

