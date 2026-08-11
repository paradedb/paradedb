-- Hybrid search via Reciprocal Rank Fusion:
--
--   SELECT ...
--   FROM t
--   WHERE <text predicate>                                  -- candidate set
--   ORDER BY pdb.rrf(pdb.score(id), vec <op> '[...]' [, k]) -- fused ranking
--   LIMIT n;
--
-- pdb.rrf() fuses the query's BM25 ranking with a vector-distance ranking
-- and returns the fused rank (1 = best), so plain ascending ORDER BY puts
-- the best match first (pgvector convention, no DESC needed).
--
-- pdb.score(relation, type) projects the components:
--   * 'bm25'   — the text predicate's BM25 score (what pdb.score(relation)
--                means in every scan shape)
--   * 'vector' — the vector leg's similarity
--   * 'hybrid' — the positive fused RRF score pdb.rrf() ranks by
--   * 'rank'   — the 1-based fused rank, identical to pdb.rrf()'s value
--
-- The corpus is built so BM25 scores and vector similarities are distinct
-- (no rank ties) in the flagship queries. Scores are rounded to 6 decimals.

CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE hyb (
    id    int PRIMARY KEY,
    label text,
    vec   vector(3)
);

INSERT INTO hyb VALUES
    (1, 'east wind',        '[1,   0,   0]'),
    (2, 'east gate',        '[0.9, 0,   0.1]'),
    (3, 'north wind blows', '[0,   1,   0]'),
    (4, 'up draft',         '[0,   0,   1]'),
    (5, 'mid point',        '[0.7, 0.7, 0]');

CREATE INDEX hyb_idx ON hyb
    USING bm25 (id, label, vec vector_cosine_ops)
    WITH (key_field = id);


-- ============================================================
-- flagship: fused ordering + all score components
-- ============================================================
-- Matches docs 1, 2, 3. BM25 ranks: 1, 2, 3. Vector ranks for [0.6,0.8,0]:
-- doc3=1, doc1=2, doc2=3. RRF (k=60): doc1 = 1/61+1/62, doc3 = 1/63+1/61,
-- doc2 = 1/62+1/63, so the fused order (1, 3, 2) differs from both legs.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM hyb
WHERE label ||| 'east wind'
ORDER BY pdb.rrf(pdb.score(id), vec <=> '[0.6,0.8,0]')
LIMIT 3;

SELECT id,
       pdb.rrf(pdb.score(id), vec <=> '[0.6,0.8,0]')          AS fused_rank,
       round(pdb.score(id)::numeric, 6)                       AS bm25,
       round(pdb.score(id, type => 'bm25')::numeric, 6)       AS bm25_typed,
       round(pdb.score(id, type => 'vector')::numeric, 6)     AS similarity,
       round(pdb.score(id, type => 'hybrid')::numeric, 6)     AS hybrid,
       pdb.score(id, type => 'rank')                          AS rank
FROM hyb
WHERE label ||| 'east wind'
ORDER BY pdb.rrf(pdb.score(id), vec <=> '[0.6,0.8,0]')
LIMIT 3;

-- ordering by the output alias works too
SELECT id, r AS fused_rank
FROM (
    SELECT id, pdb.rrf(pdb.score(id), vec <=> '[0.6,0.8,0]') AS r
    FROM hyb
    WHERE label ||| 'east wind'
    ORDER BY r
    LIMIT 3
) fused;

-- the legs may be written in either order
SELECT id, pdb.rrf(vec <=> '[0.6,0.8,0]', pdb.score(id)) AS fused_rank
FROM hyb
WHERE label ||| 'east wind'
ORDER BY pdb.rrf(vec <=> '[0.6,0.8,0]', pdb.score(id))
LIMIT 3;

-- DESC honestly reverses the fused ranking (worst first)
SELECT id, pdb.rrf(pdb.score(id), vec <=> '[0.6,0.8,0]') AS fused_rank
FROM hyb
WHERE label ||| 'east wind'
ORDER BY pdb.rrf(pdb.score(id), vec <=> '[0.6,0.8,0]') DESC
LIMIT 3;

-- k is tunable via named argument
SELECT id,
       pdb.rrf(pdb.score(id), vec <=> '[0.6,0.8,0]', k => 1)      AS fused_rank,
       round(pdb.score(id, type => 'hybrid')::numeric, 6)         AS hybrid
FROM hyb
WHERE label ||| 'east wind'
ORDER BY pdb.rrf(pdb.score(id), vec <=> '[0.6,0.8,0]', k => 1)
LIMIT 3;

-- so is the per-leg candidate window (the overfetch); it shows in EXPLAIN
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM hyb
WHERE label ||| 'east wind'
ORDER BY pdb.rrf(pdb.score(id), vec <=> '[0.6,0.8,0]', window_size => 200)
LIMIT 3;
SELECT id, pdb.rrf(pdb.score(id), vec <=> '[0.6,0.8,0]', window_size => 200) AS fused_rank
FROM hyb
WHERE label ||| 'east wind'
ORDER BY pdb.rrf(pdb.score(id), vec <=> '[0.6,0.8,0]', window_size => 200)
LIMIT 3;


-- per-arm windows: shrinking the vector arm's window to the page size
-- truncates its contribution. Vector top-3 for [0.05,0.1,0.99] is docs
-- 4, 2, 5, so doc 3 (vector rank 4 with the default window) loses its
-- vector contribution and doc 4 overtakes it: 2, 1, 4 instead of 2, 1, 3.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM hyb
WHERE label ||| 'east wind' OR vec ~~~ '[0.05,0.1,0.99]'
ORDER BY pdb.rrf(id, vector_window_size => 3)
LIMIT 3;
SELECT id, pdb.score(id, type => 'rank') AS fused_rank
FROM hyb
WHERE label ||| 'east wind' OR vec ~~~ '[0.05,0.1,0.99]'
ORDER BY pdb.rrf(id, vector_window_size => 3)
LIMIT 3;


-- ::pdb.top(n) arm annotations: per-arm windows written on the arms
-- themselves. Equivalent to the vector_window_size => 3 query above (2,1,4),
-- and the windows are visible in the Tantivy Query.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM hyb
WHERE (label ||| 'east wind')::pdb.top(100) OR (vec ~~~ '[0.05,0.1,0.99]')::pdb.top(3)
ORDER BY pdb.rrf(id)
LIMIT 3;
SELECT id, pdb.score(id, type => 'rank') AS fused_rank
FROM hyb
WHERE (label ||| 'east wind')::pdb.top(100) OR (vec ~~~ '[0.05,0.1,0.99]')::pdb.top(3)
ORDER BY pdb.rrf(id)
LIMIT 3;

-- a ::pdb.top arm cannot mix text and knn predicates
SELECT id FROM hyb
WHERE (label ||| 'east wind' AND vec ~~~ '[1,0,0]')::pdb.top(5)
ORDER BY pdb.rrf(id)
LIMIT 3;

-- multiple text arms are not supported yet
SELECT id FROM hyb
WHERE (label ||| 'east')::pdb.top(5) OR (label ||| 'wind')::pdb.top(5) OR vec ~~~ '[1,0,0]'
ORDER BY pdb.rrf(id)
LIMIT 3;

-- a window given both on the arm and on pdb.rrf() is a conflict
SELECT id FROM hyb
WHERE label ||| 'east wind' OR (vec ~~~ '[0.05,0.1,0.99]')::pdb.top(3)
ORDER BY pdb.rrf(id, vector_window_size => 5)
LIMIT 3;

-- ::pdb.top requires the rank-fusion ordering
SELECT id FROM hyb
WHERE (label ||| 'east wind')::pdb.top(5)
ORDER BY id
LIMIT 3;


-- ============================================================
-- filtered semantics: WHERE defines the candidate set
-- ============================================================
-- doc4 is the nearest neighbor of [0.2,0.1,0.97] but does not match the text
-- predicate, so it never surfaces. (docs 1 and 2 tie on the fused score by
-- symmetric rank swap; the tie breaks deterministically by document order.)
SELECT id, pdb.rrf(pdb.score(id), vec <=> '[0.2,0.1,0.97]') AS fused_rank
FROM hyb
WHERE label ||| 'east wind'
ORDER BY pdb.rrf(pdb.score(id), vec <=> '[0.2,0.1,0.97]')
LIMIT 4;


-- ============================================================
-- union hybrid via the ~~~ knn predicate
-- ============================================================
-- Predicate scoping follows boolean position: predicates ANDed outside the
-- OR apply to both legs; predicates grouped inside a branch apply to that
-- leg only. With a ~~~ predicate the fully-implied form `pdb.rrf(id)`
-- suffices: the BM25 leg is the relation's text query and the vector leg its
-- knn predicate, so the query vector is written once. (The explicit two-leg
-- form is still allowed and validated against the predicate.)

-- doc4 'up draft' is the nearest neighbor of [0.05,0.1,0.99] but matches no
-- text; with `OR vec ~~~ ...` it surfaces (union semantics), with a NULL
-- bm25 score. Vector ranks over all docs: 4, 2, 5, 3, 1; bm25 ranks: 1, 2, 3.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM hyb
WHERE label ||| 'east wind' OR vec ~~~ '[0.05,0.1,0.99]'
ORDER BY pdb.rrf(id)
LIMIT 5;

SELECT id,
       round(pdb.score(id)::numeric, 6)                   AS bm25,
       round(pdb.score(id, type => 'vector')::numeric, 6) AS similarity,
       round(pdb.score(id, type => 'hybrid')::numeric, 6) AS hybrid
FROM hyb
WHERE label ||| 'east wind' OR vec ~~~ '[0.05,0.1,0.99]'
ORDER BY pdb.rrf(id)
LIMIT 5;

-- a shared filter (ANDed outside the OR) constrains BOTH legs: doc5 is gone
-- from the vector leg entirely
SELECT id,
       round(pdb.score(id)::numeric, 6) AS bm25,
       pdb.score(id, type => 'rank')    AS fused_rank
FROM hyb
WHERE id < 5 AND (label ||| 'east wind' OR vec ~~~ '[0.05,0.1,0.99]')
ORDER BY pdb.rrf(pdb.score(id), vec <=> '[0.05,0.1,0.99]')
LIMIT 5;

-- a predicate grouped with the text matcher applies to the bm25 leg only:
-- doc3 matches the text but is excluded from the bm25 leg by id < 3, so it
-- only surfaces (and ranks) via the vector leg
SELECT id,
       round(pdb.score(id)::numeric, 6) AS bm25,
       pdb.score(id, type => 'rank')    AS fused_rank
FROM hyb
WHERE (label ||| 'east wind' AND id < 3) OR vec ~~~ '[0.05,0.1,0.99]'
ORDER BY pdb.rrf(id)
LIMIT 5;

-- intersection: text AND ~~~ — candidates must match BOTH legs, and the
-- fusion still ranks by text relevance and proximity: bm25 ranks 1,2,3
-- (docs 1,2,3) fuse with vector ranks 2,1,3 giving the order 2, 1, 3
SELECT id,
       round(pdb.score(id)::numeric, 6)                   AS bm25,
       round(pdb.score(id, type => 'hybrid')::numeric, 6) AS hybrid,
       pdb.score(id, type => 'rank')                      AS fused_rank
FROM hyb
WHERE label ||| 'east wind' AND vec ~~~ '[0.05,0.1,0.99]'
ORDER BY pdb.rrf(id)
LIMIT 5;

-- knn-only WHERE: every row surfaces via the vector leg, text scores NULL
SELECT id, round(pdb.score(id)::numeric, 6) AS bm25, pdb.score(id, type => 'rank') AS fused_rank
FROM hyb
WHERE vec ~~~ '[0.05,0.1,0.99]'
ORDER BY pdb.rrf(pdb.score(id))  -- relation reference may also be spelled pdb.score(id)
LIMIT 3;


-- ============================================================
-- parameterized query vector (`<=> $1`) inside pdb.rrf()
-- ============================================================
SET plan_cache_mode = force_generic_plan;
PREPARE hyb_p(vector) AS
SELECT id FROM hyb
WHERE label ||| 'east wind'
ORDER BY pdb.rrf(pdb.score(id), vec <=> $1)
LIMIT 3;

EXECUTE hyb_p('[0.6,0.8,0]');
EXECUTE hyb_p('[0.2,0.1,0.97]');

DEALLOCATE hyb_p;
RESET plan_cache_mode;


-- ============================================================
-- typed score projection under a plain vector-distance ordering
-- ============================================================
-- pdb.score(relation) means the BM25 score in every scan shape, including
-- here where the scan itself is ordered by similarity; 'vector' projects
-- that similarity.
SELECT id,
       round(pdb.score(id)::numeric, 6)                   AS bm25,
       round(pdb.score(id, type => 'vector')::numeric, 6) AS similarity,
       pdb.score(id) = pdb.score(id, type => 'bm25')      AS score_is_bm25
FROM hyb
WHERE label ||| 'east wind'
ORDER BY vec <=> '[0.6,0.8,0]' LIMIT 3;

-- plain pdb.score(id) on a text query is unchanged
SELECT id, pdb.score(id) = pdb.score(id, type => 'bm25') AS typed_matches_untyped
FROM hyb
WHERE label @@@ 'wind'
ORDER BY id;


-- ============================================================
-- error cases
-- ============================================================
-- pdb.rrf() must drive the ordering
SELECT id, pdb.rrf(pdb.score(id), vec <=> '[1,0,0]') FROM hyb WHERE label @@@ 'wind';

-- 'hybrid' requires a rank-fusion ordering
SELECT id, pdb.score(id, type => 'hybrid') FROM hyb WHERE label @@@ 'wind';

-- 'vector' requires a vector ranking
SELECT id, pdb.score(id, type => 'vector') FROM hyb WHERE label @@@ 'wind';

-- 'rank' requires a rank-fusion ordering
SELECT id, pdb.score(id, type => 'rank') FROM hyb WHERE label @@@ 'wind';

-- unrecognized score type
SELECT id, pdb.score(id, type => 'nope') FROM hyb WHERE label @@@ 'wind';

-- the score type must be a constant
SELECT id, pdb.score(id, label) FROM hyb WHERE label @@@ 'wind';

-- query vector dimensionality must match the indexed field
SELECT id FROM hyb
WHERE label ||| 'east wind'
ORDER BY pdb.rrf(pdb.score(id), vec <=> '[1,2]')
LIMIT 3;
SELECT id FROM hyb
WHERE label ||| 'east wind'
ORDER BY vec <=> '[1,2]'
LIMIT 3;

-- pdb.rrf() without a distance leg requires a ~~~ predicate
SELECT id FROM hyb
WHERE label @@@ 'wind'
ORDER BY pdb.rrf(id)
LIMIT 3;

-- ~~~ requires the rank-fusion ordering
SELECT id FROM hyb WHERE label ||| 'east wind' OR vec ~~~ '[1,0,0]';

-- ~~~ cannot be negated
SELECT id FROM hyb
WHERE label @@@ 'wind' AND NOT (vec ~~~ '[1,0,0]')
ORDER BY pdb.rrf(pdb.score(id), vec <=> '[1,0,0]')
LIMIT 3;

-- only one ~~~ predicate per query
SELECT id FROM hyb
WHERE vec ~~~ '[1,0,0]' OR vec ~~~ '[0,1,0]'
ORDER BY pdb.rrf(pdb.score(id), vec <=> '[1,0,0]')
LIMIT 3;

-- the ~~~ query vector must match pdb.rrf()'s distance leg
SELECT id FROM hyb
WHERE label ||| 'east wind' OR vec ~~~ '[1,0,0]'
ORDER BY pdb.rrf(pdb.score(id), vec <=> '[0,1,0]')
LIMIT 3;


-- partitioned tables are rejected: per-partition fused ranks cannot be
-- merged into a global ranking
CREATE TABLE hyb_part (id int, label text, vec vector(3), PRIMARY KEY (id)) PARTITION BY RANGE (id);
CREATE TABLE hyb_part_1 PARTITION OF hyb_part FOR VALUES FROM (0) TO (100);
CREATE TABLE hyb_part_2 PARTITION OF hyb_part FOR VALUES FROM (100) TO (200);
INSERT INTO hyb_part VALUES (1, 'east wind', '[1,0,0]'), (101, 'east gate', '[0.9,0,0.1]');
CREATE INDEX hyb_part_idx ON hyb_part USING bm25 (id, label, vec vector_cosine_ops) WITH (key_field = id);
SELECT id FROM hyb_part
WHERE label ||| 'east wind' OR vec ~~~ '[1,0,0]'
ORDER BY pdb.rrf(id)
LIMIT 3;
DROP TABLE hyb_part;

DROP TABLE hyb;
