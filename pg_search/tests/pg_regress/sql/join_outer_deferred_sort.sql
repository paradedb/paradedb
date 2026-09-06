-- A deferred string column of an outer join's nullable side must come out NULL for
-- the null-extended rows, both as a Top-K sort key and as an output column.
-- `oj_dim` row 1 carries the smallest `tag`, so a null-extended row that borrows the
-- first document's ordinal would sort ahead of every real row.

SET max_parallel_workers_per_gather = 0;
SET enable_indexscan TO OFF;

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS oj_dim CASCADE;
DROP TABLE IF EXISTS oj_fact CASCADE;
DROP TABLE IF EXISTS oj_side CASCADE;

CREATE TABLE oj_dim (id INT PRIMARY KEY, k INT, txt TEXT, tag TEXT);
CREATE TABLE oj_fact (id INT PRIMARY KEY, k INT, txt TEXT);
CREATE TABLE oj_side (id INT PRIMARY KEY, txt TEXT);

-- oj_fact rows 21..40 have no oj_dim partner.
INSERT INTO oj_dim
SELECT g, g, 'alpha item ' || g, CASE WHEN g = 1 THEN 'aaa' ELSE 'tag' || lpad(g::text, 2, '0') END
FROM generate_series(1, 20) g;
INSERT INTO oj_fact
SELECT g, g, 'beta item ' || g
FROM generate_series(1, 40) g;
INSERT INTO oj_side
SELECT g, 'gamma item ' || g
FROM generate_series(1, 3) g;

CREATE INDEX oj_dim_idx ON oj_dim
USING paradedb (id, k, txt, tag)
WITH (key_field='id', numeric_fields='{"k":{"fast":true}}', text_fields='{"txt":{"fast":true},"tag":{"fast":true}}');
CREATE INDEX oj_fact_idx ON oj_fact
USING paradedb (id, k, txt)
WITH (key_field='id', numeric_fields='{"k":{"fast":true}}', text_fields='{"txt":{"fast":true}}');
CREATE INDEX oj_side_idx ON oj_side
USING paradedb (id, txt)
WITH (key_field='id', text_fields='{"txt":{"fast":true}}');

ANALYZE oj_dim;
ANALYZE oj_fact;
ANALYZE oj_side;

SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- RIGHT JOIN: the left input is null-extended
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT d.id, d.tag, f.id FROM oj_dim d RIGHT JOIN oj_fact f ON d.k = f.k WHERE f.txt @@@ 'beta' ORDER BY d.tag, f.id LIMIT 25;
SELECT d.id, d.tag, f.id FROM oj_dim d RIGHT JOIN oj_fact f ON d.k = f.k WHERE f.txt @@@ 'beta' ORDER BY d.tag, f.id LIMIT 25;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT d.id, d.tag, f.id FROM oj_dim d RIGHT JOIN oj_fact f ON d.k = f.k WHERE f.txt @@@ 'beta' ORDER BY d.tag DESC, f.id LIMIT 25;
SELECT d.id, d.tag, f.id FROM oj_dim d RIGHT JOIN oj_fact f ON d.k = f.k WHERE f.txt @@@ 'beta' ORDER BY d.tag DESC, f.id LIMIT 25;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT d.id, d.tag, f.id FROM oj_dim d RIGHT JOIN oj_fact f ON d.k = f.k WHERE f.txt @@@ 'beta' ORDER BY d.tag NULLS FIRST, f.id LIMIT 25;
SELECT d.id, d.tag, f.id FROM oj_dim d RIGHT JOIN oj_fact f ON d.k = f.k WHERE f.txt @@@ 'beta' ORDER BY d.tag NULLS FIRST, f.id LIMIT 25;

-- A mixed ON condition keeps the nullable side deferred as well.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT d.id, d.tag, f.id FROM oj_dim d RIGHT JOIN oj_fact f ON d.k = f.k AND d.id <= f.id WHERE f.txt @@@ 'beta' ORDER BY d.tag, f.id LIMIT 25;
SELECT d.id, d.tag, f.id FROM oj_dim d RIGHT JOIN oj_fact f ON d.k = f.k AND d.id <= f.id WHERE f.txt @@@ 'beta' ORDER BY d.tag, f.id LIMIT 25;

-- =============================================================================
-- LEFT JOIN: the right input is null-extended
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT d.id, d.tag, f.id FROM oj_fact f LEFT JOIN oj_dim d ON d.k = f.k WHERE f.txt @@@ 'beta' ORDER BY d.tag, f.id LIMIT 25;
SELECT d.id, d.tag, f.id FROM oj_fact f LEFT JOIN oj_dim d ON d.k = f.k WHERE f.txt @@@ 'beta' ORDER BY d.tag, f.id LIMIT 25;

-- =============================================================================
-- FULL JOIN: both inputs are null-extended
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT d.id, d.tag, f.id FROM oj_dim d FULL JOIN oj_fact f ON d.k = f.k WHERE f.txt @@@ 'beta' OR d.txt @@@ 'alpha' ORDER BY d.tag, f.id LIMIT 25;
SELECT d.id, d.tag, f.id FROM oj_dim d FULL JOIN oj_fact f ON d.k = f.k WHERE f.txt @@@ 'beta' OR d.txt @@@ 'alpha' ORDER BY d.tag, f.id LIMIT 25;

-- =============================================================================
-- A third relation under the outer join, and the column as plain output
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT d.id, d.tag, f.id, s.id FROM oj_dim d RIGHT JOIN oj_fact f ON d.k = f.k CROSS JOIN oj_side s WHERE f.txt @@@ 'beta' ORDER BY d.tag, f.id, s.id LIMIT 70;
SELECT d.id, d.tag, f.id, s.id FROM oj_dim d RIGHT JOIN oj_fact f ON d.k = f.k CROSS JOIN oj_side s WHERE f.txt @@@ 'beta' ORDER BY d.tag, f.id, s.id LIMIT 70;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT d.id, d.tag, f.id FROM oj_dim d RIGHT JOIN oj_fact f ON d.k = f.k WHERE f.txt @@@ 'beta' ORDER BY f.id LIMIT 40;
SELECT d.id, d.tag, f.id FROM oj_dim d RIGHT JOIN oj_fact f ON d.k = f.k WHERE f.txt @@@ 'beta' ORDER BY f.id LIMIT 40;

DROP TABLE oj_side;
DROP TABLE oj_fact;
DROP TABLE oj_dim;
