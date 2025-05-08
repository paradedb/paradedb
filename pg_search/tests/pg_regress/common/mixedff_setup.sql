CREATE EXTENSION IF NOT EXISTS pg_search;

-- This setup file creates the basic schema needed for all tests

-- Drop any existing test tables
DROP TABLE IF EXISTS documents CASCADE;
DROP TABLE IF EXISTS files CASCADE;
DROP TABLE IF EXISTS pages CASCADE;
DROP TABLE IF EXISTS mixed_numeric_string_test CASCADE;
DROP TABLE IF EXISTS corner_case_test CASCADE;
DROP TABLE IF EXISTS nullable_test CASCADE;

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

-- Create test table for mixed numeric/string testing
CREATE TABLE mixed_numeric_string_test (
    id TEXT PRIMARY KEY,
    numeric_field1 INTEGER NOT NULL,
    numeric_field2 BIGINT NOT NULL,
    string_field1 TEXT NOT NULL,
    string_field2 TEXT NOT NULL,
    string_field3 TEXT NOT NULL,
    content TEXT
);

-- Create index with both numeric and string fast fields
CREATE INDEX mixed_test_search ON mixed_numeric_string_test USING bm25 (
    id,
    numeric_field1,
    numeric_field2,
    string_field1,
    string_field2,
    string_field3,
    content
) WITH (
    key_field = 'id',
    text_fields = '{"string_field1": {"tokenizer": {"type": "default"}, "fast": true}, "string_field2": {"tokenizer": {"type": "default"}, "fast": true}, "string_field3": {"tokenizer": {"type": "default"}, "fast": true}, "content": {"tokenizer": {"type": "default"}}}',
    numeric_fields = '{"numeric_field1": {"fast": true}, "numeric_field2": {"fast": true}}'
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

-- Insert sample data for mixed fields test
INSERT INTO mixed_numeric_string_test (id, numeric_field1, numeric_field2, string_field1, string_field2, string_field3, content) VALUES
('mix1', 100, 10000, 'Apple', 'Red', 'Fruit', 'This is a red apple'),
('mix2', 200, 20000, 'Banana', 'Yellow', 'Fruit', 'This is a yellow banana'),
('mix3', 300, 30000, 'Carrot', 'Orange', 'Vegetable', 'This is an orange carrot'),
('mix4', 400, 40000, 'Donut', 'Brown', 'Dessert', 'This is a chocolate donut'),
('mix5', 500, 50000, 'Egg', 'White', 'Protein', 'This is a white egg');

-- Also set up the corner case test table
CREATE TABLE corner_case_test (
    id TEXT PRIMARY KEY,
    -- String fields with different characteristics
    empty_string TEXT NOT NULL,
    very_long_string TEXT NOT NULL,
    special_chars TEXT NOT NULL,
    non_utf8_bytes BYTEA NOT NULL,
    -- Numeric fields with different characteristics
    extreme_large BIGINT NOT NULL,
    extreme_small BIGINT NOT NULL,
    float_value FLOAT NOT NULL,
    zero_value INTEGER NOT NULL,
    negative_value INTEGER NOT NULL,
    -- Boolean field
    bool_field BOOLEAN NOT NULL,
    -- Regular fields for testing
    content TEXT
);

-- Create BM25 index with fast fields for all columns
CREATE INDEX corner_case_search ON corner_case_test USING bm25 (
    id,
    empty_string,
    very_long_string,
    special_chars,
    extreme_large,
    extreme_small,
    float_value,
    zero_value,
    negative_value,
    bool_field,
    content
) WITH (
    key_field = 'id',
    text_fields = '{"empty_string": {"tokenizer": {"type": "default"}, "fast": true}, "very_long_string": {"tokenizer": {"type": "default"}, "fast": true}, "special_chars": {"tokenizer": {"type": "default"}, "fast": true}, "content": {"tokenizer": {"type": "default"}}}',
    numeric_fields = '{"extreme_large": {"fast": true}, "extreme_small": {"fast": true}, "float_value": {"fast": true}, "zero_value": {"fast": true}, "negative_value": {"fast": true}}',
    boolean_fields = '{"bool_field": {"fast": true}}'
);

-- Insert extreme test data
INSERT INTO corner_case_test (
    id, 
    empty_string, 
    very_long_string, 
    special_chars, 
    non_utf8_bytes,
    extreme_large, 
    extreme_small, 
    float_value, 
    zero_value, 
    negative_value, 
    bool_field, 
    content
) VALUES
('case1', '', repeat('a', 8000), '!@#$%^&*()_+{}[]|:;"''<>,.?/', E'\\x00', 9223372036854775807, -9223372036854775808, 1.7976931348623157e+308, 0, -2147483648, true, 'Contains test term'),
('case2', '', repeat('b', 2), '-_.+', E'\\x00', 0, 0, 0.0, 0, 0, false, 'Contains test term'),
('case3', 'not_empty', '', '漢字', E'\\x00', 42, -42, 3.14159, 0, -1, true, 'Contains test term');

-- Set up the nullable test table
CREATE TABLE nullable_test (
    id TEXT PRIMARY KEY,
    string_field TEXT,
    numeric_field INTEGER,
    content TEXT
);

CREATE INDEX nullable_search ON nullable_test USING bm25 (
    id, string_field, numeric_field, content
) WITH (
    key_field = 'id',
    text_fields = '{"string_field": {"tokenizer": {"type": "default"}, "fast": true}, "content": {"tokenizer": {"type": "default"}}}',
    numeric_fields = '{"numeric_field": {"fast": true}}'
);

INSERT INTO nullable_test (id, string_field, numeric_field, content) VALUES
('null1', NULL, NULL, 'null test case'),
('null2', 'not null', 42, 'null test case');

-- Add data for CTE testing (from 09_cte_test.sql)
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

-- Add string edge cases from 11_string_edge_cases.sql
INSERT INTO mixed_numeric_string_test (id, numeric_field1, numeric_field2, string_field1, string_field2, string_field3, content) VALUES
('edge1', 1, 1, '', 'empty_first', 'test', 'edge case test'),
('edge2', 2, 2, 'special_chars_!@#$%^&*()', 'test', 'test', 'edge case test'),
('edge3', 3, 3, repeat('very_long_string_', 10), 'test', 'test', 'edge case test');

-- Add complex string patterns from 12_complex_string_patterns.sql
INSERT INTO corner_case_test (
    id, 
    empty_string, 
    very_long_string, 
    special_chars, 
    non_utf8_bytes,
    extreme_large, 
    extreme_small, 
    float_value, 
    zero_value, 
    negative_value, 
    bool_field, 
    content
) VALUES
('complex1', 'pattern with spaces', 'line1
line2
line3', 'tab    tab', E'\\x00', 1, 1, 1.0, 1, 1, true, 'complex pattern test'),
('complex2', 'quotation "marks"', 'backslash\\test', 'percent%test', E'\\x00', 2, 2, 2.0, 2, 2, false, 'complex pattern test');

-- Test the extension is loaded correctly
SELECT pg_backend_pid() > 0; 
