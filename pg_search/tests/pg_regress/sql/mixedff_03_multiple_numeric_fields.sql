-- Tests handling of multiple numeric fast fields

\i common/mixedff_setup.sql
-- Disable parallel workers to avoid differences in plans
SET max_parallel_workers_per_gather = 0;

\echo 'Test: Multiple numeric fast fields'

-- Check execution plan
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT numeric_field1, numeric_field2
FROM mixed_numeric_string_test
WHERE content @@@ 'red'
ORDER BY id;

-- Test query with multiple numeric fields
SELECT numeric_field1, numeric_field2
FROM mixed_numeric_string_test
WHERE content @@@ 'red'
ORDER BY id;

-- Reset parallel workers setting to default
RESET max_parallel_workers_per_gather; 

\i common/mixedff_cleanup.sql
