-- Simple test to confirm join materialization issue
-- This creates a minimal reproduction case

-- Create small test tables
CREATE TABLE test_docs (
    id SERIAL PRIMARY KEY,
    content TEXT
);

CREATE TABLE test_files (
    id SERIAL PRIMARY KEY,
    doc_id INTEGER REFERENCES test_docs(id),
    filename TEXT
);

-- Insert minimal data - just enough to see the pattern
INSERT INTO test_docs (content) VALUES 
('document one'), ('document two'), ('document three');

INSERT INTO test_files (doc_id, filename) VALUES 
(1, 'file1.txt'), (1, 'file2.txt'),
(2, 'file3.txt'), (2, 'file4.txt'),
(3, 'file5.txt'), (3, 'file6.txt');

-- Create ParadeDB indexes
CREATE INDEX test_docs_idx ON test_docs USING bm25 (id, content) WITH (key_field='id');
CREATE INDEX test_files_idx ON test_files USING bm25 (id, doc_id, filename) WITH (key_field='id');

-- Force ParadeDB usage
SET enable_indexscan = off;
SET enable_bitmapscan = off;
SET max_parallel_workers_per_gather = 0;

-- Test 1: Single table - should use TopNScanExecState
EXPLAIN (ANALYZE, VERBOSE)
SELECT id, content, paradedb.score(id) as score
FROM test_docs 
WHERE content @@@ 'document'
ORDER BY score DESC 
LIMIT 1;

EXPLAIN (ANALYZE, VERBOSE)
SELECT id, content, paradedb.score(id) as score
FROM test_docs 
WHERE content LIKE 'document%'
ORDER BY score DESC 
LIMIT 1;

-- Test 2: Join - should NOT use TopNScanExecState and materialize everything
EXPLAIN (ANALYZE, VERBOSE)
SELECT d.id, f.id, 
       paradedb.score(d.id) + paradedb.score(f.id) as combined_score
FROM test_docs d 
JOIN test_files f ON d.id = f.doc_id
WHERE d.content @@@ 'document' AND f.filename @@@ 'file'
ORDER BY combined_score DESC
LIMIT 1;

EXPLAIN (ANALYZE, VERBOSE)
SELECT d.id, f.id, 
       paradedb.score(d.id) + paradedb.score(f.id) as combined_score
FROM test_docs d 
JOIN test_files f ON d.id = f.doc_id
WHERE d.content LIKE 'document%' AND f.filename LIKE 'file%'
ORDER BY combined_score DESC
LIMIT 1;

-- Test 3: Join - should NOT use TopNScanExecState and materialize everything
EXPLAIN (ANALYZE, VERBOSE)
SELECT d.id, f.id, 
       paradedb.score(d.id) as score_d,
       paradedb.score(f.id) as score_f
FROM test_docs d 
JOIN test_files f ON d.id = f.doc_id
WHERE d.content @@@ 'document' AND f.filename @@@ 'file'
ORDER BY score_d DESC
LIMIT 1;

EXPLAIN (ANALYZE, VERBOSE)
SELECT d.id, f.id, 
       paradedb.score(d.id) as score_d,
       paradedb.score(f.id) as score_f
FROM test_docs d 
JOIN test_files f ON d.id = f.doc_id
WHERE d.content LIKE 'document%' AND f.filename LIKE 'file%'
ORDER BY score_d DESC
LIMIT 1;

-- Test 3: Join - should NOT use TopNScanExecState and materialize everything
EXPLAIN (ANALYZE, VERBOSE)
SELECT d.id, f.id, 
       paradedb.score(d.id) as score_d,
       paradedb.score(f.id) as score_f
FROM test_docs d 
JOIN test_files f ON d.id = f.doc_id
WHERE d.content @@@ 'document' AND f.filename @@@ 'file'
ORDER BY score_d, score_f DESC
LIMIT 1;

EXPLAIN (ANALYZE, VERBOSE)
SELECT d.id, f.id, 
       paradedb.score(d.id) as score_d,
       paradedb.score(f.id) as score_f
FROM test_docs d 
JOIN test_files f ON d.id = f.doc_id
WHERE d.content LIKE 'document%' AND f.filename LIKE 'file%'
ORDER BY score_d, score_f DESC
LIMIT 1;

-- The key difference to look for:
-- 1. Single table: Shows "TopNScanExecState" and "Top N Limit: 1"
-- 2. Join: Shows regular exec methods and processes ALL join results before limiting

-- Cleanup
DROP TABLE test_files;
DROP TABLE test_docs; 
