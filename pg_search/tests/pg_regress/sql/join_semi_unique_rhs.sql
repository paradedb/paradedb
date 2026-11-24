-- A semijoin whose right-hand side is a join can be planned as an inner join over a
-- unique-ified right-hand side. JoinScan must still read the join predicates below that node.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET max_parallel_workers_per_gather = 0;

DROP TABLE IF EXISTS semi_uniq_p, semi_uniq_c, semi_uniq_d CASCADE;
CREATE TABLE semi_uniq_p (id int PRIMARY KEY, company_id int NOT NULL, description text);
CREATE TABLE semi_uniq_c (id int PRIMARY KEY, name text);
CREATE TABLE semi_uniq_d (id int PRIMARY KEY, company_id int NOT NULL, name text);

INSERT INTO semi_uniq_p
SELECT i, i % 500, CASE WHEN i % 3 = 0 THEN 'widget' ELSE 'gadget' END
FROM generate_series(1, 20000) i;
INSERT INTO semi_uniq_c SELECT i, 'company ' || i FROM generate_series(0, 499) i;
INSERT INTO semi_uniq_d
SELECT i, i % 40, CASE WHEN i % 2 = 0 THEN 'Engineering' ELSE 'Sales' END
FROM generate_series(1, 2000) i;

CREATE INDEX semi_uniq_p_company ON semi_uniq_p (company_id);
CREATE INDEX semi_uniq_p_idx ON semi_uniq_p USING bm25 (id, company_id, description);
CREATE INDEX semi_uniq_c_idx ON semi_uniq_c USING bm25 (id, name);
CREATE INDEX semi_uniq_d_idx ON semi_uniq_d USING bm25 (id, company_id, name);
ANALYZE semi_uniq_p;
ANALYZE semi_uniq_c;
ANALYZE semi_uniq_d;

SET enable_sort = off;
SET enable_incremental_sort = off;
SET enable_hashjoin = off;
SET enable_mergejoin = off;
SET enable_memoize = off;
SET enable_material = off;

-- The plan Postgres picks: a HashAggregate makes the right-hand side unique.
SET paradedb.enable_join_custom_scan = off;
EXPLAIN (COSTS OFF)
SELECT p.id
FROM semi_uniq_p p
WHERE p.company_id IN (
    SELECT c.id
    FROM semi_uniq_c c
    JOIN semi_uniq_d d ON d.company_id = c.id
    WHERE d.name ||| 'Engineering'
)
AND p.description ||| 'widget'
ORDER BY p.company_id, p.id
LIMIT 10;

SELECT p.id
FROM semi_uniq_p p
WHERE p.company_id IN (
    SELECT c.id
    FROM semi_uniq_c c
    JOIN semi_uniq_d d ON d.company_id = c.id
    WHERE d.name ||| 'Engineering'
)
AND p.description ||| 'widget'
ORDER BY p.company_id, p.id
LIMIT 10;

-- JoinScan replaces it and returns the same rows.
SET paradedb.enable_join_custom_scan = on;
EXPLAIN (COSTS OFF)
SELECT p.id
FROM semi_uniq_p p
WHERE p.company_id IN (
    SELECT c.id
    FROM semi_uniq_c c
    JOIN semi_uniq_d d ON d.company_id = c.id
    WHERE d.name ||| 'Engineering'
)
AND p.description ||| 'widget'
ORDER BY p.company_id, p.id
LIMIT 10;

SELECT p.id
FROM semi_uniq_p p
WHERE p.company_id IN (
    SELECT c.id
    FROM semi_uniq_c c
    JOIN semi_uniq_d d ON d.company_id = c.id
    WHERE d.name ||| 'Engineering'
)
AND p.description ||| 'widget'
ORDER BY p.company_id, p.id
LIMIT 10;

RESET enable_sort;
RESET enable_incremental_sort;
RESET enable_hashjoin;
RESET enable_mergejoin;
RESET enable_memoize;
RESET enable_material;
RESET max_parallel_workers_per_gather;
DROP TABLE semi_uniq_p, semi_uniq_c, semi_uniq_d CASCADE;
