-- Tests Common Table Expressions with mixed fast fields

\i common/mixedff_setup.sql

-- Disable parallel workers to avoid differences in plans
SET max_parallel_workers_per_gather = 0;

\echo 'Test: Basic CTE with mixed fields'

-- Check execution plan to verify mixed fast fields used in CTEs
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
WITH searchable_docs AS (
    SELECT d.id, d.title, d.parents
    FROM documents d
    WHERE d.title @@@ 'CTE Test' AND d.parents @@@ 'Reports'
),
matching_files AS (
    SELECT f.id, f.documentId, f.title, f.file_path, f.file_size
    FROM files f
    JOIN searchable_docs sd ON f.documentId = sd.id
    WHERE f.title @@@ 'CTE Test'
),
relevant_pages AS (
    SELECT p.id, p.fileId, p.page_number
    FROM pages p
    JOIN matching_files mf ON p.fileId = mf.id
    WHERE p.content @@@ 'searchable OR testing'
)
SELECT 
    sd.title as document_title, 
    mf.title as file_title, 
    mf.file_size, 
    rp.page_number
FROM searchable_docs sd
JOIN matching_files mf ON sd.id = mf.documentId
JOIN relevant_pages rp ON mf.id = rp.fileId
ORDER BY document_title, file_title, page_number;

-- Test with CTE
WITH searchable_docs AS (
    SELECT d.id, d.title, d.parents
    FROM documents d
    WHERE d.title @@@ 'CTE Test' AND d.parents @@@ 'Reports'
),
matching_files AS (
    SELECT f.id, f.documentId, f.title, f.file_path, f.file_size
    FROM files f
    JOIN searchable_docs sd ON f.documentId = sd.id
    WHERE f.title @@@ 'CTE Test'
),
relevant_pages AS (
    SELECT p.id, p.fileId, p.page_number
    FROM pages p
    JOIN matching_files mf ON p.fileId = mf.id
    WHERE p.content @@@ 'searchable OR testing'
)
SELECT 
    sd.title as document_title, 
    mf.title as file_title, 
    mf.file_size, 
    rp.page_number
FROM searchable_docs sd
JOIN matching_files mf ON sd.id = mf.documentId
JOIN relevant_pages rp ON mf.id = rp.fileId
ORDER BY document_title, file_title, page_number;

-- Reset parallel workers setting to default
RESET max_parallel_workers_per_gather; 

\i common/mixedff_cleanup.sql
