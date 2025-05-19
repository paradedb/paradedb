-- uses the same schema as the mixed fast fields queries
\i common/mixedff_queries_setup.sql

-- Disable parallel workers to avoid differences in plans
SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;

-- by turning off this GUC we're forcing pg_search to choose its "NormalScanExecState", which is the method under test
SET paradedb.enable_mixed_fast_field_exec = false;

-- this should return one row
EXPLAIN (COSTS OFF) SELECT d.id, d.parents, f.title, f.file_path, p.fileId, p.page_number
FROM documents d
         JOIN files f ON d.id = f.documentId
         JOIN pages p ON p.fileId = f.id
WHERE d.parents @@@ 'Factures'
  AND f.title @@@ 'Receipt'
  AND p.content @@@ 'Socienty'
ORDER BY d.id, f.id, p.id;
SELECT d.id, d.parents, f.title, f.file_path, p.fileId, p.page_number
FROM documents d
         JOIN files f ON d.id = f.documentId
         JOIN pages p ON p.fileId = f.id
WHERE d.parents @@@ 'Factures'
  AND f.title @@@ 'Receipt'
  AND p.content @@@ 'Socienty'
ORDER BY d.id, f.id, p.id;

-- this should return one row too, but through a parallel custom scan
SET max_parallel_workers_per_gather = 2;
EXPLAIN (COSTS OFF) SELECT d.id, d.parents, f.title, f.file_path, p.fileId, p.page_number
FROM documents d
         JOIN files f ON d.id = f.documentId
         JOIN pages p ON p.fileId = f.id
WHERE d.parents @@@ 'Factures'
  AND f.title @@@ 'Receipt'
  AND p.content @@@ 'Socienty'
ORDER BY d.id, f.id, p.id;
SELECT d.id, d.parents, f.title, f.file_path, p.fileId, p.page_number
FROM documents d
         JOIN files f ON d.id = f.documentId
         JOIN pages p ON p.fileId = f.id
WHERE d.parents @@@ 'Factures'
  AND f.title @@@ 'Receipt'
  AND p.content @@@ 'Socienty'
ORDER BY d.id, f.id, p.id;

-- be a good citizen
RESET max_parallel_workers_per_gather;
RESET enable_indexscan;
\i common/mixedff_queries_cleanup.sql