-- Tests mixed fast and non-fast fields

\i common/mixedff_advanced_setup.sql

\echo 'Test: Mixed fast/non-fast fields'

-- Check execution plan for fast fields (should use mixed fast exec)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT fileId, page_number
FROM pages
WHERE content @@@ 'Socienty'
ORDER BY fileId, page_number;

-- Query with only fast fields
SELECT fileId, page_number
FROM pages
WHERE content @@@ 'Socienty'
ORDER BY fileId, page_number;

-- Check execution plan for non-fast field (should not use fast exec)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT content
FROM pages
WHERE content @@@ 'Socienty'
ORDER BY id;

-- Query with non-fast field
SELECT content
FROM pages
WHERE content @@@ 'Socienty'
ORDER BY id;

\i common/mixedff_advanced_cleanup.sql
