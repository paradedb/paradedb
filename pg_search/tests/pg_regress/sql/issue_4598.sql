-- Test for issue #4598: parallel worker segfault with subqueries in HeapFieldFilter
-- Combination of ORDER BY score, subqueries, parallel workers and EXPLAIN ANALYZE
-- causes a parallel worker to segfault.
--
-- Root cause: PARAM_EXEC nodes from InitPlan subqueries are not available
-- in parallel workers' EState. The fix pre-evaluates these into Const nodes.

CREATE EXTENSION IF NOT EXISTS pg_search;

-- Force parallel workers to reproduce the issue
SET max_parallel_workers_per_gather = 2;
SET min_parallel_table_scan_size = 0;
SET parallel_tuple_cost = 0;
SET parallel_setup_cost = 0;

-- Create test table
DROP TABLE IF EXISTS issue_4598_test CASCADE;

CREATE TABLE issue_4598_test (
    id SERIAL PRIMARY KEY,
    content TEXT NOT NULL,
    allowed_users INTEGER[] DEFAULT '{}'
);

-- Insert enough rows to trigger parallel scan
INSERT INTO issue_4598_test (content, allowed_users)
SELECT
    'test content ' || i,
    CASE WHEN i % 3 = 0 THEN ARRAY[1, 2, 3]
         WHEN i % 3 = 1 THEN ARRAY[4, 5]
         ELSE '{}'
    END
FROM generate_series(1, 1000) i;

-- Create BM25 index
CREATE INDEX issue_4598_idx ON issue_4598_test
    USING bm25 (id, content)
    WITH (key_field = 'id');

-- Test 1: Basic query with subquery in non-indexed predicate + ORDER BY score
-- This is the core reproducer from issue #4598
SELECT id, content
FROM issue_4598_test
WHERE id @@@ 'content:test'
  AND (allowed_users && (SELECT array[1, 2]) OR allowed_users = '{}')
ORDER BY paradedb.score(id) DESC
LIMIT 5;

-- Test 2: EXPLAIN ANALYZE with same query (the crash specifically happened with EXPLAIN ANALYZE)
EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, SUMMARY OFF)
SELECT id, content
FROM issue_4598_test
WHERE id @@@ 'content:test'
  AND (allowed_users && (SELECT array[1, 2]) OR allowed_users = '{}')
ORDER BY paradedb.score(id) DESC
LIMIT 5;

-- Test 3: Multiple subqueries in the same expression
SELECT count(*)
FROM issue_4598_test
WHERE id @@@ 'content:test'
  AND (allowed_users && (SELECT array[1, 2]) OR allowed_users && (SELECT array[3]));

-- Test 4: NULL subquery result — should not crash, just filter correctly
SELECT count(*)
FROM issue_4598_test
WHERE id @@@ 'content:test'
  AND allowed_users && (SELECT NULL::int[]);

-- Cleanup
DROP TABLE issue_4598_test CASCADE;
RESET max_parallel_workers_per_gather;
RESET min_parallel_table_scan_size;
RESET parallel_tuple_cost;
RESET parallel_setup_cost;
