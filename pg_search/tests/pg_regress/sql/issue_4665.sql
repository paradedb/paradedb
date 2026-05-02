-- Issue #4665: Parallel worker selection differs between CUSTOM and GENERIC prepared modes.
--
-- Root cause: query_input_restrict returned UNKNOWN_SELECTIVITY (0.00001) when the
-- RHS of the BM25 operator was a Param node (GENERIC plan mode). That collapsed
-- row estimates and drove compute_nworkers to 0 in GENERIC mode only.
--
-- This test reproduces the scenario with multiple segments and asserts that CUSTOM
-- and GENERIC prepared plans return the same rows.

CREATE EXTENSION IF NOT EXISTS pg_search;

SET max_parallel_workers_per_gather = 2;
SET max_parallel_workers = 4;
SET paradedb.min_rows_per_worker = 0;
SET enable_indexscan TO OFF;

-- Force each INSERT batch to become its own segment
SET paradedb.global_mutable_segment_rows = 0;

CREATE TABLE issue_4665_test (
    id SERIAL PRIMARY KEY,
    content TEXT
);

CREATE INDEX issue_4665_idx ON issue_4665_test
USING bm25 (id, content)
WITH (key_field = 'id');

-- Four separate INSERTs so we get (at least) four segments
INSERT INTO issue_4665_test (content)
SELECT 'document about ' ||
       (ARRAY['technology', 'science', 'cooking', 'sports', 'music', 'art'])[1 + (i % 6)] ||
       ' with details on topic number ' || i || ' covering various aspects'
FROM generate_series(1, 2500) AS i;

INSERT INTO issue_4665_test (content)
SELECT 'document about ' ||
       (ARRAY['technology', 'science', 'cooking', 'sports', 'music', 'art'])[1 + (i % 6)] ||
       ' with details on topic number ' || i || ' covering various aspects'
FROM generate_series(2501, 5000) AS i;

INSERT INTO issue_4665_test (content)
SELECT 'document about ' ||
       (ARRAY['technology', 'science', 'cooking', 'sports', 'music', 'art'])[1 + (i % 6)] ||
       ' with details on topic number ' || i || ' covering various aspects'
FROM generate_series(5001, 7500) AS i;

INSERT INTO issue_4665_test (content)
SELECT 'document about ' ||
       (ARRAY['technology', 'science', 'cooking', 'sports', 'music', 'art'])[1 + (i % 6)] ||
       ' with details on topic number ' || i || ' covering various aspects'
FROM generate_series(7501, 10000) AS i;

ANALYZE issue_4665_test;

-- Confirm multiple segments were produced
SELECT COUNT(*) > 1 AS has_multiple_segments
FROM paradedb.index_info('issue_4665_idx');

-- ============================================================================
-- CUSTOM plan
-- ============================================================================
SET plan_cache_mode = force_custom_plan;

PREPARE issue_4665_custom(text) AS
SELECT id FROM issue_4665_test
WHERE content ||| $1
ORDER BY pdb.score(id) DESC
LIMIT 10;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
EXECUTE issue_4665_custom('technology');

EXECUTE issue_4665_custom('technology');

DEALLOCATE issue_4665_custom;

-- ============================================================================
-- GENERIC plan
-- ============================================================================
SET plan_cache_mode = force_generic_plan;

PREPARE issue_4665_generic(text) AS
SELECT id FROM issue_4665_test
WHERE content ||| $1
ORDER BY pdb.score(id) DESC
LIMIT 10;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
EXECUTE issue_4665_generic('technology');

EXECUTE issue_4665_generic('technology');

DEALLOCATE issue_4665_generic;

-- ============================================================================
-- Parameterized LIMIT in both modes
-- ============================================================================
SET plan_cache_mode = force_custom_plan;

PREPARE issue_4665_custom_plim(text, int) AS
SELECT id FROM issue_4665_test
WHERE content ||| $1
ORDER BY pdb.score(id) DESC
LIMIT $2;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
EXECUTE issue_4665_custom_plim('technology', 10);

EXECUTE issue_4665_custom_plim('technology', 10);

DEALLOCATE issue_4665_custom_plim;

SET plan_cache_mode = force_generic_plan;

PREPARE issue_4665_generic_plim(text, int) AS
SELECT id FROM issue_4665_test
WHERE content ||| $1
ORDER BY pdb.score(id) DESC
LIMIT $2;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
EXECUTE issue_4665_generic_plim('technology', 10);

EXECUTE issue_4665_generic_plim('technology', 10);

DEALLOCATE issue_4665_generic_plim;

-- ============================================================================
-- Parameterized OFFSET: LIMIT 3 OFFSET $2 in both modes (must match).
-- OFFSET 7 > LIMIT 3 so the pre-fix TopK-undercount bug returns 0 rows
-- unambiguously.
-- ============================================================================
SET plan_cache_mode = force_custom_plan;

PREPARE issue_4665_off_custom(text, int) AS
SELECT id FROM issue_4665_test
WHERE content ||| $1
ORDER BY pdb.score(id) DESC
LIMIT 3 OFFSET $2;

EXECUTE issue_4665_off_custom('technology', 7);

DEALLOCATE issue_4665_off_custom;

SET plan_cache_mode = force_generic_plan;

PREPARE issue_4665_off_generic(text, int) AS
SELECT id FROM issue_4665_test
WHERE content ||| $1
ORDER BY pdb.score(id) DESC
LIMIT 3 OFFSET $2;

EXECUTE issue_4665_off_generic('technology', 7);

DEALLOCATE issue_4665_off_generic;

-- ============================================================================
-- Parameterized LIMIT + OFFSET: LIMIT $2 OFFSET $3 in both modes (must match)
-- ============================================================================
SET plan_cache_mode = force_custom_plan;

PREPARE issue_4665_both_custom(text, int, int) AS
SELECT id FROM issue_4665_test
WHERE content ||| $1
ORDER BY pdb.score(id) DESC
LIMIT $2 OFFSET $3;

EXECUTE issue_4665_both_custom('technology', 3, 7);

DEALLOCATE issue_4665_both_custom;

SET plan_cache_mode = force_generic_plan;

PREPARE issue_4665_both_generic(text, int, int) AS
SELECT id FROM issue_4665_test
WHERE content ||| $1
ORDER BY pdb.score(id) DESC
LIMIT $2 OFFSET $3;

EXECUTE issue_4665_both_generic('technology', 3, 7);

DEALLOCATE issue_4665_both_generic;

-- ============================================================================
-- Parameterized LIMIT + Const OFFSET: LIMIT $2 OFFSET 7
-- The pre-fix bug: GENERIC mode returned 0 rows because TopK fetched only
-- LIMIT (=3) rows and then PG's outer Limit OFFSET 7 skipped all of them.
-- ============================================================================
SET plan_cache_mode = force_custom_plan;

PREPARE issue_4665_plimcoff_custom(text, int) AS
SELECT id FROM issue_4665_test
WHERE content ||| $1
ORDER BY pdb.score(id) DESC
LIMIT $2 OFFSET 7;

EXECUTE issue_4665_plimcoff_custom('technology', 3);

DEALLOCATE issue_4665_plimcoff_custom;

SET plan_cache_mode = force_generic_plan;

PREPARE issue_4665_plimcoff_generic(text, int) AS
SELECT id FROM issue_4665_test
WHERE content ||| $1
ORDER BY pdb.score(id) DESC
LIMIT $2 OFFSET 7;

EXECUTE issue_4665_plimcoff_generic('technology', 3);

DEALLOCATE issue_4665_plimcoff_generic;

-- Cleanup
DROP TABLE issue_4665_test CASCADE;
RESET max_parallel_workers_per_gather;
RESET max_parallel_workers;
RESET paradedb.min_rows_per_worker;
RESET paradedb.global_mutable_segment_rows;
RESET enable_indexscan;
RESET plan_cache_mode;
