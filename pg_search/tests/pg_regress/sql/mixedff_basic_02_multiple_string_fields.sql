-- Tests multiple string fast fields functionality

\i common/mixedff_basic_setup.sql

\echo 'Test: Multiple string fast fields'

-- Query with multiple string fields 
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT string_field1, string_field2, string_field3
FROM mixed_numeric_string_test
WHERE content @@@ 'red'
ORDER BY id;

-- Execute query and check results
SELECT string_field1, string_field2, string_field3
FROM mixed_numeric_string_test
WHERE content @@@ 'red'
ORDER BY id;

\i common/mixedff_basic_cleanup.sql
