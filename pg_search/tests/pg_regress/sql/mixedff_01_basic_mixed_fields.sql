-- Tests the basic mixed field functionality

\i common/mixedff_setup.sql

-- Disable parallel workers to avoid differences in plans
SET max_parallel_workers_per_gather = 0;

-- Test 1: Mixed string and numeric fields
\echo 'Test: Basic mixed string and numeric fields'

-- Check execution plan
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT fileId, page_number
FROM pages
WHERE content @@@ 'Socienty'
ORDER BY fileId, page_number;

-- Test actual query
SELECT fileId, page_number
FROM pages
WHERE content @@@ 'Socienty'
ORDER BY fileId, page_number;

-- Reset parallel workers setting to default
RESET max_parallel_workers_per_gather; 

\i common/mixedff_cleanup.sql
