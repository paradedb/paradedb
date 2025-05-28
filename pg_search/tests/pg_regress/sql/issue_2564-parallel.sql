-- uses the same schema as the mixed fast fields queries
\i common/mixedff_queries_setup.sql

-- Enable parallel workers to ensure they work too
SET max_parallel_workers_per_gather = 2;
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

-- this should return two rows
-- it's exercising "TopN" being a parameterized plan in the subselect
EXPLAIN (COSTS OFF) SELECT d.id, d.title, d.parents,
       (
           SELECT f.title
           FROM files f
           WHERE f.documentId = d.id AND f.title @@@ 'Invoice'
           ORDER BY paradedb.score(f.id) DESC LIMIT 1
       ) AS file_title
FROM documents d
WHERE d.parents @@@ 'Factures'
ORDER BY d.id;
SELECT d.id, d.title, d.parents,
       (
           SELECT f.title
           FROM files f
           WHERE f.documentId = d.id AND f.title @@@ 'Invoice'
           ORDER BY paradedb.score(f.id) DESC LIMIT 1
       ) AS file_title
FROM documents d
WHERE d.parents @@@ 'Factures'
ORDER BY d.id;

-- be a good citizen
RESET max_parallel_workers_per_gather;
RESET enable_indexscan;
\i common/mixedff_queries_cleanup.sql