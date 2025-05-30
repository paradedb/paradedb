-- Test basic join hook functionality
-- This test verifies that our custom join hook is being called

-- Create the extension first
CREATE EXTENSION IF NOT EXISTS pg_search;

-- Enable the custom join feature
SET paradedb.enable_custom_join = true;

SET paradedb.enable_topn_join_optimization = true;

-- Create test tables with BM25 indexes
CREATE TABLE documents_join_test (
    id SERIAL PRIMARY KEY,
    title TEXT,
    content TEXT
);

CREATE TABLE files_join_test (
    id SERIAL PRIMARY KEY,
    document_id INTEGER,
    filename TEXT,
    content TEXT
);

-- Create a third table for 3-way joins
CREATE TABLE authors_join_test (
    id SERIAL PRIMARY KEY,
    document_id INTEGER,
    author_name TEXT,
    bio TEXT
);

-- Insert test data
INSERT INTO documents_join_test (title, content) VALUES 
    ('Document 1', 'This is the first document about technology'),
    ('Document 2', 'This is the second document about science and research'),
    ('Document 3', 'This is the third document about research and data analysis');

INSERT INTO files_join_test (document_id, filename, content) VALUES 
    (1, 'file1.txt', 'Technology file content with data'),
    (2, 'file2.txt', 'Science file content with research data'),
    (3, 'file3.txt', 'Research file content and analysis data');

INSERT INTO authors_join_test (document_id, author_name, bio) VALUES 
    (1, 'John Smith', 'Expert in technology and innovation with research background'),
    (2, 'Jane Doe', 'Scientist specializing in research methods and data analysis'),
    (3, 'Bob Wilson', 'Research analyst with focus on data science and research');

-- Create BM25 indexes

CREATE INDEX documents_join_test_idx ON documents_join_test USING bm25 (
    id,
    title,
    content
) WITH (
    key_field = 'id',
    text_fields = '{"title": {"tokenizer": {"type": "default"}}, "content": {"tokenizer": {"type": "default"}}}'
);

CREATE INDEX files_join_test_idx ON files_join_test USING bm25 (
    id,
    document_id,
    filename,
    content
) WITH (
    key_field = 'id',
    numeric_fields = '{"document_id": {"fast": true}}',
    text_fields = '{"filename": {"tokenizer": {"type": "default"}}, "content": {"tokenizer": {"type": "default"}}}'
);

CREATE INDEX authors_join_test_idx ON authors_join_test USING bm25 (
    id,
    document_id,
    author_name,
    bio
) WITH (
    key_field = 'id',
    numeric_fields = '{"document_id": {"fast": true}}',
    text_fields = '{"author_name": {"tokenizer": {"type": "default"}}, "bio": {"tokenizer": {"type": "default"}}}'
);

-- Test 1: Simple INNER JOIN with search predicates
-- This should trigger our join hook and show debug output
SELECT d.id, d.title, f.filename
FROM documents_join_test d
JOIN files_join_test f ON d.id = f.document_id
WHERE d.content @@@ 'technology' AND f.content @@@ 'file';

-- Test 2: JOIN without search predicates (should not trigger custom join)
SELECT d.id, d.title, f.filename  
FROM documents_join_test d
JOIN files_join_test f ON d.id = f.document_id
WHERE d.id = 1;

-- Test 3: Search on only one side (unilateral join)
SELECT d.id, d.title, f.filename
FROM documents_join_test d  
JOIN files_join_test f ON d.id = f.document_id
WHERE d.content @@@ 'science';

-- Test 4: 3-way join with bilateral search (documents + files)
SELECT d.id, d.title, f.filename, a.author_name
FROM documents_join_test d
JOIN files_join_test f ON d.id = f.document_id
JOIN authors_join_test a ON d.id = a.document_id
WHERE d.content @@@ 'technology' AND f.content @@@ 'file';

-- Test 5: 3-way join with trilateral search (all three tables)
SELECT d.id, d.title, f.filename, a.author_name
FROM documents_join_test d
JOIN files_join_test f ON d.id = f.document_id
JOIN authors_join_test a ON d.id = a.document_id
WHERE d.content @@@ 'science' AND f.content @@@ 'file' AND a.bio @@@ 'research';

-- Test 6: 3-way join with unilateral search (only documents)
SELECT d.id, d.title, f.filename, a.author_name
FROM documents_join_test d
JOIN files_join_test f ON d.id = f.document_id
JOIN authors_join_test a ON d.id = a.document_id
WHERE d.content @@@ 'research';

-- Cleanup
DROP TABLE documents_join_test CASCADE;
DROP TABLE files_join_test CASCADE;
DROP TABLE authors_join_test CASCADE;

RESET paradedb.enable_custom_join;
RESET paradedb.enable_topn_join_optimization;
