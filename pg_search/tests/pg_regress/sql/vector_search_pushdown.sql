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
    (1, 'east wind',  '[1,    0,   0]'),
    (2, 'east gate',  '[0.9,  0,   0.1]'),
    (3, 'north wind', '[0,    1,   0]'),
    (4, 'up draft',   '[0,    0,   1]'),
    (5, 'mid point',  '[0.7,  0.7, 0]');


-- ============================================================
-- vector_l2_ops
-- ============================================================
CREATE INDEX vsp_idx ON vsp
    USING paradedb (id, label, vec vector_l2_ops);

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
    USING paradedb (id, label, vec vector_cosine_ops);

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
    USING paradedb (id, label, vec vector_ip_ops);

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
    USING paradedb (id, label, vec vector_cosine_ops);

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
    USING paradedb (id, label, vec vector_cosine_ops);

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
    USING paradedb (id, label, vec vector_cosine_ops);

-- match: <=> pushes down
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM vsp WHERE id @@@ pdb.all() ORDER BY vec <=> '[1,0,0]' LIMIT 2;
SELECT id FROM vsp WHERE id @@@ pdb.all() ORDER BY vec <=> '[1,0,0]' LIMIT 2;

DROP INDEX vsp_idx;

CREATE INDEX vsp_idx ON vsp
    USING paradedb (id, label, vec vector_ip_ops);
DROP INDEX vsp_idx;

-- a bare vector column resolves to vector_l2_ops, the AM default
CREATE INDEX vsp_idx ON vsp
    USING paradedb (id, label, vec);

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
    USING paradedb (id, label, vec vector_cosine_ops);

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
    USING paradedb (id, (cat::pdb.literal), vec vector_l2_ops);

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


-- ============================================================
-- Join / semi-join with vector ORDER BY
-- ============================================================
-- Parent table (`200` rows across `2` persisted segments, `uuid` PK) holds
-- scalar/JSONB filters; child table (`986` rows in `1` IVF segment, `uuid` PK
-- and `uuid` FK) holds chunks and embeddings linked by `parent_id`:
--   - Segment 0 of `vsp_parents` (`150` rows, `ord = 51..200`, the largest
--     segment): `1` `'delta'` row (keeps Segment 0 unpruned), `20` correlated
--     `'gamma' + 'north'` rows (`10%` of `200`), and `129` `'noise'` rows.
--   - Segment 1 of `vsp_parents` (`50` rows, `ord = 1..50`): canonical rows
--     `1..4` (`'alpha'` / `'beta'`), `35` `'delta'` rows (`36 / 200 = 18%`
--     overall, skewed toward the smaller segment), and `11` `'noise'` rows.
--   - In `vsp_children`, the `700` `'noise'` chunks (`[0.99, 0.01, 0]`) sit
--     closer to `[1, 0, 0]` than `'alpha'` parents `2` and `4`, `'gamma'`,
--     and `'delta'`, so any unpushed probe scan must walk past `'noise'` rows.
SET client_min_messages = WARNING;
SET max_parallel_workers_per_gather = 0;
SET paradedb.vector_clustering_threshold = 64;
SET paradedb.vector_cluster_max_probe = 100000;

-- Omit PRIMARY KEY on `vsp_parents.id` so PG 18.7+ keeps the `IN (SELECT ...)`
-- tests below as true Semi joins alongside the explicit `JOIN` Inner-join tests.
CREATE TABLE vsp_parents (
    id       uuid NOT NULL,
    ord      int NOT NULL,
    category text NOT NULL,
    region   text NOT NULL,
    metadata jsonb NOT NULL
) WITH (autovacuum_enabled = false);

CREATE TABLE vsp_children (
    id         uuid PRIMARY KEY,
    seq        int NOT NULL,
    parent_id  uuid NOT NULL,
    parent_ord int NOT NULL,
    vec        vector(3) NOT NULL
) WITH (autovacuum_enabled = false);

CREATE INDEX vsp_parents_idx ON vsp_parents
    USING paradedb (id, ord, (category::pdb.literal), (region::pdb.literal), (metadata::pdb.literal))
    WITH (
        target_segment_count = 8,
        mutable_segment_rows = 0,
        background_layer_sizes = '0'
    );

INSERT INTO vsp_parents
SELECT md5('vsp_parent_' || i)::uuid,
       i,
       CASE
           WHEN i = 51 THEN 'delta'
           WHEN i BETWEEN 52 AND 71 THEN 'gamma'
           ELSE 'noise'
       END,
       CASE
           WHEN i BETWEEN 52 AND 71 THEN 'north'
           ELSE 'south'
       END,
       CASE
           WHEN i = 51 THEN '{"score": 40}'::jsonb
           WHEN i BETWEEN 52 AND 71 THEN '{"score": 50}'::jsonb
           ELSE '{"score": 0}'::jsonb
       END
FROM generate_series(51, 200) i;

INSERT INTO vsp_parents
SELECT CASE
           WHEN i <= 3 THEN ('00000000-0000-0000-0000-' || lpad(i::text, 12, '0'))::uuid
           WHEN i = 4  THEN 'ffffffff-ffff-ffff-ffff-000000000004'::uuid
           ELSE             md5('vsp_parent_' || i)::uuid
       END,
       i,
       CASE
           WHEN i IN (1, 2, 4) THEN 'alpha'
           WHEN i = 3 THEN 'beta'
           WHEN i BETWEEN 5 AND 39 THEN 'delta'
           ELSE 'noise'
       END,
       CASE
           WHEN i IN (1, 2) THEN 'north'
           WHEN i BETWEEN 5 AND 39 THEN 'west'
           ELSE 'south'
       END,
       CASE
           WHEN i = 1 THEN '{"score": 10}'::jsonb
           WHEN i IN (2, 3) THEN '{"score": 20}'::jsonb
           WHEN i = 4 THEN '{"score": 30}'::jsonb
           WHEN i BETWEEN 5 AND 39 THEN '{"score": 40}'::jsonb
           ELSE '{"score": 0}'::jsonb
       END
FROM generate_series(1, 50) i;

INSERT INTO vsp_children
SELECT ('00000000-0000-0000-0000-' || lpad(v.seq::text, 12, '0'))::uuid,
       v.seq,
       v.p_id,
       v.p_ord,
       v.vec::vector
FROM (
    SELECT c.seq, p.id AS p_id, p.ord AS p_ord, c.vec
    FROM (
        VALUES
            (10, 1, '[1,   0,   0]'),
            (11, 1, '[0,   1,   0]'),
            (20, 2, '[0.9, 0.1, 0]'),
            (21, 2, '[0,   0,   1]'),
            (30, 3, '[1,   0,   0]'),
            (40, 4, '[0.8, 0.2, 0]')
    ) AS c(seq, p_ord, vec)
    JOIN vsp_parents p ON p.ord = c.p_ord
    UNION ALL
    SELECT 1000 + (p.ord - 5) * 5 + c_idx,
           p.id,
           p.ord,
           CASE p.category
               WHEN 'noise' THEN '[0.99, 0.01, ' || (((p.ord * 5 + c_idx) % 50) * 0.001)::text || ']'
               WHEN 'gamma' THEN '[0.7,  0.3,  0]'
               ELSE              '[0.6,  0.4,  0]'
           END
    FROM vsp_parents p
    CROSS JOIN generate_series(0, 4) c_idx
    WHERE p.ord >= 5
) AS v(seq, p_id, p_ord, vec);

-- Build `vsp_children_idx` after populating `vsp_children` so `persist_build_segments`
-- clusters the `986 >= 64` rows into a single IVF segment (`quantization = '0'` avoids
-- random rotation seeds on `vector(3)` while exercising IVF cluster routing).
CREATE INDEX vsp_children_idx ON vsp_children
    USING paradedb (id, seq, parent_id, parent_ord, vec vector_cosine_ops)
    WITH (
        quantization = '0',
        target_segment_count = 1,
        mutable_segment_rows = 0,
        background_layer_sizes = '0',
        target_leaf_size = 20,
        training_sample_ratio = 1.0
    );

ANALYZE vsp_parents, vsp_children;

SELECT
    (SELECT count(*) FROM paradedb.index_info('vsp_parents_idx')) AS parent_segments,
    (SELECT count(*) FROM paradedb.index_info('vsp_children_idx')) AS child_segments,
    (SELECT bool_and(vector_format = 'ivf') FROM paradedb.vector_info('vsp_children_idx', 'vec')) AS child_is_ivf;

-- Semi-join (`IN (SELECT ...)`) under HashJoinExec (Regime 2)
EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT c.seq, c.parent_id
FROM vsp_children c
WHERE c.id @@@ pdb.all()
  AND c.parent_id IN (
    SELECT p.id FROM vsp_parents p
    WHERE p.id @@@ pdb.all()
      AND p.category = ANY('{alpha}')
      AND (p.metadata->>'score')::float8 BETWEEN 20 AND 30
  )
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

SELECT c.seq, c.parent_id
FROM vsp_children c
WHERE c.id @@@ pdb.all()
  AND c.parent_id IN (
    SELECT p.id FROM vsp_parents p
    WHERE p.id @@@ pdb.all()
      AND p.category = ANY('{alpha}')
      AND (p.metadata->>'score')::float8 BETWEEN 20 AND 30
  )
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

-- Outer `GROUP BY parent_id ORDER BY min(dist)` over top-K semi-join subquery
SELECT sub.parent_id, round(min(sub.dist)::numeric, 4) AS min_dist
FROM (
    SELECT c.parent_id, c.vec <=> '[1,0,0]'::vector AS dist
    FROM vsp_children c
    WHERE c.id @@@ pdb.all()
      AND c.parent_id IN (
        SELECT p.id FROM vsp_parents p
        WHERE p.id @@@ pdb.all()
          AND p.category = ANY('{alpha}')
          AND (p.metadata->>'score')::float8 BETWEEN 20 AND 30
      )
    ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
    LIMIT 4
) sub
GROUP BY sub.parent_id
ORDER BY min_dist, sub.parent_id
LIMIT 2;

-- Semi-join (`EXISTS`) and duplicate build-key deduplication
SELECT c.seq, c.parent_id
FROM vsp_children c
WHERE c.id @@@ pdb.all()
  AND EXISTS (
    SELECT 1 FROM vsp_parents p
    WHERE p.id @@@ pdb.all()
      AND p.category = ANY('{alpha}')
      AND (p.metadata->>'score')::float8 BETWEEN 20 AND 30
      AND p.id = c.parent_id
  )
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

SELECT c1.seq, c1.parent_id
FROM vsp_children c1
WHERE c1.id @@@ pdb.all()
  AND c1.parent_id IN (
    SELECT c2.parent_id FROM vsp_children c2
    WHERE c2.id @@@ pdb.all()
      AND c2.parent_id IN (
        '00000000-0000-0000-0000-000000000001'::uuid,
        '00000000-0000-0000-0000-000000000002'::uuid
      )
  )
ORDER BY c1.vec <=> '[1,0,0]'::vector, c1.id
LIMIT 3;

SET paradedb.rank_join_min_rows = 0;

SELECT c1.seq, c1.parent_id
FROM vsp_children c1
WHERE c1.id @@@ pdb.all()
  AND c1.parent_id IN (
    SELECT c2.parent_id FROM vsp_children c2
    WHERE c2.id @@@ pdb.all()
      AND c2.parent_id IN (
        '00000000-0000-0000-0000-000000000001'::uuid,
        '00000000-0000-0000-0000-000000000002'::uuid
      )
  )
ORDER BY c1.vec <=> '[1,0,0]'::vector, c1.id
LIMIT 3;

RESET paradedb.rank_join_min_rows;

-- Inner join (`JOIN`) under HashJoinExec (Regime 2), projecting distance and tie-breaker `DESC`
EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT c.seq, c.parent_id, p.category, round((c.vec <=> '[1,0,0]'::vector)::numeric, 4) AS dist
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all()
  AND p.id @@@ pdb.all()
  AND p.category = ANY('{alpha}')
  AND (p.metadata->>'score')::float8 BETWEEN 20 AND 30
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id DESC
LIMIT 2;

SELECT c.seq, c.parent_id, p.category, round((c.vec <=> '[1,0,0]'::vector)::numeric, 4) AS dist
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all()
  AND p.id @@@ pdb.all()
  AND p.category = ANY('{alpha}')
  AND (p.metadata->>'score')::float8 BETWEEN 20 AND 30
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id DESC
LIMIT 2;

-- OFFSET and prepared / runtime-expression query vectors in joins
SELECT c.seq, c.parent_id
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all()
  AND p.id @@@ pdb.all()
  AND p.category = ANY('{alpha}')
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2 OFFSET 1;

PREPARE vsp_join_prep(vector) AS
SELECT c.seq, c.parent_id
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all()
  AND p.id @@@ pdb.all()
  AND p.category = ANY('{alpha}')
  AND (p.metadata->>'score')::float8 BETWEEN 20 AND 30
ORDER BY c.vec <=> $1, c.id
LIMIT 2;

EXECUTE vsp_join_prep('[1,0,0]'::vector);
DEALLOCATE vsp_join_prep;

SELECT set_config('vsp.join_qvec', '[1,0,0]', false);
SELECT c.seq, c.parent_id
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all()
  AND p.id @@@ pdb.all()
  AND p.category = ANY('{alpha}')
  AND (p.metadata->>'score')::float8 BETWEEN 20 AND 30
ORDER BY c.vec <=> current_setting('vsp.join_qvec')::vector, c.id
LIMIT 2;

-- Regime 1 (`RankJoinExec`) for Semi-join and Inner join, plus selectivity threshold GUC
SET paradedb.rank_join_min_rows = 0;

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT c.seq, c.parent_id
FROM vsp_children c
WHERE c.id @@@ pdb.all()
  AND c.parent_id IN (
    SELECT p.id FROM vsp_parents p
    WHERE p.id @@@ pdb.all()
      AND p.category = ANY('{alpha}')
      AND (p.metadata->>'score')::float8 BETWEEN 20 AND 30
  )
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

SELECT c.seq, c.parent_id
FROM vsp_children c
WHERE c.id @@@ pdb.all()
  AND c.parent_id IN (
    SELECT p.id FROM vsp_parents p
    WHERE p.id @@@ pdb.all()
      AND p.category = ANY('{alpha}')
      AND (p.metadata->>'score')::float8 BETWEEN 20 AND 30
  )
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT c.seq, c.parent_id
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all()
  AND p.id @@@ pdb.all()
  AND p.category = ANY('{alpha}')
  AND (p.metadata->>'score')::float8 BETWEEN 20 AND 30
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

SELECT c.seq, c.parent_id
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all()
  AND p.id @@@ pdb.all()
  AND p.category = ANY('{alpha}')
  AND (p.metadata->>'score')::float8 BETWEEN 20 AND 30
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

-- Threshold GUC (`paradedb.rank_join_unscored_selectivity_threshold`) switches between
-- `RankJoinExec` and `HashJoinExec` when `rank_join_min_rows > 0`, and parent selectivity
-- underestimation (multi-clause independence and largest-segment skew) pushes queries whose
-- actual selectivity exceeds `0.08` into `HashJoinExec`.
SET paradedb.rank_join_min_rows = 1;
SET paradedb.rank_join_unscored_selectivity_threshold = 0.0;

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT c.seq, c.parent_id
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all()
  AND p.id @@@ pdb.all()
  AND p.category = 'beta'
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

SET paradedb.rank_join_unscored_selectivity_threshold = 0.99;

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT c.seq, c.parent_id
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all()
  AND p.id @@@ pdb.all()
  AND p.category = 'beta'
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

RESET paradedb.rank_join_unscored_selectivity_threshold;
SET paradedb.hash_join_inlist_pushdown_max_distinct_values = 10;

-- Correlated multi-clause TermQuery filter on parent (`20 / 200 = 10% > 0.08`, discounted by
-- `BooleanQuery::estimate_docs` to `11 / 200 = 5.5% < 0.08` -> declines `RankJoinExec` into
-- unpushed `HashJoinExec`)
EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT c.seq, c.parent_id
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all()
  AND p.id @@@ pdb.all()
  AND p.category = 'gamma'
  AND p.region = 'north'
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

SELECT c.seq, c.parent_id
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all()
  AND p.id @@@ pdb.all()
  AND p.category = 'gamma'
  AND p.region = 'north'
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

-- Multi-segment skew on parent (`36 / 200 = 18% > 0.08`, but largest Segment 0 has only
-- `1 / 150` `'delta'` rows -> `estimate_docs` estimates `1` row and declines `RankJoinExec`
-- into unpushed `HashJoinExec`)
EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT c.seq, c.parent_id
FROM vsp_children c
WHERE c.id @@@ pdb.all()
  AND c.parent_id IN (
    SELECT p.id FROM vsp_parents p
    WHERE p.category === 'delta'
  )
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

SELECT c.seq, c.parent_id
FROM vsp_children c
WHERE c.id @@@ pdb.all()
  AND c.parent_id IN (
    SELECT p.id FROM vsp_parents p
    WHERE p.category === 'delta'
  )
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

RESET paradedb.hash_join_inlist_pushdown_max_distinct_values;
RESET paradedb.rank_join_min_rows;

-- Cross-table OR filter (`ScanMode::Tagged`) + residual filter with vector ORDER BY
EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT c.seq, c.parent_id, p.category
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE (c.seq @@@ '20'::pdb.query OR p.category @@@ 'beta'::pdb.query)
  AND c.seq + p.ord > 0
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

SELECT c.seq, c.parent_id, p.category
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE (c.seq @@@ '20'::pdb.query OR p.category @@@ 'beta'::pdb.query)
  AND c.seq + p.ord > 0
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

SET paradedb.rank_join_min_rows = 0;

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT c.seq, c.parent_id, p.category
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE (c.seq @@@ '20'::pdb.query OR p.category @@@ 'beta'::pdb.query)
  AND c.seq + p.ord > 0
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

SELECT c.seq, c.parent_id, p.category
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE (c.seq @@@ '20'::pdb.query OR p.category @@@ 'beta'::pdb.query)
  AND c.seq + p.ord > 0
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

RESET paradedb.rank_join_min_rows;

-- ============================================================
-- LazySort edge cases in vector-ordered joins
-- ============================================================

-- Edge Case 1: High-density build key set where `term_set_bitset_max_density_multi = 0.0`
-- dispatches linear TermSet on numeric join keys (`p.ord = c.parent_ord`) and automaton on
-- string-backed `uuid` keys (`p.id = c.parent_id`).
SET paradedb.term_set_bitset_max_density_multi = 0.0;

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT c.seq, c.parent_ord
FROM vsp_children c
JOIN vsp_parents p ON p.ord = c.parent_ord
WHERE c.id @@@ pdb.all()
  AND p.id @@@ pdb.all()
  AND p.category = ANY('{alpha}')
  AND (p.metadata->>'score')::float8 BETWEEN 20 AND 30
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

SELECT c.seq, c.parent_ord
FROM vsp_children c
JOIN vsp_parents p ON p.ord = c.parent_ord
WHERE c.id @@@ pdb.all()
  AND p.id @@@ pdb.all()
  AND p.category = ANY('{alpha}')
  AND (p.metadata->>'score')::float8 BETWEEN 20 AND 30
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

RESET paradedb.term_set_bitset_max_density_multi;

-- Edge Case 2: Build distinct-key count exceeding `hash_join_inlist_pushdown_max_distinct_values`
-- on `uuid` (`Utf8View`) keys (`IN` and `JOIN`): `InList` pushdown does not fire, `lazy_sort`
-- ignores build selectivity (`initial_window = 2 * K`), `build_filters` drops `Utf8View`
-- `HashTableLookupExpr` (leaving only coarse string min/max bounds), and expanding IVF searches
-- walk past all `'noise'` rows.
SET paradedb.hash_join_inlist_pushdown_max_distinct_values = 1;

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT c.seq, c.parent_id
FROM vsp_children c
WHERE c.id @@@ pdb.all()
  AND c.parent_id IN (
    SELECT p.id FROM vsp_parents p
    WHERE p.id @@@ pdb.all()
      AND p.category = ANY('{alpha}')
      AND (p.metadata->>'score')::float8 BETWEEN 20 AND 30
  )
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

SELECT c.seq, c.parent_id
FROM vsp_children c
WHERE c.id @@@ pdb.all()
  AND c.parent_id IN (
    SELECT p.id FROM vsp_parents p
    WHERE p.id @@@ pdb.all()
      AND p.category = ANY('{alpha}')
      AND (p.metadata->>'score')::float8 BETWEEN 20 AND 30
  )
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT c.seq, c.parent_id
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all()
  AND p.id @@@ pdb.all()
  AND p.category = ANY('{alpha}')
  AND (p.metadata->>'score')::float8 BETWEEN 20 AND 30
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

SELECT c.seq, c.parent_id
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all()
  AND p.id @@@ pdb.all()
  AND p.category = ANY('{alpha}')
  AND (p.metadata->>'score')::float8 BETWEEN 20 AND 30
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

RESET paradedb.hash_join_inlist_pushdown_max_distinct_values;

-- Edge Case 3: Sparse join where total matching rows < K (exhausts LazySort cleanly)
SELECT c.seq, c.parent_id
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all()
  AND p.id @@@ pdb.all()
  AND p.category = ANY('{beta}')
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 10;

-- Edge Case 4: Anti-correlated parent filter with tiny initial window forcing multiple
-- LazySort window expansions (both HashJoinExec with InList disabled and RankJoinExec),
-- plus RankJoinExec single-batch `lazy_sort` + `Utf8View` memory reservation under tight `work_mem`.
SET paradedb.limit_fetch_multiplier = 1;
SET paradedb.topk_retry_scale_factor = 2;
SET paradedb.hash_join_inlist_pushdown_max_distinct_values = 0;

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT c.seq, c.parent_id
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all()
  AND p.id @@@ pdb.all()
  AND p.category = ANY('{alpha}')
  AND (p.metadata->>'score')::float8 BETWEEN 20 AND 30
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 1;

SELECT c.seq, c.parent_id
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all()
  AND p.id @@@ pdb.all()
  AND p.category = ANY('{alpha}')
  AND (p.metadata->>'score')::float8 BETWEEN 20 AND 30
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 1;

SET paradedb.rank_join_min_rows = 0;

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT c.seq, c.parent_id
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all()
  AND p.id @@@ pdb.all()
  AND p.category = ANY('{alpha}')
  AND (p.metadata->>'score')::float8 BETWEEN 20 AND 30
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 1;

SELECT c.seq, c.parent_id
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all()
  AND p.id @@@ pdb.all()
  AND p.category = ANY('{alpha}')
  AND (p.metadata->>'score')::float8 BETWEEN 20 AND 30
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 1;

RESET paradedb.limit_fetch_multiplier;
SET hash_mem_multiplier = 1.0;
SET work_mem = '64kB';

SELECT c.seq, c.parent_id
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all()
  AND p.id @@@ pdb.all()
  AND p.category = ANY('{alpha}')
  AND (p.metadata->>'score')::float8 BETWEEN 20 AND 30
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

RESET work_mem;
RESET hash_mem_multiplier;
RESET paradedb.topk_retry_scale_factor;
RESET paradedb.hash_join_inlist_pushdown_max_distinct_values;
RESET paradedb.rank_join_min_rows;

-- Edge Case 5: Residual non-equi join filter (`c.seq > p.ord * 10`) and MVCC dead-tuple filtering
SELECT c.seq, c.parent_id
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id AND c.seq > p.ord * 10
WHERE c.id @@@ pdb.all()
  AND p.id @@@ pdb.all()
  AND p.category = ANY('{alpha}')
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

BEGIN;
DELETE FROM vsp_children WHERE seq = 20;

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT c.seq, c.parent_id
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all()
  AND p.id @@@ pdb.all()
  AND p.category = ANY('{alpha}')
  AND (p.metadata->>'score')::float8 BETWEEN 20 AND 30
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

SELECT c.seq, c.parent_id
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all()
  AND p.id @@@ pdb.all()
  AND p.category = ANY('{alpha}')
  AND (p.metadata->>'score')::float8 BETWEEN 20 AND 30
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

ROLLBACK;

-- ============================================================
-- Planner decline warnings for unsupported vector join shapes
-- ============================================================
SET paradedb.ForceCustomScan = false;

-- 1. Wrong distance metric (<-> L2 on cosine index)
SELECT c.seq
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all() AND p.id @@@ pdb.all()
ORDER BY c.vec <-> '[1,0,0]'::vector, c.id
LIMIT 2;

-- 2. Descending vector distance
SELECT c.seq
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all() AND p.id @@@ pdb.all()
ORDER BY c.vec <=> '[1,0,0]'::vector DESC, c.id
LIMIT 2;

-- 3. Vector distance not in primary ORDER BY position
SELECT c.seq
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all() AND p.id @@@ pdb.all()
ORDER BY c.id, c.vec <=> '[1,0,0]'::vector
LIMIT 2;

-- 4. Missing LIMIT on vector-ordered join
SELECT c.seq
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all() AND p.id @@@ pdb.all()
  AND p.category = ANY('{alpha,beta}')
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id;

-- 5. DISTINCT with vector ORDER BY
SELECT DISTINCT c.id, c.vec <=> '[1,0,0]'::vector
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all() AND p.id @@@ pdb.all()
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

-- 6. Cross-table tie-breaker (tie-breaker on p instead of c)
SELECT c.seq
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all() AND p.id @@@ pdb.all()
ORDER BY c.vec <=> '[1,0,0]'::vector, p.category
LIMIT 2;

-- 7. Tie-breaker using pdb.score(c.id)
SELECT c.seq
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all() AND p.id @@@ pdb.all()
ORDER BY c.vec <=> '[1,0,0]'::vector, pdb.score(c.id) DESC, c.id
LIMIT 2;

-- 8. Conflicting pdb.score(c.id) projection on the vector-ordered table
SELECT c.seq, pdb.score(c.id)
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all() AND p.id @@@ pdb.all()
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

-- 9. Non-preserved join spine (LEFT JOIN where vector table is on the nullable right side)
SELECT p.ord, c.seq
FROM vsp_parents p
LEFT JOIN vsp_children c ON p.id = c.parent_id AND c.id @@@ pdb.all()
WHERE p.id @@@ pdb.all()
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

RESET paradedb.ForceCustomScan;

-- Edge Case 6: IVF segment where matching child rows (`100` `'gamma'` chunks) exceed initial
-- LazySort window (`4`) under HashJoinExec (exercises `uuid` InList pushdown on IVF and verifies
-- that HashJoinExec stops after the first LazySort window satisfies LIMIT K rather than pulling
-- additional expanding windows to fill an 8192-row batch).
-- Use `rank_join_unscored_selectivity_threshold = 0.99` so `gamma` (`10%`) runs on `HashJoinExec`.
SET paradedb.rank_join_min_rows = 1;
SET paradedb.rank_join_unscored_selectivity_threshold = 0.99;

-- 6a. Semi-join (`IN (SELECT ...)`) with 100 matching child rows and LIMIT 2
EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT c.seq, c.parent_id
FROM vsp_children c
WHERE c.id @@@ pdb.all()
  AND c.parent_id IN (
    SELECT p.id FROM vsp_parents p
    WHERE p.id @@@ pdb.all()
      AND p.category = ANY('{gamma}')
  )
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

SELECT c.seq, c.parent_id
FROM vsp_children c
WHERE c.id @@@ pdb.all()
  AND c.parent_id IN (
    SELECT p.id FROM vsp_parents p
    WHERE p.id @@@ pdb.all()
      AND p.category = ANY('{gamma}')
  )
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

-- 6b. Inner join (`JOIN`) with 100 matching child rows and LIMIT 2
EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT c.seq, c.parent_id
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all()
  AND p.id @@@ pdb.all()
  AND p.category = ANY('{gamma}')
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

SELECT c.seq, c.parent_id
FROM vsp_children c
JOIN vsp_parents p ON p.id = c.parent_id
WHERE c.id @@@ pdb.all()
  AND p.id @@@ pdb.all()
  AND p.category = ANY('{gamma}')
ORDER BY c.vec <=> '[1,0,0]'::vector, c.id
LIMIT 2;

RESET paradedb.rank_join_unscored_selectivity_threshold;
RESET paradedb.rank_join_min_rows;
RESET paradedb.vector_cluster_max_probe;
RESET paradedb.vector_clustering_threshold;
RESET max_parallel_workers_per_gather;
RESET client_min_messages;

DROP TABLE vsp_children, vsp_parents;


DROP TABLE vsp;
