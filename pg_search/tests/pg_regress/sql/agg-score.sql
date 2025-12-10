-- Test aggregating paradedb.score with parallel custom scan
-- This tests the fix for: "Aggregating paradedb.score when there's a parallel custom scan can produce empty scores"
-- The fix uses PlaceHolderVar to ensure scores are passed through Gather nodes in parallel plans.

CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE mock_items (id SERIAL PRIMARY KEY, description TEXT);
CREATE INDEX search_idx ON mock_items USING bm25 (id, description) WITH (key_field = 'id');

INSERT INTO mock_items (description) VALUES ('keyboard');
INSERT INTO mock_items (description) VALUES ('keyboard');
INSERT INTO mock_items (description) VALUES ('keyboard');
INSERT INTO mock_items (description) VALUES ('keyboard');
INSERT INTO mock_items (description) VALUES ('keyboard');
INSERT INTO mock_items (description) VALUES ('keyboard');
INSERT INTO mock_items (description) VALUES ('keyboard');
INSERT INTO mock_items (description) VALUES ('keyboard');

-- Force parallel query settings to ensure we test the parallel code path
SET debug_parallel_query = on;
SET max_parallel_workers_per_gather = 2;
SET parallel_tuple_cost = 0;
SET parallel_setup_cost = 0;
SET min_parallel_table_scan_size = 0;

-- Test case 1: Basic max(score) - should work with parallel execution
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT max(paradedb.score(id)) FROM mock_items WHERE description @@@ 'keyboard';

SELECT max(paradedb.score(id)) FROM mock_items WHERE description @@@ 'keyboard';

-- Test case 2: min(score) - should work with parallel execution
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT min(paradedb.score(id)) FROM mock_items WHERE description @@@ 'keyboard';

SELECT min(paradedb.score(id)) FROM mock_items WHERE description @@@ 'keyboard';

-- Test case 3: avg(score) - should work with parallel execution
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT avg(paradedb.score(id)) FROM mock_items WHERE description @@@ 'keyboard';

SELECT avg(paradedb.score(id)) FROM mock_items WHERE description @@@ 'keyboard';

-- Test case 4: count with score condition - uses score in WHERE, not in aggregate projection
-- This can still use parallelism because score is evaluated in the WHERE clause
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT count(*) FROM mock_items WHERE description @@@ 'keyboard' AND paradedb.score(id) > 0;

SELECT count(*) FROM mock_items WHERE description @@@ 'keyboard' AND paradedb.score(id) > 0;

-- Test case 5: sum(score) - should work with parallel execution
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT sum(paradedb.score(id)) FROM mock_items WHERE description @@@ 'keyboard';

SELECT sum(paradedb.score(id)) FROM mock_items WHERE description @@@ 'keyboard';

-- Test case 6: Without parallel to verify it works in non-parallel mode too
SET debug_parallel_query = off;
SET max_parallel_workers_per_gather = 0;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT max(paradedb.score(id)) FROM mock_items WHERE description @@@ 'keyboard';

SELECT max(paradedb.score(id)) FROM mock_items WHERE description @@@ 'keyboard';

-- Test case 7: Multiple score aggregates in one query
SET debug_parallel_query = on;
SET max_parallel_workers_per_gather = 2;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT min(paradedb.score(id)), max(paradedb.score(id)), avg(paradedb.score(id)) 
FROM mock_items WHERE description @@@ 'keyboard';

SELECT min(paradedb.score(id)), max(paradedb.score(id)), avg(paradedb.score(id)) 
FROM mock_items WHERE description @@@ 'keyboard';

-- Clean up
DROP TABLE mock_items;
