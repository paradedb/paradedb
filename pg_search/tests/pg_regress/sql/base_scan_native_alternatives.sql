\i common/common_setup.sql

-- Full fallback estimates must not discard cheaper or already-ordered native
-- paths. Keep several segments so both serial and CostedBoth policies run.
SET paradedb.global_mutable_segment_rows = 0;
SET paradedb.min_rows_per_worker = 0;
SET paradedb.planner_warnings = 'off';
SET jit = off;
SET enable_indexscan = on;
SET enable_indexonlyscan = on;

CREATE TABLE native_limit (id INT PRIMARY KEY, body TEXT, padding TEXT);
ALTER TABLE native_limit ALTER COLUMN padding SET STORAGE PLAIN;
CREATE INDEX native_limit_search ON native_limit USING paradedb (id, body)
WITH (layer_sizes = '0', background_layer_sizes = '0');
INSERT INTO native_limit SELECT i, 'common', repeat(md5(i::text), 32)
FROM generate_series(1, 1000) i;
INSERT INTO native_limit SELECT i, 'common', repeat(md5(i::text), 32)
FROM generate_series(1001, 2000) i;
INSERT INTO native_limit SELECT i, 'common', repeat(md5(i::text), 32)
FROM generate_series(2001, 3000) i;
INSERT INTO native_limit SELECT i, 'common', repeat(md5(i::text), 32)
FROM generate_series(3001, 4000) i;
VACUUM ANALYZE native_limit;

CREATE FUNCTION pg_temp.native_limit_plan(query TEXT) RETURNS JSONB
LANGUAGE plpgsql AS $$
DECLARE
    plan JSONB;
BEGIN
    EXECUTE 'EXPLAIN (FORMAT JSON) ' || query INTO plan;
    RETURN plan;
END
$$;

-- The covering ParadeDB index can return ids without fetching wide heap rows.
SELECT jsonb_path_exists(plan, '$.** ? (@."Node Type" == "Index Only Scan")')
       AS native_index_only
FROM (SELECT pg_temp.native_limit_plan($q$
    SELECT id FROM native_limit WHERE body ||| 'common' ORDER BY sqrt(id::float8) LIMIT 10
$q$) AS plan) p;

CREATE INDEX native_limit_order ON native_limit ((sqrt(id::float8)));

-- Scale operator cost to isolate path preservation from the search operator's
-- large native-filter cost. Disabling Sort then favors the ordered index on all
-- supported PostgreSQL versions, without relying on PG18 disabled-node ranking.
SET cpu_operator_cost = 0;
SET enable_sort = off;
SELECT jsonb_path_exists(plan, '$.** ? (@."Index Name" == "native_limit_order")')
       AS ordered_native_index,
       NOT jsonb_path_exists(plan, '$.** ? (@."Node Type" == "Sort")') AS avoids_sort
FROM (SELECT pg_temp.native_limit_plan($q$
    SELECT id FROM native_limit WHERE body ||| 'common' ORDER BY sqrt(id::float8) LIMIT 10
$q$) AS plan) p;

SET max_parallel_workers_per_gather = 2;
SET parallel_setup_cost = 0;
SET parallel_tuple_cost = 0;
SELECT jsonb_path_exists(plan, '$.** ? (@."Index Name" == "native_limit_order")')
       AS ordered_native_index,
       NOT jsonb_path_exists(plan, '$.** ? (@."Node Type" == "Sort")') AS avoids_sort
FROM (SELECT pg_temp.native_limit_plan($q$
    SELECT id FROM native_limit WHERE body ||| 'common' ORDER BY sqrt(id::float8) LIMIT 10
$q$) AS plan) p;

-- A generic predicate cannot provide a traversal cost, so it follows the worker
-- heuristic rather than CostedBoth. That path must retain native ordering too.
SET plan_cache_mode = force_generic_plan;
PREPARE native_limit_runtime(text) AS
    SELECT id FROM native_limit WHERE body ||| $1 AND id > 0
    ORDER BY sqrt(id::float8) LIMIT 10;
SELECT jsonb_path_exists(plan, '$.** ? (@."Index Name" == "native_limit_order")')
       AS ordered_native_index,
       NOT jsonb_path_exists(plan, '$.** ? (@."Node Type" == "Sort")') AS avoids_sort
FROM (SELECT pg_temp.native_limit_plan('EXECUTE native_limit_runtime(''common'')') AS plan) p;
PREPARE native_limit_runtime_score(text) AS
    SELECT id, pdb.score(id) FROM native_limit WHERE body ||| $1 AND id > 0
    ORDER BY sqrt(id::float8) LIMIT 10;
SELECT jsonb_path_exists(plan,
       '$.** ? (@."Custom Plan Provider" == "ParadeDB Base Scan")') AS custom_for_runtime_score
FROM (SELECT pg_temp.native_limit_plan('EXECUTE native_limit_runtime_score(''common'')') AS plan) p;
DEALLOCATE native_limit_runtime;
DEALLOCATE native_limit_runtime_score;
RESET plan_cache_mode;

SELECT array_agg(id ORDER BY id) AS ids FROM (
    SELECT id FROM native_limit WHERE body ||| 'common' ORDER BY sqrt(id::float8) LIMIT 10
) q;

-- A score projection still requires custom execution even when the ordered
-- native index is otherwise preferable.
SELECT jsonb_path_exists(plan,
       '$.** ? (@."Custom Plan Provider" == "ParadeDB Base Scan")') AS custom_for_score
FROM (SELECT pg_temp.native_limit_plan($q$
    SELECT id, pdb.score(id) FROM native_limit WHERE body ||| 'common'
    ORDER BY sqrt(id::float8) LIMIT 10
$q$) AS plan) p;
SELECT count(*) = 10 AND bool_and(score > 0) AS valid_scores FROM (
    SELECT id, pdb.score(id) AS score FROM native_limit WHERE body ||| 'common'
    ORDER BY sqrt(id::float8) LIMIT 10
) q;

SELECT jsonb_path_exists(plan,
       '$.** ? (@."Custom Plan Provider" == "ParadeDB Base Scan")') AS custom_for_snippet
FROM (SELECT pg_temp.native_limit_plan($q$
    SELECT id, pdb.snippet(body) FROM native_limit WHERE body ||| 'common'
    ORDER BY sqrt(id::float8) LIMIT 10
$q$) AS plan) p;
SELECT count(*) = 10 AND bool_and(snippet = '<b>common</b>') AS valid_snippets FROM (
    SELECT id, pdb.snippet(body) AS snippet FROM native_limit WHERE body ||| 'common'
    ORDER BY sqrt(id::float8) LIMIT 10
) q;

DROP TABLE native_limit;
RESET paradedb.global_mutable_segment_rows;
RESET paradedb.min_rows_per_worker;
RESET paradedb.planner_warnings;
RESET jit;
RESET enable_indexscan;
RESET enable_indexonlyscan;
RESET cpu_operator_cost;
RESET enable_sort;
RESET max_parallel_workers_per_gather;
RESET parallel_setup_cost;
RESET parallel_tuple_cost;
\i common/common_cleanup.sql
