-- Tests handling of mixed field types in a query

\i common/mixedff_basic_setup.sql

\echo 'Test: Mixed field types in the same query'

-- Query with both string and numeric fields
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT numeric_field1, string_field1, numeric_field2, string_field2
FROM mixed_numeric_string_test
WHERE content @@@ 'red'
ORDER BY id;

-- Execute query and check results
SELECT numeric_field1, string_field1, numeric_field2, string_field2
FROM mixed_numeric_string_test
WHERE content @@@ 'red'
ORDER BY id;

\i common/mixedff_basic_cleanup.sql
