-- Tests string edge cases

\i common/mixedff_edgecases_setup.sql

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

\i common/mixedff_edgecases_cleanup.sql
