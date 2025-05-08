-- Tests string edge cases for mixed fast fields

\i common/mixedff_setup.sql

-- Disable parallel workers to avoid differences in plans
SET max_parallel_workers_per_gather = 0;

\echo 'Test: String edge cases'

-- Check execution plan for edge cases
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, string_field1, string_field2
FROM mixed_numeric_string_test
WHERE content @@@ 'edge case'
ORDER BY id;

-- Test query
SELECT id, string_field1, string_field2
FROM mixed_numeric_string_test
WHERE content @@@ 'edge case'
ORDER BY id;

-- Reset parallel workers setting to default 
RESET max_parallel_workers_per_gather; 

\i common/mixedff_cleanup.sql
