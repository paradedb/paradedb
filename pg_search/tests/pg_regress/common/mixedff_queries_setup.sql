CREATE EXTENSION IF NOT EXISTS pg_search;

-- Disable parallel workers to avoid differences in plans
SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;
SET paradedb.enable_mixed_fast_field_exec = true;
-- The `mixedff` tests allow any number of columns to be used with fast fields, in order to test
-- more permutations of selected columns.
SET paradedb.mixed_fast_field_exec_column_threshold = 100;

-- Drop any existing test tables from this group
DROP TABLE IF EXISTS documents CASCADE;
DROP TABLE IF EXISTS files CASCADE;
DROP TABLE IF EXISTS pages CASCADE;

-- Create document tables for testing relational queries
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
    text_fields = '{"fileid": {"tokenizer": {"type": "keyword"}, "fast": true}, "content": {"tokenizer": {"type": "default"}}}',
    numeric_fields = '{"page_number": {"fast": true}}'
);

-- Insert sample data for documents
INSERT INTO documents (id, title, content, parents) VALUES
('doc1', 'Invoice 2023', 'This is an invoice for services rendered in 2023', 'Factures'),
('doc2', 'Receipt 2023', 'This is a receipt for payment received in 2023', 'Factures'),
('doc3', 'Contract 2023', 'This is a contract for services in 2023', 'Contracts');

-- Insert sample data for files
INSERT INTO files (id, documentId, title, file_path, file_size) VALUES
('file1', 'doc1', 'Invoice PDF', '/invoices/2023.pdf', 1024),
('file2', 'doc1', 'Invoice Receipt', '/invoices/2023_receipt.pdf', 512),
('file3', 'doc2', 'Receipt', '/receipts/2023.pdf', 256),
('file4', 'doc3', 'Contract Document', '/contracts/2023.pdf', 2048);

-- Insert sample data for pages
INSERT INTO pages (id, fileId, page_number, content) VALUES
('page1', 'file1', 1, 'Page 1 of Invoice PDF with Socienty General details'),
('page2', 'file1', 2, 'Page 2 of Invoice PDF with payment information'),
('page3', 'file2', 1, 'Page 1 of Invoice Receipt with bank details'),
('page4', 'file3', 1, 'Page 1 of Receipt with Socienty General information'),
('page5', 'file3', 2, 'Page 2 of Receipt with transaction ID'),
('page6', 'file4', 1, 'Page 1 of Contract Document with terms and conditions');

-- Add data for CTE testing
INSERT INTO documents (id, title, content, parents) VALUES
('doc_cte1', 'CTE Test Doc 1', 'This document tests common table expressions', 'Reports'),
('doc_cte2', 'CTE Test Doc 2', 'Another document for CTE testing', 'Reports');

INSERT INTO files (id, documentId, title, file_path, file_size) VALUES
('file_cte1', 'doc_cte1', 'CTE Test File 1', '/reports/cte1.pdf', 500),
('file_cte2', 'doc_cte1', 'CTE Test File 2', '/reports/cte2.pdf', 600),
('file_cte3', 'doc_cte2', 'CTE Test File 3', '/reports/cte3.pdf', 700);

INSERT INTO pages (id, fileId, page_number, content) VALUES
('page_cte1', 'file_cte1', 1, 'Page 1 with searchable content for CTE testing'),
('page_cte2', 'file_cte1', 2, 'Page 2 with more content for testing'),
('page_cte3', 'file_cte2', 1, 'Another page with test terms to search'),
('page_cte4', 'file_cte3', 1, 'Final test page for CTE testing'); 
