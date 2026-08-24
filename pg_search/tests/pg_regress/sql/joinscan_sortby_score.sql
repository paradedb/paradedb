-- Tests for JoinScan BM25 score calculation, score aggregation, and ordering.
-- Verifies that BM25 scores across 3-table and 2-table conjunctive (AND)
-- and disjunctive (OR) joins produce meaningful, deterministic, and intuitive rankings.

SET max_parallel_workers = 0;
SET max_parallel_workers_per_gather = 0;
SET parallel_leader_participation = off;
SET enable_indexscan TO off;

CREATE EXTENSION IF NOT EXISTS pg_search;

-- =============================================================================
-- SETUP: 3-table hierarchy (documents -> files -> pages)
-- =============================================================================
-- The test data is designed with deliberate term frequency (TF) and term
-- distribution across tables so that score rankings are easy to reason about:
--
-- Term frequencies for 'postgres' in documents.content:
--   doc-1: 3x ('postgres postgres postgres ...') -> high score
--   doc-2: 2x ('postgres postgres ...')          -> medium score
--   doc-3: 1x ('postgres ...')                   -> low score
--   doc-4: 0x ('sqlite ...')                     -> no match (score 0)
--
-- Term frequencies for 'index' in files.content:
--   file-1 (doc-1): 3x ('index index index ...') -> high score
--   file-2 (doc-1): 1x ('index ...')             -> low score
--   file-3 (doc-2): 2x ('index index ...')       -> medium score
--   file-4 (doc-2): 1x ('index ...')             -> low score
--   file-5 (doc-3): 1x ('index ...')             -> low score
--   file-6 (doc-4): 0x ('memory ...')            -> no match (score 0)
--
-- Term frequencies for 'vector' in pages.content:
--   page-1 (file-1): 3x ('vector vector vector ...') -> high score
--   page-2 (file-1): 1x ('vector ...')               -> low score
--   page-3 (file-2): 2x ('vector vector ...')         -> medium score
--   page-4 (file-3): 2x ('vector vector ...')         -> medium score
--   page-5 (file-4): 1x ('vector ...')               -> low score
--   page-6 (file-5): 1x ('vector ...')               -> low score
--   page-7 (file-6): 0x ('algorithms ...')           -> no match (score 0)
--
-- Cross-table term 'search' distribution:
--   doc-1:  matches 'search' (content & title)
--   file-1: matches 'search' (content)
--   page-1: matches 'search' (content & title)
--   page-3: matches 'search' (content & title)
--   page-4: matches 'search' (content & title)
-- =============================================================================

DROP TABLE IF EXISTS pages CASCADE;
DROP TABLE IF EXISTS files CASCADE;
DROP TABLE IF EXISTS documents CASCADE;

CREATE TABLE documents (
    id TEXT PRIMARY KEY,
    parents TEXT,
    content TEXT,
    title TEXT
);

CREATE TABLE files (
    id TEXT PRIMARY KEY,
    "documentId" TEXT,
    content TEXT,
    title TEXT
);

CREATE TABLE pages (
    id TEXT PRIMARY KEY,
    "fileId" TEXT,
    content TEXT,
    title TEXT
);

INSERT INTO documents (id, parents, content, title) VALUES
('doc-1', 'project alpha', 'postgres postgres postgres database core search', 'Postgres Alpha Documentation'),
('doc-2', 'project beta',  'postgres postgres database core',                'Postgres Beta Guide'),
('doc-3', 'project gamma', 'postgres database core',                         'Gamma Reference Manual'),
('doc-4', 'archive',       'sqlite embedded key value store',                'Other Systems');

INSERT INTO files (id, "documentId", content, title) VALUES
('file-1', 'doc-1', 'index index index fast access search', 'Postgres Index Architecture'),
('file-2', 'doc-1', 'index fast access',                    'Index Overview'),
('file-3', 'doc-2', 'index index fast access',              'Indexing Guide'),
('file-4', 'doc-2', 'index fast access',                    'Index Maintenance'),
('file-5', 'doc-3', 'index fast access',                    'Gamma Index Details'),
('file-6', 'doc-4', 'memory buffer cache management',       'Buffer Pool');

INSERT INTO pages (id, "fileId", content, title) VALUES
('page-1', 'file-1', 'vector vector vector similarity search', 'Vector Search Implementation'),
('page-2', 'file-1', 'vector similarity scan',                 'Basic Vector Scan'),
('page-3', 'file-2', 'vector vector similarity search',         'Vector Index Tuning'),
('page-4', 'file-3', 'vector vector similarity search',         'Vector Operations'),
('page-5', 'file-4', 'vector similarity scan',                 'Vector Utilities'),
('page-6', 'file-5', 'vector similarity scan',                 'Gamma Vector Helpers'),
('page-7', 'file-6', 'page replacement algorithms',            'Page Eviction');

CREATE INDEX pages_bm25 ON pages
USING bm25 (id, content, title, "fileId")
WITH (
    key_field = 'id',
    text_fields = '{
        "fileId": {"tokenizer": {"type": "keyword"}, "fast": true},
        "content": {"tokenizer": {"type": "default"}, "fast": true},
        "title": {"tokenizer": {"type": "default"}, "fast": true}
    }'
);

CREATE INDEX files_bm25 ON files
USING bm25 (id, content, "documentId", title)
WITH (
    key_field = 'id',
    text_fields = '{
        "documentId": {"tokenizer": {"type": "keyword"}, "fast": true},
        "content": {"tokenizer": {"type": "default"}, "fast": true},
        "title": {"tokenizer": {"type": "default"}, "fast": true}
    }'
);

CREATE INDEX documents_bm25 ON documents
USING bm25 (id, content, title, parents)
WITH (
    key_field = 'id',
    text_fields = '{
        "content": {"tokenizer": {"type": "default"}, "fast": true},
        "title": {"tokenizer": {"type": "default"}, "fast": true},
        "parents": {"tokenizer": {"type": "default"}, "fast": true}
    }'
);

-- =============================================================================
-- PREAMBLE: Baseline BM25 scores on individual tables (no joins)
-- =============================================================================
-- These queries establish the ground-truth scores on each isolated table.
-- They serve as a direct reference to verify that join-level score propagation
-- and aggregation calculate correct values without distortion or RTI confusion.

-- Baseline 1: Single-table scores for the conjunctive join terms
SELECT 'documents' AS tbl, id, 'postgres' AS query, paradedb.score(id) AS score
FROM documents WHERE content @@@ 'postgres'
UNION ALL
SELECT 'files' AS tbl, id, 'index' AS query, paradedb.score(id) AS score
FROM files WHERE content @@@ 'index'
UNION ALL
SELECT 'pages' AS tbl, id, 'vector' AS query, paradedb.score(id) AS score
FROM pages WHERE content @@@ 'vector'
ORDER BY tbl, score DESC, id ASC;

-- Baseline 2: Single-table scores for the disjunctive cross-table term ('search')
SELECT 'documents' AS tbl, id, 'search' AS query, paradedb.score(id) AS score
FROM documents WHERE content @@@ 'search'
UNION ALL
SELECT 'files' AS tbl, id, 'search' AS query, paradedb.score(id) AS score
FROM files WHERE content @@@ 'search'
UNION ALL
SELECT 'pages' AS tbl, id, 'search' AS query, paradedb.score(id) AS score
FROM pages WHERE content @@@ 'search'
ORDER BY tbl, score DESC, id ASC;

-- Baseline 3: Single-table scores for within-table disjunction ('postgres OR search')
SELECT 'documents' AS tbl, id, 'postgres OR search' AS query, paradedb.score(id) AS score
FROM documents WHERE content @@@ 'postgres OR search'
ORDER BY score DESC, id ASC;

SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 1: 3-way conjunctive join with summed scores and ORDER BY score DESC
-- =============================================================================
-- All three tables have search predicates. Rows are ranked by sum of BM25 scores:
--   doc-1/file-1/page-1: High(doc) + High(file) + High(page) -> Rank 1
--   doc-2/file-3/page-4: Med(doc)  + Med(file)  + Med(page)  -> Rank 2
--   doc-1/file-1/page-2: High(doc) + High(file) + Low(page)  -> Rank 3
--   doc-1/file-2/page-3: High(doc) + Low(file)  + Med(page)  -> Rank 4
--   doc-2/file-4/page-5: Med(doc)  + Low(file)  + Low(page)  -> Rank 5
--   doc-3/file-5/page-6: Low(doc)  + Low(file)  + Low(page)  -> Rank 6

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT documents.id AS doc_id,
       files.id AS file_id,
       pages.id AS page_id,
       paradedb.score(documents.id) AS doc_score,
       paradedb.score(files.id) AS file_score,
       paradedb.score(pages.id) AS page_score,
       paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score
FROM documents
JOIN files ON documents.id = files."documentId"
JOIN pages ON pages."fileId" = files.id
WHERE documents.content @@@ 'postgres'
  AND files.content @@@ 'index'
  AND pages.content @@@ 'vector'
ORDER BY score DESC
LIMIT 10;

SELECT documents.id AS doc_id,
       files.id AS file_id,
       pages.id AS page_id,
       paradedb.score(documents.id) AS doc_score,
       paradedb.score(files.id) AS file_score,
       paradedb.score(pages.id) AS page_score,
       paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score
FROM documents
JOIN files ON documents.id = files."documentId"
JOIN pages ON pages."fileId" = files.id
WHERE documents.content @@@ 'postgres'
  AND files.content @@@ 'index'
  AND pages.content @@@ 'vector'
ORDER BY score DESC
LIMIT 10;

-- =============================================================================
-- TEST 2: 2-way conjunctive join with summed scores and ORDER BY score DESC
-- =============================================================================
-- Two-table join (documents JOIN files) with distinct score sum ranking:
--   doc-1/file-1: High(doc) + High(file) -> Rank 1
--   doc-2/file-3: Med(doc)  + Med(file)  -> Rank 2
--   doc-1/file-2: High(doc) + Low(file)  -> Rank 3
--   doc-2/file-4: Med(doc)  + Low(file)  -> Rank 4
--   doc-3/file-5: Low(doc)  + Low(file)  -> Rank 5

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT documents.id AS doc_id,
       files.id AS file_id,
       paradedb.score(documents.id) AS doc_score,
       paradedb.score(files.id) AS file_score,
       paradedb.score(documents.id) + paradedb.score(files.id) AS score
FROM documents
JOIN files ON documents.id = files."documentId"
WHERE documents.content @@@ 'postgres'
  AND files.content @@@ 'index'
ORDER BY score DESC
LIMIT 10;

SELECT documents.id AS doc_id,
       files.id AS file_id,
       paradedb.score(documents.id) AS doc_score,
       paradedb.score(files.id) AS file_score,
       paradedb.score(documents.id) + paradedb.score(files.id) AS score
FROM documents
JOIN files ON documents.id = files."documentId"
WHERE documents.content @@@ 'postgres'
  AND files.content @@@ 'index'
ORDER BY score DESC
LIMIT 10;

-- =============================================================================
-- TEST 3: Multi-key ORDER BY with score sum and secondary column
-- =============================================================================
-- Verifies that sorting by score sum combined with a deterministic secondary
-- column (pages.id ASC) plans and executes correctly.

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT documents.id AS doc_id,
       files.id AS file_id,
       pages.id AS page_id,
       paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS pdb_score
FROM documents
JOIN files ON documents.id = files."documentId"
JOIN pages ON pages."fileId" = files.id
WHERE documents.content @@@ 'postgres'
  AND files.content @@@ 'index'
  AND pages.content @@@ 'vector'
ORDER BY pdb_score DESC, pages.id ASC
LIMIT 10;

SELECT documents.id AS doc_id,
       files.id AS file_id,
       pages.id AS page_id,
       paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS pdb_score
FROM documents
JOIN files ON documents.id = files."documentId"
JOIN pages ON pages."fileId" = files.id
WHERE documents.content @@@ 'postgres'
  AND files.content @@@ 'index'
  AND pages.content @@@ 'vector'
ORDER BY pdb_score DESC, pages.id ASC
LIMIT 10;

-- =============================================================================
-- TEST 4: 3-way cross-table disjunctive join (OR across tables)
-- =============================================================================
-- Cross-table OR query: rows matching across 3 tables score higher than 2 tables,
-- which score higher than 1 table. Non-matching tables contribute 0 to the sum.
--   doc-1/file-1/page-1: 3-table match (doc, file, page) -> Rank 1
--   doc-1/file-1/page-2: 2-table match (doc, file)       -> Rank 2
--   doc-1/file-2/page-3: 2-table match (doc, page)       -> Rank 3
--   doc-2/file-3/page-4: 1-table match (page only)       -> Rank 4

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT documents.id AS doc_id,
       files.id AS file_id,
       pages.id AS page_id,
       paradedb.score(documents.id) AS doc_score,
       paradedb.score(files.id) AS file_score,
       paradedb.score(pages.id) AS page_score,
       paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score
FROM documents
JOIN files ON documents.id = files."documentId"
JOIN pages ON pages."fileId" = files.id
WHERE documents.content @@@ 'search'
   OR files.content @@@ 'search'
   OR pages.content @@@ 'search'
ORDER BY score DESC, documents.id ASC, files.id ASC, pages.id ASC
LIMIT 10;

SELECT documents.id AS doc_id,
       files.id AS file_id,
       pages.id AS page_id,
       paradedb.score(documents.id) AS doc_score,
       paradedb.score(files.id) AS file_score,
       paradedb.score(pages.id) AS page_score,
       paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score
FROM documents
JOIN files ON documents.id = files."documentId"
JOIN pages ON pages."fileId" = files.id
WHERE documents.content @@@ 'search'
   OR files.content @@@ 'search'
   OR pages.content @@@ 'search'
ORDER BY score DESC, documents.id ASC, files.id ASC, pages.id ASC
LIMIT 10;

-- =============================================================================
-- TEST 4b: 3-way cross-table disjunction with single-table score ordering and LIMIT
-- =============================================================================
-- Verifies that TopK dynamic score filtering behaves correctly over a tagged scan
-- when ordering by a single relation's score rather than a score sum.

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT documents.id AS doc_id,
       files.id AS file_id,
       pages.id AS page_id,
       paradedb.score(pages.id) AS page_score
FROM documents
JOIN files ON documents.id = files."documentId"
JOIN pages ON pages."fileId" = files.id
WHERE documents.content @@@ 'search'
   OR files.content @@@ 'search'
   OR pages.content @@@ 'search'
ORDER BY paradedb.score(pages.id) DESC, documents.id ASC, files.id ASC, pages.id ASC
LIMIT 10;

SELECT documents.id AS doc_id,
       files.id AS file_id,
       pages.id AS page_id,
       paradedb.score(pages.id) AS page_score
FROM documents
JOIN files ON documents.id = files."documentId"
JOIN pages ON pages."fileId" = files.id
WHERE documents.content @@@ 'search'
   OR files.content @@@ 'search'
   OR pages.content @@@ 'search'
ORDER BY paradedb.score(pages.id) DESC, documents.id ASC, files.id ASC, pages.id ASC
LIMIT 10;

-- =============================================================================
-- TEST 5: Mixed conjunctive and disjunctive join across 3 tables (A AND (B OR C))
-- =============================================================================
-- Requires documents.content @@@ 'postgres' (mandatory) AND either files or pages
-- matches 'search'. Documents matching more terms across joined tables rank higher.

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT documents.id AS doc_id,
       files.id AS file_id,
       pages.id AS page_id,
       paradedb.score(documents.id) AS doc_score,
       paradedb.score(files.id) AS file_score,
       paradedb.score(pages.id) AS page_score,
       paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score
FROM documents
JOIN files ON documents.id = files."documentId"
JOIN pages ON pages."fileId" = files.id
WHERE documents.content @@@ 'postgres'
  AND (files.content @@@ 'search' OR pages.content @@@ 'search')
ORDER BY score DESC, documents.id ASC, files.id ASC, pages.id ASC
LIMIT 10;

SELECT documents.id AS doc_id,
       files.id AS file_id,
       pages.id AS page_id,
       paradedb.score(documents.id) AS doc_score,
       paradedb.score(files.id) AS file_score,
       paradedb.score(pages.id) AS page_score,
       paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score
FROM documents
JOIN files ON documents.id = files."documentId"
JOIN pages ON pages."fileId" = files.id
WHERE documents.content @@@ 'postgres'
  AND (files.content @@@ 'search' OR pages.content @@@ 'search')
ORDER BY score DESC, documents.id ASC, files.id ASC, pages.id ASC
LIMIT 10;

-- =============================================================================
-- TEST 6: 3-way join sorting by single-relation score
-- =============================================================================
-- Verifies that ORDER BY paradedb.score(pages.id) sorts solely by pages relevance
-- rather than the sum across tables.

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT documents.id AS doc_id,
       files.id AS file_id,
       pages.id AS page_id,
       paradedb.score(pages.id) AS page_score,
       paradedb.score(documents.id) AS doc_score
FROM documents
JOIN files ON documents.id = files."documentId"
JOIN pages ON pages."fileId" = files.id
WHERE documents.content @@@ 'postgres'
  AND files.content @@@ 'index'
  AND pages.content @@@ 'vector'
ORDER BY paradedb.score(pages.id) DESC, documents.id ASC, files.id ASC, pages.id ASC
LIMIT 10;

SELECT documents.id AS doc_id,
       files.id AS file_id,
       pages.id AS page_id,
       paradedb.score(pages.id) AS page_score,
       paradedb.score(documents.id) AS doc_score
FROM documents
JOIN files ON documents.id = files."documentId"
JOIN pages ON pages."fileId" = files.id
WHERE documents.content @@@ 'postgres'
  AND files.content @@@ 'index'
  AND pages.content @@@ 'vector'
ORDER BY paradedb.score(pages.id) DESC, documents.id ASC, files.id ASC, pages.id ASC
LIMIT 10;

-- =============================================================================
-- TEST 7: 3-way join with within-table disjunction (OR query in documents)
-- =============================================================================
-- documents matches 'postgres OR search': doc-1 matches both terms and gets a
-- higher BM25 score than doc-2 (which only matches 'postgres').

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT documents.id AS doc_id,
       files.id AS file_id,
       pages.id AS page_id,
       paradedb.score(documents.id) AS doc_score,
       paradedb.score(files.id) AS file_score,
       paradedb.score(pages.id) AS page_score,
       paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score
FROM documents
JOIN files ON documents.id = files."documentId"
JOIN pages ON pages."fileId" = files.id
WHERE documents.content @@@ 'postgres OR search'
  AND files.content @@@ 'index'
  AND pages.content @@@ 'vector'
ORDER BY score DESC, documents.id ASC, files.id ASC, pages.id ASC
LIMIT 10;

SELECT documents.id AS doc_id,
       files.id AS file_id,
       pages.id AS page_id,
       paradedb.score(documents.id) AS doc_score,
       paradedb.score(files.id) AS file_score,
       paradedb.score(pages.id) AS page_score,
       paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score
FROM documents
JOIN files ON documents.id = files."documentId"
JOIN pages ON pages."fileId" = files.id
WHERE documents.content @@@ 'postgres OR search'
  AND files.content @@@ 'index'
  AND pages.content @@@ 'vector'
ORDER BY score DESC, documents.id ASC, files.id ASC, pages.id ASC
LIMIT 10;

-- =============================================================================
-- CLEANUP
-- =============================================================================

DROP TABLE IF EXISTS pages CASCADE;
DROP TABLE IF EXISTS files CASCADE;
DROP TABLE IF EXISTS documents CASCADE;

RESET max_parallel_workers;
RESET max_parallel_workers_per_gather;
RESET parallel_leader_participation;
RESET enable_indexscan;
RESET paradedb.enable_join_custom_scan;
