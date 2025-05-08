-- Tests handling of multiple string fast fields

\i common/mixedff_setup.sql

-- Disable parallel workers to avoid differences in plans
SET max_parallel_workers_per_gather = 0;

\echo 'Test: Multiple string fast fields'

-- Check execution plan
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT string_field1, string_field2, string_field3
FROM mixed_numeric_string_test
WHERE content @@@ 'red'
ORDER BY id;

-- Test query with multiple string fields
SELECT string_field1, string_field2, string_field3
FROM mixed_numeric_string_test
WHERE content @@@ 'red'
ORDER BY id;

-- Reset parallel workers setting to default
RESET max_parallel_workers_per_gather; 

\i common/mixedff_cleanup.sql
