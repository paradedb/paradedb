-- Tests various corner cases for mixed fast fields

\i common/mixedff_setup.sql

-- Disable parallel workers to avoid differences in plans
SET max_parallel_workers_per_gather = 0;

\echo 'Test: Empty strings'
-- Check execution plan
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, empty_string
FROM corner_case_test
WHERE content @@@ 'test'
ORDER BY id;

-- Test handling of empty strings
SELECT id, empty_string
FROM corner_case_test
WHERE content @@@ 'test'
ORDER BY id;

\echo 'Test: Very long strings'
-- Test handling of very long strings
SELECT id, length(very_long_string) as long_string_length
FROM corner_case_test
WHERE content @@@ 'test'
ORDER BY id;

\echo 'Test: Special characters'
-- Test handling of special characters
SELECT id, special_chars
FROM corner_case_test
WHERE content @@@ 'test'
ORDER BY id;

\echo 'Test: Extreme numeric values'
-- Check execution plan
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, extreme_large, extreme_small
FROM corner_case_test
WHERE content @@@ 'test'
ORDER BY id;

-- Test handling of extreme numeric values
SELECT id, extreme_large, extreme_small
FROM corner_case_test
WHERE content @@@ 'test'
ORDER BY id;

\echo 'Test: Boolean values'
-- Test boolean field handling
SELECT id, bool_field
FROM corner_case_test
WHERE content @@@ 'test'
ORDER BY id;

-- Reset parallel workers setting to default
RESET max_parallel_workers_per_gather; 

\i common/mixedff_cleanup.sql
