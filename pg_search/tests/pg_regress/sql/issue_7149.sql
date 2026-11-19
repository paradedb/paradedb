-- Join filters over NUMERIC columns must see decoded values, not the scaled
-- integers in the fast field: numeric(10,2) stores 1.10 as 110 and
-- numeric(10,3) stores 1.100 as 1100.
CREATE EXTENSION IF NOT EXISTS pg_search;
SET max_parallel_workers_per_gather = 0;
SET paradedb.enable_join_custom_scan = off;

CREATE TABLE pa (id bigint PRIMARY KEY, txt text NOT NULL, amt numeric(10,2));
CREATE TABLE pb (id bigint PRIMARY KEY, amt numeric(10,3));
CREATE TABLE pc (id bigint PRIMARY KEY, amt numeric(10,2));
CREATE TABLE pd (id bigint PRIMARY KEY, amt numeric(20,2));
CREATE TABLE sa (id bigint PRIMARY KEY, txt text NOT NULL, amt numeric(10,2));
CREATE TABLE sc (id bigint PRIMARY KEY, amt numeric(10,2));
INSERT INTO pa SELECT g, 'match', g + 0.10 FROM generate_series(1, 5) g;
INSERT INTO pb SELECT g, g + 0.100 FROM generate_series(1, 5) g;
INSERT INTO pc SELECT g, g + 0.10 FROM generate_series(1, 5) g;
INSERT INTO pd SELECT g, g + 0.10 FROM generate_series(1, 5) g;
INSERT INTO sa SELECT g, 'match', 0.50 FROM generate_series(1, 5) g;
INSERT INTO sc SELECT g, 0.50 FROM generate_series(1, 5) g;
CREATE INDEX pa_idx ON pa USING paradedb (id, txt, amt)
WITH (text_fields='{"txt":{"fast":true}}', numeric_fields='{"amt":{"fast":true}}');
CREATE INDEX pb_idx ON pb USING paradedb (id, amt) WITH (numeric_fields='{"amt":{"fast":true}}');
CREATE INDEX pc_idx ON pc USING paradedb (id, amt) WITH (numeric_fields='{"amt":{"fast":true}}');
CREATE INDEX pd_idx ON pd USING paradedb (id, amt) WITH (numeric_fields='{"amt":{"fast":true}}');
CREATE INDEX sa_idx ON sa USING paradedb (id, txt, amt)
WITH (text_fields='{"txt":{"fast":true}}', numeric_fields='{"amt":{"fast":true}}');
CREATE INDEX sc_idx ON sc USING paradedb (id, amt) WITH (numeric_fields='{"amt":{"fast":true}}');

-- Different scales: every pair compares equal.
EXPLAIN (COSTS OFF)
SELECT count(*) FROM pa a JOIN pb b ON a.id = b.id AND a.amt <> b.amt WHERE a.txt @@@ 'match';
SELECT count(*) FROM pa a JOIN pb b ON a.id = b.id AND a.amt <> b.amt WHERE a.txt @@@ 'match';
SELECT count(*) FROM pa a JOIN pb b ON a.id = b.id AND a.amt < b.amt WHERE a.txt @@@ 'match';

-- Different scales under OR.
SELECT count(*) FROM pa a JOIN pb b ON a.id = b.id AND (a.amt > 3 OR b.amt > 4) WHERE a.txt @@@ 'match';

-- Same scale, different storage (Numeric64 and NumericBytes).
SELECT count(*) FROM pa a JOIN pd d ON a.id = d.id AND a.amt <> d.amt WHERE a.txt @@@ 'match';

-- Same scale: a plain comparison.
EXPLAIN (COSTS OFF)
SELECT count(*) FROM pa a JOIN pc c ON a.id = c.id AND a.amt <> c.amt WHERE a.txt @@@ 'match';
SELECT count(*) FROM pa a JOIN pc c ON a.id = c.id AND a.amt <> c.amt WHERE a.txt @@@ 'match';

-- Same scale: arithmetic and functions must not run over scaled integers.
SELECT count(*) FROM sa a JOIN sc c ON a.id = c.id AND a.amt * c.amt < a.amt WHERE a.txt @@@ 'match';
SELECT count(*) FROM pa a JOIN pc c ON a.id = c.id AND floor(a.amt) <> c.amt WHERE a.txt @@@ 'match';

-- The aggregate scan's multi-table filter.
EXPLAIN (COSTS OFF)
SELECT count(*) FROM pa a LEFT JOIN pc c ON a.id = c.id
WHERE a.txt @@@ 'match' AND (c.amt IS NULL OR floor(a.amt) <> c.amt);
SELECT count(*) FROM pa a LEFT JOIN pc c ON a.id = c.id
WHERE a.txt @@@ 'match' AND (c.amt IS NULL OR floor(a.amt) <> c.amt);

-- JoinScan evaluates its filters the same way.
SET paradedb.enable_join_custom_scan = on;
EXPLAIN (COSTS OFF)
SELECT a.id FROM pa a JOIN pb b ON a.id = b.id AND a.amt <> b.amt
WHERE a.txt @@@ 'match' ORDER BY a.id LIMIT 10;
SELECT a.id FROM pa a JOIN pb b ON a.id = b.id AND a.amt <> b.amt
WHERE a.txt @@@ 'match' ORDER BY a.id LIMIT 10;
SELECT a.id FROM pa a JOIN pc c ON a.id = c.id AND floor(a.amt) <> c.amt
WHERE a.txt @@@ 'match' ORDER BY a.id LIMIT 10;

RESET paradedb.enable_join_custom_scan;
DROP TABLE pa, pb, pc, pd, sa, sc;
