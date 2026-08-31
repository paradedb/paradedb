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

DROP FUNCTION IF EXISTS vector_info(index regclass, field text);

CREATE OR REPLACE FUNCTION vector_info(index regclass, field text) RETURNS TABLE(segno text, vector_field text, vector_format text, vector_num_vectors pg_catalog."numeric", vector_num_centroids pg_catalog."numeric", vector_min_cluster_size pg_catalog."numeric", vector_max_cluster_size pg_catalog."numeric", vector_avg_cluster_size pg_catalog.float8, vector_empty_clusters pg_catalog."numeric", vector_total_memberships pg_catalog."numeric", quantized bool, layers pg_catalog.int4[], bytes_per_row pg_catalog.int4, format pg_catalog.int4) AS 'MODULE_PATHNAME', 'vector_info_wrapper' LANGUAGE c STRICT;
