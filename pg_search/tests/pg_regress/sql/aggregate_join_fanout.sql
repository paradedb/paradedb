-- #6341: the ordinal-aggregate decision must measure the aggregate's input.
--
-- The group key (a.category) is a deferred string with a tiny dictionary
-- (3 terms, so terms * MIN_ROWS_PER_TERM is 12), and it stays deferred through
-- the join: the join is on the numeric id, not on the group key. The far side
-- of the join is small (5 rows), so the join output is far below either scan's
-- own estimate.
-- Old behavior: scanned_rows ~= 60 >= 3*4 keeps the deferred ordinal grouping:
-- Final Aggregate -> TantivyDecodeExec -> Partial Aggregate -> HashJoinExec.
-- New behavior: the aggregate-input join estimate ~= 5 < 3*4 rejects the
-- ordinal Partial aggregate, so the plan remains a Single Aggregate.
-- The ungrouped column is then eagerly decoded in the scan (structural
-- Expansion::Yes on the non-unique product_id key), so there is no
-- TantivyDecodeExec left: Single Aggregate -> HashJoinExec with
-- eager=[category]. The EXPLAIN shape distinguishes the new behavior
-- from the old Final/Decode/Partial shape.
--
-- Both implementations return the same rows, so the EXPLAIN shape is the
-- regression assertion.

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.enable_join_custom_scan TO on;
SET max_parallel_workers_per_gather TO 0;

CREATE TABLE fanout_a (
    id SERIAL PRIMARY KEY,
    title TEXT,
    category TEXT
);
CREATE TABLE fanout_b (
    id SERIAL PRIMARY KEY,
    product_id INTEGER
);

-- Three categories over many rows on the grouped side; five rows on the far
-- side referencing them, so the join output (~5 rows) is far below the
-- grouped scan's own estimate (~60 rows).
INSERT INTO fanout_a (title, category)
SELECT 'doc ' || i, (ARRAY['a', 'b', 'c'])[1 + i % 3]
FROM generate_series(1, 60) AS i;
INSERT INTO fanout_b (product_id)
SELECT id FROM fanout_a WHERE id <= 5;

CREATE INDEX fanout_a_idx ON fanout_a
USING bm25 (id, title, category)
WITH (text_fields='{"title": {}, "category": {"fast": true}}');
CREATE INDEX fanout_b_idx ON fanout_b
USING bm25 (id, product_id)
WITH (numeric_fields='{"product_id": {"fast": true}}');

-- Scan estimates need reltuples; without ANALYZE both sides report Unknown and
-- the decision falls back to the scan estimate either way.
ANALYZE fanout_a;
ANALYZE fanout_b;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT a.category, COUNT(*)
FROM fanout_a a JOIN fanout_b b ON a.id = b.product_id
WHERE a.title @@@ 'doc'
GROUP BY a.category
ORDER BY a.category;

SELECT a.category, COUNT(*)
FROM fanout_a a JOIN fanout_b b ON a.id = b.product_id
WHERE a.title @@@ 'doc'
GROUP BY a.category
ORDER BY a.category;

DROP TABLE fanout_a, fanout_b;
