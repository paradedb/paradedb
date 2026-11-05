-- Tests for Semi and Anti Joins with Join Custom Scan
CREATE EXTENSION IF NOT EXISTS pg_search;
SET max_parallel_workers_per_gather = 0;

DROP TABLE IF EXISTS table_a CASCADE;
CREATE TABLE table_a (
    id bigint NOT NULL PRIMARY KEY,
    category character varying
);

INSERT INTO table_a (id, category)
SELECT
    i,
    CASE WHEN i % 2 = 0 THEN 'target_category' ELSE 'other_category' END
FROM generate_series(1, 2000) as i;

DROP TABLE IF EXISTS table_b CASCADE;
CREATE TABLE table_b (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    group_id character varying,
    a_id bigint,
    category character varying
);

INSERT INTO table_b (group_id, a_id, category)
SELECT
    CASE WHEN i % 10 = 0 THEN 'group_1' ELSE 'group_2' END,
    i,
    CASE WHEN i % 2 = 0 THEN 'target_category' ELSE 'other_category' END
FROM generate_series(1, 2000) as i;

CREATE INDEX table_a_idx ON table_a
USING paradedb (id, (category::pdb.literal_normalized('lowercase=false', 'normalizer=lowercase')));

CREATE INDEX table_b_group_id_idx ON table_b USING btree (group_id);
CREATE INDEX table_b_group_id_a_id_idx ON table_b USING btree (group_id, a_id);

CREATE INDEX table_b_idx ON table_b
USING paradedb (id, (group_id::pdb.literal), a_id, (category::pdb.literal_normalized('lowercase=false', 'normalizer=lowercase')));

-- Query execution
SET paradedb.enable_join_custom_scan TO on;

-- =====================================================================
-- 1. Semi Join Only
-- =====================================================================
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, category
FROM table_a
WHERE id IN (
    SELECT a_id
    FROM table_b
    WHERE group_id IN ('group_1')
)
AND category ### 'target_category'
ORDER BY id ASC
LIMIT 10;

-- =====================================================================
-- 2a. Anti Join (No Pushdown)
-- =====================================================================
-- This does not trigger an anti join because Postgres cannot prove that group_id is not NULL.
-- TODO: This query only triggers set_rel_pathlist_hook and not set_join_pathlist_hook, and so does
-- not render our warning. See https://github.com/paradedb/paradedb/issues/4236 about resolving this.
SELECT id, category
FROM table_a
WHERE id NOT IN (
    SELECT a_id
    FROM table_b
    WHERE group_id IN ('group_3', 'group_4')
)
AND category ### 'target_category'
ORDER BY id ASC
LIMIT 10;

-- =====================================================================
-- 2b. Anti Join (Pushdown)
-- =====================================================================
-- This does trigger an anti join because we use NOT EXISTS
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, category
FROM table_a
WHERE NOT EXISTS (
    SELECT 1
    FROM table_b
    WHERE table_b.a_id = table_a.id
    AND table_b.group_id IN ('group_3', 'group_4')
)
AND table_a.category ### 'target_category'
ORDER BY table_a.id ASC
LIMIT 10;

-- =====================================================================
-- 3. Both Semi and Anti Join
-- =====================================================================
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, category
FROM table_a
WHERE id IN (
    SELECT a_id
    FROM table_b
    WHERE group_id IN ('group_1')
)
AND id NOT IN (
    SELECT a_id
    FROM table_b
    WHERE group_id IN ('group_3', 'group_4')
)
AND category ### 'target_category'
ORDER BY id ASC
LIMIT 10;

SELECT id, category
FROM table_a
WHERE id IN (
    SELECT a_id
    FROM table_b
    WHERE group_id IN ('group_1')
)
AND id NOT IN (
    SELECT a_id
    FROM table_b
    WHERE group_id IN ('group_3', 'group_4')
)
AND category ### 'target_category'
ORDER BY id ASC
LIMIT 10;

-- =====================================================================
-- 4. Tuple Semi Join
-- =====================================================================
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, category
FROM table_a
WHERE (id, category) IN (
    SELECT a_id, category
    FROM table_b
    WHERE group_id IN ('group_1')
)
AND id @@@ pdb.all()
ORDER BY id ASC
LIMIT 10;

SELECT id, category
FROM table_a
WHERE (id, category) IN (
    SELECT a_id, category
    FROM table_b
    WHERE group_id IN ('group_1')
)
AND id @@@ pdb.all()
ORDER BY id ASC
LIMIT 10;

-- =====================================================================
-- 5. SELECT * and ORDER BY varchar column (RelabelType wrapper reproduction)
-- =====================================================================
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT *
FROM table_a
WHERE id IN (
    SELECT a_id
    FROM table_b
    WHERE group_id IN ('group_1')
)
AND category ### 'target_category'
ORDER BY category ASC, id ASC
LIMIT 10;

SELECT *
FROM table_a
WHERE id IN (
    SELECT a_id
    FROM table_b
    WHERE group_id IN ('group_1')
)
AND category ### 'target_category'
ORDER BY category ASC, id ASC
LIMIT 10;

-- =====================================================================
-- 6. Semi Join with selective outer filter
-- =====================================================================
-- The outer filter is so selective that this query requires our ability to force the left side
-- to be partitioned (even when that might be less efficient).
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, category
FROM table_a
WHERE id IN (
    SELECT a_id
    FROM table_b
)
AND id @@@ pdb.all() AND id = 1
ORDER BY id ASC
LIMIT 10;

SELECT id, category
FROM table_a
WHERE id IN (
    SELECT a_id
    FROM table_b
)
AND id @@@ pdb.all() AND id = 1
ORDER BY id ASC
LIMIT 10;

-- =====================================================================
-- 7. Window aggregates over a non-columnar column, with a NOT IN subquery
-- =====================================================================
-- table_a.extra is not indexed. It is attno 3, the same attno as the
-- indexed table_b.a_id, and table_b is rti 1 inside the lifted NOT IN
-- subquery, just like table_a in the outer query. Resolving a window
-- aggregate's column by (rti, attno) across every join source, including
-- the pruned subquery side, used to find table_b's fast field: JoinScan
-- accepted these queries, then returned COUNT 0 / SUM NULL for 7a and
-- panicked for 7b. JoinScan must decline, and results must match Postgres.
ALTER TABLE table_a ADD COLUMN extra int DEFAULT 3;

DROP TABLE IF EXISTS table_c CASCADE;
CREATE TABLE table_c (
    id bigint NOT NULL PRIMARY KEY,
    a_id bigint
);
INSERT INTO table_c (id, a_id) SELECT i, i FROM generate_series(1, 2000) AS i;
CREATE INDEX table_c_idx ON table_c USING paradedb (id, a_id);
ANALYZE table_c;

-- Ground truth from Postgres.
SET paradedb.enable_join_custom_scan TO off;
SELECT a.id, COUNT(a.extra) OVER () AS cnt, SUM(a.extra) OVER () AS total
FROM table_a a
JOIN table_c c ON a.id = c.a_id
WHERE a.id NOT IN (
    SELECT a_id
    FROM table_b
    WHERE group_id IN ('group_3', 'group_4')
)
AND a.category ### 'target_category'
ORDER BY a.id ASC
LIMIT 10;

SELECT a.id, COUNT(*) FILTER (WHERE a.extra > 2) OVER () AS cnt
FROM table_a a
JOIN table_c c ON a.id = c.a_id
WHERE a.id NOT IN (
    SELECT a_id
    FROM table_b
    WHERE group_id IN ('group_3', 'group_4')
)
AND a.category ### 'target_category'
ORDER BY a.id ASC
LIMIT 10;
SET paradedb.enable_join_custom_scan TO on;

-- 7a. Window aggregate argument.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT a.id, COUNT(a.extra) OVER () AS cnt, SUM(a.extra) OVER () AS total
FROM table_a a
JOIN table_c c ON a.id = c.a_id
WHERE a.id NOT IN (
    SELECT a_id
    FROM table_b
    WHERE group_id IN ('group_3', 'group_4')
)
AND a.category ### 'target_category'
ORDER BY a.id ASC
LIMIT 10;

SELECT a.id, COUNT(a.extra) OVER () AS cnt, SUM(a.extra) OVER () AS total
FROM table_a a
JOIN table_c c ON a.id = c.a_id
WHERE a.id NOT IN (
    SELECT a_id
    FROM table_b
    WHERE group_id IN ('group_3', 'group_4')
)
AND a.category ### 'target_category'
ORDER BY a.id ASC
LIMIT 10;

-- 7b. Window aggregate FILTER column.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT a.id, COUNT(*) FILTER (WHERE a.extra > 2) OVER () AS cnt
FROM table_a a
JOIN table_c c ON a.id = c.a_id
WHERE a.id NOT IN (
    SELECT a_id
    FROM table_b
    WHERE group_id IN ('group_3', 'group_4')
)
AND a.category ### 'target_category'
ORDER BY a.id ASC
LIMIT 10;

SELECT a.id, COUNT(*) FILTER (WHERE a.extra > 2) OVER () AS cnt
FROM table_a a
JOIN table_c c ON a.id = c.a_id
WHERE a.id NOT IN (
    SELECT a_id
    FROM table_b
    WHERE group_id IN ('group_3', 'group_4')
)
AND a.category ### 'target_category'
ORDER BY a.id ASC
LIMIT 10;

-- Cleanup
DROP TABLE table_a CASCADE;
DROP TABLE table_b CASCADE;
DROP TABLE table_c CASCADE;

RESET paradedb.enable_join_custom_scan;
