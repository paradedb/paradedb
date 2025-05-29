-- Tests complex join queries with mixed fields

\i common/mixedff_queries_setup.sql

\echo 'Test: Complex joins'

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

-- Test complex join with mixed fields
SELECT pages.fileId, pages.page_number, pages.content
FROM documents 
JOIN files ON documents.id = files.documentId
JOIN pages ON pages.fileId = files.id
WHERE documents.parents @@@ 'Factures' AND files.title @@@ 'Receipt' AND pages.content @@@ 'Socienty'
ORDER BY pages.fileId, pages.page_number
LIMIT 10;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT pages.fileId, pages.page_number, pages.content
FROM documents 
JOIN files ON documents.id = files.documentId
JOIN pages ON pages.fileId = files.id
WHERE documents.parents @@@ 'Factures' AND files.title @@@ 'Receipt' AND pages.content @@@ 'Socienty'
ORDER BY pages.fileId, pages.page_number
LIMIT 10;

\i common/mixedff_queries_cleanup.sql
