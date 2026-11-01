-- A heap filter on `ctid` must see the fetched tuple's CTID (#7037).
-- tests/tests/heap_filter_system_columns.rs compares the rows with PostgreSQL.
-- This file shows the plans and the rows of the query from the issue.
\i common/common_setup.sql
SET paradedb.enable_join_custom_scan = on;

CREATE TABLE heap_filter_a (id int PRIMARY KEY, val text, note text);
CREATE TABLE heap_filter_b (id int PRIMARY KEY, a_id int, val text);
INSERT INTO heap_filter_a SELECT i, 'val ' || i, 'n' FROM generate_series(1, 5) i;
-- In insertion order, b.id 14 is at (0,1), 13 at (0,2), 12 at (0,3) and 11 at (0,4).
INSERT INTO heap_filter_b VALUES (14, 99, 'val 14'), (13, 3, 'val 13'), (12, 2, 'val 12'), (11, 1, 'val 11');
CREATE INDEX heap_filter_a_idx ON heap_filter_a USING paradedb (id, val);
CREATE INDEX heap_filter_b_idx ON heap_filter_b USING paradedb (id, a_id, val);
ANALYZE heap_filter_a;
ANALYZE heap_filter_b;

-- Join Scan must leave out b.id 13 at (0,2).
EXPLAIN (COSTS OFF)
SELECT a.id, b.id AS bid, b.ctid AS b_ctid
FROM heap_filter_a a JOIN heap_filter_b b ON a.id = b.a_id
WHERE a.val ||| 'val' AND b.ctid > '(0,2)'::tid
ORDER BY a.id LIMIT 10;

SELECT a.id, b.id AS bid, b.ctid AS b_ctid
FROM heap_filter_a a JOIN heap_filter_b b ON a.id = b.a_id
WHERE a.val ||| 'val' AND b.ctid > '(0,2)'::tid
ORDER BY a.id LIMIT 10;

-- Base Scan uses the same heap filter.
EXPLAIN (COSTS OFF)
SELECT id, ctid FROM heap_filter_b
WHERE val ||| 'val' AND ctid > '(0,2)'::tid
ORDER BY id;

SELECT id, ctid FROM heap_filter_b
WHERE val ||| 'val' AND ctid > '(0,2)'::tid
ORDER BY id;

DROP TABLE heap_filter_a, heap_filter_b;
RESET paradedb.enable_join_custom_scan;
