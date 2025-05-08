-- Tests ORDER BY behavior with mixed fast fields

\i common/mixedff_setup.sql

-- Disable parallel workers to avoid differences in plans
SET max_parallel_workers_per_gather = 0;

\echo 'Test: ORDER BY with mixed fields'

-- Check execution plan
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT fileId, page_number
FROM pages
WHERE content @@@ 'Socienty'
ORDER BY fileid, page_number;

-- Test query with ORDER BY
SELECT fileId, page_number
FROM pages
WHERE content @@@ 'Socienty'
ORDER BY fileid, page_number;

-- Reset parallel workers setting to default
RESET max_parallel_workers_per_gather; 

\i common/mixedff_cleanup.sql
