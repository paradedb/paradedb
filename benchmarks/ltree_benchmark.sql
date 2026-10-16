-- ============================================================================
-- ParadeDB Benchmark: ltree Facet-Pushdown vs. GiST Bitmap Intersection (#6093)
-- ============================================================================
--
-- Purpose:
--   Compares the performance and memory/buffer characteristics of:
--     Strategy A: In-Index ltree Facet Pushdown (ParadeDB index on id, title, path)
--     Strategy B: Standard ltree GiST Index + ParadeDB Bitmap Intersection
--                 (ParadeDB index on id, title + GiST index on path)
--
-- Tested Regimes:
--   1. ltree-only selective: broad text query + narrow ltree path
--      Tests if in-index facet avoids building a TIDBitmap per query.
--   2. Both clauses selective: narrow text query + selective ltree path
--   3. Text-dominant selective: rare text query + broad ltree prefix
--   4. Extended GiST operators: lquery pattern matching (~), ancestor (@>)
--
-- Usage:
--   psql -d <database> -f benchmarks/ltree_benchmark.sql
-- ============================================================================

\timing on
SET client_min_messages = warning;

CREATE EXTENSION IF NOT EXISTS pg_search;
CREATE EXTENSION IF NOT EXISTS ltree;

DROP TABLE IF EXISTS bench_ltree_hierarchy CASCADE;

CREATE TABLE bench_ltree_hierarchy (
    id BIGINT PRIMARY KEY,
    title TEXT NOT NULL,
    path LTREE NOT NULL
);

-- Generate 100,000 hierarchical rows:
--   10 Departments x 20 Groups x 25 Categories x 20 Items = 100,000 rows
-- Text titles are assigned with controlled keyword frequencies:
--   - 'report' appears in 80% of rows (broad)
--   - 'quantum' appears in 5% of rows (medium selectivity)
--   - 'telescope' appears in 0.05% (50 rows) (high selectivity)
INSERT INTO bench_ltree_hierarchy (id, title, path)
SELECT
    i,
    CASE
        WHEN i % 2000 = 0 THEN 'confidential quantum telescope discovery document ' || i
        WHEN i % 20 = 0   THEN 'quarterly quantum research analysis ' || i
        ELSE 'general business report summary and notes ' || i
    END,
    (
        'Top.Dept' || (i % 10) ||
        '.Group' || ((i / 10) % 20) ||
        '.Cat'   || ((i / 200) % 25) ||
        '.Item'  || ((i / 5000) % 20)
    )::ltree
FROM generate_series(0, 99999) AS i;

VACUUM ANALYZE bench_ltree_hierarchy;

-- ============================================================================
-- STRATEGY A: In-Index Facet Pushdown (ParadeDB index on id, title, path)
-- ============================================================================
\echo '========================================================================'
\echo 'BUILDING STRATEGY A INDEX (ParadeDB with in-index facet ltree)...'
\echo '========================================================================'

DROP INDEX IF EXISTS bench_ltree_idx;
CREATE INDEX bench_ltree_idx
ON bench_ltree_hierarchy
USING paradedb (id, title, path);

VACUUM ANALYZE bench_ltree_hierarchy;

\echo ''
\echo '>>> [Strategy A - Regime 1: ltree ONLY selective]'
\echo '>>> Query: title ||| "report" (80k rows) AND path <@ "Top.Dept3.Group7.Cat12.Item5" (1 row)'
EXPLAIN (ANALYZE, BUFFERS, COSTS OFF)
SELECT id, title, path FROM bench_ltree_hierarchy
WHERE title ||| 'report' AND path <@ 'Top.Dept3.Group7.Cat12.Item5'::ltree
ORDER BY id LIMIT 10;

\echo ''
\echo '>>> [Strategy A - Regime 2: Both clauses selective]'
\echo '>>> Query: title ||| "quantum" (5k rows) AND path <@ "Top.Dept3.Group7" (250 rows)'
EXPLAIN (ANALYZE, BUFFERS, COSTS OFF)
SELECT id, title, path FROM bench_ltree_hierarchy
WHERE title ||| 'quantum' AND path <@ 'Top.Dept3.Group7'::ltree
ORDER BY id LIMIT 10;

\echo ''
\echo '>>> [Strategy A - Regime 3: Text ONLY selective]'
\echo '>>> Query: title ||| "telescope" (50 rows) AND path <@ "Top.Dept3" (10k rows)'
EXPLAIN (ANALYZE, BUFFERS, COSTS OFF)
SELECT id, title, path FROM bench_ltree_hierarchy
WHERE title ||| 'telescope' AND path <@ 'Top.Dept3'::ltree
ORDER BY id LIMIT 10;

-- ============================================================================
-- STRATEGY B: GiST Index + ParadeDB Bitmap Intersection (#6088)
-- ============================================================================
\echo ''
\echo '========================================================================'
\echo 'BUILDING STRATEGY B INDEXES (ParadeDB text + external GiST ltree)...'
\echo '========================================================================'

DROP INDEX IF EXISTS bench_ltree_idx;

CREATE INDEX bench_ltree_pdb_idx
ON bench_ltree_hierarchy
USING paradedb (id, title);

CREATE INDEX bench_ltree_gist_idx
ON bench_ltree_hierarchy
USING gist (path);

VACUUM ANALYZE bench_ltree_hierarchy;

\echo ''
\echo '>>> [Strategy B - Regime 1: ltree ONLY selective (Bitmap Intersection)]'
\echo '>>> Query: title ||| "report" (80k rows) AND path <@ "Top.Dept3.Group7.Cat12.Item5" (1 row)'
EXPLAIN (ANALYZE, BUFFERS, COSTS OFF)
SELECT id, title, path FROM bench_ltree_hierarchy
WHERE title ||| 'report' AND path <@ 'Top.Dept3.Group7.Cat12.Item5'::ltree
ORDER BY id LIMIT 10;

\echo ''
\echo '>>> [Strategy B - Regime 2: Both clauses selective (Bitmap Intersection)]'
\echo '>>> Query: title ||| "quantum" (5k rows) AND path <@ "Top.Dept3.Group7" (250 rows)'
EXPLAIN (ANALYZE, BUFFERS, COSTS OFF)
SELECT id, title, path FROM bench_ltree_hierarchy
WHERE title ||| 'quantum' AND path <@ 'Top.Dept3.Group7'::ltree
ORDER BY id LIMIT 10;

\echo ''
\echo '>>> [Strategy B - Regime 3: Text ONLY selective (Bitmap Intersection)]'
\echo '>>> Query: title ||| "telescope" (50 rows) AND path <@ "Top.Dept3" (10k rows)'
EXPLAIN (ANALYZE, BUFFERS, COSTS OFF)
SELECT id, title, path FROM bench_ltree_hierarchy
WHERE title ||| 'telescope' AND path <@ 'Top.Dept3'::ltree
ORDER BY id LIMIT 10;

\echo ''
\echo '>>> [Strategy B - Regime 4: Extended GiST Operator - lquery regex (~)]'
\echo '>>> Query: title ||| "quantum" AND path ~ "*.Cat12.*{1}" (not supported by facet pushdown)'
EXPLAIN (ANALYZE, BUFFERS, COSTS OFF)
SELECT id, title, path FROM bench_ltree_hierarchy
WHERE title ||| 'quantum' AND path ~ '*.Cat12.*{1}'::lquery
ORDER BY id LIMIT 10;

\echo ''
\echo '========================================================================'
\echo 'BENCHMARK COMPLETED'
\echo '========================================================================'
