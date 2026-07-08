-- Regression test for parallel 3-way DISTINCT JoinScan absorption.
--
-- Under parallel execution, the cheapest_total_path of an intermediate join
-- rel can be wrapped as `Gather Merge -> Sort -> Hash Join`. The 3-way
-- JoinScan path reconstruction peels the Gather/GatherMerge wrappers but used
-- to stop at the intervening `Sort`, so `is_join_path` returned false, the
-- sub-join failed to reconstruct, and the third relation was never absorbed.
-- JoinScan then declined the whole join -- surfacing (misleadingly) as
-- "DISTINCT columns must be fast fields" -- and the query fell back to a
-- native plan.
--
-- The fix peels `SortPath` too (see `unwrap_path_wrappers`). This test guards
-- that a parallel 3-way DISTINCT search join runs as a ParadeDB Join Scan and
-- returns correct results. Without the fix the EXPLAIN below shows a native
-- Hash/Nested Loop plan instead of `Parallel Custom Scan (ParadeDB Join Scan)`.

CREATE EXTENSION IF NOT EXISTS pg_search;

SET client_min_messages = warning;

DROP TABLE IF EXISTS js_parallel_distinct_users CASCADE;
DROP TABLE IF EXISTS js_parallel_distinct_products CASCADE;
DROP TABLE IF EXISTS js_parallel_distinct_orders CASCADE;

CREATE TABLE js_parallel_distinct_users (
    id SERIAL8 NOT NULL PRIMARY KEY,
    uuid UUID,
    name TEXT,
    age INTEGER
);

CREATE TABLE js_parallel_distinct_products (
    id SERIAL8 NOT NULL PRIMARY KEY,
    uuid UUID,
    name TEXT,
    age INTEGER
);

CREATE TABLE js_parallel_distinct_orders (
    id SERIAL8 NOT NULL PRIMARY KEY,
    uuid UUID,
    name TEXT,
    age INTEGER
);

CREATE INDEX js_parallel_distinct_users_idx
ON js_parallel_distinct_users
USING bm25 (id, uuid, name, age)
WITH (
    key_field = 'id',
    text_fields = '{
        "uuid": { "tokenizer": { "type": "keyword" }, "fast": true },
        "name": { "tokenizer": { "type": "keyword" }, "fast": true }
    }',
    numeric_fields = '{ "age": { "fast": true } }',
    target_segment_count = 2
);

CREATE INDEX js_parallel_distinct_products_idx
ON js_parallel_distinct_products
USING bm25 (id, uuid, name, age)
WITH (
    key_field = 'id',
    text_fields = '{
        "uuid": { "tokenizer": { "type": "keyword" }, "fast": true },
        "name": { "tokenizer": { "type": "keyword" }, "fast": true }
    }',
    numeric_fields = '{ "age": { "fast": true } }',
    target_segment_count = 2
);

CREATE INDEX js_parallel_distinct_orders_idx
ON js_parallel_distinct_orders
USING bm25 (id, uuid, name, age)
WITH (
    key_field = 'id',
    text_fields = '{
        "uuid": { "tokenizer": { "type": "keyword" }, "fast": true },
        "name": { "tokenizer": { "type": "keyword" }, "fast": true }
    }',
    numeric_fields = '{ "age": { "fast": true } }',
    target_segment_count = 2
);

INSERT INTO js_parallel_distinct_users (uuid, name, age)
SELECT
    '550e8400-e29b-41d4-a716-446655440000'::uuid,
    CASE WHEN i % 2 = 0 THEN 'bob' ELSE 'alice' END,
    i
FROM generate_series(1, 100) AS g(i);

INSERT INTO js_parallel_distinct_products (uuid, name, age)
SELECT
    '550e8400-e29b-41d4-a716-446655440000'::uuid,
    CASE WHEN i % 2 = 0 THEN 'bob' ELSE 'alice' END,
    i
FROM generate_series(1, 100) AS g(i);

INSERT INTO js_parallel_distinct_orders (uuid, name, age)
SELECT
    '550e8400-e29b-41d4-a716-446655440000'::uuid,
    CASE WHEN i % 2 = 0 THEN 'bob' ELSE 'alice' END,
    i
FROM generate_series(1, 100) AS g(i);

ANALYZE js_parallel_distinct_users;
ANALYZE js_parallel_distinct_products;
ANALYZE js_parallel_distinct_orders;

-- Force a parallel plan so the intermediate {users, products} sub-join is
-- reconstructed from a `Gather Merge -> Sort -> Hash Join` cheapest_total_path.
SET paradedb.enable_join_custom_scan = true;
SET enable_seqscan = true;
SET enable_indexscan = false;
SET max_parallel_workers = 8;
SET max_parallel_workers_per_gather = 2;
SET parallel_leader_participation = false;
SET debug_parallel_query = on;

-- The plan must be a Parallel Custom Scan (ParadeDB Join Scan); without the fix
-- this is a native Hash/Nested Loop plan and a "JoinScan not used" warning.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT DISTINCT u.id, u.name, p.id, o.id
FROM js_parallel_distinct_users u
JOIN js_parallel_distinct_products p ON u.id = p.id
JOIN js_parallel_distinct_orders o ON p.age = o.age
WHERE u.name @@@ 'bob'
  AND p.name @@@ 'bob'
ORDER BY u.id, p.id, o.id
LIMIT 48;

-- And it must return the correct rows (DISTINCT + LIMIT).
SELECT count(*) AS distinct_limited_rows
FROM (
    SELECT DISTINCT u.id, u.name, p.id, o.id
    FROM js_parallel_distinct_users u
    JOIN js_parallel_distinct_products p ON u.id = p.id
    JOIN js_parallel_distinct_orders o ON p.age = o.age
    WHERE u.name @@@ 'bob'
      AND p.name @@@ 'bob'
    ORDER BY u.id, p.id, o.id
    LIMIT 48
) q;
