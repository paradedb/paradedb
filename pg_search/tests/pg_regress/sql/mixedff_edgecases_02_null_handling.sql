-- Tests null handling in mixed fast fields

\i common/mixedff_edgecases_setup.sql

\echo 'Test: NULL handling'

-- Test with nullable fields
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, string_field, numeric_field
FROM nullable_test
WHERE content @@@ 'null'
ORDER BY id;

-- Test retrieval of NULL values
SELECT id, string_field, numeric_field
FROM nullable_test
WHERE content @@@ 'null'
ORDER BY id;

\i common/mixedff_edgecases_cleanup.sql
