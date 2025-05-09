-- Tests the basic mixed field functionality

\i common/mixedff_basic_setup.sql

\echo 'Test: Basic mixed fields'

-- Simple query with multiple field types
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT numeric_field1, numeric_field2, string_field1, string_field2
FROM mixed_numeric_string_test
WHERE content @@@ 'red';

-- Execute query and check results
SELECT numeric_field1, numeric_field2, string_field1, string_field2
FROM mixed_numeric_string_test
WHERE content @@@ 'red';

\i common/mixedff_basic_cleanup.sql 
