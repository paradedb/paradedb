-- Test correlated subqueries with aggregate custom scan
CREATE EXTENSION IF NOT EXISTS pg_search;

-- Disable parallel workers to avoid differences in plans
SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;

-- Enable aggregate custom scan and filter pushdown (required for correlated subqueries)
SET paradedb.enable_aggregate_custom_scan = true;
SET paradedb.enable_filter_pushdown = true;

-- Drop any existing test tables
DROP TABLE IF EXISTS pages CASCADE;
DROP TABLE IF EXISTS files CASCADE;
DROP TABLE IF EXISTS documents CASCADE;

-- Create document tables for testing
CREATE TABLE documents (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    content TEXT,
    parents TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE TABLE files (
    id TEXT NOT NULL UNIQUE,
    documentId TEXT NOT NULL,
    title TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_size INTEGER,
    created_at TIMESTAMP DEFAULT NOW(),
    PRIMARY KEY (id, documentId),
    FOREIGN KEY (documentId) REFERENCES documents(id)
);

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

-- Create BM25 indexes with fast fields
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
    file_path,
    file_size
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

-- Insert sample data
INSERT INTO documents (id, title, content, parents) VALUES
('doc1', 'Invoice 2023', 'This is an invoice for services rendered in 2023', 'Factures'),
('doc2', 'Receipt 2023', 'This is a receipt for payment received in 2023', 'Factures'),
('doc3', 'Contract 2023', 'This is a contract for services in 2023', 'Contracts');

INSERT INTO files (id, documentId, title, file_path, file_size) VALUES
('file1', 'doc1', 'Invoice PDF', '/invoices/2023.pdf', 1024),
('file2', 'doc1', 'Invoice Receipt', '/invoices/2023_receipt.pdf', 512),
('file3', 'doc2', 'Receipt', '/receipts/2023.pdf', 256),
('file4', 'doc3', 'Contract Document', '/contracts/2023.pdf', 2048);

INSERT INTO pages (id, fileId, page_number, content) VALUES
('page1', 'file1', 1, 'Page 1 of Invoice PDF with Socienty General details'),
('page2', 'file1', 2, 'Page 2 of Invoice PDF with payment information'),
('page3', 'file2', 1, 'Page 1 of Invoice Receipt with bank details'),
('page4', 'file3', 1, 'Page 1 of Receipt with Socienty General information'),
('page5', 'file3', 2, 'Page 2 of Receipt with transaction ID'),
('page6', 'file4', 1, 'Page 1 of Contract Document with terms and conditions');

\echo '==========================================';
\echo 'Test 1: Basic correlated subquery with COUNT(*)';
\echo '==========================================';

-- EXPLAIN to verify aggregate custom scan is used
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT d.id, d.title,
    (
        SELECT COUNT(*)
        FROM files f
        WHERE f.documentId = d.id AND f.title @@@ 'Invoice'
    ) AS invoice_file_count
FROM documents d
WHERE d.parents @@@ 'Factures'
ORDER BY d.id;

-- Execute the query
SELECT d.id, d.title,
    (
        SELECT COUNT(*)
        FROM files f
        WHERE f.documentId = d.id AND f.title @@@ 'Invoice'
    ) AS invoice_file_count
FROM documents d
WHERE d.parents @@@ 'Factures'
ORDER BY d.id;

\echo '==========================================';
\echo 'Test 2: Verify correctness - compare with aggregate scan disabled';
\echo '==========================================';

-- Temporarily disable aggregate custom scan to get baseline results
SET paradedb.enable_aggregate_custom_scan = false;

SELECT d.id, d.title,
    (
        SELECT COUNT(*)
        FROM files f
        WHERE f.documentId = d.id AND f.title @@@ 'Invoice'
    ) AS invoice_file_count
FROM documents d
WHERE d.parents @@@ 'Factures'
ORDER BY d.id;

-- Re-enable aggregate custom scan
SET paradedb.enable_aggregate_custom_scan = true;

-- Results should match the previous query
SELECT d.id, d.title,
    (
        SELECT COUNT(*)
        FROM files f
        WHERE f.documentId = d.id AND f.title @@@ 'Invoice'
    ) AS invoice_file_count
FROM documents d
WHERE d.parents @@@ 'Factures'
ORDER BY d.id;

\echo '==========================================';
\echo 'Test 3: Correlated subquery with non-indexed field';
\echo '==========================================';

-- EXPLAIN with correlation using equality on non-indexed field
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT d.id, d.title,
    (
        SELECT COUNT(*)
        FROM files f
        WHERE f.documentId = d.id AND f.file_size > 500
    ) AS large_file_count
FROM documents d
WHERE d.parents @@@ 'Factures'
ORDER BY d.id;

-- Execute the query
SELECT d.id, d.title,
    (
        SELECT COUNT(*)
        FROM files f
        WHERE f.documentId = d.id AND f.file_size > 500
    ) AS large_file_count
FROM documents d
WHERE d.parents @@@ 'Factures'
ORDER BY d.id;

\echo '==========================================';
\echo 'Test 4: Empty result case';
\echo '==========================================';

-- Query that should return 0 for all documents
SELECT d.id, d.title,
    (
        SELECT COUNT(*)
        FROM files f
        WHERE f.documentId = d.id AND f.title @@@ 'NonExistent'
    ) AS nonexistent_count
FROM documents d
WHERE d.parents @@@ 'Factures'
ORDER BY d.id;

\echo '==========================================';
\echo 'Test 5: NULL handling in correlation';
\echo '==========================================';

-- Query where some documents might not have matching files
SELECT d.id, d.title,
    (
        SELECT COUNT(*)
        FROM files f
        WHERE f.documentId = d.id
    ) AS all_file_count
FROM documents d
WHERE d.parents @@@ 'Contracts' OR d.parents @@@ 'Factures'
ORDER BY d.id;

-- Cleanup
DROP TABLE IF EXISTS pages CASCADE;
DROP TABLE IF EXISTS files CASCADE;
DROP TABLE IF EXISTS documents CASCADE;

-- Reset settings
RESET max_parallel_workers_per_gather;
RESET enable_indexscan;
RESET paradedb.enable_aggregate_custom_scan;
RESET paradedb.enable_filter_pushdown;
