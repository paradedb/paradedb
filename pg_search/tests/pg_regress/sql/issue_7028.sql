\i common/common_setup.sql

-- A constant output column next to pdb.agg() must not make AggregateScan fail.
-- https://github.com/paradedb/paradedb/issues/7028

SET paradedb.enable_aggregate_custom_scan = on;
SET paradedb.global_mutable_segment_rows = 0;

DROP TABLE IF EXISTS issue_7028_logs CASCADE;
CREATE TABLE issue_7028_logs (id int PRIMARY KEY, description text, score int);
INSERT INTO issue_7028_logs VALUES
  (1, 'error in application', 10),
  (2, 'error in database', 20),
  (3, 'warning message', 30),
  (4, 'error in network', 40);
CREATE INDEX issue_7028_logs_idx ON issue_7028_logs USING paradedb (id, description, score);
ANALYZE issue_7028_logs;

-- Control: pdb.agg() alone.
EXPLAIN (COSTS OFF)
SELECT pdb.agg('{"value_count":{"field":"score"}}')
FROM issue_7028_logs WHERE description @@@ 'error';

SELECT pdb.agg('{"value_count":{"field":"score"}}')
FROM issue_7028_logs WHERE description @@@ 'error';

-- A constant before pdb.agg().
EXPLAIN (COSTS OFF, VERBOSE)
SELECT 'constant', pdb.agg('{"value_count":{"field":"score"}}')
FROM issue_7028_logs WHERE description @@@ 'error';

SELECT 'constant', pdb.agg('{"value_count":{"field":"score"}}')
FROM issue_7028_logs WHERE description @@@ 'error';

-- A constant after pdb.agg(), and constants of other types including NULL.
SELECT pdb.agg('{"value_count":{"field":"score"}}'), 42, NULL::text, true
FROM issue_7028_logs WHERE description @@@ 'error';

-- A constant next to a wrapped pdb.agg() goes through the wrapped projection.
SELECT 'constant', jsonb_pretty(pdb.agg('{"value_count":{"field":"score"}}'))
FROM issue_7028_logs WHERE description @@@ 'error';

-- Constants next to a grouping column and plain aggregates.
EXPLAIN (COSTS OFF, VERBOSE)
SELECT 'bucket', score, COUNT(*), 7
FROM issue_7028_logs WHERE description @@@ 'error'
GROUP BY score ORDER BY score;

SELECT 'bucket', score, COUNT(*), 7
FROM issue_7028_logs WHERE description @@@ 'error'
GROUP BY score ORDER BY score;

-- No matching rows: the constant is still returned with the aggregate.
SELECT 'constant', COUNT(*), pdb.agg('{"value_count":{"field":"score"}}')
FROM issue_7028_logs WHERE description @@@ 'nomatch';

-- Skew the per-score counts: 10 -> 3 rows, 20 -> 2 rows, 40 -> 1 row.
INSERT INTO issue_7028_logs VALUES
  (5, 'error in cache', 10),
  (6, 'error in queue', 10),
  (7, 'error in disk', 20);

-- GROUP BY a constant by position.
EXPLAIN (COSTS OFF, VERBOSE)
SELECT 'x' AS c, score, COUNT(*)
FROM issue_7028_logs WHERE description @@@ 'error'
GROUP BY 1, 2 ORDER BY 2;

SELECT 'x' AS c, score, COUNT(*)
FROM issue_7028_logs WHERE description @@@ 'error'
GROUP BY 1, 2 ORDER BY 2;

-- ORDER BY a constant first, with a LIMIT.
EXPLAIN (COSTS OFF, VERBOSE)
SELECT 'x', score, COUNT(*)
FROM issue_7028_logs WHERE description @@@ 'error'
GROUP BY score ORDER BY 1, score LIMIT 2;

SELECT 'x', score, COUNT(*)
FROM issue_7028_logs WHERE description @@@ 'error'
GROUP BY score ORDER BY 1, score LIMIT 2;

-- A by-reference (jsonb) constant read by a Sort above the scan.
EXPLAIN (COSTS OFF, VERBOSE)
SELECT * FROM (
  SELECT '{"k":[1]}'::jsonb j, score, COUNT(*) n
  FROM issue_7028_logs WHERE description @@@ 'error'
  GROUP BY score
) s ORDER BY n DESC, score;

SELECT * FROM (
  SELECT '{"k":[1]}'::jsonb j, score, COUNT(*) n
  FROM issue_7028_logs WHERE description @@@ 'error'
  GROUP BY score
) s ORDER BY n DESC, score;

-- HAVING still declines the scan; PostgreSQL computes the result.
EXPLAIN (COSTS OFF, VERBOSE)
SELECT 'x', score, COUNT(*)
FROM issue_7028_logs WHERE description @@@ 'error'
GROUP BY score HAVING COUNT(*) > 1 ORDER BY score;

SELECT 'x', score, COUNT(*)
FROM issue_7028_logs WHERE description @@@ 'error'
GROUP BY score HAVING COUNT(*) > 1 ORDER BY score;

DROP TABLE issue_7028_logs;

RESET paradedb.enable_aggregate_custom_scan;
RESET paradedb.global_mutable_segment_rows;

\i common/common_cleanup.sql
