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
-- CLEANUP
-- =============================================================================

DROP TABLE IF EXISTS hash_t1 CASCADE;
DROP TABLE IF EXISTS hash_t2 CASCADE;

RESET max_parallel_workers_per_gather;
RESET enable_indexscan;
RESET paradedb.enable_join_custom_scan;
