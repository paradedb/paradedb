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

DROP TABLE IF EXISTS pages CASCADE;
CREATE TABLE pages (
    id TEXT NOT NULL UNIQUE,
    fileId TEXT NOT NULL,
    page_number INTEGER NOT NULL,
    content TEXT NOT NULL,
    metadata JSONB,
    created_at TIMESTAMP DEFAULT NOW(),
    PRIMARY KEY (id, fileId),
    FOREIGN KEY (fileId) REFERENCES files(id)
);

DROP INDEX IF EXISTS documents_search CASCADE;
DROP INDEX IF EXISTS files_search CASCADE;
DROP INDEX IF EXISTS pages_search CASCADE;

INSERT INTO documents (id, title, content, parents)
SELECT
  md5(i::text || 'documents')::uuid::text,
  'Invoice 2023',
  'This is an invoice for services rendered in 2023',
  'Factures'
FROM generate_series(1,15000) AS s(i);

INSERT INTO files (id, documentId, title, file_path, file_size)
SELECT
  md5(i::text || 'files')::uuid::text,
  (
    SELECT id
    FROM   documents
    WHERE  id = md5((i % 15000 + 1)::text || 'documents')::uuid::text
  ),
  'Receipt',
  '/files/path',
  15000
FROM generate_series(1,15000) AS s(i);

INSERT INTO pages (id, fileId, page_number, content, metadata)
SELECT
  md5(i::text || 'pages')::uuid::text,
  (
    SELECT id
    FROM   files
    WHERE  id = md5((i % 15000 + 1)::text || 'files')::uuid::text
  ),
  (i % 100 + 1)::int,
  'Socienty',
  '{"color":"red"}'::json
FROM generate_series(1,15000) AS s(i);

-- Create BM25 indexes with fast fields using inline field options
CREATE INDEX documents_search ON documents USING bm25 (
    id,
    title,
    parents,
    content
) WITH (
    key_field = 'id',
    text_fields = '{"title": {"tokenizer": {"type": "default"}, "fast": true}, "parents": {"tokenizer": {"type": "default"}, "fast": true}, "content": {"tokenizer": {"type": "default"}, "fast": true}}'
);

CREATE INDEX files_search ON files USING bm25 (
    id,
    documentId,
    title,
    file_path
) WITH (
    key_field = 'id',
    text_fields = '{"documentid": {"tokenizer": {"type": "keyword"}, "fast": true}, "title": {"tokenizer": {"type": "default"}, "fast": true}, "file_path": {"tokenizer": {"type": "default"}, "fast": true}}'
);

CREATE INDEX pages_search ON pages USING bm25 (
    id,
    fileId,
    content,
    page_number
) WITH (
    key_field = 'id',
    text_fields = '{"fileid": {"tokenizer": {"type": "keyword"}, "fast": true}, "content": {"tokenizer": {"type": "default"}, "fast": true}}',
    numeric_fields = '{"page_number": {"fast": true}}'
);

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
LIMIT 10;

\i common/mixedff_queries_cleanup.sql
