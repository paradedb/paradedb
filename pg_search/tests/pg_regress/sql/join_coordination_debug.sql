-- Debug test for JOIN coordination
-- This test helps us understand why JOIN coordination is not being triggered

CREATE EXTENSION IF NOT EXISTS pg_search;

-- Create simple test tables
CREATE TABLE test_docs (
    id SERIAL PRIMARY KEY,
    content TEXT
);

CREATE TABLE test_files (
    id SERIAL PRIMARY KEY,
    doc_id INTEGER REFERENCES test_docs(id),
    title TEXT
);

-- Create BM25 indexes
CALL paradedb.create_bm25_test_table(
    table_name => 'test_docs',
    schema_name => 'public'
);

CALL paradedb.create_bm25_test_table(
    table_name => 'test_files',
    schema_name => 'public'
);

CREATE INDEX test_docs_search_idx ON test_docs USING bm25 (id, content) WITH (key_field='id');
CREATE INDEX test_files_search_idx ON test_files USING bm25 (id, doc_id, title) WITH (key_field='id');

-- Insert test data
INSERT INTO test_docs (content) VALUES
    ('technology document'),
    ('science document');

INSERT INTO test_files (doc_id, title) VALUES
    (1, 'tech report'),
    (2, 'science report');

-- Test: Multi-table query with search predicates and LIMIT
-- This should trigger JOIN coordination if our logic is working
EXPLAIN (ANALYZE, BUFFERS, VERBOSE)
SELECT d.id, f.id
FROM test_docs d
JOIN test_files f ON d.id = f.doc_id
WHERE d.content @@@ 'technology' 
  AND f.title @@@ 'report'
LIMIT 5;

-- Cleanup
DROP TABLE test_files;
DROP TABLE test_docs; 
