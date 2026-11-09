-- Report each IVF segment's router kind and per-level centroid counts in vector_info.

DROP FUNCTION IF EXISTS vector_info(index regclass, field text);

CREATE OR REPLACE FUNCTION vector_info(index regclass, field text) RETURNS TABLE(segno text, vector_field text, vector_format text, vector_num_vectors pg_catalog."numeric", vector_num_centroids pg_catalog."numeric", vector_min_cluster_size pg_catalog."numeric", vector_max_cluster_size pg_catalog."numeric", vector_avg_cluster_size pg_catalog.float8, vector_empty_clusters pg_catalog."numeric", vector_total_rows pg_catalog."numeric", vector_router text, vector_router_levels pg_catalog.int4[], quantized bool, layers pg_catalog.int4[], quantizer_kinds text[], bytes_per_row pg_catalog.int4) AS 'MODULE_PATHNAME', 'vector_info_wrapper' LANGUAGE c STRICT;
