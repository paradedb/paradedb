-- A deferred string column of an outer join's nullable side must come out NULL for the
-- null-extended rows when it is a Top-K sort key. `oj_dim` row 1 carries the smallest
-- `tag`, so a null-extended row that borrows the first document's ordinal would sort
-- ahead of every real row. Row 10 has a NULL `tag` of its own, and `oj_dim` spans more
-- than one segment so a borrowed ordinal can name a segment other than the first.

SET max_parallel_workers_per_gather = 0;
SET enable_indexscan TO OFF;

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS oj_dim CASCADE;
DROP TABLE IF EXISTS oj_fact CASCADE;
DROP TABLE IF EXISTS oj_side CASCADE;
DROP TABLE IF EXISTS oj_ref CASCADE;

CREATE TABLE oj_dim (id INT PRIMARY KEY, k INT, txt TEXT, tag TEXT, amt NUMERIC);
CREATE TABLE oj_fact (id INT PRIMARY KEY, k INT, txt TEXT);
CREATE TABLE oj_side (id INT PRIMARY KEY, txt TEXT);
-- `tag` here is a plain integer column that shares its name with the deferred column.
CREATE TABLE oj_ref (id INT PRIMARY KEY, k INT, tag OID);

CREATE INDEX oj_dim_idx ON oj_dim
USING paradedb (id, k, txt, tag, amt)
WITH (numeric_fields='{"k":{"fast":true},"amt":{"fast":true}}', text_fields='{"txt":{"fast":true},"tag":{"fast":true}}', mutable_segment_rows = 5);
CREATE INDEX oj_fact_idx ON oj_fact
USING paradedb (id, k, txt)
WITH (numeric_fields='{"k":{"fast":true}}', text_fields='{"txt":{"fast":true}}');
CREATE INDEX oj_side_idx ON oj_side
USING paradedb (id, txt)
WITH (text_fields='{"txt":{"fast":true}}');
CREATE INDEX oj_ref_idx ON oj_ref
USING paradedb (id, k, tag)
WITH (numeric_fields='{"k":{"fast":true},"tag":{"fast":true}}');

-- One insert of twenty rows lands in more than one segment. Rows 16..20 have a NULL `amt`.
INSERT INTO oj_dim
SELECT g, g, 'alpha item ' || g,
       CASE WHEN g = 1 THEN 'aaa' WHEN g = 10 THEN NULL ELSE 'tag' || lpad(g::text, 2, '0') END,
       CASE WHEN g > 15 THEN NULL ELSE g * 1.5 END
FROM generate_series(1, 20) g;
-- oj_fact rows 21..40 have no oj_dim partner.
INSERT INTO oj_fact
SELECT g, g, 'beta item ' || g
FROM generate_series(1, 40) g;
INSERT INTO oj_side
SELECT g, 'gamma item ' || g
FROM generate_series(1, 3) g;
INSERT INTO oj_ref
SELECT g, g, (1000 + g)::oid
FROM generate_series(1, 40) g;

ANALYZE oj_dim;
ANALYZE oj_fact;
ANALYZE oj_side;
ANALYZE oj_ref;

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

-- A bytes-backed deferred column (NUMERIC) on the nullable side.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT d.id, d.amt, f.id FROM oj_dim d RIGHT JOIN oj_fact f ON d.k = f.k WHERE f.txt @@@ 'beta' ORDER BY d.amt, f.id LIMIT 25;
SELECT d.id, d.amt, f.id FROM oj_dim d RIGHT JOIN oj_fact f ON d.k = f.k WHERE f.txt @@@ 'beta' ORDER BY d.amt, f.id LIMIT 25;

-- =============================================================================
-- The scan resolves the term ordinals itself
-- =============================================================================

SET paradedb.defer_column_fetch = off;
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT d.id, d.tag, f.id FROM oj_dim d RIGHT JOIN oj_fact f ON d.k = f.k WHERE f.txt @@@ 'beta' ORDER BY d.tag, f.id LIMIT 25;
SELECT d.id, d.tag, f.id FROM oj_dim d RIGHT JOIN oj_fact f ON d.k = f.k WHERE f.txt @@@ 'beta' ORDER BY d.tag, f.id LIMIT 25;
RESET paradedb.defer_column_fetch;

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
-- A third relation under the outer join
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT d.id, d.tag, f.id, s.id FROM oj_dim d RIGHT JOIN oj_fact f ON d.k = f.k CROSS JOIN oj_side s WHERE f.txt @@@ 'beta' ORDER BY d.tag, f.id, s.id LIMIT 70;
SELECT d.id, d.tag, f.id, s.id FROM oj_dim d RIGHT JOIN oj_fact f ON d.k = f.k CROSS JOIN oj_side s WHERE f.txt @@@ 'beta' ORDER BY d.tag, f.id, s.id LIMIT 70;

-- With no sort on the column, the join scan reads it from the heap.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT d.id, d.tag, f.id FROM oj_dim d RIGHT JOIN oj_fact f ON d.k = f.k WHERE f.txt @@@ 'beta' ORDER BY f.id LIMIT 40;
SELECT d.id, d.tag, f.id FROM oj_dim d RIGHT JOIN oj_fact f ON d.k = f.k WHERE f.txt @@@ 'beta' ORDER BY f.id LIMIT 40;

-- =============================================================================
-- A plain integer column with the deferred column's name on the other relation
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT d.id, d.tag, r.tag FROM oj_dim d JOIN oj_ref r ON d.k = r.k WHERE d.txt @@@ 'alpha' ORDER BY d.tag, r.tag LIMIT 5;
SELECT d.id, d.tag, r.tag FROM oj_dim d JOIN oj_ref r ON d.k = r.k WHERE d.txt @@@ 'alpha' ORDER BY d.tag, r.tag LIMIT 5;

DROP TABLE oj_ref;
DROP TABLE oj_side;
DROP TABLE oj_fact;
DROP TABLE oj_dim;
