-- Test error cases and edge conditions for custom join hook
-- This test verifies proper error handling and edge case behavior

-- Create the extension first
CREATE EXTENSION IF NOT EXISTS pg_search;

-- Enable the custom join feature
SET paradedb.enable_custom_join = true;

-- Test 1: Empty tables
CREATE TABLE empty_docs (
    id SERIAL PRIMARY KEY,
    title TEXT,
    content TEXT
);

CREATE TABLE empty_files (
    id SERIAL PRIMARY KEY,
    document_id INTEGER,
    filename TEXT,
    content TEXT
);

-- Create BM25 indexes on empty tables
CREATE INDEX empty_docs_idx ON empty_docs USING bm25 (
    id,
    title,
    content
) WITH (
    key_field = 'id',
    text_fields = '{"title": {"tokenizer": {"type": "default"}}, "content": {"tokenizer": {"type": "default"}}}'
);

CREATE INDEX empty_files_idx ON empty_files USING bm25 (
    id,
    document_id,
    filename,
    content
) WITH (
    key_field = 'id',
    numeric_fields = '{"document_id": {"fast": true}}',
    text_fields = '{"filename": {"tokenizer": {"type": "default"}}, "content": {"tokenizer": {"type": "default"}}}'
);

-- Test join on empty tables
SELECT d.id, d.title, f.filename
FROM empty_docs d
JOIN empty_files f ON d.id = f.document_id
WHERE d.content @@@ 'nonexistent' AND f.content @@@ 'missing';

-- Test 2: Tables with no matching search results
CREATE TABLE no_match_docs (
    id SERIAL PRIMARY KEY,
    title TEXT,
    content TEXT
);

CREATE TABLE no_match_files (
    id SERIAL PRIMARY KEY,
    document_id INTEGER,
    filename TEXT,
    content TEXT
);

INSERT INTO no_match_docs (title, content) VALUES 
    ('Doc 1', 'This document contains cats and dogs'),
    ('Doc 2', 'This document contains birds and fish');

INSERT INTO no_match_files (document_id, filename, content) VALUES 
    (1, 'file1.txt', 'File about animals and pets'),
    (2, 'file2.txt', 'File about wildlife and nature');

CREATE INDEX no_match_docs_idx ON no_match_docs USING bm25 (
    id,
    title,
    content
) WITH (
    key_field = 'id',
    text_fields = '{"title": {"tokenizer": {"type": "default"}}, "content": {"tokenizer": {"type": "default"}}}'
);

CREATE INDEX no_match_files_idx ON no_match_files USING bm25 (
    id,
    document_id,
    filename,
    content
) WITH (
    key_field = 'id',
    numeric_fields = '{"document_id": {"fast": true}}',
    text_fields = '{"filename": {"tokenizer": {"type": "default"}}, "content": {"tokenizer": {"type": "default"}}}'
);

-- Test search with no matches
SELECT d.id, d.title, f.filename
FROM no_match_docs d
JOIN no_match_files f ON d.id = f.document_id
WHERE d.content @@@ 'robots' AND f.content @@@ 'spaceships';

-- Test 3: Mismatched join keys (no join matches)
CREATE TABLE mismatch_docs (
    id SERIAL PRIMARY KEY,
    title TEXT,
    content TEXT
);

CREATE TABLE mismatch_files (
    id SERIAL PRIMARY KEY,
    document_id INTEGER,
    filename TEXT,
    content TEXT
);

INSERT INTO mismatch_docs (title, content) VALUES 
    ('Doc 1', 'Technology document'),
    ('Doc 2', 'Science document');

INSERT INTO mismatch_files (document_id, filename, content) VALUES 
    (99, 'file1.txt', 'Technology file'),
    (100, 'file2.txt', 'Science file');

CREATE INDEX mismatch_docs_idx ON mismatch_docs USING bm25 (
    id,
    title,
    content
) WITH (
    key_field = 'id',
    text_fields = '{"title": {"tokenizer": {"type": "default"}}, "content": {"tokenizer": {"type": "default"}}}'
);

CREATE INDEX mismatch_files_idx ON mismatch_files USING bm25 (
    id,
    document_id,
    filename,
    content
) WITH (
    key_field = 'id',
    numeric_fields = '{"document_id": {"fast": true}}',
    text_fields = '{"filename": {"tokenizer": {"type": "default"}}, "content": {"tokenizer": {"type": "default"}}}'
);

-- Test join with mismatched keys
SELECT d.id, d.title, f.filename
FROM mismatch_docs d
JOIN mismatch_files f ON d.id = f.document_id
WHERE d.content @@@ 'technology' AND f.content @@@ 'file';

-- Test 4: Large result sets (stress test)
CREATE TABLE large_docs (
    id SERIAL PRIMARY KEY,
    title TEXT,
    content TEXT
);

CREATE TABLE large_files (
    id SERIAL PRIMARY KEY,
    document_id INTEGER,
    filename TEXT,
    content TEXT
);

-- Insert many rows to test performance
INSERT INTO large_docs (title, content) 
SELECT 
    'Document ' || i,
    'This is document number ' || i || ' with technology content'
FROM generate_series(1, 100) i;

INSERT INTO large_files (document_id, filename, content)
SELECT 
    (i % 100) + 1,
    'file' || i || '.txt',
    'This is file number ' || i || ' with technology content'
FROM generate_series(1, 500) i;

CREATE INDEX large_docs_idx ON large_docs USING bm25 (
    id,
    title,
    content
) WITH (
    key_field = 'id',
    text_fields = '{"title": {"tokenizer": {"type": "default"}}, "content": {"tokenizer": {"type": "default"}}}'
);

CREATE INDEX large_files_idx ON large_files USING bm25 (
    id,
    document_id,
    filename,
    content
) WITH (
    key_field = 'id',
    numeric_fields = '{"document_id": {"fast": true}}',
    text_fields = '{"filename": {"tokenizer": {"type": "default"}}, "content": {"tokenizer": {"type": "default"}}}'
);

-- Test large result set with LIMIT
SELECT d.id, d.title, f.filename
FROM large_docs d
JOIN large_files f ON d.id = f.document_id
WHERE d.content @@@ 'technology' AND f.content @@@ 'technology'
LIMIT 10;

-- Test 5: Complex search queries
CREATE TABLE complex_docs (
    id SERIAL PRIMARY KEY,
    title TEXT,
    content TEXT
);

CREATE TABLE complex_files (
    id SERIAL PRIMARY KEY,
    document_id INTEGER,
    filename TEXT,
    content TEXT
);

INSERT INTO complex_docs (title, content) VALUES 
    ('Advanced Technology', 'This document discusses advanced technology and artificial intelligence'),
    ('Basic Science', 'This document covers basic science and research methods'),
    ('Data Analysis', 'This document explains data analysis and machine learning');

INSERT INTO complex_files (document_id, filename, content) VALUES 
    (1, 'tech.txt', 'Advanced technology file with AI content'),
    (2, 'science.txt', 'Basic science file with research data'),
    (3, 'data.txt', 'Data analysis file with ML algorithms');

CREATE INDEX complex_docs_idx ON complex_docs USING bm25 (
    id,
    title,
    content
) WITH (
    key_field = 'id',
    text_fields = '{"title": {"tokenizer": {"type": "default"}}, "content": {"tokenizer": {"type": "default"}}}'
);

CREATE INDEX complex_files_idx ON complex_files USING bm25 (
    id,
    document_id,
    filename,
    content
) WITH (
    key_field = 'id',
    numeric_fields = '{"document_id": {"fast": true}}',
    text_fields = '{"filename": {"tokenizer": {"type": "default"}}, "content": {"tokenizer": {"type": "default"}}}'
);

-- Test complex boolean queries
SELECT d.id, d.title, f.filename
FROM complex_docs d
JOIN complex_files f ON d.id = f.document_id
WHERE d.content @@@ 'technology AND artificial' AND f.content @@@ 'AI OR algorithms';

-- Test phrase queries
SELECT d.id, d.title, f.filename
FROM complex_docs d
JOIN complex_files f ON d.id = f.document_id
WHERE d.content @@@ '"data analysis"' AND f.content @@@ '"ML algorithms"';

-- Cleanup
DROP TABLE empty_docs CASCADE;
DROP TABLE empty_files CASCADE;
DROP TABLE no_match_docs CASCADE;
DROP TABLE no_match_files CASCADE;
DROP TABLE mismatch_docs CASCADE;
DROP TABLE mismatch_files CASCADE;
DROP TABLE large_docs CASCADE;
DROP TABLE large_files CASCADE;
DROP TABLE complex_docs CASCADE;
DROP TABLE complex_files CASCADE;

RESET paradedb.enable_custom_join; 
