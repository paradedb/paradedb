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
-- Tiny fixtures: lower the centroid-training floor.
SET paradedb.vector_min_training_rows = 1;

CREATE TABLE vsp (
    id    int PRIMARY KEY,
    label text,
    vec   vector(3)
);

INSERT INTO vsp VALUES
    (1, 'east wind',  '[1,    0,   0]'),
    (2, 'east gate',  '[0.9,  0,   0.1]'),
    (3, 'north wind', '[0,    1,   0]'),
    (4, 'up draft',   '[0,    0,   1]'),
    (5, 'mid point',  '[0.7,  0.7, 0]');


-- ============================================================
-- vector_l2_ops
-- ============================================================
CREATE INDEX vsp_idx ON vsp
    USING paradedb (id, label, vec vector_l2_ops)
    WITH (key_field = id);

-- match: <-> pushes down
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM vsp WHERE id @@@ pdb.all() ORDER BY vec <-> '[1,0,0]' LIMIT 2;
SELECT id FROM vsp WHERE id @@@ pdb.all() ORDER BY vec <-> '[1,0,0]' LIMIT 2;

-- mismatch: <=> falls back, planner warns
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM vsp WHERE id @@@ pdb.all() ORDER BY vec <=> '[1,0,0]' LIMIT 2;
SELECT id FROM vsp WHERE id @@@ pdb.all() ORDER BY vec <=> '[1,0,0]' LIMIT 2;

-- mismatch: <#> falls back, planner warns
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM vsp WHERE id @@@ pdb.all() ORDER BY vec <#> '[1,0,0]' LIMIT 2;
SELECT id FROM vsp WHERE id @@@ pdb.all() ORDER BY vec <#> '[1,0,0]' LIMIT 2;

DROP INDEX vsp_idx;


-- ============================================================
-- vector_cosine_ops
-- ============================================================
CREATE INDEX vsp_idx ON vsp
    USING paradedb (id, label, vec vector_cosine_ops)
    WITH (key_field = id);

-- mismatch: <-> falls back, planner warns
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM vsp WHERE id @@@ pdb.all() ORDER BY vec <-> '[1,0,0]' LIMIT 2;
SELECT id FROM vsp WHERE id @@@ pdb.all() ORDER BY vec <-> '[1,0,0]' LIMIT 2;

-- match: <=> pushes down
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM vsp WHERE id @@@ pdb.all() ORDER BY vec <=> '[1,0,0]' LIMIT 2;
SELECT id FROM vsp WHERE id @@@ pdb.all() ORDER BY vec <=> '[1,0,0]' LIMIT 2;

-- mismatch: <#> falls back, planner warns
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM vsp WHERE id @@@ pdb.all() ORDER BY vec <#> '[1,0,0]' LIMIT 2;
SELECT id FROM vsp WHERE id @@@ pdb.all() ORDER BY vec <#> '[1,0,0]' LIMIT 2;

DROP INDEX vsp_idx;


-- ============================================================
-- vector_ip_ops
-- ============================================================
CREATE INDEX vsp_idx ON vsp
    USING paradedb (id, label, vec vector_ip_ops)
    WITH (key_field = id);

-- mismatch: <-> falls back, planner warns
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM vsp WHERE id @@@ pdb.all() ORDER BY vec <-> '[1,0,0]' LIMIT 2;
SELECT id FROM vsp WHERE id @@@ pdb.all() ORDER BY vec <-> '[1,0,0]' LIMIT 2;

-- mismatch: <=> falls back, planner warns
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM vsp WHERE id @@@ pdb.all() ORDER BY vec <=> '[1,0,0]' LIMIT 2;
SELECT id FROM vsp WHERE id @@@ pdb.all() ORDER BY vec <=> '[1,0,0]' LIMIT 2;

-- match: <#> pushes down
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM vsp WHERE id @@@ pdb.all() ORDER BY vec <#> '[1,0,0]' LIMIT 2;
SELECT id FROM vsp WHERE id @@@ pdb.all() ORDER BY vec <#> '[1,0,0]' LIMIT 2;

DROP INDEX vsp_idx;


-- ============================================================
-- Runtime query-vector operand (not a Const, not a Param)
-- ============================================================
-- A stable, Var-free operand such as current_setting(...)::vector must push
-- down through TopK (evaluated once at execution start), and must reflect the
-- GUC's current value rather than a stale plan-time fold.
CREATE INDEX vsp_idx ON vsp
    USING paradedb (id, label, vec vector_cosine_ops)
    WITH (key_field = id);

SET vsp.q = '[1,0,0]';

-- pushes down (Custom Scan / TopKScanExecState), not a Sort fallback
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM vsp WHERE id @@@ pdb.all()
    ORDER BY vec <=> current_setting('vsp.q')::vector LIMIT 2;
-- ... and returns the same ranking as the equivalent literal
SELECT id FROM vsp WHERE id @@@ pdb.all()
    ORDER BY vec <=> current_setting('vsp.q')::vector LIMIT 2;

-- changing the GUC changes the ranking (proves no stale plan-time fold)
SET vsp.q = '[0,0,1]';
SELECT id FROM vsp WHERE id @@@ pdb.all()
    ORDER BY vec <=> current_setting('vsp.q')::vector LIMIT 2;

DROP INDEX vsp_idx;


-- ============================================================
-- Parameterized query-vector operand (`<=> $1`)
-- ============================================================
-- A bound Param must push down through TopK and be resolved from the
-- executor's parameter list at execution time. force_generic_plan keeps the
-- Param unfolded in the plan; a custom plan would inline it as a Const and
-- never exercise this path.
CREATE INDEX vsp_idx ON vsp
    USING paradedb (id, label, vec vector_cosine_ops)
    WITH (key_field = id);

SET plan_cache_mode = force_generic_plan;
PREPARE vsp_p(vector) AS
SELECT id FROM vsp WHERE id @@@ pdb.all() ORDER BY vec <=> $1 LIMIT 2;

-- pushes down (Custom Scan / TopKScanExecState), not a Sort fallback
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) EXECUTE vsp_p('[1,0,0]');
-- ... and returns the same ranking as the equivalent literal
EXECUTE vsp_p('[1,0,0]');

-- re-executing the same generic plan with a different vector must re-resolve
-- the Param, not reuse the previous execution's vector
EXECUTE vsp_p('[0,0,1]');
EXECUTE vsp_p('[1,0,0]');

DEALLOCATE vsp_p;
RESET plan_cache_mode;
DROP INDEX vsp_idx;


-- ============================================================
-- USING paradedb access method
-- ============================================================
-- Opclasses are declared per access method: the `paradedb` AM accepts the
-- same three opclasses as `bm25`, with vector_l2_ops as its DEFAULT too.
CREATE INDEX vsp_idx ON vsp
    USING paradedb (id, label, vec vector_cosine_ops)
    WITH (key_field = id);

-- match: <=> pushes down
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM vsp WHERE id @@@ pdb.all() ORDER BY vec <=> '[1,0,0]' LIMIT 2;
SELECT id FROM vsp WHERE id @@@ pdb.all() ORDER BY vec <=> '[1,0,0]' LIMIT 2;

DROP INDEX vsp_idx;

CREATE INDEX vsp_idx ON vsp
    USING paradedb (id, label, vec vector_ip_ops)
    WITH (key_field = id);
DROP INDEX vsp_idx;

-- a bare vector column resolves to vector_l2_ops, the AM default
CREATE INDEX vsp_idx ON vsp
    USING paradedb (id, label, vec)
    WITH (key_field = id);

SELECT opc.opcname
FROM pg_index i
JOIN unnest(i.indclass) WITH ORDINALITY AS c(opcoid, ord) ON true
JOIN pg_opclass opc ON opc.oid = c.opcoid
WHERE i.indexrelid = 'vsp_idx'::regclass AND opc.opcname LIKE 'vector%';

-- match: <-> pushes down
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM vsp WHERE id @@@ pdb.all() ORDER BY vec <-> '[1,0,0]' LIMIT 2;
SELECT id FROM vsp WHERE id @@@ pdb.all() ORDER BY vec <-> '[1,0,0]' LIMIT 2;

DROP INDEX vsp_idx;


-- ============================================================
-- Search operators (=== / &&& / ||| / ###) with vector ORDER BY
-- ============================================================
-- The predicate doesn't have to be `@@@ pdb.all()`: the search operators
-- combined with a vector ORDER BY must still push down through TopK,
-- ranking only the rows the predicate matches.
CREATE INDEX vsp_idx ON vsp
    USING paradedb (id, label, vec vector_cosine_ops)
    WITH (key_field = id);

-- === (term): rows 1 and 3 contain 'wind'; ranked 1 then 3 by distance
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM vsp WHERE label === 'wind' ORDER BY vec <=> '[1,0,0]' LIMIT 2;
SELECT id FROM vsp WHERE label === 'wind' ORDER BY vec <=> '[1,0,0]' LIMIT 2;

-- &&& (all terms): only row 1 has both 'east' and 'wind'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM vsp WHERE label &&& 'east wind' ORDER BY vec <=> '[1,0,0]' LIMIT 2;
SELECT id FROM vsp WHERE label &&& 'east wind' ORDER BY vec <=> '[1,0,0]' LIMIT 2;

-- ||| (any term): rows 1, 2, 3 match 'gate' or 'wind'; top-2 are 1 then 2
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM vsp WHERE label ||| 'gate wind' ORDER BY vec <=> '[1,0,0]' LIMIT 2;
SELECT id FROM vsp WHERE label ||| 'gate wind' ORDER BY vec <=> '[1,0,0]' LIMIT 2;

-- ### (phrase): only row 1 contains the phrase 'east wind'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM vsp WHERE label ### 'east wind' ORDER BY vec <=> '[1,0,0]' LIMIT 2;
SELECT id FROM vsp WHERE label ### 'east wind' ORDER BY vec <=> '[1,0,0]' LIMIT 2;

DROP INDEX vsp_idx;


-- ============================================================
-- Tiebreaking: secondary ORDER BY keys after vector distance
-- ============================================================
-- Rows 1-4 are exact duplicates of the query vector, so their distances tie
-- and the secondary key decides both the ordering and which rows survive the
-- top-K heap when LIMIT is smaller than the tie group.
CREATE TABLE vsp_tie (
    id  int PRIMARY KEY,
    cat text,
    vec vector(3)
);

INSERT INTO vsp_tie VALUES
    (1, 'b', '[1,0,0]'),
    (2, 'a', '[1,0,0]'),
    (3, 'b', '[1,0,0]'),
    (4, 'a', '[1,0,0]'),
    (5, 'x', '[0,1,0]'),
    (6, 'y', '[0,0.9,0.1]');

CREATE INDEX vsp_tie_idx ON vsp_tie
    USING paradedb (id, (cat::pdb.literal), vec vector_l2_ops)
    WITH (key_field = id);

-- pushes down: both pathkeys are absorbed by the TopK scan (no Sort node)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM vsp_tie WHERE id @@@ pdb.all() ORDER BY vec <-> '[1,0,0]', id LIMIT 3;

-- LIMIT 3 < the 4-way tie: the three lowest ids must win the heap
SELECT id FROM vsp_tie WHERE id @@@ pdb.all() ORDER BY vec <-> '[1,0,0]', id LIMIT 3;

-- descending tiebreak
SELECT id FROM vsp_tie WHERE id @@@ pdb.all() ORDER BY vec <-> '[1,0,0]', id DESC LIMIT 3;

-- OFFSET pagination across the tie is deterministic and non-overlapping
SELECT id FROM vsp_tie WHERE id @@@ pdb.all() ORDER BY vec <-> '[1,0,0]', id LIMIT 2 OFFSET 2;

-- LIMIT past the tie group: farther rows are ordered by distance, not tiebreak
SELECT id FROM vsp_tie WHERE id @@@ pdb.all() ORDER BY vec <-> '[1,0,0]', id LIMIT 6;

-- two tiebreak keys: cat ASC then id DESC within equal distance
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, cat FROM vsp_tie WHERE id @@@ pdb.all()
    ORDER BY vec <-> '[1,0,0]', cat, id DESC LIMIT 4;
SELECT id, cat FROM vsp_tie WHERE id @@@ pdb.all()
    ORDER BY vec <-> '[1,0,0]', cat, id DESC LIMIT 4;

DROP TABLE vsp_tie;


DROP TABLE vsp;
