-- Test subqueries with mixed fast fields

\i common/mixedff_queries_setup.sql

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

SELECT d.id, d.title, d.parents,
    (
        SELECT COUNT(*)
        FROM files f
        WHERE f.documentId = d.id AND (f.title = 'Invoice Receipt' OR f.title = 'Invoice PDF')
    ) AS invoice_file_count
FROM documents d
WHERE d.parents = 'Factures'
ORDER BY invoice_file_count DESC, d.id;


EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT d.id, d.title, d.parents,
    (
        SELECT COUNT(*)
        FROM files f
        WHERE f.documentId = d.id AND (f.title = 'Invoice Receipt' OR f.title = 'Invoice PDF')
    ) AS invoice_file_count
FROM documents d
WHERE d.parents = 'Factures'
ORDER BY invoice_file_count DESC, d.id;

\i common/mixedff_queries_cleanup.sql
