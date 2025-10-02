-- Tests ORDER BY with mixed fast fields

\i common/mixedff_queries_setup.sql

\echo 'Test: ORDER BY with mixed fast fields'

-- Query with ORDER BY
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT fileId, page_number
FROM pages
WHERE content @@@ 'Socienty'
ORDER BY fileid, page_number;

-- Execute query and verify results are ordered
SELECT fileId, page_number
FROM pages
WHERE content @@@ 'Socienty'
ORDER BY fileid, page_number;

\i common/mixedff_queries_cleanup.sql
