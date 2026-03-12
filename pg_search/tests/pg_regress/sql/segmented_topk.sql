-- Test for SegmentedTopKExec: per-segment Top K pruning using term ordinals.
-- Verifies that SegmentedTopKExec is injected below TantivyLookupExec for
-- ORDER BY <deferred_string_column> LIMIT K queries, and that results are correct.

SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;

CREATE EXTENSION IF NOT EXISTS pg_search;

-- =============================================================================
-- SETUP
-- =============================================================================

DROP TABLE IF EXISTS stk_files CASCADE;
DROP TABLE IF EXISTS stk_documents CASCADE;

CREATE TABLE stk_documents (
    id TEXT PRIMARY KEY,
    category TEXT
);

-- 10 documents. 5 belong to PROJECT_ALPHA (scattered).
INSERT INTO stk_documents (id, category) VALUES
('doc-01', 'PROJECT_ALPHA design review'),
('doc-02', 'BETA_GROUP budget overview'),
('doc-03', 'PROJECT_ALPHA roadmap planning'),
('doc-04', 'GAMMA_DIVISION quarterly report'),
('doc-05', 'PROJECT_ALPHA feedback notes'),
('doc-06', 'BETA_GROUP marketing strategy'),
('doc-07', 'PROJECT_ALPHA milestone tracker'),
('doc-08', 'GAMMA_DIVISION vendor evaluation'),
('doc-09', 'PROJECT_ALPHA resource allocation'),
('doc-10', 'BETA_GROUP incident response');

CREATE TABLE stk_files (
    id SERIAL PRIMARY KEY,
    document_id TEXT,
    title TEXT,
    content TEXT
);

-- 100 files, each referencing one of the 10 documents via round-robin.
-- Titles are 'File Title NNN' for deterministic sort order.
INSERT INTO stk_files (document_id, title, content)
SELECT
    'doc-' || LPAD(((i - 1) % 10 + 1)::TEXT, 2, '0'),
    'File Title ' || LPAD(i::TEXT, 3, '0'),
    'file content for item ' || i
FROM generate_series(1, 100) AS i;

CREATE INDEX stk_documents_bm25_idx ON stk_documents USING bm25 (id, category)
WITH (key_field = 'id', text_fields = '{"category": {"fast": true}}');

CREATE INDEX stk_files_bm25_idx ON stk_files USING bm25 (id, document_id, title, content)
WITH (key_field = 'id', text_fields = '{"document_id": {"tokenizer": {"type": "keyword"}, "fast": true}, "title": {"fast": true}, "content": {"fast": true}}');

SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 1: ASC LIMIT — SegmentedTopKExec should appear in plan
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.id, f.title
FROM stk_files f
WHERE f.document_id IN (
    SELECT d.id FROM stk_documents d WHERE d.category @@@ 'PROJECT_ALPHA'
)
ORDER BY f.title ASC
LIMIT 3;

SELECT f.id, f.title
FROM stk_files f
WHERE f.document_id IN (
    SELECT d.id FROM stk_documents d WHERE d.category @@@ 'PROJECT_ALPHA'
)
ORDER BY f.title ASC
LIMIT 3;

-- =============================================================================
-- TEST 2: DESC LIMIT — verify DESC direction works
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.id, f.title
FROM stk_files f
WHERE f.document_id IN (
    SELECT d.id FROM stk_documents d WHERE d.category @@@ 'PROJECT_ALPHA'
)
ORDER BY f.title DESC
LIMIT 3;

SELECT f.id, f.title
FROM stk_files f
WHERE f.document_id IN (
    SELECT d.id FROM stk_documents d WHERE d.category @@@ 'PROJECT_ALPHA'
)
ORDER BY f.title DESC
LIMIT 3;

-- =============================================================================
-- TEST 3: EXPLAIN ANALYZE — verify metrics (rows_input, rows_output, rows_pruned)
-- =============================================================================

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT f.id, f.title
FROM stk_files f
WHERE f.document_id IN (
    SELECT d.id FROM stk_documents d WHERE d.category @@@ 'PROJECT_ALPHA'
)
ORDER BY f.title ASC
LIMIT 3;

-- =============================================================================
-- TEST 4: K > total rows — no pruning, all rows survive
-- =============================================================================

SELECT f.id, f.title
FROM stk_files f
WHERE f.document_id IN (
    SELECT d.id FROM stk_documents d WHERE d.category @@@ 'PROJECT_ALPHA'
)
ORDER BY f.title ASC
LIMIT 1000;

-- =============================================================================
-- TEST 5: K = 1 — maximum pruning
-- =============================================================================

SELECT f.id, f.title
FROM stk_files f
WHERE f.document_id IN (
    SELECT d.id FROM stk_documents d WHERE d.category @@@ 'PROJECT_ALPHA'
)
ORDER BY f.title ASC
LIMIT 1;

-- =============================================================================
-- TEST 6: Sort by non-deferred column (id, numeric) — SegmentedTopKExec
-- should NOT appear (only applies to deferred string/bytes columns)
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.id, f.title
FROM stk_files f
WHERE f.document_id IN (
    SELECT d.id FROM stk_documents d WHERE d.category @@@ 'PROJECT_ALPHA'
)
ORDER BY f.id ASC
LIMIT 3;

-- =============================================================================
-- TEST 7: No LIMIT — SegmentedTopKExec should NOT appear
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.id, f.title
FROM stk_files f
WHERE f.document_id IN (
    SELECT d.id FROM stk_documents d WHERE d.category @@@ 'PROJECT_ALPHA'
)
ORDER BY f.title ASC;

-- =============================================================================
-- TEST 8: GUC toggle — disabling segmented_topk removes the node
-- =============================================================================

SET paradedb.enable_segmented_topk = off;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.id, f.title
FROM stk_files f
WHERE f.document_id IN (
    SELECT d.id FROM stk_documents d WHERE d.category @@@ 'PROJECT_ALPHA'
)
ORDER BY f.title ASC
LIMIT 3;

-- Verify results are the same with optimization disabled
SELECT f.id, f.title
FROM stk_files f
WHERE f.document_id IN (
    SELECT d.id FROM stk_documents d WHERE d.category @@@ 'PROJECT_ALPHA'
)
ORDER BY f.title ASC
LIMIT 3;

RESET paradedb.enable_segmented_topk;

-- =============================================================================
-- TEST 9: Compound sort correctness
--
-- SegmentedTopKExec correctly evaluates tiebreaker columns.
-- This test verifies that optimization ON returns the same ground truth result
-- as optimization OFF when heavy duplicates exist on the primary sort column.
-- =============================================================================

-- Insert 30 files with partially identical titles to create duplicate groups.
-- We round-robin across the 5 PROJECT_ALPHA documents so d.category varies.
-- Titles are grouped by 'Group A' through 'Group E' (6 rows per group)
-- Content is zero-padded to ensure clean alphabetic sorting.
TRUNCATE stk_files;
INSERT INTO stk_files (document_id, title, content)
SELECT
    'doc-' || LPAD(((i - 1) % 5 * 2 + 1)::TEXT, 2, '0'),
    'Group ' || CHR(65 + ((i - 1) / 6)::INT) || ' Title',
    'content ' || LPAD(i::TEXT, 2, '0')
FROM generate_series(1, 30) AS i;

-- Reference result: optimization OFF — this is the ground truth.
SET paradedb.enable_segmented_topk = off;

SELECT f.id, f.title
FROM stk_files f
WHERE f.document_id IN (
    SELECT d.id FROM stk_documents d WHERE d.category @@@ 'PROJECT_ALPHA'
)
ORDER BY f.title ASC, f.id DESC
LIMIT 5;

-- Same query with optimization ON.
SET paradedb.enable_segmented_topk = on;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.id, f.title
FROM stk_files f
WHERE f.document_id IN (
    SELECT d.id FROM stk_documents d WHERE d.category @@@ 'PROJECT_ALPHA'
)
ORDER BY f.title ASC, f.id DESC
LIMIT 5;

SELECT f.id, f.title
FROM stk_files f
WHERE f.document_id IN (
    SELECT d.id FROM stk_documents d WHERE d.category @@@ 'PROJECT_ALPHA'
)
ORDER BY f.title ASC, f.id DESC
LIMIT 5;

-- =============================================================================
-- TEST 10: ORDER BY containing two different string fields from the same table
-- =============================================================================

SET paradedb.enable_segmented_topk = off;

SELECT f.id, f.title, f.content
FROM stk_files f
WHERE f.document_id IN (
    SELECT d.id FROM stk_documents d WHERE d.category @@@ 'PROJECT_ALPHA'
)
ORDER BY f.title ASC, f.content DESC
LIMIT 5;

SET paradedb.enable_segmented_topk = on;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.id, f.title, f.content
FROM stk_files f
WHERE f.document_id IN (
    SELECT d.id FROM stk_documents d WHERE d.category @@@ 'PROJECT_ALPHA'
)
ORDER BY f.title ASC, f.content DESC
LIMIT 5;

SELECT f.id, f.title, f.content
FROM stk_files f
WHERE f.document_id IN (
    SELECT d.id FROM stk_documents d WHERE d.category @@@ 'PROJECT_ALPHA'
)
ORDER BY f.title ASC, f.content DESC
LIMIT 5;

-- =============================================================================
-- TEST 11: ORDER BY containing two different string fields from different tables
-- =============================================================================

SET paradedb.enable_segmented_topk = off;

SELECT f.id, f.title, d.category
FROM stk_files f
JOIN stk_documents d ON f.document_id = d.id
WHERE d.category @@@ 'PROJECT_ALPHA'
ORDER BY f.title ASC, d.category DESC, f.id ASC
LIMIT 5;

SET paradedb.enable_segmented_topk = on;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.id, f.title, d.category
FROM stk_files f
JOIN stk_documents d ON f.document_id = d.id
WHERE d.category @@@ 'PROJECT_ALPHA'
ORDER BY f.title ASC, d.category DESC, f.id ASC
LIMIT 5;

SELECT f.id, f.title, d.category
FROM stk_files f
JOIN stk_documents d ON f.document_id = d.id
WHERE d.category @@@ 'PROJECT_ALPHA'
ORDER BY f.title ASC, d.category DESC, f.id ASC
LIMIT 5;

-- =============================================================================
-- CLEANUP
-- =============================================================================

DROP TABLE stk_files CASCADE;
DROP TABLE stk_documents CASCADE;
