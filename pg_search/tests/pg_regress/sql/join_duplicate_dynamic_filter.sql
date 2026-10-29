CREATE EXTENSION IF NOT EXISTS pg_search;
SET max_parallel_workers_per_gather = 0;
SET paradedb.enable_rank_join = off;
SET paradedb.enable_aggregate_custom_scan = off;
SET join_collapse_limit = 1;

CREATE TABLE j6953_dyn_users (id bigint PRIMARY KEY, group_id bigint);
CREATE TABLE j6953_dyn_products (id bigint PRIMARY KEY, group_id bigint);
CREATE TABLE j6953_dyn_orders (id bigint PRIMARY KEY, group_id bigint, maxproduct bigint, maxuser bigint);
INSERT INTO j6953_dyn_users SELECT i, 1 FROM generate_series(12000, 1, -1) i;
INSERT INTO j6953_dyn_products VALUES (1, 1), (2, 1);
INSERT INTO j6953_dyn_orders VALUES (11, 1, 2, 12000);
CREATE INDEX ON j6953_dyn_users USING paradedb (id, group_id);
CREATE INDEX ON j6953_dyn_products USING paradedb (id, group_id);
CREATE INDEX ON j6953_dyn_orders USING paradedb (id, group_id, maxproduct, maxuser);

EXPLAIN (VERBOSE, COSTS OFF)
SELECT p.id, u.id, o.id
FROM (j6953_dyn_users u JOIN j6953_dyn_products p ON p.group_id = u.group_id)
JOIN j6953_dyn_orders o ON o.group_id = p.group_id
WHERE o.id @@@ pdb.all()
ORDER BY p.id, u.id LIMIT 2;

SELECT p.id, u.id, o.id
FROM (j6953_dyn_users u JOIN j6953_dyn_products p ON p.group_id = u.group_id)
JOIN j6953_dyn_orders o ON o.group_id = p.group_id
WHERE o.id @@@ pdb.all()
ORDER BY p.id, u.id LIMIT 2;

-- the top-k filter must use products.id, not the duplicate users.id.
EXPLAIN (COSTS OFF)
SELECT p.id AS product_id, u.id AS user_id
FROM (j6953_dyn_users u JOIN j6953_dyn_products p ON p.group_id = u.group_id)
JOIN j6953_dyn_orders o ON o.group_id = p.group_id
    AND o.maxproduct >= p.id AND o.maxuser >= u.id
WHERE o.id @@@ pdb.all()
ORDER BY p.id LIMIT 10000;

CREATE TEMP TABLE j6953_dynamic_scan AS
SELECT p.id AS product_id, u.id AS user_id
FROM (j6953_dyn_users u JOIN j6953_dyn_products p ON p.group_id = u.group_id)
JOIN j6953_dyn_orders o ON o.group_id = p.group_id
    AND o.maxproduct >= p.id AND o.maxuser >= u.id
WHERE o.id @@@ pdb.all()
ORDER BY p.id LIMIT 10000;
SELECT count(*) AS total, count(*) FILTER (WHERE product_id = 1) AS product_one
FROM j6953_dynamic_scan;

SET paradedb.enable_join_custom_scan = off;
SELECT p.id, u.id, o.id
FROM (j6953_dyn_users u JOIN j6953_dyn_products p ON p.group_id = u.group_id)
JOIN j6953_dyn_orders o ON o.group_id = p.group_id
WHERE o.id @@@ pdb.all()
ORDER BY p.id, u.id LIMIT 2;

CREATE TEMP TABLE j6953_dynamic_native AS
SELECT p.id AS product_id, u.id AS user_id
FROM (j6953_dyn_users u JOIN j6953_dyn_products p ON p.group_id = u.group_id)
JOIN j6953_dyn_orders o ON o.group_id = p.group_id
    AND o.maxproduct >= p.id AND o.maxuser >= u.id
WHERE o.id @@@ pdb.all()
ORDER BY p.id LIMIT 10000;
SELECT count(*) AS total, count(*) FILTER (WHERE product_id = 1) AS product_one
FROM j6953_dynamic_native;
DROP TABLE j6953_dynamic_scan, j6953_dynamic_native;

DROP TABLE j6953_dyn_orders CASCADE;
DROP TABLE j6953_dyn_products CASCADE;
DROP TABLE j6953_dyn_users CASCADE;
RESET paradedb.enable_join_custom_scan;
RESET paradedb.enable_aggregate_custom_scan;
RESET paradedb.enable_rank_join;
RESET join_collapse_limit;
RESET max_parallel_workers_per_gather;
