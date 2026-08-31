-- The extension control file fixes the installation schema to `paradedb`.

CREATE FUNCTION paradedb.vector_estimator_info(
    index regclass,
    field text,
    queries vector[] DEFAULT NULL
) RETURNS TABLE(
    depth integer,
    bias real,
    spread real,
    sample_rows integer,
    query_count integer,
    query_source text
)
STABLE PARALLEL UNSAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'vector_estimator_info_internal_wrapper';

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
	"vector_total_memberships" NUMERIC,  /* Option < AnyNumeric > */
	"quantized" bool,  /* bool */
	"layers" INT[],  /* :: std :: option :: Option < Vec < i32 > > */
	"bytes_per_row" INT,  /* Option < i32 > */
	"format" INT  /* Option < i32 > */
)
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'vector_info_wrapper';
