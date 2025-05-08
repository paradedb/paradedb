-- Tests complex join queries with mixed fields

\i common/mixedff_setup.sql

-- Disable parallel workers to avoid differences in plans
SET max_parallel_workers_per_gather = 0;

\echo 'Test: Complex join with mixed fields'

-- Check execution plan
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT d.id, d.parents, f.title, f.file_path, p.fileId, p.page_number
FROM documents d
JOIN files f ON d.id = f.documentId
JOIN pages p ON p.fileId = f.id
WHERE d.parents @@@ 'Factures' AND f.title @@@ 'Receipt' AND p.content @@@ 'Socienty'
ORDER BY d.id, f.id, p.id;

-- Test complex join
SELECT d.id, d.parents, f.title, f.file_path, p.fileId, p.page_number
FROM documents d
JOIN files f ON d.id = f.documentId
JOIN pages p ON p.fileId = f.id
WHERE d.parents @@@ 'Factures' AND f.title @@@ 'Receipt' AND p.content @@@ 'Socienty'
ORDER BY d.id, f.id, p.id;

-- Reset parallel workers setting to default
RESET max_parallel_workers_per_gather; 

\i common/mixedff_cleanup.sql
