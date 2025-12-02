\i common/common_setup.sql

-- Test prepared statements with parallel scans
-- This test verifies that parallel index scans work correctly with prepared statements
-- and generic plans, especially when combined with joins and non-indexed filters

-- Create test tables
CREATE TABLE core (
    dwf_doid BIGINT PRIMARY KEY,
    author TEXT,
    created_at TIMESTAMPTZ
);

CREATE TABLE document_text (
    dwf_doid BIGINT PRIMARY KEY,
    full_text TEXT
);

-- Insert test data
INSERT INTO core (dwf_doid, author, created_at)
SELECT 
    i,
    CASE 
        WHEN i % 3 = 0 THEN 'brian griffin'
        WHEN i % 3 = 1 THEN 'barabara pewterschmidt'
        ELSE 'bonnie swanson'
    END,
    '2024-01-01'::timestamptz + (i || ' days')::interval
FROM generate_series(1, 2000) i;

INSERT INTO document_text (dwf_doid, full_text)
SELECT 
    i,
    'This is document ' || i || ' with text containing ea'
FROM generate_series(1, 20000) i;

-- Create BM25 indexes
CREATE INDEX idx_parade_core ON core
USING bm25 (dwf_doid, author)
WITH (key_field='dwf_doid');

CREATE INDEX idx_parade_document_text ON document_text
USING bm25 (dwf_doid, full_text)
WITH (key_field='dwf_doid');

-- Enable parallel workers
SET max_parallel_workers_per_gather = 2;

-- Test 1: Prepared statement with join and date filter
-- This mimics the customer's scenario
PREPARE test_parallel_join(text, timestamptz, timestamptz) AS
SELECT COUNT(*)
FROM document_text dt
JOIN core c ON dt.dwf_doid = c.dwf_doid
WHERE dt.dwf_doid @@@ paradedb.parse_with_field('full_text', $1)
  AND c.dwf_doid @@@ paradedb.match('author', 'brian griffin')
  AND c.created_at::date >= $2::date
  AND c.created_at::date <= $3::date;

-- Execute multiple times to trigger generic plan
-- All executions should return consistent results
EXECUTE test_parallel_join('ea', '2024-01-01', '2025-01-01');
EXECUTE test_parallel_join('ea', '2024-01-01', '2025-01-01');
EXECUTE test_parallel_join('ea', '2024-01-01', '2025-01-01');
EXECUTE test_parallel_join('ea', '2024-01-01', '2025-01-01');
EXECUTE test_parallel_join('ea', '2024-01-01', '2025-01-01');

-- 6th execution should use generic plan and return same result
EXECUTE test_parallel_join('ea', '2024-01-01', '2025-01-01');

-- Try with different parameters
EXECUTE test_parallel_join('ea', '2024-06-01', '2025-01-01');

-- Force generic plan and test
SET plan_cache_mode = force_generic_plan;

DEALLOCATE test_parallel_join;

PREPARE test_parallel_join_generic(text, timestamptz, timestamptz) AS
SELECT COUNT(*)
FROM document_text dt
JOIN core c ON dt.dwf_doid = c.dwf_doid
WHERE dt.dwf_doid @@@ paradedb.parse_with_field('full_text', $1)
  AND c.dwf_doid @@@ paradedb.match('author', 'brian griffin')
  AND c.created_at::date >= $2::date
  AND c.created_at::date <= $3::date;

-- These should all return consistent results
EXECUTE test_parallel_join_generic('ea', '2024-01-01', '2025-01-01');
EXECUTE test_parallel_join_generic('ea', '2024-01-01', '2025-01-01');
EXECUTE test_parallel_join_generic('ea', '2024-06-01', '2025-01-01');

DEALLOCATE test_parallel_join_generic;

-- Test 2: Simple parallel index scan with parameters
RESET plan_cache_mode;

PREPARE test_simple_parallel(text) AS
SELECT COUNT(*)
FROM document_text
WHERE dwf_doid @@@ paradedb.parse_with_field('full_text', $1);

-- Execute multiple times
EXECUTE test_simple_parallel('ea');
EXECUTE test_simple_parallel('ea');
EXECUTE test_simple_parallel('ea');
EXECUTE test_simple_parallel('ea');
EXECUTE test_simple_parallel('ea');
-- 6th execution with generic plan
EXECUTE test_simple_parallel('ea');

-- Different parameter
EXECUTE test_simple_parallel('document');

DEALLOCATE test_simple_parallel;

-- Clean up
DROP TABLE document_text;
DROP TABLE core;

