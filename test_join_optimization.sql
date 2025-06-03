-- Test script to verify the JOIN + ORDER BY + LIMIT optimization fix
-- This should demonstrate that ParadeDB custom scans now claim sorted output
-- allowing PostgreSQL to use efficient top-N heapsort

-- Setup test tables
CREATE TABLE test_docs_opt (
    id SERIAL PRIMARY KEY,
    content TEXT
);

CREATE TABLE test_files_opt (
    id SERIAL PRIMARY KEY,
    doc_id INTEGER REFERENCES test_docs_opt(id),
    filename TEXT
);

-- Insert test data
INSERT INTO test_docs_opt (content) VALUES 
('document one'), ('document two'), ('document three');

INSERT INTO test_files_opt (doc_id, filename) VALUES 
(1, 'file1.txt'), (1, 'file2.txt'),
(2, 'file3.txt'), (2, 'file4.txt'),
(3, 'file5.txt'), (3, 'file6.txt');

-- Create ParadeDB indexes
CREATE INDEX test_docs_opt_idx ON test_docs_opt USING bm25 (id, content) WITH (key_field='id');
CREATE INDEX test_files_opt_idx ON test_files_opt USING bm25 (id, doc_id, filename) WITH (key_field='id');

-- Force ParadeDB usage
SET enable_indexscan = off;
SET enable_bitmapscan = off;
SET max_parallel_workers_per_gather = 0;

-- Enable mixed fast field execution to get better execution methods
SET paradedb.enable_mixed_fast_field_exec = true;

-- Test the fix: this should now show "top-N heapsort" instead of "quicksort"
-- because ParadeDB custom scans should now claim to provide sorted output
EXPLAIN (ANALYZE, VERBOSE)
SELECT d.id, f.id, 
       paradedb.score(d.id) + paradedb.score(f.id) as combined_score
FROM test_docs_opt d 
JOIN test_files_opt f ON d.id = f.doc_id
WHERE d.content @@@ 'document' AND f.filename @@@ 'file'
ORDER BY combined_score DESC
LIMIT 1;

-- Test with score only (should also use top-N heapsort)
EXPLAIN (ANALYZE, VERBOSE)
SELECT d.id, paradedb.score(d.id) as score
FROM test_docs_opt d 
WHERE d.content @@@ 'document'
ORDER BY score DESC
LIMIT 1;

-- Test with field ordering (should also work)
EXPLAIN (ANALYZE, VERBOSE)  
SELECT d.content, f.filename
FROM test_docs_opt d 
JOIN test_files_opt f ON d.id = f.doc_id
WHERE d.content @@@ 'document' AND f.filename @@@ 'file'
ORDER BY d.content, f.filename
LIMIT 1;

-- Cleanup
DROP TABLE test_files_opt;
DROP TABLE test_docs_opt; 
