-- Issue #6734: a parallel Base Scan with a bitmap intersection, under a Gather
-- that is rescanned (here, in a correlated subquery). Postgres reinitializes the
-- DSM, launches the workers, and only then runs the leader's deferred ReScan,
-- which freed the claim table and the bitmap the workers had just attached.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET client_min_messages = warning;

DROP TABLE IF EXISTS clean_units CASCADE;
DROP TABLE IF EXISTS drv CASCADE;
CREATE TABLE clean_units (id bigint PRIMARY KEY, bank_id text NOT NULL, body text NOT NULL);
CREATE INDEX clean_bank ON clean_units (bank_id);
-- No mutable segment, so each INSERT below is its own segment: three segments
-- give the parallel plan two workers on every machine.
CREATE INDEX clean_search ON clean_units USING paradedb (id, body)
    WITH (mutable_segment_rows = '0');
INSERT INTO clean_units
SELECT i, 'bank' || (i % 4), 'alpha beta ' || i FROM generate_series(1, 7000) i;
INSERT INTO clean_units
SELECT i, 'bank' || (i % 4), 'alpha beta ' || i FROM generate_series(7001, 14000) i;
INSERT INTO clean_units
SELECT i, 'bank' || (i % 4), 'alpha beta ' || i FROM generate_series(14001, 20000) i;
ANALYZE clean_units;
CREATE TABLE drv AS SELECT g AS n FROM generate_series(1, 3) g;

SET paradedb.enable_bitmap_intersection TO on;
SET max_parallel_workers_per_gather TO 2;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;
SET min_parallel_table_scan_size TO 0;
SET min_parallel_index_scan_size TO 0;
SET paradedb.min_rows_per_worker TO 0;
SET debug_parallel_query TO on;

-- The issue's query: Gather Merge, rescanned once per outer row.
EXPLAIN (COSTS OFF)
SELECT d.n, (SELECT array_agg(s.id ORDER BY s.id) FROM (
    SELECT id FROM clean_units
    WHERE bank_id = 'bank1' AND id @@@ paradedb.match('body', 'alpha')
    ORDER BY paradedb.score(id) DESC, id LIMIT 3) s WHERE d.n > 0) AS ids
FROM drv d ORDER BY d.n;
SELECT d.n, (SELECT array_agg(s.id ORDER BY s.id) FROM (
    SELECT id FROM clean_units
    WHERE bank_id = 'bank1' AND id @@@ paradedb.match('body', 'alpha')
    ORDER BY paradedb.score(id) DESC, id LIMIT 3) s WHERE d.n > 0) AS ids
FROM drv d ORDER BY d.n;

-- The bitmap follows an initPlan of the outer row, so each round rebuilds it
-- before its workers launch.
EXPLAIN (COSTS OFF)
SELECT d.n, (SELECT array_agg(s.id ORDER BY s.id) FROM (
    SELECT id FROM clean_units
    WHERE bank_id = (SELECT 'bank' || d.n) AND id @@@ paradedb.match('body', 'alpha')
    ORDER BY paradedb.score(id) DESC, id LIMIT 3) s) AS ids
FROM drv d ORDER BY d.n;
SELECT d.n, (SELECT array_agg(s.id ORDER BY s.id) FROM (
    SELECT id FROM clean_units
    WHERE bank_id = (SELECT 'bank' || d.n) AND id @@@ paradedb.match('body', 'alpha')
    ORDER BY paradedb.score(id) DESC, id LIMIT 3) s) AS ids
FROM drv d ORDER BY d.n;

-- A plain Gather under a Limit, rescanned the same way.
EXPLAIN (COSTS OFF)
SELECT d.n, (SELECT count(*) FROM (
    SELECT id FROM (
        SELECT id FROM clean_units
        WHERE bank_id = 'bank1' AND id @@@ paradedb.match('body', 'alpha') OFFSET 0) x
    LIMIT 3) s WHERE d.n > 0) AS rows
FROM drv d ORDER BY d.n;
SELECT d.n, (SELECT count(*) FROM (
    SELECT id FROM (
        SELECT id FROM clean_units
        WHERE bank_id = 'bank1' AND id @@@ paradedb.match('body', 'alpha') OFFSET 0) x
    LIMIT 3) s WHERE d.n > 0) AS rows
FROM drv d ORDER BY d.n;

-- The same queries serially, for reference.
SET max_parallel_workers_per_gather TO 0;
SELECT d.n, (SELECT array_agg(s.id ORDER BY s.id) FROM (
    SELECT id FROM clean_units
    WHERE bank_id = 'bank1' AND id @@@ paradedb.match('body', 'alpha')
    ORDER BY paradedb.score(id) DESC, id LIMIT 3) s WHERE d.n > 0) AS ids
FROM drv d ORDER BY d.n;
SELECT d.n, (SELECT array_agg(s.id ORDER BY s.id) FROM (
    SELECT id FROM clean_units
    WHERE bank_id = (SELECT 'bank' || d.n) AND id @@@ paradedb.match('body', 'alpha')
    ORDER BY paradedb.score(id) DESC, id LIMIT 3) s) AS ids
FROM drv d ORDER BY d.n;
SELECT d.n, (SELECT count(*) FROM (
    SELECT id FROM (
        SELECT id FROM clean_units
        WHERE bank_id = 'bank1' AND id @@@ paradedb.match('body', 'alpha') OFFSET 0) x
    LIMIT 3) s WHERE d.n > 0) AS rows
FROM drv d ORDER BY d.n;

RESET debug_parallel_query;
RESET paradedb.min_rows_per_worker;
RESET min_parallel_index_scan_size;
RESET min_parallel_table_scan_size;
RESET parallel_tuple_cost;
RESET parallel_setup_cost;
RESET max_parallel_workers_per_gather;
RESET paradedb.enable_bitmap_intersection;
DROP TABLE drv;
DROP TABLE clean_units;
