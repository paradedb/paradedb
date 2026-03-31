-- Test: Multi-index SegmentedTopK for ORDER BY on columns from multiple tables.
-- Verifies that SegmentedTopKExec is injected for multi-table sorts
-- and produces correct results compared to the non-SegmentedTopK fallback.
--
-- Issue: https://github.com/paradedb/paradedb/issues/4347

SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;

CREATE EXTENSION IF NOT EXISTS pg_search;

-- =============================================================================
-- SETUP
-- =============================================================================

DROP TABLE IF EXISTS topk_files CASCADE;
DROP TABLE IF EXISTS topk_docs CASCADE;

CREATE TABLE topk_docs (
    id INTEGER PRIMARY KEY,
    title TEXT,
    category TEXT
);

CREATE TABLE topk_files (
    id INTEGER PRIMARY KEY,
    doc_id INTEGER REFERENCES topk_docs(id),
    filename TEXT,
    content TEXT
);

-- Insert enough data with target_segment_count to get multiple segments.
INSERT INTO topk_docs (id, title, category)
SELECT
    i,
    'doc_' || lpad(i::text, 4, '0') || '_' ||
    CASE i % 5
        WHEN 0 THEN 'engineering'
        WHEN 1 THEN 'marketing'
        WHEN 2 THEN 'sales'
        WHEN 3 THEN 'support'
        WHEN 4 THEN 'research'
    END,
    CASE i % 3
        WHEN 0 THEN 'internal'
        WHEN 1 THEN 'external'
        WHEN 2 THEN 'confidential'
    END
FROM generate_series(1, 500) AS i;

INSERT INTO topk_files (id, doc_id, filename, content)
SELECT
    i,
    (i % 500) + 1,
    'file_' || lpad(i::text, 4, '0') || '.pdf',
    'content about ' ||
    CASE i % 4
        WHEN 0 THEN 'quarterly results'
        WHEN 1 THEN 'project updates'
        WHEN 2 THEN 'team meetings'
        WHEN 3 THEN 'product roadmap'
    END
FROM generate_series(1, 1000) AS i;

-- Create BM25 indexes with target_segment_count to force multiple segments
-- and fast: true on the text fields used in ORDER BY
CREATE INDEX topk_docs_idx ON topk_docs USING bm25 (id, title, category)
WITH (
    key_field = 'id',
    target_segment_count = 4,
    text_fields = '{"title": {"tokenizer": {"type": "default"}, "fast": true}, "category": {"tokenizer": {"type": "default"}, "fast": true}}'
);

CREATE INDEX topk_files_idx ON topk_files USING bm25 (id, doc_id, filename, content)
WITH (
    key_field = 'id',
    target_segment_count = 4,
    numeric_fields = '{"doc_id": {"fast": true}}',
    text_fields = '{"filename": {"tokenizer": {"type": "default"}, "fast": true}, "content": {"tokenizer": {"type": "default"}, "fast": true}}'
);

SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 1: Multi-index ORDER BY with LIMIT — verify SegmentedTopKExec appears
-- =============================================================================
SET paradedb.enable_segmented_topk = true;

-- Check that SegmentedTopKExec with mode=multi_index appears in the EXPLAIN
EXPLAIN (FORMAT TEXT)
SELECT d.title, f.filename
FROM topk_docs d
JOIN topk_files f ON d.id = f.doc_id
WHERE d.title @@@ 'engineering'
  AND f.content @@@ 'quarterly'
ORDER BY d.title ASC, f.filename ASC
LIMIT 5;

-- =============================================================================
-- TEST 2: Correctness — multi-index TopK results match non-TopK results
-- =============================================================================

-- With SegmentedTopK enabled
SET paradedb.enable_segmented_topk = true;
SELECT d.title, f.filename
FROM topk_docs d
JOIN topk_files f ON d.id = f.doc_id
WHERE d.title @@@ 'engineering'
  AND f.content @@@ 'quarterly'
ORDER BY d.title ASC, f.filename ASC
LIMIT 5;

-- Without SegmentedTopK — should produce identical results
SET paradedb.enable_segmented_topk = false;
SELECT d.title, f.filename
FROM topk_docs d
JOIN topk_files f ON d.id = f.doc_id
WHERE d.title @@@ 'engineering'
  AND f.content @@@ 'quarterly'
ORDER BY d.title ASC, f.filename ASC
LIMIT 5;

-- =============================================================================
-- TEST 3: Single-index sort in a join still gets threshold pushdown
-- =============================================================================
SET paradedb.enable_segmented_topk = true;

-- Only sorting by one table's column — should use SingleIndex mode
EXPLAIN (FORMAT TEXT)
SELECT d.title, f.filename
FROM topk_docs d
JOIN topk_files f ON d.id = f.doc_id
WHERE d.title @@@ 'engineering'
  AND f.content @@@ 'quarterly'
ORDER BY d.title ASC
LIMIT 5;

-- =============================================================================
-- TEST 4: Edge case — single deferred column + non-deferred column
-- =============================================================================
SET paradedb.enable_segmented_topk = true;

-- d.title is deferred (text), f.doc_id is non-deferred (integer).
-- This should use SingleIndex mode since only one index has deferred columns.
SELECT d.title, f.doc_id
FROM topk_docs d
JOIN topk_files f ON d.id = f.doc_id
WHERE d.title @@@ 'marketing'
ORDER BY d.title ASC, f.doc_id ASC
LIMIT 5;

-- =============================================================================
-- TEST 5: Descending multi-index sort
-- =============================================================================
SET paradedb.enable_segmented_topk = true;

SELECT d.title, f.filename
FROM topk_docs d
JOIN topk_files f ON d.id = f.doc_id
WHERE d.title @@@ 'engineering'
  AND f.content @@@ 'quarterly'
ORDER BY d.title DESC, f.filename DESC
LIMIT 5;

-- Verify against non-TopK
SET paradedb.enable_segmented_topk = false;

SELECT d.title, f.filename
FROM topk_docs d
JOIN topk_files f ON d.id = f.doc_id
WHERE d.title @@@ 'engineering'
  AND f.content @@@ 'quarterly'
ORDER BY d.title DESC, f.filename DESC
LIMIT 5;

-- =============================================================================
-- CLEANUP
-- =============================================================================
DROP TABLE topk_files CASCADE;
DROP TABLE topk_docs CASCADE;
