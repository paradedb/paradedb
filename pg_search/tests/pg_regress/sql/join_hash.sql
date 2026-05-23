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
-- Check in particular for the `dynamic_filter_pushdown=<strategy>` token (one of
-- {gallop, linear, bitset_from_postings, automaton, empty}; falls back to `true`
-- on the rare path where pushdown was indicated but no strategy tag was
-- recorded), which signals dynamic filters were pushed down into the Tantivy
-- query and reports the dispatched TermSet strategy.
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
--   2a. With paradedb.term_set_gallop_enabled = OFF, gallop is rejected
--       via the kill-switch. K/N = 0.75 is also above both bitset
--       density gates, so the planner falls through to LinearScan →
--       EXPLAIN says dynamic_filter_pushdown=linear.
--   2b. With paradedb.term_set_gallop_enabled at its default ON, gallop
--       fires on this sorted segment → EXPLAIN says
--       dynamic_filter_pushdown=gallop.

DROP TABLE IF EXISTS hash_sorted_t1 CASCADE;
DROP TABLE IF EXISTS hash_sorted_t2 CASCADE;

CREATE TABLE hash_sorted_t1 (id INTEGER PRIMARY KEY, val TEXT);
CREATE TABLE hash_sorted_t2 (id INTEGER PRIMARY KEY, t1_id INTEGER, val TEXT);

-- t1: 1500 rows (clears the TermSetQuery FastField cardinality threshold of
-- 1024 so the FastField dispatch path is reached).
-- t2: 2000 rows with t1_id repeating across 1..1500 — K/N = 1500/2000 = 0.75.
-- TEST 2a flips `term_set_gallop_enabled` off to reject gallop via the
-- kill-switch; TEST 2b leaves the default on so gallop fires.
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

-- TEST 2a: gallop kill-switch engaged. K/N = 0.75 is also above both
-- bitset gates (1/2000 and 1/200), so dispatch falls through to
-- LinearScan.
SET paradedb.term_set_gallop_enabled = OFF;

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

RESET paradedb.term_set_gallop_enabled;

-- TEST 2b: default behavior. Same data, same query — `gallop_enabled`
-- is ON by default, sorted segment + matching field → gallop fires.

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

-- =============================================================================
-- TEST 3: paradedb.hash_join_inlist_pushdown_max_distinct_values = 0 disables
-- =============================================================================
-- The GUC is documented as a kill switch. DataFusion's HashJoinExec applies
-- the same predicate (`num_of_distinct_key > max_distinct_values` ⇒ fallback
-- to hash-table map), so 0 disables the InList materialization path on both
-- sides of the boundary. EXPLAIN should NOT show `dynamic_filter_pushdown=`.
SET paradedb.hash_join_inlist_pushdown_max_distinct_values = 0;

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT t1.val, t2.val
FROM hash_sorted_t1 t1
JOIN hash_sorted_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.id ASC
LIMIT 10;

RESET paradedb.hash_join_inlist_pushdown_max_distinct_values;

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
