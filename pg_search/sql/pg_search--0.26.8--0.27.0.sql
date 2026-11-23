\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.27.0'" to load this file. \quit

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

-- ============================================================================
-- Fragment: 7399.expand.sql
-- ============================================================================
-- No release has pdb.with_synonyms. Only a database that was built from main has it.
DROP FUNCTION IF EXISTS pdb.with_synonyms(pdb.query, regclass);
DROP FUNCTION IF EXISTS pdb.with_synonyms(text, regclass);

CREATE FUNCTION "expand_with_index"(
    "query" TEXT,
    "index" regclass
) RETURNS pdb.Query
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'expand_with_index_wrapper';

CREATE FUNCTION pdb."expand_agg_expand_agg_state"(
    "this" internal,
    "query" TEXT,
    "rule" TEXT
) RETURNS internal
LANGUAGE c
AS 'MODULE_PATHNAME', 'expand_agg_expand_agg_state_wrapper';

CREATE FUNCTION pdb."expand_agg_expand_agg_finalize"(
    "this" internal
) RETURNS pdb.Query
LANGUAGE c
AS 'MODULE_PATHNAME', 'expand_agg_expand_agg_finalize_wrapper';

CREATE AGGREGATE pdb.expand (
    "query" TEXT,
    "rule" TEXT
)
(
    SFUNC = pdb."expand_agg_expand_agg_state",
    STYPE = internal,
    FINALFUNC = pdb."expand_agg_expand_agg_finalize",
    PARALLEL = SAFE
);

-- ============================================================================
-- Fragment: 7503.explain_score.sql
-- ============================================================================
CREATE FUNCTION pdb."explain_score"(
    "relation_reference" anyelement
) RETURNS jsonb
STRICT STABLE PARALLEL SAFE COST 1
LANGUAGE c
AS 'MODULE_PATHNAME', 'explain_score_from_relation_wrapper';

ALTER FUNCTION pdb.explain_score SUPPORT paradedb.placeholder_support;
