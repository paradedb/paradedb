-- Test for issue #3055: Don't spin up extra parallel workers for tiny segments
--
-- This test verifies that the paradedb.min_rows_per_worker GUC correctly
-- limits parallel workers based on row count, so each worker processes
-- enough rows for parallelism to be worthwhile (~10ms startup overhead).
--
-- Based on benchmarks, the crossover point where parallel becomes beneficial
-- is around 300K rows total. Default min_rows_per_worker is 300000.
--
-- See: https://github.com/paradedb/paradedb/issues/3055

CREATE EXTENSION IF NOT EXISTS pg_search;

-- Enable parallel workers globally
SET max_parallel_workers_per_gather = 2;
SET max_parallel_workers = 8;

-- Force postgres to consider parallel plans
SET parallel_tuple_cost = 0;
SET parallel_setup_cost = 0;
SET min_parallel_table_scan_size = 0;

-- Create test table
DROP TABLE IF EXISTS items CASCADE;
CREATE TABLE items (
    id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    name TEXT NOT NULL
);

-- Create BM25 index BEFORE inserting data to create multiple segments
CREATE INDEX items_bm25_idx ON items
USING bm25 (id, name)
WITH (key_field = 'id');

-- Insert first batch of data (creates segment 1)
INSERT INTO items (name) SELECT 'item ' || g FROM generate_series(1, 5000) g;

-- Insert second batch (creates segment 2)
INSERT INTO items (name) SELECT 'item ' || g FROM generate_series(5001, 10000) g;

-- ANALYZE to get accurate row estimates (required for threshold check to work)
ANALYZE items;

-- Verify reltuples is set correctly
SELECT relname, reltuples FROM pg_class WHERE relname = 'items';

-- Test 1: Default behavior (min_rows_per_worker=300000)
-- With 10000 rows, max_workers = 10000/300000 = 0, so parallel should be DISABLED
SET paradedb.min_rows_per_worker = 300000;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) 
SELECT id, name FROM items WHERE name @@@ 'item' ORDER BY id LIMIT 10;

-- Test 2: Lower the threshold (min_rows_per_worker=5000)
-- With 10000 rows, max_workers = 10000/5000 = 2, parallel SHOULD be used
SET paradedb.min_rows_per_worker = 5000;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, name FROM items WHERE name @@@ 'item' ORDER BY id LIMIT 10;

-- Test 3: Disable threshold completely (0)
-- Parallel SHOULD be used based on segment count only
SET paradedb.min_rows_per_worker = 0;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, name FROM items WHERE name @@@ 'item' ORDER BY id LIMIT 10;

-- Test 4: TopN query with ORDER BY score and LIMIT
-- With high threshold, parallel should be disabled for 10000 rows
SET paradedb.min_rows_per_worker = 300000;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, name FROM items 
WHERE name @@@ pdb.match('item')
ORDER BY paradedb.score(id) DESC
LIMIT 10;

SELECT id, name FROM items 
WHERE name @@@ pdb.match('item')
ORDER BY paradedb.score(id) DESC
LIMIT 10;

-- Test 5: Verify unanalyzed table behavior
-- When reltuples is unknown (-1), parallel should still be allowed
-- (we shouldn't limit workers when we can't trust the row estimate)
DROP TABLE items;

CREATE TABLE items (
    id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE INDEX items_bm25_idx ON items
USING bm25 (id, name)
WITH (key_field = 'id');

-- Insert data in two batches but DON'T analyze
INSERT INTO items (name) SELECT 'item ' || g FROM generate_series(1, 5000) g;
INSERT INTO items (name) SELECT 'item ' || g FROM generate_series(5001, 10000) g;

-- Verify reltuples is -1 (unanalyzed)
SELECT relname, reltuples FROM pg_class WHERE relname = 'items';

-- Even with high threshold that would normally disable parallel,
-- parallel should still be used because we don't have reliable row estimates
SET paradedb.min_rows_per_worker = 300000;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, name FROM items WHERE name @@@ 'item' ORDER BY id LIMIT 10;

-- Now ANALYZE the table and run again - parallel should be DISABLED
-- because we now have reliable row estimates (10000 rows / 300000 = 0 workers)
ANALYZE items;

SELECT relname, reltuples FROM pg_class WHERE relname = 'items';

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, name FROM items WHERE name @@@ 'item' ORDER BY id LIMIT 10;

-- Clean up
DROP TABLE items;

-- Reset GUCs
RESET paradedb.min_rows_per_worker;
RESET max_parallel_workers_per_gather;
RESET max_parallel_workers;
RESET parallel_tuple_cost;
RESET parallel_setup_cost;
RESET min_parallel_table_scan_size;
