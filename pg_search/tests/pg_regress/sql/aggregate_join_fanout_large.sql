-- #6341: large fan-out with a deferred string join key stays correct.
--
-- The join and the grouping are both on the deferred string key (category)
-- with a tiny dictionary (3 terms). Because the join itself evaluates the
-- deferred column, the logical rule anchors materialization below the join,
-- so both scans emit decoded strings and no deferred column reaches the
-- aggregate or any decode point: there is no TantivyDecodeExec and the
-- aggregate stays Single in both old and new behavior.
-- The EXPLAIN shape should show Single Aggregate -> HashJoinExec with no
-- Decode node, and identical result rows, for both.
-- This test guards the join-on-deferred anchor path and correctness under
-- large fan-out (60 * 80 / 3 = 1600 rows), not ordinal grouping.

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.enable_join_custom_scan TO on;
SET max_parallel_workers_per_gather TO 0;

CREATE TABLE fanout_large_a (
    id SERIAL PRIMARY KEY,
    title TEXT,
    category TEXT
);
CREATE TABLE fanout_large_b (
    id SERIAL PRIMARY KEY,
    category TEXT
);

-- 3 categories over many rows on both sides; many-to-many on category.
INSERT INTO fanout_large_a (title, category)
SELECT 'doc ' || i, (ARRAY['a', 'b', 'c'])[1 + i % 3]
FROM generate_series(1, 60) AS i;
INSERT INTO fanout_large_b (category)
SELECT (ARRAY['a', 'b', 'c'])[1 + i % 3]
FROM generate_series(1, 80) AS i;

CREATE INDEX fanout_large_a_idx ON fanout_large_a
USING bm25 (id, title, category)
WITH (text_fields='{"title": {}, "category": {"fast": true}}');
CREATE INDEX fanout_large_b_idx ON fanout_large_b
USING bm25 (id, category)
WITH (text_fields='{"category": {"fast": true}}');

ANALYZE fanout_large_a;
ANALYZE fanout_large_b;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT a.category, COUNT(*)
FROM fanout_large_a a JOIN fanout_large_b b ON a.category = b.category
WHERE a.title @@@ 'doc'
GROUP BY a.category
ORDER BY a.category;

SELECT a.category, COUNT(*)
FROM fanout_large_a a JOIN fanout_large_b b ON a.category = b.category
WHERE a.title @@@ 'doc'
GROUP BY a.category
ORDER BY a.category;

DROP TABLE fanout_large_a, fanout_large_b;