SET client_min_messages = WARNING;
CREATE EXTENSION IF NOT EXISTS vector;
\i common/common_setup.sql

SET paradedb.vector_clustering_threshold = 64;
SET paradedb.vector_stats = on;

-- Integer coordinates in [-50, 50]: every squared L2 distance is an exact float, so the
-- exhaustive order below is the true order, ties broken by id.
CREATE FUNCTION filtered_scan_vector(d integer, n integer)
RETURNS vector
LANGUAGE SQL IMMUTABLE PARALLEL SAFE
AS $$
    SELECT (
        '[' || string_agg((((n * 31 + i * 17) % 101) - 50)::text, ',' ORDER BY i) || ']'
    )::vector
    FROM generate_series(1, d) i
$$;

CREATE FUNCTION filtered_scan_segment(query_text text)
RETURNS jsonb
LANGUAGE plpgsql
AS $$
DECLARE
    plan jsonb;
BEGIN
    EXECUTE 'EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, TIMING OFF, BUFFERS ON, SUMMARY OFF, FORMAT JSON) '
        || query_text
        INTO plan;
    RETURN (jsonb_path_query_first(plan, '$.**."Segment Info"') #>> '{}')::jsonb;
END
$$;

-- The segment's access path, its located matches, and whether its located-path cap is positive.
CREATE FUNCTION filtered_scan_path(query_text text)
RETURNS TABLE (access_path text, located_matches bigint, positive_cap boolean)
LANGUAGE SQL
AS $$
    SELECT
        jsonb_path_query_first(value, '$.**.access_path') #>> '{}',
        (jsonb_path_query_first(value, '$.**.located_matches') #>> '{}')::bigint,
        (jsonb_path_query_first(value, '$.**.direct_cap') #>> '{}')::bigint > 0
    FROM filtered_scan_segment(query_text) AS value
$$;

-- The exact-plan and plan-mix stats are reported, and every layer-0 cluster is read one way.
CREATE FUNCTION filtered_scan_plan(query_text text)
RETURNS TABLE (exact_stats_reported boolean, plan_mix_reported boolean, every_cluster_planned_once boolean)
LANGUAGE SQL
AS $$
    SELECT
        jsonb_typeof(jsonb_path_query_first(value, '$.**.exact_reads')) = 'number'
            AND jsonb_typeof(jsonb_path_query_first(value, '$.**.layer0_exact_rows')) = 'number',
        jsonb_typeof(jsonb_path_query_first(value, '$.**.layer0_exact_clusters')) = 'number'
            AND jsonb_typeof(jsonb_path_query_first(value, '$.**.layer0_sparse_clusters')) = 'number'
            AND jsonb_typeof(jsonb_path_query_first(value, '$.**.layer0_full_clusters')) = 'number',
        (jsonb_path_query_first(value, '$.**.layer0_exact_clusters') #>> '{}')::bigint
            + (jsonb_path_query_first(value, '$.**.layer0_sparse_clusters') #>> '{}')::bigint
            + (jsonb_path_query_first(value, '$.**.layer0_full_clusters') #>> '{}')::bigint
            = (jsonb_path_query_first(value, '$.**.postings_row') #>> '{}')::bigint
    FROM filtered_scan_segment(query_text) AS value
$$;

CREATE TABLE filtered_scan_items (id integer PRIMARY KEY, category integer, vec vector(768));
CREATE INDEX filtered_scan_items_idx ON filtered_scan_items
USING paradedb (id, category, vec vector_l2_ops)
WITH (
    max_leaf_size = 5,
    training_sample_ratio = 1.0,
    target_segment_count = 1,
    mutable_segment_rows = 0,
    layer_sizes = '400kb',
    background_layer_sizes = '0'
);
INSERT INTO filtered_scan_items
SELECT g, g % 50, filtered_scan_vector(768, g) FROM generate_series(1, 100) g;
INSERT INTO filtered_scan_items
SELECT g, g % 50, filtered_scan_vector(768, g) FROM generate_series(101, 200) g;
VACUUM filtered_scan_items;

-- The same rows without a paradedb index: Postgres scans and sorts them exhaustively.
CREATE TABLE filtered_scan_oracle AS SELECT * FROM filtered_scan_items;

-- Twelve of two hundred documents match: the segment locates them.
SELECT * FROM filtered_scan_path(
    'SELECT id FROM filtered_scan_items '
    'WHERE category @@@ paradedb.range(''category'', int4range(0, 3, ''[)'')) '
    'ORDER BY vec <-> filtered_scan_vector(768, 7), id LIMIT 5'
);
SELECT * FROM filtered_scan_plan(
    'SELECT id FROM filtered_scan_items '
    'WHERE category @@@ paradedb.range(''category'', int4range(0, 3, ''[)'')) '
    'ORDER BY vec <-> filtered_scan_vector(768, 7), id LIMIT 5'
);

-- The located top-k is the exhaustive top-k.
SELECT id FROM filtered_scan_items
WHERE category @@@ paradedb.range('category', int4range(0, 3, '[)'))
ORDER BY vec <-> filtered_scan_vector(768, 7), id
LIMIT 5;
SELECT id FROM filtered_scan_oracle
WHERE category >= 0 AND category < 3
ORDER BY vec <-> filtered_scan_vector(768, 7), id
LIMIT 5;
SELECT
    ARRAY(
        SELECT id FROM filtered_scan_items
        WHERE category @@@ paradedb.range('category', int4range(0, 3, '[)'))
        ORDER BY vec <-> filtered_scan_vector(768, 7), id
        LIMIT 5
    ) = ARRAY(
        SELECT id FROM filtered_scan_oracle
        WHERE category >= 0 AND category < 3
        ORDER BY vec <-> filtered_scan_vector(768, 7), id
        LIMIT 5
    ) AS located_matches_exhaustive;

-- A dense filter (196 of 200) matches more documents than the probe budget buys: it routes.
SELECT * FROM filtered_scan_path(
    'SELECT id FROM filtered_scan_items '
    'WHERE category @@@ paradedb.range(''category'', int4range(0, 49, ''[)'')) '
    'ORDER BY vec <-> filtered_scan_vector(768, 7), id LIMIT 5'
);
SELECT * FROM filtered_scan_plan(
    'SELECT id FROM filtered_scan_items '
    'WHERE category @@@ paradedb.range(''category'', int4range(0, 49, ''[)'')) '
    'ORDER BY vec <-> filtered_scan_vector(768, 7), id LIMIT 5'
);

-- Unfiltered queries always route.
SELECT * FROM filtered_scan_path(
    'SELECT id FROM filtered_scan_items WHERE id @@@ pdb.all() '
    'ORDER BY vec <-> filtered_scan_vector(768, 7), id LIMIT 5'
);
SELECT * FROM filtered_scan_plan(
    'SELECT id FROM filtered_scan_items WHERE id @@@ pdb.all() '
    'ORDER BY vec <-> filtered_scan_vector(768, 7), id LIMIT 5'
);

DROP TABLE filtered_scan_oracle;
DROP TABLE filtered_scan_items;
DROP FUNCTION filtered_scan_plan(text);
DROP FUNCTION filtered_scan_path(text);
DROP FUNCTION filtered_scan_segment(text);
DROP FUNCTION filtered_scan_vector(integer, integer);
RESET paradedb.vector_stats;
RESET paradedb.vector_clustering_threshold;
