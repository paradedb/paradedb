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

CREATE TABLE parallel_router_items (id integer PRIMARY KEY, vec vector(64));
CREATE INDEX parallel_router_idx ON parallel_router_items
USING paradedb (id, vec vector_l2_ops)
WITH (max_leaf_size = 16, training_sample_ratio = 1.0, mutable_segment_rows = 0,
      target_segment_count = 4, layer_sizes = '0', background_layer_sizes = '0');
INSERT INTO parallel_router_items SELECT g, parallel_router_vector(g) FROM generate_series(1, 512) g;
INSERT INTO parallel_router_items SELECT g, parallel_router_vector(g) FROM generate_series(513, 1024) g;
INSERT INTO parallel_router_items SELECT g, parallel_router_vector(g) FROM generate_series(1025, 1536) g;
INSERT INTO parallel_router_items SELECT g, parallel_router_vector(g) FROM generate_series(1537, 2048) g;
DELETE FROM parallel_router_items WHERE id % 7 = 0;
VACUUM ANALYZE parallel_router_items;
CREATE TABLE parallel_router_oracle AS SELECT * FROM parallel_router_items;

CREATE FUNCTION parallel_router_explain(query_text text) RETURNS jsonb
LANGUAGE plpgsql AS $$
DECLARE plan jsonb;
BEGIN
    EXECUTE 'EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, TIMING OFF, SUMMARY OFF, FORMAT JSON) ' || query_text INTO plan;
    RETURN plan;
END
$$;

SELECT jsonb_path_exists(plan, '$.** ? (@."Workers Launched" > 0)') AS workers_launched,
       jsonb_path_exists(plan, '$.** ? (@."Vector Routing" == "Leader")') AS leader_routed
FROM parallel_router_explain(
    'SELECT id FROM parallel_router_items WHERE id @@@ pdb.all() ORDER BY vec <-> parallel_router_vector(7), id LIMIT 7 OFFSET 3'
) plan;

SET plan_cache_mode = force_generic_plan;
PREPARE parallel_router_check(vector) AS
SELECT ARRAY(SELECT id FROM parallel_router_items WHERE id @@@ pdb.all() ORDER BY vec <-> $1, id LIMIT 7 OFFSET 3)
     = ARRAY(SELECT id FROM parallel_router_oracle ORDER BY vec <-> $1, id LIMIT 7 OFFSET 3) AS matches_oracle;
EXECUTE parallel_router_check(parallel_router_vector(7));
EXECUTE parallel_router_check(parallel_router_vector(53));
SET parallel_leader_participation = off;
EXECUTE parallel_router_check(parallel_router_vector(19));
SET max_parallel_workers = 0;
EXECUTE parallel_router_check(parallel_router_vector(41));
RESET max_parallel_workers;
RESET parallel_leader_participation;

SET enable_material = off;
SET enable_hashjoin = off;
SET enable_mergejoin = off;
SELECT count(*) = 21 AS rescanned_results
FROM generate_series(1, 3) g
LEFT JOIN LATERAL (
    SELECT id FROM parallel_router_items WHERE id @@@ pdb.all() ORDER BY vec <-> parallel_router_vector(7), id LIMIT 7
) nearest ON nearest.id > g;

SELECT jsonb_path_exists(plan, '$.** ? (@."Node Type" == "Gather Merge" && @."Actual Loops" > 1)') AS parallel_rescan
FROM parallel_router_explain(
    'SELECT count(*) FROM generate_series(1, 3) g LEFT JOIN LATERAL '
    '(SELECT id FROM parallel_router_items WHERE id @@@ pdb.all() ORDER BY vec <-> parallel_router_vector(7), id LIMIT 7) nearest ON nearest.id > g'
) plan;

DEALLOCATE parallel_router_check;
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
