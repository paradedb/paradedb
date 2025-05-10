-- Tests corner cases for mixed fast fields

\i common/mixedff_edgecases_setup.sql

\echo 'Test: Corner cases and edge values'

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
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, length(very_long_string) as long_string_length
FROM corner_case_test
WHERE content @@@ 'test'
ORDER BY id;

SELECT id, length(very_long_string) as long_string_length
FROM corner_case_test
WHERE content @@@ 'test'
ORDER BY id;

\echo 'Test: Special characters'
-- Test handling of special characters
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, special_chars
FROM corner_case_test
WHERE content @@@ 'test'
ORDER BY id;

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
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, bool_field
FROM corner_case_test
WHERE content @@@ 'test'
ORDER BY id;

SELECT id, bool_field
FROM corner_case_test
WHERE content @@@ 'test'
ORDER BY id;

\i common/mixedff_edgecases_cleanup.sql
