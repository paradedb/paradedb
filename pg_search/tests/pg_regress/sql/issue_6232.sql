\i common/common_setup.sql

-- Mixed aggregate + grouping-column output expressions on Tantivy AggregateScan.

SET max_parallel_workers_per_gather = 0;
SET paradedb.enable_aggregate_custom_scan = on;
SET enable_seqscan = off;

-- Grouping column is heap attribute 2.
CREATE TABLE mixed_check (id integer PRIMARY KEY, category text);
INSERT INTO mixed_check VALUES (1, 'a'), (2, 'a'), (3, 'b');
CREATE INDEX mixed_check_idx ON mixed_check USING paradedb (id, category)
  WITH (text_fields='{"category":{"fast":true}}');
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
CREATE INDEX mixed_check3_idx ON mixed_check3 USING paradedb (id, category)
  WITH (text_fields='{"category":{"fast":true}}');
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

-- Grouping by an indexed expression. The slot already holds reverse(category), so the
-- mixed expression must read it from the slot rather than apply reverse() to it again.
CREATE TABLE mixed_rev (id integer PRIMARY KEY, category text);
INSERT INTO mixed_rev VALUES (1, 'abc'), (2, 'abc'), (3, 'xyz');
CREATE INDEX mixed_rev_idx ON mixed_rev USING paradedb (id, (reverse(category)::pdb.literal));
ANALYZE mixed_rev;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT reverse(category), COUNT(*)::text || reverse(category) AS mixed
FROM mixed_rev WHERE id @@@ pdb.all()
GROUP BY reverse(category) ORDER BY 1;

SELECT reverse(category), COUNT(*)::text || reverse(category) AS mixed
FROM mixed_rev WHERE id @@@ pdb.all()
GROUP BY reverse(category) ORDER BY 1;

SELECT COUNT(*)::text || reverse(category) AS mixed, reverse(category)
FROM mixed_rev WHERE id @@@ pdb.all()
GROUP BY reverse(category) ORDER BY 2;

-- Grouping by the bare column: reverse() in the mixed expression runs once, on the slot value.
CREATE TABLE mixed_rev_col (id integer PRIMARY KEY, category text);
INSERT INTO mixed_rev_col VALUES (1, 'abc'), (2, 'abc'), (3, 'xyz');
CREATE INDEX mixed_rev_col_idx ON mixed_rev_col USING paradedb (id, category)
  WITH (text_fields='{"category":{"fast":true}}');
ANALYZE mixed_rev_col;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT category, COUNT(*)::text || reverse(category) AS mixed
FROM mixed_rev_col WHERE id @@@ pdb.all()
GROUP BY category ORDER BY 1;

SELECT category, COUNT(*)::text || reverse(category) AS mixed
FROM mixed_rev_col WHERE id @@@ pdb.all()
GROUP BY category ORDER BY 1;

-- Same queries without the custom scan, as the reference.
SET paradedb.enable_aggregate_custom_scan = off;

SELECT reverse(category), COUNT(*)::text || reverse(category) AS mixed
FROM mixed_rev WHERE id @@@ pdb.all()
GROUP BY reverse(category) ORDER BY 1;

SELECT category, COUNT(*)::text || reverse(category) AS mixed
FROM mixed_rev_col WHERE id @@@ pdb.all()
GROUP BY category ORDER BY 1;

RESET paradedb.enable_aggregate_custom_scan;

DROP TABLE mixed_check CASCADE;
DROP TABLE mixed_check3 CASCADE;
DROP TABLE mixed_rev CASCADE;
DROP TABLE mixed_rev_col CASCADE;
