-- Per-opclass coverage for vector ORDER BY pushdown.
--
-- For each pgvector opclass (vector_l2_ops, vector_cosine_ops,
-- vector_ip_ops):
--   1. Build a BM25 index that names the opclass on the vector column.
--   2. EXPLAIN three queries — one per distance operator (<->, <=>, <#>).
--      The matching operator must push down through TopK; the other two
--      must fall back to a regular sort with the planner emitting the
--      "vector metric / opclass mismatch" warning.
--   3. Run each query to verify the actual ordering.
--
-- We use COSTS OFF for stable EXPLAIN diffs, and a 5-row corpus where
-- the K=2 ordering is unambiguous under all three metrics.

CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE vsp (
    id    int PRIMARY KEY,
    label text,
    vec   vector(3)
);

INSERT INTO vsp VALUES
    (1, 'east',  '[1,    0,   0]'),
    (2, 'east2', '[0.9,  0,   0.1]'),
    (3, 'north', '[0,    1,   0]'),
    (4, 'up',    '[0,    0,   1]'),
    (5, 'mid',   '[0.7,  0.7, 0]');


-- ============================================================
-- vector_l2_ops
-- ============================================================
CREATE INDEX vsp_idx ON vsp
    USING bm25 (id, label, vec vector_l2_ops)
    WITH (key_field = id);

-- match: <-> pushes down
EXPLAIN (COSTS OFF)
SELECT id FROM vsp WHERE id @@@ paradedb.all() ORDER BY vec <-> '[1,0,0]' LIMIT 2;
SELECT id FROM vsp WHERE id @@@ paradedb.all() ORDER BY vec <-> '[1,0,0]' LIMIT 2;

-- mismatch: <=> falls back, planner warns
EXPLAIN (COSTS OFF)
SELECT id FROM vsp WHERE id @@@ paradedb.all() ORDER BY vec <=> '[1,0,0]' LIMIT 2;
SELECT id FROM vsp WHERE id @@@ paradedb.all() ORDER BY vec <=> '[1,0,0]' LIMIT 2;

-- mismatch: <#> falls back, planner warns
EXPLAIN (COSTS OFF)
SELECT id FROM vsp WHERE id @@@ paradedb.all() ORDER BY vec <#> '[1,0,0]' LIMIT 2;
SELECT id FROM vsp WHERE id @@@ paradedb.all() ORDER BY vec <#> '[1,0,0]' LIMIT 2;

DROP INDEX vsp_idx;


-- ============================================================
-- vector_cosine_ops
-- ============================================================
CREATE INDEX vsp_idx ON vsp
    USING bm25 (id, label, vec vector_cosine_ops)
    WITH (key_field = id);

-- mismatch: <-> falls back, planner warns
EXPLAIN (COSTS OFF)
SELECT id FROM vsp WHERE id @@@ paradedb.all() ORDER BY vec <-> '[1,0,0]' LIMIT 2;
SELECT id FROM vsp WHERE id @@@ paradedb.all() ORDER BY vec <-> '[1,0,0]' LIMIT 2;

-- match: <=> pushes down
EXPLAIN (COSTS OFF)
SELECT id FROM vsp WHERE id @@@ paradedb.all() ORDER BY vec <=> '[1,0,0]' LIMIT 2;
SELECT id FROM vsp WHERE id @@@ paradedb.all() ORDER BY vec <=> '[1,0,0]' LIMIT 2;

-- mismatch: <#> falls back, planner warns
EXPLAIN (COSTS OFF)
SELECT id FROM vsp WHERE id @@@ paradedb.all() ORDER BY vec <#> '[1,0,0]' LIMIT 2;
SELECT id FROM vsp WHERE id @@@ paradedb.all() ORDER BY vec <#> '[1,0,0]' LIMIT 2;

DROP INDEX vsp_idx;


-- ============================================================
-- vector_ip_ops
-- ============================================================
CREATE INDEX vsp_idx ON vsp
    USING bm25 (id, label, vec vector_ip_ops)
    WITH (key_field = id);

-- mismatch: <-> falls back, planner warns
EXPLAIN (COSTS OFF)
SELECT id FROM vsp WHERE id @@@ paradedb.all() ORDER BY vec <-> '[1,0,0]' LIMIT 2;
SELECT id FROM vsp WHERE id @@@ paradedb.all() ORDER BY vec <-> '[1,0,0]' LIMIT 2;

-- mismatch: <=> falls back, planner warns
EXPLAIN (COSTS OFF)
SELECT id FROM vsp WHERE id @@@ paradedb.all() ORDER BY vec <=> '[1,0,0]' LIMIT 2;
SELECT id FROM vsp WHERE id @@@ paradedb.all() ORDER BY vec <=> '[1,0,0]' LIMIT 2;

-- match: <#> pushes down
EXPLAIN (COSTS OFF)
SELECT id FROM vsp WHERE id @@@ paradedb.all() ORDER BY vec <#> '[1,0,0]' LIMIT 2;
SELECT id FROM vsp WHERE id @@@ paradedb.all() ORDER BY vec <#> '[1,0,0]' LIMIT 2;

DROP INDEX vsp_idx;


DROP TABLE vsp;
