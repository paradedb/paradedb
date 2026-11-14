CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE issue_6930_i (id int PRIMARY KEY, body text, f float8, r float4);
CREATE TABLE issue_6930_e (id int PRIMARY KEY, tag text, b int4, z float4);
INSERT INTO issue_6930_i VALUES (1, 'alpha', 0.0, 0.0), (2, 'beta', 1.0, 3.0);
INSERT INTO issue_6930_e VALUES (1, 'x', 1, 1.0), (2, 'y', 0, 0.0);
CREATE INDEX issue_6930_i_idx ON issue_6930_i USING paradedb (id, body, f, r);
CREATE INDEX issue_6930_e_idx ON issue_6930_e USING paradedb (id, tag, b, z);

SET paradedb.enable_join_custom_scan = on;

-- Float division in a Join Scan is evaluated natively
EXPLAIN (COSTS OFF)
SELECT i.id FROM issue_6930_i i JOIN issue_6930_e e ON e.id = i.id
WHERE i.body @@@ 'beta' AND (i.f / e.b) > 0 LIMIT 10;

-- Division by zero raises an error instead of yielding Infinity
SELECT i.id FROM issue_6930_i i JOIN issue_6930_e e ON e.id = i.id
WHERE i.body @@@ 'beta' AND (i.f / e.b) > 0 LIMIT 10;
SELECT i.id, (i.f / e.b) AS q FROM issue_6930_i i JOIN issue_6930_e e ON e.id = i.id
WHERE i.body @@@ 'beta' LIMIT 10;
SELECT i.id, (i.r / e.z) AS q FROM issue_6930_i i JOIN issue_6930_e e ON e.id = i.id
WHERE i.body @@@ 'beta' LIMIT 10;
SELECT i.id, (e.z / i.r) AS q FROM issue_6930_i i JOIN issue_6930_e e ON e.id = i.id
WHERE i.body @@@ 'alpha' LIMIT 10;

-- Division by a non-zero value is unchanged
SELECT i.id, (i.f / e.b) AS q, (i.r / e.z) AS p FROM issue_6930_i i JOIN issue_6930_e e ON e.id = i.id
WHERE i.body @@@ 'alpha' AND (i.f / e.b) >= 0 LIMIT 10;

-- Float division runs natively (#7135): x and d are float8, y and e are float4
CREATE TABLE issue_6930_a (id int PRIMARY KEY, body text, x float8, y float4);
CREATE TABLE issue_6930_b (id int PRIMARY KEY, tag text, d float8, e float4);
INSERT INTO issue_6930_a VALUES
    (1, 'ok', 6, 6), (2, 'ok', '-0', '-0'), (3, 'ok', 1, 1), (4, 'ok', -1, -1),
    (5, 'ok', 'Infinity', 'Infinity'), (6, 'ok', '-Infinity', '-Infinity'),
    (7, 'ok', 'Infinity', 'Infinity'), (8, 'ok', 'NaN', 'NaN'), (9, 'ok', 1, 1),
    (10, 'ok', NULL, NULL), (11, 'ok', 1, 1), (12, 'ok', 3e38, 3e38), (13, 'ok', 0.1, 0.1),
    (20, 'negzero', 1, 1), (21, 'zerozero', 0, 0), (22, 'infzero', 'Infinity', 'Infinity'),
    (23, 'big', 1e308, 3e38), (24, 'tiny', 1e-308, 1e-38),
    (30, 'round', 6, 6), (31, 'roundzero', 6, 6), (32, 'lnzero', 6, 6),
    (33, 'addover', 1, 3e38), (34, 'chain', 6, 6);
INSERT INTO issue_6930_b VALUES
    (1, 'x', 3, 3), (2, 'x', 5, 5), (3, 'x', 'Infinity', 'Infinity'),
    (4, 'x', 'Infinity', 'Infinity'), (5, 'x', 2, 2), (6, 'x', 2, 2),
    (7, 'x', 'Infinity', 'Infinity'), (8, 'x', 0, 0), (9, 'x', 'NaN', 'NaN'),
    (10, 'x', 0, 0), (11, 'x', NULL, NULL), (12, 'x', 0.5, 0.5), (13, 'x', 0.1, 0.1),
    (20, 'x', '-0', '-0'), (21, 'x', 0, 0), (22, 'x', 0, 0),
    (23, 'x', 0.5, 0.5), (24, 'x', 1e300, 1e30),
    (30, 'x', 2.5, 2.5), (31, 'x', 0.5, 0.5), (32, 'x', 0, 0),
    (33, 'x', 1, 1), (34, 'x', 1, 1);
CREATE INDEX issue_6930_a_idx ON issue_6930_a USING paradedb (id, body, x, y);
CREATE INDEX issue_6930_b_idx ON issue_6930_b USING paradedb (id, tag, d, e);

-- All float type mixes use pdb_float_div
EXPLAIN (COSTS OFF)
SELECT a.id, a.x / b.d, a.y / b.e, a.y / b.d FROM issue_6930_a a JOIN issue_6930_b b ON b.id = a.id
WHERE a.body @@@ 'ok' AND (a.x / b.e) > 0 LIMIT 20;

-- Signed zero, infinities, NaN, NULL and exact float4 widening (id 13).
-- 3e38 / 0.5 overflows float4 but not float8 (id 12).
SELECT a.id, a.x / b.d AS f8, a.y / b.e AS f4, a.y / b.d AS f48, a.x / b.e AS f84
FROM issue_6930_a a JOIN issue_6930_b b ON b.id = a.id
WHERE a.body @@@ 'ok' AND a.id <> 12 ORDER BY a.id LIMIT 20;
SELECT a.id, a.y / b.d AS f48, a.x / b.e AS f84
FROM issue_6930_a a JOIN issue_6930_b b ON b.id = a.id
WHERE a.body @@@ 'ok' AND a.id = 12 LIMIT 20;

-- real to double precision casts widen the exact float4 (id 13)
EXPLAIN (COSTS OFF)
SELECT a.id, a.y::float8 AS w, a.y::float8 / b.d AS q FROM issue_6930_a a JOIN issue_6930_b b ON b.id = a.id
WHERE a.body @@@ 'ok' AND a.id = 13 LIMIT 20;
SELECT a.id, a.y::float8 AS w, a.y::float8 / b.d AS q FROM issue_6930_a a JOIN issue_6930_b b ON b.id = a.id
WHERE a.body @@@ 'ok' AND a.id = 13 LIMIT 20;

-- Filters with OR and IS NULL
EXPLAIN (COSTS OFF)
SELECT a.id FROM issue_6930_a a JOIN issue_6930_b b ON b.id = a.id
WHERE a.body @@@ 'ok' AND a.id <> 12 AND ((a.x / b.d) > 1 OR (a.x / b.d) IS NULL)
ORDER BY a.id LIMIT 20;
SELECT a.id FROM issue_6930_a a JOIN issue_6930_b b ON b.id = a.id
WHERE a.body @@@ 'ok' AND a.id <> 12 AND ((a.x / b.d) > 1 OR (a.x / b.d) IS NULL)
ORDER BY a.id LIMIT 20;
-- -0 equals 0, as in PostgreSQL
SELECT a.id, a.x / b.d AS q FROM issue_6930_a a JOIN issue_6930_b b ON b.id = a.id
WHERE a.body @@@ 'ok' AND (a.x / b.d) = 0 ORDER BY a.id LIMIT 20;

-- Integer operands are cast to float natively, rounding like PostgreSQL
CREATE TABLE issue_6930_n (id int PRIMARY KEY, body text, s int2, i int4, l int8);
INSERT INTO issue_6930_n VALUES
    (1, 'ok', 7, 7, 7), (2, 'ok', -3, 16777217, 9007199254740993), (3, 'ok', 0, 0, 0);
CREATE INDEX issue_6930_n_idx ON issue_6930_n USING paradedb (id, body, s, i, l);
EXPLAIN (COSTS OFF)
SELECT a.x / n.i, a.y / n.s, n.l::float4 FROM issue_6930_a a JOIN issue_6930_n n ON n.id = a.id
WHERE a.body @@@ 'ok' AND (a.x / n.l) >= 0 LIMIT 20;
SELECT a.id, a.x / n.s AS f8s, a.x / n.i AS f8i, a.x / n.l AS f8l, a.y / n.i AS f4i,
       n.l::float8 AS l8, n.l::float4 AS l4, n.i::real AS i4, a.x > n.i AS gt
FROM issue_6930_a a JOIN issue_6930_n n ON n.id = a.id
WHERE a.body @@@ 'ok' AND a.id IN (1, 2) ORDER BY a.id LIMIT 20;
SELECT a.x / n.i FROM issue_6930_a a JOIN issue_6930_n n ON n.id = a.id
WHERE a.body @@@ 'ok' AND a.id = 3 LIMIT 20;

-- LEFT JOIN: id 4 has no match and is NULL
EXPLAIN (COSTS OFF)
SELECT a.id, a.x / n.i AS q FROM issue_6930_a a LEFT JOIN issue_6930_n n ON n.id = a.id
WHERE a.body @@@ 'ok' AND a.id IN (1, 2, 4) ORDER BY a.id LIMIT 20;
SELECT a.id, a.x / n.i AS q FROM issue_6930_a a LEFT JOIN issue_6930_n n ON n.id = a.id
WHERE a.body @@@ 'ok' AND a.id IN (1, 2, 4) ORDER BY a.id LIMIT 20;

-- Domains over float4
CREATE DOMAIN issue_6930_real AS real;
CREATE TABLE issue_6930_dm (id int PRIMARY KEY, body text, v issue_6930_real);
INSERT INTO issue_6930_dm VALUES (13, 'ok', 0.1);
CREATE INDEX issue_6930_dm_idx ON issue_6930_dm USING paradedb (id, body, v);
EXPLAIN (COSTS OFF)
SELECT dm.id, dm.v / b.d AS q
FROM issue_6930_dm dm JOIN issue_6930_b b ON b.id = dm.id
WHERE dm.body @@@ 'ok' LIMIT 20;
SELECT dm.id, dm.v / b.d AS q
FROM issue_6930_dm dm JOIN issue_6930_b b ON b.id = dm.id
WHERE dm.body @@@ 'ok' LIMIT 20;

-- Errors: division by zero, overflow and underflow
EXPLAIN (COSTS OFF)
SELECT a.x / b.d, a.y / b.e FROM issue_6930_a a JOIN issue_6930_b b ON b.id = a.id
WHERE a.body @@@ 'big' LIMIT 20;
SELECT a.x / b.d FROM issue_6930_a a JOIN issue_6930_b b ON b.id = a.id
WHERE a.body @@@ 'negzero' LIMIT 20;
SELECT a.x / b.d FROM issue_6930_a a JOIN issue_6930_b b ON b.id = a.id
WHERE a.body @@@ 'zerozero' LIMIT 20;
SELECT a.y / b.e FROM issue_6930_a a JOIN issue_6930_b b ON b.id = a.id
WHERE a.body @@@ 'infzero' LIMIT 20;
SELECT a.x / b.d FROM issue_6930_a a JOIN issue_6930_b b ON b.id = a.id
WHERE a.body @@@ 'big' LIMIT 20;
SELECT a.y / b.e FROM issue_6930_a a JOIN issue_6930_b b ON b.id = a.id
WHERE a.body @@@ 'big' LIMIT 20;
SELECT a.x / b.d FROM issue_6930_a a JOIN issue_6930_b b ON b.id = a.id
WHERE a.body @@@ 'tiny' LIMIT 20;
SELECT a.y / b.e FROM issue_6930_a a JOIN issue_6930_b b ON b.id = a.id
WHERE a.body @@@ 'tiny' LIMIT 20;

-- Operands DataFusion evaluates differently from PostgreSQL use the fallback
EXPLAIN (COSTS OFF)
SELECT a.x / round(b.d), a.x / ln(b.d), (a.y + a.y) / b.d
FROM issue_6930_a a JOIN issue_6930_b b ON b.id = a.id
WHERE a.body @@@ 'round' LIMIT 20;
SELECT a.x / round(b.d) FROM issue_6930_a a JOIN issue_6930_b b ON b.id = a.id
WHERE a.body @@@ 'round' LIMIT 20;
SELECT a.x / round(b.d) FROM issue_6930_a a JOIN issue_6930_b b ON b.id = a.id
WHERE a.body @@@ 'roundzero' LIMIT 20;
SELECT a.x / ln(b.d) FROM issue_6930_a a JOIN issue_6930_b b ON b.id = a.id
WHERE a.body @@@ 'lnzero' LIMIT 20;
SELECT (a.y + a.y) / b.d FROM issue_6930_a a JOIN issue_6930_b b ON b.id = a.id
WHERE a.body @@@ 'addover' LIMIT 20;

-- Long division chains use the fallback every ninth division
EXPLAIN (COSTS OFF)
SELECT a.id FROM issue_6930_a a JOIN issue_6930_b b ON b.id = a.id
WHERE a.body @@@ 'chain'
  AND a.x / b.d / b.d / b.d / b.d / b.d / b.d / b.d / b.d / b.d / b.d > 0 LIMIT 20;
-- The limit is on depth, so two shorter chains stay native
EXPLAIN (COSTS OFF)
SELECT a.id FROM issue_6930_a a JOIN issue_6930_b b ON b.id = a.id
WHERE a.body @@@ 'chain'
  AND (a.x / b.d / b.d / b.d / b.d) / (a.x / b.d / b.d / b.d / b.d) > 0 LIMIT 20;
SELECT format('SELECT a.id, a.x%s AS q FROM issue_6930_a a JOIN issue_6930_b b ON b.id = a.id '
              'WHERE a.body @@@ %L AND a.x%s > 0 LIMIT 20',
              repeat(' / b.d', 100), 'chain', repeat(' / b.d', 100)) \gexec
SELECT format('SELECT a.id, a.y%s AS q FROM issue_6930_a a JOIN issue_6930_b b ON b.id = a.id '
              'WHERE a.body @@@ %L AND a.y%s > 0 LIMIT 20',
              repeat(' / b.e', 100), 'chain', repeat(' / b.e', 100)) \gexec

-- MPP
CREATE TABLE issue_6930_ma (id int PRIMARY KEY, body text, x float8);
CREATE TABLE issue_6930_mb (id int PRIMARY KEY, d float8);
CREATE INDEX issue_6930_ma_idx ON issue_6930_ma USING paradedb (id, body, x);
CREATE INDEX issue_6930_mb_idx ON issue_6930_mb USING paradedb (id, d);
SET paradedb.global_mutable_segment_rows TO 0;
INSERT INTO issue_6930_ma SELECT g, 'mpp', g FROM generate_series(1, 50) g;
INSERT INTO issue_6930_ma SELECT g, 'mpp', g FROM generate_series(51, 100) g;
INSERT INTO issue_6930_ma VALUES (101, 'mppzero', 1);
INSERT INTO issue_6930_mb SELECT g, 1 + g % 4 FROM generate_series(1, 50) g;
INSERT INTO issue_6930_mb SELECT g, 1 + g % 4 FROM generate_series(51, 100) g;
INSERT INTO issue_6930_mb VALUES (101, 0);
RESET paradedb.global_mutable_segment_rows;
ANALYZE issue_6930_ma;
ANALYZE issue_6930_mb;

SET paradedb.mpp_min_rows TO 0;
SET paradedb.enable_range_partitioned_join TO on;
SET max_parallel_workers_per_gather TO 3;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;
EXPLAIN (COSTS OFF)
SELECT a.id, a.x / b.d AS q FROM issue_6930_ma a JOIN issue_6930_mb b ON b.id = a.id
WHERE a.body @@@ 'mpp' AND a.x / b.d > 30 ORDER BY a.id LIMIT 5;
SELECT a.id, a.x / b.d AS q FROM issue_6930_ma a JOIN issue_6930_mb b ON b.id = a.id
WHERE a.body @@@ 'mpp' AND a.x / b.d > 30 ORDER BY a.id LIMIT 5;
SELECT a.id, a.x / b.d AS q FROM issue_6930_ma a JOIN issue_6930_mb b ON b.id = a.id
WHERE a.body @@@ 'mppzero' ORDER BY a.id LIMIT 5;
-- A worker reports the error like the fallback does
SELECT a.id FROM issue_6930_ma a JOIN issue_6930_mb b ON b.id = a.id
WHERE a.body @@@ 'mppzero' AND a.x / b.d > 0 ORDER BY a.id LIMIT 5;
RESET paradedb.mpp_min_rows;
RESET paradedb.enable_range_partitioned_join;
RESET max_parallel_workers_per_gather;
RESET max_parallel_workers;
RESET min_parallel_table_scan_size;
RESET parallel_setup_cost;
RESET parallel_tuple_cost;

RESET paradedb.enable_join_custom_scan;

DROP TABLE issue_6930_i;
DROP TABLE issue_6930_e;
DROP TABLE issue_6930_a;
DROP TABLE issue_6930_b;
DROP TABLE issue_6930_n;
DROP TABLE issue_6930_dm;
DROP TABLE issue_6930_ma;
DROP TABLE issue_6930_mb;
DROP DOMAIN issue_6930_real;
