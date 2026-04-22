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

-- TODO: Does not get SegmentedTopK: see https://github.com/paradedb/paradedb/issues/4347

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
-- TEST 12: Multi-segment global threshold pruning
--
-- Forces the probe-side table (stk_files) into multiple segments via
-- mutable_segment_rows, then runs a HashJoin + ORDER BY title LIMIT K.
-- After flushing segment 0's top K, SegmentedTopKExec publishes a global
-- threshold expressed as a string literal. The scanner translates it to
-- per-segment ordinal bounds and prunes rows from later segments at scan
-- level before they ever reach SegmentedTopKExec.
-- =============================================================================

DROP TABLE IF EXISTS stk_files CASCADE;
DROP TABLE IF EXISTS stk_documents CASCADE;

CREATE TABLE stk_documents (
    id TEXT PRIMARY KEY,
    category TEXT
);

INSERT INTO stk_documents (id, category) VALUES
('doc-01', 'PROJECT_ALPHA design review'),
('doc-02', 'BETA_GROUP budget overview'),
('doc-03', 'PROJECT_ALPHA roadmap planning'),
('doc-04', 'GAMMA_DIVISION quarterly report'),
('doc-05', 'PROJECT_ALPHA feedback notes');

CREATE TABLE stk_files (
    id SERIAL PRIMARY KEY,
    document_id TEXT,
    title TEXT,
    content TEXT
);

-- Create indexes BEFORE inserting data so inserts go through the mutable
-- segment pathway, producing multiple segments.
CREATE INDEX stk_documents_bm25_idx ON stk_documents USING bm25 (id, category)
WITH (key_field = 'id', text_fields = '{"category": {"fast": true}}');

CREATE INDEX stk_files_bm25_idx ON stk_files USING bm25 (id, document_id, title, content)
WITH (key_field = 'id', text_fields = '{"document_id": {"tokenizer": {"type": "keyword"}, "fast": true}, "title": {"fast": true}, "content": {"fast": true}}', mutable_segment_rows = 5000);

-- Insert 50K files across multiple segments (mutable_segment_rows=5000).
-- Round-robin across the 5 documents. Titles zero-padded for clean sort.
INSERT INTO stk_files (document_id, title, content)
SELECT
    'doc-' || LPAD(((i - 1) % 5 + 1)::TEXT, 2, '0'),
    'File Title ' || LPAD(i::TEXT, 5, '0'),
    'file content for item ' || i
FROM generate_series(1, 50000) AS i;

ANALYZE stk_files;
ANALYZE stk_documents;

-- Ground truth: optimization OFF
SET paradedb.enable_segmented_topk = off;

SELECT f.id, f.title
FROM stk_files f
WHERE f.document_id IN (
    SELECT d.id FROM stk_documents d WHERE d.category @@@ 'PROJECT_ALPHA'
)
ORDER BY f.title ASC
LIMIT 5;

-- With optimization ON: results must match ground truth.
SET paradedb.enable_segmented_topk = on;

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT f.id, f.title
FROM stk_files f
WHERE f.document_id IN (
    SELECT d.id FROM stk_documents d WHERE d.category @@@ 'PROJECT_ALPHA'
)
ORDER BY f.title ASC
LIMIT 5;

SELECT f.id, f.title
FROM stk_files f
WHERE f.document_id IN (
    SELECT d.id FROM stk_documents d WHERE d.category @@@ 'PROJECT_ALPHA'
)
ORDER BY f.title ASC
LIMIT 5;

-- =============================================================================
-- TEST 13: Visibility filtering — deleted rows must not appear in STK results
-- =============================================================================
-- Verifies that SegmentedTopKExec correctly filters MVCC-invisible (deleted)
-- rows via its absorbed VisibilityFilterExec data. Both the prune-cycle check
-- (maybe_compact) and the final-emit check (emit_final_topk) are exercised.

DROP TABLE IF EXISTS stk_vis_files CASCADE;
DROP TABLE IF EXISTS stk_vis_docs CASCADE;

CREATE TABLE stk_vis_docs (
    id TEXT PRIMARY KEY,
    category TEXT
);

INSERT INTO stk_vis_docs (id, category) VALUES
('vis-01', 'VISIBILITY_TEST alpha'),
('vis-02', 'VISIBILITY_TEST beta'),
('vis-03', 'VISIBILITY_TEST gamma');

CREATE TABLE stk_vis_files (
    id SERIAL PRIMARY KEY,
    doc_id TEXT,
    title TEXT
);

-- 15 files: titles 'Title 01'..'Title 15', round-robin across 3 docs.
INSERT INTO stk_vis_files (doc_id, title)
SELECT
    'vis-' || LPAD(((i - 1) % 3 + 1)::TEXT, 2, '0'),
    'Title ' || LPAD(i::TEXT, 2, '0')
FROM generate_series(1, 15) AS i;

CREATE INDEX stk_vis_docs_bm25_idx ON stk_vis_docs USING bm25 (id, category)
WITH (key_field = 'id', text_fields = '{"category": {"fast": true}}');

CREATE INDEX stk_vis_files_bm25_idx ON stk_vis_files USING bm25 (id, doc_id, title)
WITH (key_field = 'id', text_fields = '{"doc_id": {"tokenizer": {"type": "keyword"}, "fast": true}, "title": {"fast": true}}');

-- Ground truth before deletion (STK OFF): all 15 rows, first 5 in ASC order.
SET paradedb.enable_segmented_topk = off;

SELECT f.id, f.title
FROM stk_vis_files f
WHERE f.doc_id IN (
    SELECT d.id FROM stk_vis_docs d WHERE d.category @@@ 'VISIBILITY_TEST'
)
ORDER BY f.title ASC
LIMIT 5;

-- Delete 3 rows: these become dead MVCC tuples still visible to the BM25
-- index but invisible to the heap. STK must not return them.
DELETE FROM stk_vis_files WHERE title IN ('Title 01', 'Title 02', 'Title 03');

-- Ground truth after deletion (STK OFF): deleted rows must be absent.
SET paradedb.enable_segmented_topk = off;

SELECT f.id, f.title
FROM stk_vis_files f
WHERE f.doc_id IN (
    SELECT d.id FROM stk_vis_docs d WHERE d.category @@@ 'VISIBILITY_TEST'
)
ORDER BY f.title ASC
LIMIT 5;

-- STK ON ASC: must match ground truth (no deleted rows in result).
SET paradedb.enable_segmented_topk = on;

SELECT f.id, f.title
FROM stk_vis_files f
WHERE f.doc_id IN (
    SELECT d.id FROM stk_vis_docs d WHERE d.category @@@ 'VISIBILITY_TEST'
)
ORDER BY f.title ASC
LIMIT 5;

-- STK ON DESC: deleted rows must also be absent from the top end.
SELECT f.id, f.title
FROM stk_vis_files f
WHERE f.doc_id IN (
    SELECT d.id FROM stk_vis_docs d WHERE d.category @@@ 'VISIBILITY_TEST'
)
ORDER BY f.title DESC
LIMIT 5;

-- STK ON, LIMIT > remaining: must return exactly 12 rows (15 inserted - 3 deleted).
SELECT count(*) FROM (
    SELECT f.id, f.title
    FROM stk_vis_files f
    WHERE f.doc_id IN (
        SELECT d.id FROM stk_vis_docs d WHERE d.category @@@ 'VISIBILITY_TEST'
    )
    ORDER BY f.title ASC
    LIMIT 100
) sub;

-- STK OFF: count must match STK ON.
SET paradedb.enable_segmented_topk = off;

SELECT count(*) FROM (
    SELECT f.id, f.title
    FROM stk_vis_files f
    WHERE f.doc_id IN (
        SELECT d.id FROM stk_vis_docs d WHERE d.category @@@ 'VISIBILITY_TEST'
    )
    ORDER BY f.title ASC
    LIMIT 100
) sub;

RESET paradedb.enable_segmented_topk;

DROP TABLE stk_vis_files CASCADE;
DROP TABLE stk_vis_docs CASCADE;

-- =============================================================================
-- TEST 14: Regression — small segment (fewer than 2×k rows) must not be dropped
--
-- When a segment has fewer than 2×k rows, segment_cutoffs has no entry for it
-- (QuickSelect never fires). The old is_some_and returned false on None, silently
-- dropping every row from that segment. Fix: is_none_or passes all rows through
-- when no cutoff entry exists.
-- =============================================================================

DROP TABLE IF EXISTS stk_small_docs CASCADE;
DROP TABLE IF EXISTS stk_small_files CASCADE;

CREATE TABLE stk_small_docs (id TEXT PRIMARY KEY, category TEXT);
CREATE TABLE stk_small_files (id SERIAL PRIMARY KEY, doc_id TEXT, title TEXT);

-- Only 4 rows total — well below 2×k=52 for LIMIT 26. No cutoff will ever be set.
INSERT INTO stk_small_docs VALUES ('s1', 'SMALLTEST alpha'), ('s2', 'SMALLTEST beta');
INSERT INTO stk_small_files (doc_id, title) VALUES
    ('s1', 'Apple'), ('s1', 'Banana'), ('s2', 'Cherry'), ('s2', 'Date');

CREATE INDEX ON stk_small_docs USING bm25 (id, category)
    WITH (key_field = 'id', text_fields = '{"category": {"fast": true}}');
CREATE INDEX ON stk_small_files USING bm25 (id, doc_id, title)
    WITH (key_field = 'id', text_fields = '{"doc_id": {"tokenizer": {"type": "keyword"}, "fast": true}, "title": {"fast": true}}');

ANALYZE stk_small_docs;
ANALYZE stk_small_files;

-- Ground truth: STK OFF
SET paradedb.enable_segmented_topk = off;

SELECT count(*) FROM (
    SELECT f.id FROM stk_small_files f
    WHERE f.doc_id IN (SELECT d.id FROM stk_small_docs d WHERE d.category @@@ 'SMALLTEST')
    ORDER BY f.title ASC LIMIT 26
) sub;

-- STK ON: must return same count (4), not 0.
SET paradedb.enable_segmented_topk = on;

SELECT count(*) FROM (
    SELECT f.id FROM stk_small_files f
    WHERE f.doc_id IN (SELECT d.id FROM stk_small_docs d WHERE d.category @@@ 'SMALLTEST')
    ORDER BY f.title ASC LIMIT 26
) sub;

DROP TABLE stk_small_files CASCADE;
DROP TABLE stk_small_docs CASCADE;

-- =============================================================================
-- TEST 15: Regression — LIMIT 0 must not panic (k=0 rejected at rule level)
--
-- With k=0, select_nth_unstable(k-1) underflows usize. The fix rejects
-- Some(0) in segmented_topk_rule.rs before SegmentedTopKExec is injected.
-- =============================================================================

SET paradedb.enable_segmented_topk = on;

SELECT f.id, f.title
FROM stk_files f
WHERE f.document_id IN (
    SELECT d.id FROM stk_documents d WHERE d.category @@@ 'PROJECT_ALPHA'
)
ORDER BY f.title ASC
LIMIT 0;

-- =============================================================================
-- CLEANUP
-- =============================================================================

DROP TABLE stk_files CASCADE;
DROP TABLE stk_documents CASCADE;
