-- Test for the JoinScan Custom Scan planning
-- HashJoin mechanics and metrics via EXPLAIN ANALYZE

-- Disable parallel workers to avoid differences in plans
SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;

-- Load the pg_search extension
CREATE EXTENSION IF NOT EXISTS pg_search;

-- Make sure the GUC is enabled
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 1: Hash Join
-- =============================================================================

DROP TABLE IF EXISTS hash_t1 CASCADE;
DROP TABLE IF EXISTS hash_t2 CASCADE;

CREATE TABLE hash_t1 (id INTEGER PRIMARY KEY, val TEXT);
CREATE TABLE hash_t2 (id INTEGER PRIMARY KEY, t1_id INTEGER, val TEXT);

INSERT INTO hash_t1 SELECT i, 'val ' || i FROM generate_series(1, 1000) i;
INSERT INTO hash_t2 SELECT i, (i % 1000) + 1, 'val ' || i FROM generate_series(1, 1000) i;

CREATE INDEX hash_t1_idx ON hash_t1 USING bm25 (id, val)
WITH (key_field = 'id', text_fields = '{"val": {"fast": true}}');

CREATE INDEX hash_t2_idx ON hash_t2 USING bm25 (id, t1_id, val)
WITH (key_field = 'id', numeric_fields = '{"t1_id": {"fast": true}}');

ANALYZE hash_t1;
ANALYZE hash_t2;

-- EXPLAIN ANALYZE to show HashJoin metrics.
-- Check in particular for dynamic_filter_pushdown=true, to indicate that dynamic filters
-- were pushed down into the Tantivy Query.
EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT t1.val, t2.val
FROM hash_t1 t1
JOIN hash_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.id ASC
LIMIT 10;

SELECT t1.val, t2.val
FROM hash_t1 t1
JOIN hash_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.id ASC
LIMIT 10;

-- =============================================================================
-- TEST 2: Hash Join with sorted segment on the FK column (issue #4895)
-- =============================================================================
-- Purpose: capture an EXPLAIN ANALYZE of a hash-join probe where the inner
-- index is sorted ASC by the foreign-key column, demonstrating that the
-- dynamic_filter_pushdown=... EXPLAIN token reports the chosen strategy.
--
-- The test asserts BOTH dispatch outcomes:
--   2a. With default gallop_max_density (1/100), K/N is too dense for gallop
--       on this small corpus → linear strategy, EXPLAIN says
--       dynamic_filter_pushdown=linear.
--   2b. With paradedb.term_set_gallop_max_density set high enough to admit
--       this corpus, gallop fires → EXPLAIN says
--       dynamic_filter_pushdown=gallop.

DROP TABLE IF EXISTS hash_sorted_t1 CASCADE;
DROP TABLE IF EXISTS hash_sorted_t2 CASCADE;

CREATE TABLE hash_sorted_t1 (id INTEGER PRIMARY KEY, val TEXT);
CREATE TABLE hash_sorted_t2 (id INTEGER PRIMARY KEY, t1_id INTEGER, val TEXT);

-- t1: 1500 rows (clears the TermSetQuery FastField cardinality threshold of
-- 1024 so the FastField dispatch path is reached).
-- t2: 2000 rows with t1_id repeating across 1..1500 — K/N = 1500/2000 = 0.75,
-- which is too dense for the default gallop threshold (1/100 = 0.01) but
-- below the override used in TEST 2b (1.0).
INSERT INTO hash_sorted_t1 SELECT i, 'val ' || i FROM generate_series(1, 1500) i;
INSERT INTO hash_sorted_t2 SELECT i, ((i - 1) % 1500) + 1, 'val ' || i FROM generate_series(1, 2000) i;

CREATE INDEX hash_sorted_t1_idx ON hash_sorted_t1 USING bm25 (id, val)
WITH (key_field = 'id', text_fields = '{"val": {"fast": true}}');

CREATE INDEX hash_sorted_t2_idx ON hash_sorted_t2 USING bm25 (id, t1_id, val)
WITH (
    key_field = 'id',
    numeric_fields = '{"t1_id": {"fast": true}}',
    sort_by = 't1_id ASC NULLS FIRST'
);

ANALYZE hash_sorted_t1;
ANALYZE hash_sorted_t2;

-- TEST 2a: default density. K/N = 0.75 ≥ 1/100, so gallop is rejected and
-- the planner lands on the LinearScan terminal.
EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT t1.val, t2.val
FROM hash_sorted_t1 t1
JOIN hash_sorted_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.id ASC
LIMIT 10;

SELECT t1.val, t2.val
FROM hash_sorted_t1 t1
JOIN hash_sorted_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.id ASC
LIMIT 10;

-- TEST 2b: density = 1.0 (admit any K < N). Same data, same query — gallop
-- now fires because K/N = 0.75 < 1.0. This is the path the DemandScience
-- workload would take in production once corpus size makes K/N << 1/100.
SET paradedb.term_set_gallop_max_density = 1.0;

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT t1.val, t2.val
FROM hash_sorted_t1 t1
JOIN hash_sorted_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.id ASC
LIMIT 10;

SELECT t1.val, t2.val
FROM hash_sorted_t1 t1
JOIN hash_sorted_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.id ASC
LIMIT 10;

RESET paradedb.term_set_gallop_max_density;

-- =============================================================================
-- TEST 3: paradedb.hash_join_inlist_pushdown_max_distinct_values = 0 disables
-- =============================================================================
-- The GUC is documented as a kill switch. DataFusion's HashJoinExec applies
-- the same predicate (`num_of_distinct_key > max_distinct_values` ⇒ fallback
-- to hash-table map), so 0 disables the InList materialization path on both
-- sides of the boundary. EXPLAIN should NOT show `dynamic_filter_pushdown=`.
SET paradedb.hash_join_inlist_pushdown_max_distinct_values = 0;
SET paradedb.term_set_gallop_max_density = 1.0;

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT t1.val, t2.val
FROM hash_sorted_t1 t1
JOIN hash_sorted_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.id ASC
LIMIT 10;

RESET paradedb.hash_join_inlist_pushdown_max_distinct_values;
RESET paradedb.term_set_gallop_max_density;

-- =============================================================================
-- CLEANUP
-- =============================================================================

DROP TABLE IF EXISTS hash_t1 CASCADE;
DROP TABLE IF EXISTS hash_t2 CASCADE;
DROP TABLE IF EXISTS hash_sorted_t1 CASCADE;
DROP TABLE IF EXISTS hash_sorted_t2 CASCADE;

RESET max_parallel_workers_per_gather;
RESET enable_indexscan;
RESET paradedb.enable_join_custom_scan;
