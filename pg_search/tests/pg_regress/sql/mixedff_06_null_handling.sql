-- Tests NULL value handling in mixed fast fields

\i common/mixedff_setup.sql

-- Disable parallel workers to avoid differences in plans
SET max_parallel_workers_per_gather = 0;

\echo 'Test: NULL value handling'

-- Check execution plan
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, string_field, numeric_field
FROM nullable_test
WHERE content @@@ 'null'
ORDER BY id;

-- Test handling NULL values
SELECT id, string_field, numeric_field
FROM nullable_test
WHERE content @@@ 'null'
ORDER BY id;

-- Reset parallel workers setting to default
RESET max_parallel_workers_per_gather; 

\i common/mixedff_cleanup.sql
