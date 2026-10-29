-- Preserve NULL keys, nested paths, missing filtered groups, and output ordering.

\i common/common_setup.sql

SET paradedb.enable_aggregate_custom_scan TO on;

CREATE TABLE flatten_paths (id int, a int, b int, value int);
INSERT INTO flatten_paths VALUES
 (1, NULL, NULL, 2), (2, NULL, 1, 3), (3, 1, NULL, 4),
 (4, 1, 1, 5), (5, 1, 1, 6), (6, 2, 2, 7);
CREATE INDEX flatten_paths_idx ON flatten_paths USING paradedb (id, a, b, value);

EXPLAIN (COSTS OFF)
SELECT a, b, count(*), sum(value) FROM flatten_paths
WHERE id @@@ pdb.all() GROUP BY a, b ORDER BY a, b;

SELECT a, b, count(*), sum(value) FROM flatten_paths
WHERE id @@@ pdb.all() GROUP BY a, b ORDER BY a, b;

EXPLAIN (COSTS OFF)
SELECT a, b, count(*), count(*) FILTER (WHERE value > 4),
 sum(value) FILTER (WHERE value < 4) FROM flatten_paths
WHERE id @@@ pdb.all() GROUP BY a, b ORDER BY a DESC, b DESC;

SELECT a, b, count(*), count(*) FILTER (WHERE value > 4),
 sum(value) FILTER (WHERE value < 4) FROM flatten_paths
WHERE id @@@ pdb.all() GROUP BY a, b ORDER BY a DESC, b DESC;

-- Many groups, with groups 35-39 missing from the second filter
CREATE TABLE flatten_many (id int, g int, value int);
INSERT INTO flatten_many SELECT i, i % 40, i FROM generate_series(1, 400) i;
CREATE INDEX flatten_many_idx ON flatten_many USING paradedb (id, g, value);

EXPLAIN (COSTS OFF)
SELECT g, count(*), count(*) FILTER (WHERE value > 200),
 sum(value) FILTER (WHERE g < 35) FROM flatten_many
WHERE id @@@ pdb.all() GROUP BY g ORDER BY g;

SELECT g, count(*), count(*) FILTER (WHERE value > 200),
 sum(value) FILTER (WHERE g < 35) FROM flatten_many
WHERE id @@@ pdb.all() GROUP BY g ORDER BY g;

-- A NaN group gets its aggregates. Ordered by text, since the scan's key order puts NaN first
-- while Postgres sorts it last.
CREATE TABLE flatten_nan (id int, f float8, value int);
INSERT INTO flatten_nan VALUES (1, 'NaN', 2), (2, 'NaN', 3), (3, 1.5, 4);
CREATE INDEX flatten_nan_idx ON flatten_nan USING paradedb (id, f, value);

EXPLAIN (COSTS OFF)
WITH groups AS MATERIALIZED (
 SELECT f, count(*), sum(value) FROM flatten_nan WHERE id @@@ pdb.all() GROUP BY f)
SELECT * FROM groups ORDER BY f::text;

WITH groups AS MATERIALIZED (
 SELECT f, count(*), sum(value) FROM flatten_nan WHERE id @@@ pdb.all() GROUP BY f)
SELECT * FROM groups ORDER BY f::text;

RESET paradedb.enable_aggregate_custom_scan;
DROP TABLE flatten_paths;
DROP TABLE flatten_many;
DROP TABLE flatten_nan;
