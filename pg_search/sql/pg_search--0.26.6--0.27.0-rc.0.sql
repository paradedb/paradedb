\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.27.0-rc.0'" to load this file. \quit

-- ============================================================================
-- Fragment: 6883.vector_info_router_levels.sql
-- ============================================================================
-- Report each IVF segment's router kind and per-level centroid counts in vector_info.

DROP FUNCTION IF EXISTS vector_info(index regclass, field text);

CREATE OR REPLACE FUNCTION vector_info(index regclass, field text) RETURNS TABLE(segno text, vector_field text, vector_format text, vector_num_vectors pg_catalog."numeric", vector_num_centroids pg_catalog."numeric", vector_min_cluster_size pg_catalog."numeric", vector_max_cluster_size pg_catalog."numeric", vector_avg_cluster_size pg_catalog.float8, vector_empty_clusters pg_catalog."numeric", vector_total_rows pg_catalog."numeric", vector_router text, vector_router_levels pg_catalog.int4[], quantized bool, layers pg_catalog.int4[], quantizer_kinds text[], bytes_per_row pg_catalog.int4) AS 'MODULE_PATHNAME', 'vector_info_wrapper' LANGUAGE c STRICT;

-- ============================================================================
-- Fragment: 7206.remove_vector_radii.sql
-- ============================================================================
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

-- ============================================================================
-- Fragment: 7290.synonym_index.sql
-- ============================================================================
CREATE FUNCTION pdb."synonym_rule_is_valid"(
    "rule" TEXT
) RETURNS bool
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'synonym_rule_is_valid_wrapper';

CREATE DOMAIN pdb.synonym_rule AS text CHECK (pdb.synonym_rule_is_valid(VALUE));

CREATE FUNCTION pdb."synonym_expansion"(
    "index" regclass,
    "input" TEXT
) RETURNS TABLE (
    "position" INT,
    "token" TEXT,
    "alternatives" TEXT[]
)
STRICT STABLE
LANGUAGE c
AS 'MODULE_PATHNAME', 'synonym_expansion_wrapper';
