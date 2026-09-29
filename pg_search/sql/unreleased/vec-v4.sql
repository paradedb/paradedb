DROP FUNCTION IF EXISTS vector_info(regclass, text);
CREATE FUNCTION vector_info(index regclass, field text)
RETURNS TABLE(segno text, vector_field text, vector_format text,
    vector_num_vectors numeric, vector_num_centroids numeric,
    vector_min_cluster_size numeric, vector_max_cluster_size numeric,
    vector_avg_cluster_size float8, vector_empty_clusters numeric, vector_total_rows numeric,
    quantized boolean, layers integer[], quantizer_kinds text[], bytes_per_row integer)
AS 'MODULE_PATHNAME', 'vector_info_wrapper' LANGUAGE c STRICT;

CREATE FUNCTION vector_config(index regclass, field text)
RETURNS TABLE(quantized boolean, layers integer[], bytes_per_row integer, format_version integer)
AS 'MODULE_PATHNAME', 'vector_config_wrapper' LANGUAGE c STRICT;
