-- Reproducer for "PlaceHolderVar found where not expected" error.
-- A 3-way join with pdb.score() summed across all three tables and
-- ORDER BY + LIMIT triggers PlaceHolderVar wrapping that the JoinScan
-- custom scan tlist did not handle.

SET max_parallel_workers_per_gather = 0;
SET enable_indexscan TO off;

CREATE EXTENSION IF NOT EXISTS pg_search;

-- =============================================================================
-- SETUP: minimal 3-table docs schema
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
('doc-1', 'project alpha notes', 'Document about project alpha', 'Alpha Doc'),
('doc-2', 'project beta notes', 'Document about project beta', 'Beta Doc');

INSERT INTO files (id, "documentId", content, title) VALUES
('file-1', 'doc-1', 'File content for alpha', 'collab12 alpha file'),
('file-2', 'doc-1', 'File content misc', 'collab12 misc file'),
('file-3', 'doc-2', 'File content for beta', 'beta file');

INSERT INTO pages (id, "fileId", content, title) VALUES
('page-1', 'file-1', 'Single Number Reach configuration', 'Page A'),
('page-2', 'file-1', 'Other page content', 'Page B'),
('page-3', 'file-2', 'Single Number Reach details', 'Page C'),
('page-4', 'file-3', 'Beta page content', 'Page D');

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

SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST: 3-way join with summed scores and ORDER BY score DESC LIMIT
-- This is the exact query pattern from the benchmark that triggers the error.
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score
FROM documents
JOIN files ON documents.id = files."documentId"
JOIN pages ON pages."fileId" = files.id
WHERE documents.parents @@@ 'project alpha'
  AND files.title @@@ 'collab12'
  AND pages.content @@@ 'Single Number Reach'
ORDER BY score DESC
LIMIT 1000;

SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score
FROM documents
JOIN files ON documents.id = files."documentId"
JOIN pages ON pages."fileId" = files.id
WHERE documents.parents @@@ 'project alpha'
  AND files.title @@@ 'collab12'
  AND pages.content @@@ 'Single Number Reach'
ORDER BY score DESC
LIMIT 1000;

-- =============================================================================
-- CLEANUP
-- =============================================================================

DROP TABLE IF EXISTS pages CASCADE;
DROP TABLE IF EXISTS files CASCADE;
DROP TABLE IF EXISTS documents CASCADE;

RESET max_parallel_workers_per_gather;
RESET enable_indexscan;
RESET paradedb.enable_join_custom_scan;
