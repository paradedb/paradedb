-- Tests subqueries with mixed fast fields

\i common/mixedff_setup.sql

-- Disable parallel workers to avoid differences in plans
SET max_parallel_workers_per_gather = 0;

\echo 'Test: Subqueries with mixed fields'

-- Check execution plan to verify mixed fast fields in subquery
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT d.id, d.title, d.parents,
    (
        SELECT COUNT(*)
        FROM files f
        WHERE f.documentId = d.id AND f.title @@@ 'Invoice'
    ) AS invoice_file_count
FROM documents d
WHERE d.parents @@@ 'Factures'
ORDER BY invoice_file_count DESC, d.id;

-- Test with subquery
SELECT d.id, d.title, d.parents,
    (
        SELECT COUNT(*)
        FROM files f
        WHERE f.documentId = d.id AND f.title @@@ 'Invoice'
    ) AS invoice_file_count
FROM documents d
WHERE d.parents @@@ 'Factures'
ORDER BY invoice_file_count DESC, d.id;

-- Reset parallel workers setting to default
RESET max_parallel_workers_per_gather; 

\i common/mixedff_cleanup.sql
