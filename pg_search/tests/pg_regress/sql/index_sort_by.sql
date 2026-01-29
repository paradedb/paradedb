-- Test for index-level sorting via the sort_by CREATE INDEX option
-- This test verifies that when an index is created with sort_by,
-- queries return results in the specified sort order WITHOUT
-- requiring an explicit ORDER BY clause.

-- Disable parallel workers for predictable behavior
SET max_parallel_workers_per_gather = 0;

-- Load the pg_search extension
CREATE EXTENSION IF NOT EXISTS pg_search;

-- =============================================================================
-- SETUP
-- =============================================================================

DROP TABLE IF EXISTS sort_test CASCADE;

-- Create test table
CREATE TABLE sort_test (
    id SERIAL PRIMARY KEY,
    title TEXT,
    content TEXT,
    priority INTEGER,
    created_at TIMESTAMP DEFAULT NOW()
);

-- =============================================================================
-- TEST 1: Index with sort_by DESC NULLS LAST
-- =============================================================================

-- Create index with sort_by
CREATE INDEX sort_test_desc_idx ON sort_test
USING bm25 (id, title, content, priority)
WITH (
    key_field = 'id',
    text_fields = '{"title": {}, "content": {}}',
    numeric_fields = '{"priority": {"fast": true}}',
    sort_by = 'priority DESC NULLS LAST'
);

-- Insert first batch
INSERT INTO sort_test (title, content, priority) VALUES
('Document A', 'This is searchable content alpha', 50),
('Document B', 'This is searchable content beta', 30),
('Document C', 'This is searchable content gamma', 70),
('Document D', 'This is searchable content delta', 10),
('Document E', 'This is searchable content epsilon', 90);

-- Insert second batch (creates new segment)
INSERT INTO sort_test (title, content, priority) VALUES
('Document F', 'More searchable content zeta', 45),
('Document G', 'More searchable content eta', 85),
('Document H', 'More searchable content theta', 25),
('Document I', 'More searchable content iota', 65),
('Document J', 'More searchable content kappa', 15);

-- Verify index has sort_by in EXPLAIN output
EXPLAIN (COSTS OFF)
SELECT id, title, priority FROM sort_test
WHERE content @@@ 'searchable';

-- Query WITHOUT ORDER BY - results should be sorted by priority DESC
SELECT id, title, priority FROM sort_test
WHERE content @@@ 'searchable';

-- Verify results are in descending order by priority
-- Expected order: 90, 85, 70, 65, 50, 45, 30, 25, 15, 10
SELECT
    CASE
        WHEN priority <= LAG(priority) OVER () OR LAG(priority) OVER () IS NULL
        THEN 'OK'
        ELSE 'NOT SORTED'
    END as sort_check,
    priority
FROM (
    SELECT priority FROM sort_test
    WHERE content @@@ 'searchable'
) sub;

-- =============================================================================
-- TEST 2: Query with LIMIT should still be sorted
-- =============================================================================

-- Query with LIMIT
SELECT id, title, priority FROM sort_test
WHERE content @@@ 'searchable'
LIMIT 5;

-- The top 5 should have the highest priorities
SELECT
    CASE
        WHEN MIN(priority) >= 45 THEN 'TOP 5 CORRECT'
        ELSE 'TOP 5 INCORRECT'
    END as limit_check
FROM (
    SELECT priority FROM sort_test
    WHERE content @@@ 'searchable'
    LIMIT 5
) sub;

-- =============================================================================
-- TEST 3: Explicit ORDER BY matching sort_by should work
-- =============================================================================

-- ORDER BY matching sort_by direction
SELECT id, title, priority FROM sort_test
WHERE content @@@ 'searchable'
ORDER BY priority DESC;

-- =============================================================================
-- TEST 4: Explicit ORDER BY differing from sort_by should still work
-- =============================================================================

-- ORDER BY opposite direction (ASC instead of DESC)
SELECT id, title, priority FROM sort_test
WHERE content @@@ 'searchable'
ORDER BY priority ASC;

-- =============================================================================
-- TEST 5: Updates should maintain correct sorted output
-- =============================================================================

-- Update a row to have the highest priority
UPDATE sort_test SET priority = 999 WHERE id = 4;

-- Query again - id=4 should now be first
SELECT id, title, priority FROM sort_test
WHERE content @@@ 'searchable'
LIMIT 3;

-- First result should have priority 999
SELECT
    CASE
        WHEN (SELECT priority FROM sort_test WHERE content @@@ 'searchable' LIMIT 1) = 999
        THEN 'UPDATE REFLECTED'
        ELSE 'UPDATE NOT REFLECTED'
    END as update_check;

-- =============================================================================
-- TEST 6: Index without sort_by (control test)
-- =============================================================================

DROP TABLE IF EXISTS no_sort_test CASCADE;

CREATE TABLE no_sort_test (
    id SERIAL PRIMARY KEY,
    content TEXT,
    value INTEGER
);

-- Index WITHOUT sort_by
CREATE INDEX no_sort_test_idx ON no_sort_test
USING bm25 (id, content, value)
WITH (
    key_field = 'id',
    text_fields = '{"content": {}}',
    numeric_fields = '{"value": {"fast": true}}'
);

INSERT INTO no_sort_test (content, value) VALUES
('test data one', 5),
('test data two', 3),
('test data three', 8),
('test data four', 1),
('test data five', 9);

-- Second batch
INSERT INTO no_sort_test (content, value) VALUES
('test data six', 2),
('test data seven', 7),
('test data eight', 4),
('test data nine', 6),
('test data ten', 10);

-- Query without ORDER BY - NOT guaranteed to be sorted
-- Just verify query works
SELECT id, content, value FROM no_sort_test
WHERE content @@@ 'test';

-- =============================================================================
-- TEST 7: sort_by with ASC NULLS FIRST
-- =============================================================================

DROP TABLE IF EXISTS sort_asc_test CASCADE;

CREATE TABLE sort_asc_test (
    id SERIAL PRIMARY KEY,
    description TEXT,
    score INTEGER
);

CREATE INDEX sort_asc_test_idx ON sort_asc_test
USING bm25 (id, description, score)
WITH (
    key_field = 'id',
    text_fields = '{"description": {}}',
    numeric_fields = '{"score": {"fast": true}}',
    sort_by = 'score ASC NULLS FIRST'
);

INSERT INTO sort_asc_test (description, score) VALUES
('item alpha', 50),
('item beta', 20),
('item gamma', 80),
('item delta', NULL),
('item epsilon', 10);

-- Second batch
INSERT INTO sort_asc_test (description, score) VALUES
('item zeta', 30),
('item eta', NULL),
('item theta', 60);

-- Query WITHOUT ORDER BY - should be sorted ASC with NULLs first
SELECT id, description, score FROM sort_asc_test
WHERE description @@@ 'item';

-- Verify NULLs come first, then ascending order
SELECT
    score,
    CASE
        WHEN score IS NULL THEN 'NULL (should be first)'
        ELSE 'VALUE: ' || score::text
    END as check
FROM sort_asc_test
WHERE description @@@ 'item';

-- =============================================================================
-- CLEANUP
-- =============================================================================

DROP TABLE IF EXISTS sort_test CASCADE;
DROP TABLE IF EXISTS no_sort_test CASCADE;
DROP TABLE IF EXISTS sort_asc_test CASCADE;
