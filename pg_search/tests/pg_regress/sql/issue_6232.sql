\i common/common_setup.sql

-- Mixed aggregate + grouping-column output expressions on Tantivy AggregateScan.

SET max_parallel_workers_per_gather = 0;
SET paradedb.enable_aggregate_custom_scan = on;
SET enable_seqscan = off;

-- Grouping column is heap attribute 2.
CREATE TABLE mixed_check (id integer PRIMARY KEY, category text);
INSERT INTO mixed_check VALUES (1, 'a'), (2, 'a'), (3, 'b');
CREATE INDEX mixed_check_idx ON mixed_check USING bm25 (id, category)
  WITH (key_field='id', text_fields='{"category":{"fast":true}}');
ANALYZE mixed_check;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT category, COUNT(*)::text || category AS mixed
FROM mixed_check WHERE mixed_check @@@ paradedb.all()
GROUP BY category ORDER BY category;

SELECT category, COUNT(*)::text || category AS mixed
FROM mixed_check WHERE mixed_check @@@ paradedb.all()
GROUP BY category ORDER BY category;

-- Grouping column is heap attribute 3.
CREATE TABLE mixed_check3 (id integer PRIMARY KEY, filler text, category text);
INSERT INTO mixed_check3 VALUES (1, 'x', 'a'), (2, 'x', 'a'), (3, 'x', 'b');
CREATE INDEX mixed_check3_idx ON mixed_check3 USING bm25 (id, category)
  WITH (key_field='id', text_fields='{"category":{"fast":true}}');
ANALYZE mixed_check3;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT category, COUNT(*)::text || category AS mixed
FROM mixed_check3 WHERE mixed_check3 @@@ paradedb.all()
GROUP BY category ORDER BY category;

SELECT category, COUNT(*)::text || category AS mixed
FROM mixed_check3 WHERE mixed_check3 @@@ paradedb.all()
GROUP BY category ORDER BY category;

-- Mixed expression before the grouping column in the target list.
SELECT COUNT(*)::text || category AS mixed, category
FROM mixed_check WHERE mixed_check @@@ paradedb.all()
GROUP BY category ORDER BY category;

DROP TABLE mixed_check CASCADE;
DROP TABLE mixed_check3 CASCADE;
