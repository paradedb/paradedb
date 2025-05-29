-- Test JOIN coordination for multi-table search scenarios
-- This test verifies that our JOIN pathlist hook is called and creates custom paths

-- Create the extension
CREATE EXTENSION IF NOT EXISTS pg_search;

-- Setup test data
CREATE TABLE test_documents (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    content TEXT NOT NULL
);

CREATE TABLE test_files (
    id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL REFERENCES test_documents(id),
    filename TEXT NOT NULL,
    content TEXT NOT NULL
);

-- Create BM25 indexes using the correct syntax
CREATE INDEX test_documents_search ON test_documents 
USING bm25 (id, title, content) WITH (key_field='id');

CREATE INDEX test_files_search ON test_files 
USING bm25 (id, document_id, filename, content) WITH (key_field='id');

-- Insert test data
INSERT INTO test_documents (id, title, content) VALUES
('doc1', 'Test Document 1', 'This is a test document about SFR'),
('doc2', 'Test Document 2', 'Another document with different content'),
('doc3', 'Test Document 3', 'Third document mentioning SFR again');

INSERT INTO test_files (id, document_id, filename, content) VALUES
('file1', 'doc1', 'collab12_report.txt', 'Collaboration report content'),
('file2', 'doc1', 'other_file.txt', 'Other file content'),
('file3', 'doc2', 'collab12_summary.txt', 'Summary with collab12 keyword'),
('file4', 'doc3', 'regular_file.txt', 'Regular file without keywords');

-- Test 1: Simple JOIN with search predicates and LIMIT
-- This should trigger our JOIN coordination hook
SELECT d.id, f.id 
FROM test_documents d 
JOIN test_files f ON d.id = f.document_id
WHERE d.content @@@ 'SFR' AND f.filename @@@ 'collab12'
LIMIT 5;

-- Test 2: EXPLAIN to see if our custom path is created
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT d.id, f.id 
FROM test_documents d 
JOIN test_files f ON d.id = f.document_id
WHERE d.content @@@ 'SFR' AND f.filename @@@ 'collab12'
LIMIT 5;

-- Test 3: JOIN without LIMIT (should not trigger coordination)
SELECT d.id, f.id 
FROM test_documents d 
JOIN test_files f ON d.id = f.document_id
WHERE d.content @@@ 'SFR' AND f.filename @@@ 'collab12';

-- Test 4: JOIN with only one search predicate (should not trigger coordination)
SELECT d.id, f.id 
FROM test_documents d 
JOIN test_files f ON d.id = f.document_id
WHERE d.content @@@ 'SFR'
LIMIT 5;

-- Cleanup
DROP TABLE test_files;
DROP TABLE test_documents;

-- Test 5: Enable JOIN coordination GUC and verify it works
SET paradedb.enable_join_coordination = true;

-- Recreate tables for testing with GUC enabled
CREATE TABLE test_documents (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    content TEXT NOT NULL
);

CREATE TABLE test_files (
    id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL REFERENCES test_documents(id),
    filename TEXT NOT NULL,
    content TEXT NOT NULL
);

-- Create BM25 indexes
CREATE INDEX test_documents_search ON test_documents 
USING bm25 (id, title, content) WITH (key_field='id');

CREATE INDEX test_files_search ON test_files 
USING bm25 (id, document_id, filename, content) WITH (key_field='id');

-- Insert test data
INSERT INTO test_documents (id, title, content) VALUES
('doc1', 'Test Document 1', 'This is a test document about SFR');

INSERT INTO test_files (id, document_id, filename, content) VALUES
('file1', 'doc1', 'collab12_report.txt', 'Collaboration report content');

-- Test with JOIN coordination enabled - should trigger the JOIN pathlist callback
SELECT d.id, f.id 
FROM test_documents d 
JOIN test_files f ON d.id = f.document_id
WHERE d.content @@@ 'SFR' AND f.filename @@@ 'collab12'
LIMIT 5;

-- Reset GUC to default
SET paradedb.enable_join_coordination = false;

-- Cleanup
DROP TABLE test_files;
DROP TABLE test_documents; 
