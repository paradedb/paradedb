-- Test for lazy segment checkout in MixedFastFieldExecState sorted path
-- This test verifies that:
-- 1. MixedFastFieldExecState is used when paradedb.enable_mixed_fast_field_exec = true
-- 2. Sorted scans with multiple segments use SortPreservingMergeExec
-- 3. Segments are checked out lazily (on-demand) during execution
-- 4. Multi-segment merge produces correctly sorted output

-- Disable parallel workers for predictable query behavior
SET max_parallel_workers_per_gather = 0;

-- Enable mixed fast field exec to use our lazy checkout code
SET paradedb.enable_mixed_fast_field_exec = true;

-- Note: Debug logging is enabled in the code (SORTED STREAM, LAZY CHECKOUT messages)
-- but we don't capture it in test output for reproducibility
-- Check postgres logs to verify lazy checkout behavior

-- Load the pg_search extension
CREATE EXTENSION IF NOT EXISTS pg_search;

-- =============================================================================
-- SETUP: Create table and index with small mutable_segment_rows
-- =============================================================================

DROP TABLE IF EXISTS lazy_checkout_test CASCADE;

CREATE TABLE lazy_checkout_test (
    id SERIAL PRIMARY KEY,
    content TEXT,
    priority INTEGER
);

-- Create index with sort_by, fast field on priority, and small mutable_segment_rows
-- This forces segments to be created more frequently
CREATE INDEX lazy_checkout_test_idx ON lazy_checkout_test
USING bm25 (id, content, priority)
WITH (
    key_field = 'id',
    text_fields = '{"content": {}}',
    numeric_fields = '{"priority": {"fast": true}}',
    sort_by = 'priority DESC NULLS LAST',
    mutable_segment_rows = 50
);

-- Insert data in separate batches with COMMITs to create multiple segments
-- Batch 1
INSERT INTO lazy_checkout_test (content, priority)
SELECT 'searchable document batch1 ' || i, 900 + (i % 10)
FROM generate_series(1, 100) AS i;

-- Force segment creation by committing
COMMIT;
BEGIN;

-- Batch 2
INSERT INTO lazy_checkout_test (content, priority)
SELECT 'searchable document batch2 ' || i, 800 + (i % 10)
FROM generate_series(1, 100) AS i;

COMMIT;
BEGIN;

-- Batch 3
INSERT INTO lazy_checkout_test (content, priority)
SELECT 'searchable document batch3 ' || i, 700 + (i % 10)
FROM generate_series(1, 100) AS i;

COMMIT;
BEGIN;

-- Batch 4
INSERT INTO lazy_checkout_test (content, priority)
SELECT 'searchable document batch4 ' || i, 600 + (i % 10)
FROM generate_series(1, 100) AS i;

COMMIT;
BEGIN;

-- Batch 5
INSERT INTO lazy_checkout_test (content, priority)
SELECT 'searchable document batch5 ' || i, 500 + (i % 10)
FROM generate_series(1, 100) AS i;

COMMIT;
BEGIN;

-- Batch 6
INSERT INTO lazy_checkout_test (content, priority)
SELECT 'searchable document batch6 ' || i, 400 + (i % 10)
FROM generate_series(1, 100) AS i;

-- =============================================================================
-- VERIFY: Check segment count from index info
-- =============================================================================

SELECT
    'Segment count' as info,
    count(*) as value
FROM paradedb.index_info('lazy_checkout_test_idx');

-- =============================================================================
-- TEST: Multi-segment sorted scan with lazy checkout
-- =============================================================================

-- Check EXPLAIN to verify MixedFastFieldExecState is used
-- Note: Without ORDER BY, the unsorted path is chosen
EXPLAIN (COSTS OFF) SELECT id, priority FROM lazy_checkout_test
WHERE content @@@ 'document';

-- Check EXPLAIN with ORDER BY to verify sorted path is chosen
-- The sorted path should eliminate the Sort node since index provides sorted output
-- Note: Must use NULLS LAST to match the index's sort_by = 'priority DESC NULLS LAST'
EXPLAIN (COSTS OFF) SELECT id, priority FROM lazy_checkout_test
WHERE content @@@ 'document'
ORDER BY priority DESC NULLS LAST;

-- Query with ORDER BY - triggers sorted path with lazy checkout for each segment
-- Check postgres logs for "SORTED STREAM" and "LAZY CHECKOUT" messages
SELECT id, priority FROM lazy_checkout_test
WHERE content @@@ 'document'
ORDER BY priority DESC NULLS LAST
FETCH FIRST 20 ROWS ONLY;

-- Verify all results are in descending order by priority
-- Must include ORDER BY with NULLS LAST to trigger the sorted path
-- Note: LAG() without ORDER BY checks the actual returned row order
SELECT
    CASE WHEN count(*) = 0 THEN 'ALL SORTED CORRECTLY' ELSE 'SORTING ERROR' END as sort_validation
FROM (
    SELECT priority, LAG(priority) OVER () as prev_priority
    FROM (
        SELECT priority FROM lazy_checkout_test
        WHERE content @@@ 'document'
        ORDER BY priority DESC NULLS LAST
    ) sub
) check_order
WHERE prev_priority IS NOT NULL AND priority > prev_priority;

-- =============================================================================
-- TEST: Sorted path gating - must NOT advertise sorted path when MixedFastFieldExec unavailable
-- =============================================================================

-- When enable_mixed_fast_field_exec is OFF, NormalScanExecState is used.
-- NormalScanExecState iterates segments sequentially without merging, so it cannot
-- provide globally sorted output. The planner MUST add a Sort node.

SET paradedb.enable_mixed_fast_field_exec = false;

-- EXPLAIN should show a Sort node because sorted path is NOT advertised
EXPLAIN (COSTS OFF) SELECT id, priority FROM lazy_checkout_test
WHERE content @@@ 'document'
ORDER BY priority DESC NULLS LAST;

-- Verify output is still sorted (thanks to the Sort node added by planner)
SELECT
    CASE WHEN count(*) = 0 THEN 'SORTED WITH SORT NODE' ELSE 'SORTING ERROR' END as sort_validation
FROM (
    SELECT priority, LAG(priority) OVER () as prev_priority
    FROM (
        SELECT priority FROM lazy_checkout_test
        WHERE content @@@ 'document'
        ORDER BY priority DESC NULLS LAST
    ) sub
) check_order
WHERE prev_priority IS NOT NULL AND priority > prev_priority;

-- Re-enable mixed fast field exec for next test
SET paradedb.enable_mixed_fast_field_exec = true;

-- =============================================================================
-- TEST: Non-fast field projection - must add Sort node
-- =============================================================================

-- Add a non-fast text field to the table for testing
ALTER TABLE lazy_checkout_test ADD COLUMN description TEXT DEFAULT 'test description';

-- When projecting a non-fast field (content), MixedFastFieldExec cannot be used.
-- The planner MUST add a Sort node because sorted path is NOT advertised.
EXPLAIN (COSTS OFF) SELECT id, content, priority FROM lazy_checkout_test
WHERE content @@@ 'document'
ORDER BY priority DESC NULLS LAST;

-- Verify output is still sorted
SELECT
    CASE WHEN count(*) = 0 THEN 'SORTED WITH NON-FAST PROJECTION' ELSE 'SORTING ERROR' END as sort_validation
FROM (
    SELECT priority, LAG(priority) OVER () as prev_priority
    FROM (
        SELECT priority FROM lazy_checkout_test
        WHERE content @@@ 'document'
        ORDER BY priority DESC NULLS LAST
    ) sub
) check_order
WHERE prev_priority IS NOT NULL AND priority > prev_priority;

-- =============================================================================
-- CLEANUP
-- =============================================================================

DROP TABLE IF EXISTS lazy_checkout_test CASCADE;
