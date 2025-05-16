-- Tests multiple numeric fast fields functionality

\i common/mixedff_basic_setup.sql


\echo 'Test: Multiple numeric fast fields'

-- Query with multiple numeric fields
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT numeric_field1, numeric_field2
FROM mixed_numeric_string_test
WHERE content @@@ 'red'
ORDER BY id;

-- Execute query and check results
SELECT numeric_field1, numeric_field2
FROM mixed_numeric_string_test
WHERE content @@@ 'red'
ORDER BY id;


\i common/mixedff_basic_cleanup.sql
