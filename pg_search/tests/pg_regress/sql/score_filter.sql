-- Tests for standalone score filtering (without JoinScan)

SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;

-- =============================================================================
-- SETUP
-- =============================================================================

CREATE EXTENSION IF NOT EXISTS pg_search;

-- setup.sql already creates regress.mock_items, but we'll use a local one for more control
DROP TABLE IF EXISTS score_items CASCADE;
CREATE TABLE score_items (
    id INTEGER PRIMARY KEY,
    description TEXT
);

INSERT INTO score_items (id, description) VALUES
(1, 'PostgreSQL is amazing'),
(2, 'Search engines are powerful'),
(3, 'Tantivy is fast');

CREATE INDEX score_items_idx ON score_items USING bm25 (id, description) WITH (key_field='id');

-- =============================================================================
-- TEST 1: Score filter >= 0 (matches everything)
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, description, paradedb.score(id)
FROM score_items
WHERE description @@@ 'PostgreSQL'
  AND paradedb.score(id) >= 0
ORDER BY id;

SELECT id, description, paradedb.score(id)
FROM score_items
WHERE description @@@ 'PostgreSQL'
  AND paradedb.score(id) >= 0
ORDER BY id;

-- =============================================================================
-- TEST 2: Score filter with threshold
-- =============================================================================

-- We expect one row to match 'PostgreSQL' with a positive score
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, description, paradedb.score(id)
FROM score_items
WHERE description @@@ 'PostgreSQL'
  AND paradedb.score(id) > 0.1
ORDER BY id;

SELECT id, description, paradedb.score(id)
FROM score_items
WHERE description @@@ 'PostgreSQL'
  AND paradedb.score(id) > 0.1
ORDER BY id;

-- =============================================================================
-- TEST 3: Score filter that excludes matches
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, description, paradedb.score(id)
FROM score_items
WHERE description @@@ 'PostgreSQL'
  AND paradedb.score(id) > 10.0
ORDER BY id;

SELECT id, description, paradedb.score(id)
FROM score_items
WHERE description @@@ 'PostgreSQL'
  AND paradedb.score(id) > 10.0
ORDER BY id;

-- =============================================================================
-- TEST 4: Multiple score filters
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, description, paradedb.score(id)
FROM score_items
WHERE description @@@ 'PostgreSQL'
  AND paradedb.score(id) >= 0
  AND paradedb.score(id) < 100
ORDER BY id;

SELECT id, description, paradedb.score(id)
FROM score_items
WHERE description @@@ 'PostgreSQL'
  AND paradedb.score(id) >= 0
  AND paradedb.score(id) < 100
ORDER BY id;

-- =============================================================================
-- TEST 5: Standalone score filter (no other search predicates)
-- =============================================================================

-- This test verifies that if there are no other query inputs, the ScoreFilter
-- correctly defaults to SearchQueryInput::All.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, description, paradedb.score(id)
FROM score_items
WHERE paradedb.score(id) >= 0
ORDER BY id;

SELECT id, description, paradedb.score(id)
FROM score_items
WHERE paradedb.score(id) >= 0
ORDER BY id;

-- CLEANUP
DROP TABLE score_items CASCADE;
