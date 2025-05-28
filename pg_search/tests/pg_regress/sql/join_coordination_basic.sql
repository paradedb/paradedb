-- Test JOIN coordination execution method
-- This test validates that the JOIN coordination execution method is selected
-- for multi-table queries with search predicates and LIMIT clauses

-- Create the extension first
CREATE EXTENSION IF NOT EXISTS pg_search;

-- Create test tables with BM25 indexes
CREATE TABLE documents (
    id SERIAL PRIMARY KEY,
    title TEXT,
    content TEXT
);

CREATE TABLE files (
    id SERIAL PRIMARY KEY,
    document_id INTEGER REFERENCES documents(id),
    filename TEXT,
    title TEXT
);

CREATE TABLE pages (
    id SERIAL PRIMARY KEY,
    file_id INTEGER REFERENCES files(id),
    page_number INTEGER,
    content TEXT
);

-- Create BM25 indexes
CALL paradedb.create_bm25_test_table(
    table_name => 'documents',
    schema_name => 'public'
);

CALL paradedb.create_bm25_test_table(
    table_name => 'files',
    schema_name => 'public'
);

CALL paradedb.create_bm25_test_table(
    table_name => 'pages',
    schema_name => 'public'
);

-- Create the actual BM25 indexes
CREATE INDEX documents_search_idx ON documents USING bm25 (id, title, content) WITH (key_field='id');
CREATE INDEX files_search_idx ON files USING bm25 (id, document_id, filename, title) WITH (key_field='id');
CREATE INDEX pages_search_idx ON pages USING bm25 (id, file_id, page_number, content) WITH (key_field='id');

-- Insert test data
INSERT INTO documents (title, content) VALUES
    ('Document 1', 'This is the first document about technology'),
    ('Document 2', 'This is the second document about science'),
    ('Document 3', 'This is the third document about research');

INSERT INTO files (document_id, filename, title) VALUES
    (1, 'file1.pdf', 'Technology Report'),
    (1, 'file2.pdf', 'Tech Analysis'),
    (2, 'file3.pdf', 'Science Paper'),
    (3, 'file4.pdf', 'Research Notes');

INSERT INTO pages (file_id, page_number, content) VALUES
    (1, 1, 'Introduction to technology trends'),
    (1, 2, 'Advanced technology concepts'),
    (2, 1, 'Technology analysis methodology'),
    (3, 1, 'Scientific research methods'),
    (4, 1, 'Research findings and conclusions');

-- Test 1: Single table query (should NOT use JOIN coordination)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT d.id, d.title
FROM documents d
WHERE d.content @@@ 'technology'
LIMIT 10;

-- Test 2: Multi-table query with search predicates and LIMIT (should use JOIN coordination)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT d.id, f.id, p.id
FROM documents d
JOIN files f ON d.id = f.document_id
JOIN pages p ON f.id = p.file_id
WHERE d.content @@@ 'technology' 
  AND f.title @@@ 'report'
  AND p.content @@@ 'introduction'
LIMIT 10;

-- Test 3: Multi-table query without LIMIT (should NOT use JOIN coordination)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT d.id, f.id
FROM documents d
JOIN files f ON d.id = f.document_id
WHERE d.content @@@ 'technology' 
  AND f.title @@@ 'report';

-- Test 4: Multi-table query with only one search predicate (should NOT use JOIN coordination)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT d.id, f.id
FROM documents d
JOIN files f ON d.id = f.document_id
WHERE d.content @@@ 'technology'
LIMIT 10;

-- Test 5: Verify that results are correct
SELECT d.title, f.filename, p.content
FROM documents d
JOIN files f ON d.id = f.document_id
JOIN pages p ON f.id = p.file_id
WHERE d.content @@@ 'technology' 
  AND f.title @@@ 'report'
  AND p.content @@@ 'introduction'
LIMIT 10;

-- Cleanup
DROP TABLE pages;
DROP TABLE files;
DROP TABLE documents; 
