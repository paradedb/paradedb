-- Reproduction case for join materialization issue with ORDER BY LIMIT
-- This demonstrates how PostgreSQL materializes the entire join before sorting and limiting

-- Create test tables with sufficient data to see the issue
CREATE TABLE docs (
    id SERIAL PRIMARY KEY,
    title TEXT,
    content TEXT
);

CREATE TABLE files (
    id SERIAL PRIMARY KEY,
    doc_id INTEGER REFERENCES docs(id),
    filename TEXT,
    data TEXT
);

CREATE TABLE pages (
    id SERIAL PRIMARY KEY,
    file_id INTEGER REFERENCES files(id),
    page_num INTEGER,
    text_content TEXT
);

-- Insert test data
INSERT INTO docs (title, content) 
SELECT 'Document ' || i, 'Content for document ' || i 
FROM generate_series(1, 10000) i;

INSERT INTO files (doc_id, filename, data)
SELECT 
    (i % 10000) + 1,
    'file_' || i || '.txt',
    'Data for file ' || i
FROM generate_series(1, 50000) i;

INSERT INTO pages (file_id, page_num, text_content)
SELECT 
    (i % 50000) + 1,
    (i % 10) + 1,
    'Page content ' || i || ' with some text'
FROM generate_series(1, 200000) i;

-- Create ParadeDB indexes
CREATE INDEX docs_idx ON docs USING bm25 (id, title, content) WITH (key_field='id');
CREATE INDEX files_idx ON files USING bm25 (id, doc_id, filename, data) WITH (key_field='id');
CREATE INDEX pages_idx ON pages USING bm25 (id, file_id, page_num, text_content) WITH (key_field='id');

-- Wait for indexing to complete
SELECT pg_sleep(2);

-- Force custom scan usage
SET enable_indexscan = off;
SET enable_bitmapscan = off;
SET max_parallel_workers_per_gather = 0;  -- Disable parallelism initially

-- Test 1: Single table with ORDER BY + LIMIT (should use TopN)
EXPLAIN (ANALYZE, VERBOSE, BUFFERS)
SELECT id, title, paradedb.score(id) as score
FROM docs 
WHERE content @@@ 'document'
ORDER BY score DESC 
LIMIT 100;

-- Test 2: JOIN without ORDER BY + LIMIT (no TopN possible)
EXPLAIN (ANALYZE, VERBOSE, BUFFERS)
SELECT d.id, f.id, d.title, f.filename,
       paradedb.score(d.id) + paradedb.score(f.id) as combined_score
FROM docs d 
JOIN files f ON d.id = f.doc_id
WHERE d.content @@@ 'document' AND f.data @@@ 'file'
LIMIT 100;

-- Test 3: JOIN with ORDER BY + LIMIT (should materialize join first)
EXPLAIN (ANALYZE, VERBOSE, BUFFERS)
SELECT d.id, f.id, d.title, f.filename,
       paradedb.score(d.id) + paradedb.score(f.id) as combined_score
FROM docs d 
JOIN files f ON d.id = f.doc_id
WHERE d.content @@@ 'document' AND f.data @@@ 'file'
ORDER BY combined_score DESC
LIMIT 100;

-- Test 4: Three-way JOIN with ORDER BY + LIMIT (even worse materialization)
EXPLAIN (ANALYZE, VERBOSE, BUFFERS)
SELECT d.id, f.id, p.id,
       paradedb.score(d.id) + paradedb.score(f.id) + paradedb.score(p.id) as total_score
FROM docs d 
JOIN files f ON d.id = f.doc_id
JOIN pages p ON f.id = p.file_id
WHERE d.content @@@ 'document' 
  AND f.data @@@ 'file'
  AND p.text_content @@@ 'content'
ORDER BY total_score DESC
LIMIT 100;

-- Test 5: Enable parallelism and see the issue from the original query
SET max_parallel_workers_per_gather = 8;
SET max_parallel_workers = 8;

EXPLAIN (ANALYZE, VERBOSE, BUFFERS)
SELECT d.id, f.id, p.id,
       paradedb.score(d.id) + paradedb.score(f.id) + paradedb.score(p.id) as total_score
FROM docs d 
JOIN files f ON d.id = f.doc_id
JOIN pages p ON f.id = p.file_id
WHERE d.content @@@ 'document' 
  AND f.data @@@ 'file'
  AND p.text_content @@@ 'content'
ORDER BY total_score DESC
LIMIT 100;

-- Test 6: Compare with PostgreSQL's handling of regular indexes
-- Create regular btree indexes for comparison
CREATE INDEX docs_title_idx ON docs(title);
CREATE INDEX files_filename_idx ON files(filename);

SET enable_indexscan = on;
SET enable_bitmapscan = on;

EXPLAIN (ANALYZE, VERBOSE, BUFFERS)
SELECT d.id, f.id, d.title, f.filename
FROM docs d 
JOIN files f ON d.id = f.doc_id
WHERE d.title LIKE 'Document 1%' 
  AND f.filename LIKE 'file_1%'
ORDER BY d.title, f.filename
LIMIT 100; 
