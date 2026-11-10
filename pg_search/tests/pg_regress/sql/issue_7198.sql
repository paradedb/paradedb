-- string_agg with a separator column runs in the Aggregate Scan (#7198).
-- Each view runs on the Aggregate Scan, then on PostgreSQL; the results must match.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET max_parallel_workers_per_gather = 0;
SET paradedb.enable_join_custom_scan = off;

CREATE TABLE issue_7198_a (id int PRIMARY KEY, name text, sep text, grp int);
CREATE TABLE issue_7198_b (id int PRIMARY KEY, a_id int, sep text);
INSERT INTO issue_7198_a VALUES
    (1, NULL, '#', 1), (2, 'b', '-', 1), (3, 'c', NULL, 1), (4, '', '+', 1), (5, 'é', '→ ', 1),
    (6, 'x', '-', 2), (7, 'y', ';', 2), (8, 'z', '|', 2),
    (9, NULL, '!', 3), (10, NULL, NULL, 3);
INSERT INTO issue_7198_b
SELECT g, g, CASE WHEN g = 7 THEN NULL ELSE '/' END FROM generate_series(1, 10) g;
CREATE INDEX issue_7198_a_idx ON issue_7198_a
USING paradedb (id, (name::pdb.literal), (sep::pdb.literal), grp);
CREATE INDEX issue_7198_b_idx ON issue_7198_b USING paradedb (id, a_id, (sep::pdb.literal));

-- The #7150 repro, with the separator column in the index.
CREATE VIEW issue_7198_repro AS
SELECT string_agg(a.name, a.sep ORDER BY a.id) AS s
FROM issue_7198_a a JOIN issue_7198_b b ON b.a_id = a.id
WHERE a.id @@@ pdb.all() AND a.grp = 2;

-- Both orders, and a separator from the joined table.
CREATE VIEW issue_7198_grouped AS
SELECT a.grp,
       string_agg(a.name, a.sep ORDER BY a.id) AS asc_sep,
       string_agg(a.name, a.sep ORDER BY a.id DESC) AS desc_sep,
       string_agg(a.name, b.sep ORDER BY a.id) AS join_sep
FROM issue_7198_a a JOIN issue_7198_b b ON b.a_id = a.id
WHERE a.id @@@ pdb.all()
GROUP BY a.grp;

-- Without ORDER BY the order is unspecified, so accept any valid order.
CREATE VIEW issue_7198_unordered AS
SELECT string_agg(a.name, a.sep)
       IN ('x;y|z', 'x|z;y', 'y-x|z', 'y|z-x', 'z-x;y', 'z;y-x') AS valid
FROM issue_7198_a a JOIN issue_7198_b b ON b.a_id = a.id
WHERE a.id @@@ pdb.all() AND a.grp = 2;

EXPLAIN (COSTS OFF) SELECT * FROM issue_7198_repro;
EXPLAIN (COSTS OFF) SELECT * FROM issue_7198_grouped ORDER BY grp;
EXPLAIN (COSTS OFF) SELECT * FROM issue_7198_unordered;

SELECT * FROM issue_7198_repro;
SELECT * FROM issue_7198_grouped ORDER BY grp;
SELECT * FROM issue_7198_unordered;

SET paradedb.enable_aggregate_custom_scan = off;
SELECT * FROM issue_7198_repro;
SELECT * FROM issue_7198_grouped ORDER BY grp;
SELECT * FROM issue_7198_unordered;
RESET paradedb.enable_aggregate_custom_scan;

-- Still PostgreSQL: a separator expression, and DISTINCT.
SELECT string_agg(a.name, a.sep || '' ORDER BY a.id)
FROM issue_7198_a a JOIN issue_7198_b b ON b.a_id = a.id
WHERE a.id @@@ pdb.all() AND a.grp = 2;

SELECT string_agg(DISTINCT a.name, a.sep)
FROM issue_7198_a a JOIN issue_7198_b b ON b.a_id = a.id
WHERE a.id @@@ pdb.all() AND a.grp = 2;

-- MPP: workers run the Partial aggregate and the leader merges their states.
-- Separate inserts create multiple index segments for MPP workers to scan.
SET paradedb.global_mutable_segment_rows = 0;
CREATE TABLE issue_7198_ma (id int PRIMARY KEY, name text, sep text, grp int);
CREATE TABLE issue_7198_mb (id int PRIMARY KEY, a_id int);
CREATE INDEX issue_7198_ma_idx ON issue_7198_ma
USING paradedb (id, (name::pdb.literal), (sep::pdb.literal), grp);
CREATE INDEX issue_7198_mb_idx ON issue_7198_mb USING paradedb (id, a_id);
INSERT INTO issue_7198_ma
SELECT g, CASE WHEN g % 7 = 0 THEN NULL ELSE 'v' || g END,
       (ARRAY['-', ';', '|', NULL, ', '])[g % 5 + 1], g % 4
FROM generate_series(1, 500) g;
INSERT INTO issue_7198_ma
SELECT g, CASE WHEN g % 7 = 0 THEN NULL ELSE 'v' || g END,
       (ARRAY['-', ';', '|', NULL, ', '])[g % 5 + 1], g % 4
FROM generate_series(501, 1000) g;
INSERT INTO issue_7198_mb SELECT g, g FROM generate_series(1, 500) g;
INSERT INTO issue_7198_mb SELECT g, g FROM generate_series(501, 1000) g;
RESET paradedb.global_mutable_segment_rows;
ANALYZE issue_7198_ma;
ANALYZE issue_7198_mb;

CREATE VIEW issue_7198_mpp AS
SELECT a.grp,
       md5(string_agg(a.name, a.sep ORDER BY a.id)) AS asc_sep,
       md5(string_agg(a.name, a.sep ORDER BY a.id DESC)) AS desc_sep
FROM issue_7198_ma a JOIN issue_7198_mb b ON b.a_id = a.id
WHERE a.id @@@ pdb.all()
GROUP BY a.grp;

SET max_parallel_workers_per_gather = 3;
SET max_parallel_workers = 8;
SET min_parallel_table_scan_size = 0;
SET parallel_setup_cost = 0;
SET parallel_tuple_cost = 0;
SET paradedb.mpp_min_rows = 0;
SET paradedb.enable_join_custom_scan = on;

EXPLAIN (COSTS OFF) SELECT * FROM issue_7198_mpp ORDER BY grp;
SELECT * FROM issue_7198_mpp ORDER BY grp;

SET paradedb.enable_aggregate_custom_scan = off;
SET max_parallel_workers_per_gather = 0;
SELECT * FROM issue_7198_mpp ORDER BY grp;

RESET paradedb.enable_aggregate_custom_scan;
RESET paradedb.enable_join_custom_scan;
RESET paradedb.mpp_min_rows;
RESET parallel_tuple_cost;
RESET parallel_setup_cost;
RESET min_parallel_table_scan_size;
RESET max_parallel_workers;
RESET max_parallel_workers_per_gather;

DROP VIEW issue_7198_mpp;
DROP VIEW issue_7198_unordered;
DROP VIEW issue_7198_grouped;
DROP VIEW issue_7198_repro;
DROP TABLE issue_7198_mb;
DROP TABLE issue_7198_ma;
DROP TABLE issue_7198_b;
DROP TABLE issue_7198_a;
