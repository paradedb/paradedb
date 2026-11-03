-- NOT with a phrase whose first match is after the first row of the text clause.
-- Debug builds panicked: Exclude asked PhraseScorer about a row before its
-- first match, which fails an assert (paradedb/tantivy#317).
CREATE EXTENSION IF NOT EXISTS pg_search;
SET max_parallel_maintenance_workers TO 0;
SET paradedb.planner_warnings = 'off';

CREATE TABLE must_not_late_phrase (id INT PRIMARY KEY, body TEXT, tag TEXT);
INSERT INTO must_not_late_phrase
SELECT i, 'alpha', CASE WHEN i >= 150 AND i % 10 = 0 THEN 'red apple' ELSE 'apple red' END
FROM generate_series(0, 299) i;

CREATE INDEX must_not_late_phrase_idx ON must_not_late_phrase
USING paradedb (id, body, tag)
WITH (target_segment_count = 1);

-- 15 rows contain "red apple", so 285 remain.
SELECT count(*) AS row_count
FROM must_not_late_phrase
WHERE body === 'alpha' AND NOT (tag ### 'red apple');

-- The same through Top K.
SELECT count(*) AS row_count
FROM (
    SELECT id FROM must_not_late_phrase
    WHERE body === 'alpha' AND NOT (tag ### 'red apple')
    ORDER BY pdb.score(id) DESC
    LIMIT 10
) top;

RESET paradedb.planner_warnings;
RESET max_parallel_maintenance_workers;
DROP TABLE must_not_late_phrase;
