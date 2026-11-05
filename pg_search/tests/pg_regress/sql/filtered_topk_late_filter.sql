-- Top K with a filter whose first match is after the first row of the text clause.
-- Debug builds panicked: two range filters overflowed in RangeDocSet, and a
-- constant-score phrase failed an assert in PhraseScorer (paradedb/tantivy#316).
CREATE EXTENSION IF NOT EXISTS pg_search;
SET max_parallel_maintenance_workers TO 0;
SET paradedb.planner_warnings = 'off';

CREATE TABLE filtered_topk_late_filter (id INT PRIMARY KEY, body TEXT, tag TEXT, n INT, m INT);
INSERT INTO filtered_topk_late_filter
SELECT i, 'alpha', CASE WHEN i >= 150 AND i % 10 = 0 THEN 'red apple' ELSE 'apple red' END, i, i
FROM generate_series(0, 299) i;

CREATE INDEX filtered_topk_late_filter_idx ON filtered_topk_late_filter
USING paradedb (id, body, tag, n, m)
WITH (target_segment_count = 1);

-- Two range filters: the first row in both ranges is 150.
SELECT count(*) AS row_count, min(id) AS min_id
FROM (
    SELECT id FROM filtered_topk_late_filter
    WHERE body === 'alpha' AND n >= 100 AND m >= 150
    ORDER BY pdb.score(id) DESC
    LIMIT 10
) top;

-- A constant-score phrase: its first match is row 150.
SELECT count(*) AS row_count, min(id) AS min_id
FROM (
    SELECT id FROM filtered_topk_late_filter
    WHERE body === 'alpha' AND tag ### 'red apple'::pdb.const(1)
    ORDER BY pdb.score(id) DESC
    LIMIT 10
) top;

RESET paradedb.planner_warnings;
RESET max_parallel_maintenance_workers;
DROP TABLE filtered_topk_late_filter;
