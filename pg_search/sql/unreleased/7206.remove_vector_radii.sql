DROP FUNCTION IF EXISTS vector_clusters(regclass, text);
CREATE FUNCTION "vector_clusters"(
    "index" regclass,
    "field" TEXT
) RETURNS TABLE (
    "segno" TEXT,
    "cluster_sizes" bigint[]
)
STRICT
LANGUAGE c
AS 'MODULE_PATHNAME', 'vector_clusters_wrapper';
