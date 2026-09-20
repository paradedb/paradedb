-- #6341: unknown statistics fallback - without ANALYZE, stats are unavailable.
--
-- When pg_statistic is unavailable (unanalyzed tables), the statistics fall back
-- to the conservative behavior (scanned_rows estimate), which should preserve
-- the old behavior.

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.enable_join_custom_scan TO on;
SET max_parallel_workers_per_gather TO 0;

CREATE TABLE fanout_noanalyze_a (
    id SERIAL PRIMARY KEY,
    title TEXT,
    category TEXT
);
CREATE TABLE fanout_noanalyze_b (
    id SERIAL PRIMARY KEY,
    category TEXT
);

-- Small dictionary (3 terms) on both sides, many-to-many join
INSERT INTO fanout_noanalyze_a (title, category)
SELECT 'doc ' || i, (ARRAY['a', 'b', 'c'])[1 + i % 3]
FROM generate_series(1, 60) AS i;
INSERT INTO fanout_noanalyze_b (category)
SELECT (ARRAY['a', 'b', 'c'])[1 + i % 3]
FROM generate_series(1, 80) AS i;

CREATE INDEX fanout_noanalyze_a_idx ON fanout_noanalyze_a
USING bm25 (id, title, category)
WITH (text_fields='{"title": {}, "category": {"fast": true}}');
CREATE INDEX fanout_noanalyze_b_idx ON fanout_noanalyze_b
USING bm25 (id, category)
WITH (text_fields='{"category": {"fast": true}}');

-- NO ANALYZE - statistics unavailable

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT a.category, COUNT(*)
FROM fanout_noanalyze_a a JOIN fanout_noanalyze_b b ON a.category = b.category
WHERE a.title @@@ 'doc'
GROUP BY a.category
ORDER BY a.category;

SELECT a.category, COUNT(*)
FROM fanout_noanalyze_a a JOIN fanout_noanalyze_b b ON a.category = b.category
WHERE a.title @@@ 'doc'
GROUP BY a.category
ORDER BY a.category;

DROP TABLE fanout_noanalyze_a, fanout_noanalyze_b;