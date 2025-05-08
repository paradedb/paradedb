-- Tests complex string patterns in mixed fast fields

\i common/mixedff_setup.sql

-- Disable parallel workers to avoid differences in plans
SET max_parallel_workers_per_gather = 0;

\echo 'Test: Complex string patterns'

-- Check execution plan for complex string patterns
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, empty_string, special_chars 
FROM corner_case_test
WHERE content @@@ 'complex pattern'
ORDER BY id;

-- Test query
SELECT id, empty_string, special_chars 
FROM corner_case_test
WHERE content @@@ 'complex pattern'
ORDER BY id;

-- Reset parallel workers setting to default
RESET max_parallel_workers_per_gather; 

\i common/mixedff_cleanup.sql
