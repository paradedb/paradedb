\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.24.0'" to load this file. \quit

DROP FUNCTION IF EXISTS field(name text, indexed bool, stored bool, fast bool, fieldnorms bool, record text, expand_dots bool, tokenizer jsonb, normalizer text);
CREATE OR REPLACE FUNCTION field(name text, indexed bool DEFAULT NULL, stored bool DEFAULT NULL, fast bool DEFAULT NULL, fieldnorms bool DEFAULT NULL, record text DEFAULT NULL, expand_dots bool DEFAULT NULL, tokenizer jsonb DEFAULT NULL, normalizer text DEFAULT NULL, alias text DEFAULT NULL) RETURNS jsonb AS 'MODULE_PATHNAME', 'field_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;

DROP FUNCTION IF EXISTS index_info(index regclass, show_invisible bool);
CREATE OR REPLACE FUNCTION index_info(index regclass, show_invisible bool DEFAULT 'false') RETURNS TABLE(index_name text, visible bool, recyclable bool, xmax xid, segno text, mutable bool, byte_size pg_catalog."numeric", num_docs pg_catalog."numeric", num_deleted pg_catalog."numeric", termdict_bytes pg_catalog."numeric", postings_bytes pg_catalog."numeric", positions_bytes pg_catalog."numeric", fast_fields_bytes pg_catalog."numeric", fieldnorms_bytes pg_catalog."numeric", store_bytes pg_catalog."numeric", deletes_bytes pg_catalog."numeric", vector_field text, vector_format text, vector_num_vectors pg_catalog."numeric", vector_num_centroids pg_catalog."numeric", vector_min_cluster_size pg_catalog."numeric", vector_max_cluster_size pg_catalog."numeric", vector_avg_cluster_size pg_catalog.float8, vector_empty_clusters pg_catalog."numeric") AS 'MODULE_PATHNAME', 'index_info_wrapper' LANGUAGE c STRICT;

-- Vector opclasses (pgvector convention). Pure metric tags: STORAGE only,
-- no strategy operators or support functions. bm25 reads the metric back at
-- build time; vector_l2_ops is DEFAULT so a bare `(embedding)` resolves to L2.
CREATE OPERATOR CLASS public.vector_l2_ops DEFAULT FOR TYPE public.vector USING bm25 AS
    STORAGE public.vector;
CREATE OPERATOR CLASS public.vector_cosine_ops FOR TYPE public.vector USING bm25 AS
    STORAGE public.vector;
CREATE OPERATOR CLASS public.vector_ip_ops FOR TYPE public.vector USING bm25 AS
    STORAGE public.vector;
