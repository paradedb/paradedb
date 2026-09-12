-- Vector indexes use one index-level centroid index. Flushed segments cluster
-- against it while mutable segments stay flat, so vector_info no longer
-- reports the removed vector_format column.
DROP FUNCTION IF EXISTS vector_info(index regclass, field text);
CREATE OR REPLACE FUNCTION vector_info(index regclass, field text) RETURNS TABLE(segno text, vector_field text, vector_num_vectors pg_catalog."numeric", vector_num_centroids pg_catalog."numeric", vector_min_cluster_size pg_catalog."numeric", vector_max_cluster_size pg_catalog."numeric", vector_avg_cluster_size pg_catalog.float8, vector_empty_clusters pg_catalog."numeric", vector_total_memberships pg_catalog."numeric") AS 'MODULE_PATHNAME', 'vector_info_wrapper' LANGUAGE c STRICT;
