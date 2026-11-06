-- Filtered parallel scans share leader routing; unfiltered scans remain serial.
SET client_min_messages = WARNING;
CREATE EXTENSION IF NOT EXISTS vector;
\i common/common_setup.sql

SET max_parallel_workers_per_gather = 2;
SET max_parallel_workers = 8;
SET parallel_setup_cost = 0;
SET parallel_tuple_cost = 0;
SET paradedb.min_rows_per_worker = 0;
SET paradedb.vector_clustering_threshold = 64;
SET paradedb.vector_cluster_max_probe = 1.0;
SET paradedb.vector_stats = on;

CREATE FUNCTION parallel_router_vector(n integer) RETURNS vector
LANGUAGE SQL IMMUTABLE PARALLEL SAFE AS $$
    SELECT ('[' || string_agg((((n * 31 + i * 17) % 101) - 50)::text, ',' ORDER BY i) || ']')::vector
    FROM generate_series(1, 64) i
$$;

CREATE TABLE parallel_router_items (id integer PRIMARY KEY, category text, vec vector(64));
CREATE INDEX parallel_router_idx ON parallel_router_items
USING paradedb (id, (category::pdb.literal), vec vector_l2_ops)
WITH (max_leaf_size = 16, training_sample_ratio = 1.0, mutable_segment_rows = 0,
      target_segment_count = 1, layer_sizes = '100kb', background_layer_sizes = '0');
DO $$
BEGIN
    FOR batch_start IN 1..2048 BY 256 LOOP
        INSERT INTO parallel_router_items
        SELECT g, CASE WHEN g <= 1024 THEN 'keep' ELSE 'other' END, parallel_router_vector(g)
        FROM generate_series(batch_start, batch_start + 255) g;
    END LOOP;
END
$$;
DELETE FROM parallel_router_items WHERE id % 7 = 0;
VACUUM ANALYZE parallel_router_items;
CREATE TABLE parallel_router_oracle AS SELECT * FROM parallel_router_items;
SELECT count(*) > 1 AND bool_and(vector_format = 'ivf') AS multiple_clustered_segments
FROM paradedb.vector_info('parallel_router_idx', 'vec');

CREATE FUNCTION parallel_router_explain(query_text text) RETURNS jsonb
LANGUAGE plpgsql AS $$
DECLARE plan jsonb;
BEGIN
    EXECUTE 'EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, TIMING OFF, SUMMARY OFF, FORMAT JSON) ' || query_text INTO plan;
    RETURN plan;
END
$$;

SELECT jsonb_path_exists(plan, '$.** ? (@."Workers Launched" > 0)') AS workers_launched,
       jsonb_path_exists(plan, '$.** ? (@."Vector Routing" == "Leader")') AS leader_routed,
       EXISTS (SELECT 1 FROM jsonb_path_query(plan, '$.**."Segment Info"') info
               WHERE jsonb_path_exists((info #>> '{}')::jsonb,
                     '$.** ? (@.access_path == "Routed" && @.postings_row > 0)')) AS routed_clusters_probed,
       EXISTS (SELECT 1 FROM jsonb_path_query(plan, '$.**."Tantivy Query"') q WHERE q::text LIKE '%category%') AS filter_pushed_down,
       NOT jsonb_path_exists(plan, '$.** ? (exists(@.Filter))') AS no_heap_filter
FROM parallel_router_explain(
    'SELECT id FROM parallel_router_items WHERE id @@@ pdb.all() AND category = ''keep'' ORDER BY vec <-> parallel_router_vector(7), id LIMIT 7 OFFSET 3'
) plan;

SELECT NOT jsonb_path_exists(plan, '$.** ? (@."Workers Launched" > 0)') AS unfiltered_serial,
       NOT jsonb_path_exists(plan, '$.** ? (@."Vector Routing" == "Leader")') AS no_shared_routing
FROM parallel_router_explain(
    'SELECT id FROM parallel_router_items WHERE id @@@ pdb.all() ORDER BY vec <-> parallel_router_vector(7), id LIMIT 7'
) plan;

SELECT ARRAY(SELECT id FROM parallel_router_items WHERE id @@@ pdb.all() AND category = 'keep' AND id <= 20 ORDER BY vec <-> parallel_router_vector(7), id LIMIT 7)
     = ARRAY(SELECT id FROM parallel_router_oracle WHERE category = 'keep' AND id <= 20 ORDER BY vec <-> parallel_router_vector(7), id LIMIT 7) AS located_matches_oracle;

SET plan_cache_mode = force_generic_plan;
PREPARE parallel_router_check(vector, text) AS
SELECT ARRAY(SELECT id FROM parallel_router_items WHERE id @@@ pdb.all() AND category = $2 ORDER BY vec <-> $1, id LIMIT 7 OFFSET 3)
     = ARRAY(SELECT id FROM parallel_router_oracle WHERE category = $2 ORDER BY vec <-> $1, id LIMIT 7 OFFSET 3) AS matches_oracle;
EXECUTE parallel_router_check(parallel_router_vector(7), 'keep');
EXECUTE parallel_router_check(parallel_router_vector(53), 'other');
SET parallel_leader_participation = off;
EXECUTE parallel_router_check(parallel_router_vector(19), 'keep');
SET max_parallel_workers = 0;
EXECUTE parallel_router_check(parallel_router_vector(41), 'other');
RESET max_parallel_workers;
RESET parallel_leader_participation;

SET enable_material = off;
SET enable_hashjoin = off;
SET enable_mergejoin = off;
PREPARE parallel_router_rescan AS
SELECT count(*) = 21 AS rescanned_results
FROM generate_series(1, 3) g
LEFT JOIN LATERAL (
    SELECT id FROM parallel_router_items WHERE id @@@ pdb.all() AND category = 'keep' ORDER BY vec <-> parallel_router_vector(7), id LIMIT 7
) nearest ON nearest.id > g;
EXECUTE parallel_router_rescan;

SET parallel_leader_participation = off;
SELECT jsonb_path_exists(plan, '$.** ? (@."Node Type" == "Gather Merge" && @."Actual Loops" > 1)') AS nonparticipating_parallel_rescan
FROM parallel_router_explain(
    'EXECUTE parallel_router_rescan'
) plan;
RESET parallel_leader_participation;

DEALLOCATE parallel_router_check;
DEALLOCATE parallel_router_rescan;
DROP FUNCTION parallel_router_explain(text);
DROP TABLE parallel_router_items, parallel_router_oracle;
DROP FUNCTION parallel_router_vector(integer);
RESET plan_cache_mode;
RESET enable_material;
RESET enable_hashjoin;
RESET enable_mergejoin;
RESET parallel_setup_cost;
RESET parallel_tuple_cost;
RESET paradedb.min_rows_per_worker;
RESET paradedb.vector_clustering_threshold;
RESET paradedb.vector_cluster_max_probe;
RESET paradedb.vector_stats;
\i common/common_cleanup.sql
