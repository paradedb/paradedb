-- Exact-distance rerank for TurboQuant vector ORDER BY.
--
-- Asserts that `paradedb.vector_rerank_multiplier > 1` engages the
-- heap-side rerank pass and produces the same top-K as a brute-force
-- exact cosine scan on the same data. The brute-force query bypasses
-- the BM25 index entirely (no `@@@` predicate), so it's guaranteed to
-- return the ground-truth ordering.

CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_search;

-- 200 rows, 8-d vectors. Small enough that a single segment covers the
-- whole corpus and TurboQuant defaults don't matter much, but we still
-- exercise the rerank plumbing end-to-end.
CREATE TABLE vrr (
    id  int PRIMARY KEY,
    emb vector(8)
);

INSERT INTO vrr
SELECT i,
       ('[' || array_to_string(
           ARRAY(SELECT random()::real FROM generate_series(1, 8)), ','
       ) || ']')::vector
FROM generate_series(1, 200) i;

CREATE INDEX vrr_idx ON vrr
    USING bm25 (id, emb vector_cosine_ops)
    WITH (key_field = id);

-- Ground truth: top-10 by exact cosine against a fixed query vector.
-- We pick row id=1's embedding as the query so the self-match is stable.
SELECT id FROM vrr
 ORDER BY emb <=> (SELECT emb FROM vrr WHERE id = 1)
 LIMIT 10;

-- Baseline: approximate top-10 via the BM25 custom scan, no rerank.
SET paradedb.vector_rerank_multiplier = 1.0;
SELECT id FROM vrr
 WHERE id @@@ paradedb.all()
 ORDER BY emb <=> (SELECT emb FROM vrr WHERE id = 1)
 LIMIT 10;

-- With rerank: over-fetches 5x and re-sorts by exact cosine. On this
-- small corpus rerank should exactly match the brute-force ground
-- truth above.
SET paradedb.vector_rerank_multiplier = 5.0;
SELECT id FROM vrr
 WHERE id @@@ paradedb.all()
 ORDER BY emb <=> (SELECT emb FROM vrr WHERE id = 1)
 LIMIT 10;

-- Filtered rerank: same behaviour when a scalar predicate is AND'd with
-- the vector ORDER BY. The filter cuts the candidate pool but the
-- rerank still produces exact-score top-K from what survives.
SELECT id FROM vrr
 WHERE id @@@ paradedb.all() AND id <= 50
 ORDER BY emb <=> (SELECT emb FROM vrr WHERE id = 1)
 LIMIT 5;

-- Reset the GUC so later tests aren't affected.
RESET paradedb.vector_rerank_multiplier;

DROP TABLE vrr;
