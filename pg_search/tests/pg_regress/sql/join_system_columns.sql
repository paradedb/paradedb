-- System columns must come from each source's fetched heap tuple (#6630).
-- tests/tests/joinscan_system_columns.rs compares all six system columns with
-- the PostgreSQL join. This file shows the plan and the values that do not
-- depend on transaction IDs or table OIDs.
CREATE EXTENSION IF NOT EXISTS pg_search;
SET max_parallel_workers_per_gather = 0;
SET paradedb.enable_join_custom_scan = on;

CREATE TABLE system_columns_a (id int PRIMARY KEY, next_id int, val text, note text);
CREATE TABLE system_columns_b (id int PRIMARY KEY, a_id int, val text);
INSERT INTO system_columns_a SELECT i, i + 1, 'val ' || i, 'original' FROM generate_series(1, 5) i;
-- The reverse insertion order gives the two sources different CTIDs.
INSERT INTO system_columns_b VALUES (14, 99, 'val 14'), (13, 3, 'val 13'), (12, 2, 'val 12'), (11, 1, 'val 11');
CREATE INDEX system_columns_a_idx ON system_columns_a USING paradedb (id, next_id, val);
CREATE INDEX system_columns_b_idx ON system_columns_b USING paradedb (id, a_id, val);
ANALYZE system_columns_a;
ANALYZE system_columns_b;

-- Each row's CTIDs must be its own tuples' CTIDs, not NULL.
EXPLAIN (COSTS OFF, VERBOSE)
SELECT a.id, b.id AS b_id,
       a.ctid AS a_ctid, a.cmin AS a_cmin, a.xmax AS a_xmax, a.cmax AS a_cmax,
       b.ctid AS b_ctid, b.cmin AS b_cmin, b.xmax AS b_xmax, b.cmax AS b_cmax
FROM system_columns_a a JOIN system_columns_b b ON a.id = b.a_id
WHERE a.val ||| 'val'
ORDER BY a.id LIMIT 3;

SELECT a.id, b.id AS b_id,
       a.ctid AS a_ctid, a.cmin AS a_cmin, a.xmax AS a_xmax, a.cmax AS a_cmax,
       b.ctid AS b_ctid, b.cmin AS b_cmin, b.xmax AS b_xmax, b.cmax AS b_cmax
FROM system_columns_a a JOIN system_columns_b b ON a.id = b.a_id
WHERE a.val ||| 'val'
ORDER BY a.id LIMIT 3;

-- b.id 14 has no match, so its a system columns must be NULL.
EXPLAIN (COSTS OFF, VERBOSE)
SELECT b.id, a.id AS a_id,
       a.ctid AS a_ctid, a.xmax AS a_xmax,
       b.ctid AS b_ctid, b.xmax AS b_xmax
FROM system_columns_b b LEFT JOIN system_columns_a a ON a.id = b.a_id
WHERE b.val ||| 'val'
ORDER BY b.id LIMIT 10;

SELECT b.id, a.id AS a_id,
       a.ctid AS a_ctid, a.xmax AS a_xmax,
       b.ctid AS b_ctid, b.xmax AS b_xmax
FROM system_columns_b b LEFT JOIN system_columns_a a ON a.id = b.a_id
WHERE b.val ||| 'val'
ORDER BY b.id LIMIT 10;

DROP TABLE system_columns_a, system_columns_b;
RESET paradedb.enable_join_custom_scan;
