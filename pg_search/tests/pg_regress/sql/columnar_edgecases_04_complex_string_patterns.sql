-- Tests complex string patterns

\i common/columnar_edgecases_setup.sql

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

\i common/columnar_edgecases_cleanup.sql
