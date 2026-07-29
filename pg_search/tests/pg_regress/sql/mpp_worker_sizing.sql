-- =====================================================================
-- #5667: the plan-first launch sizes the worker pool from the plan's
-- task fragments, with PG's parallelism GUCs as the ceiling.
--
--   - 2 segments per table under an oversized cap (per_gather = 8) must
--     launch exactly 2 producers. EXPLAIN ANALYZE's "MPP Launch:
--     workers=N" line is the launch's own report of what it spawned.
--   - 1 segment per table produces no distributable stages: the query
--     runs serially and forks nothing ("MPP Launch" line absent — it is
--     only recorded when the query actually launched).
--
-- Outcomes are collected in a table (not NOTICEs) so the expected
-- output stays stable under client_min_messages.
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_join_custom_scan TO on;
SET max_parallel_workers_per_gather TO 8;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;

CREATE TABLE mpp_ws_outcome (line text);

-- Two segments per index: two bulk inserts with the mutable segment disabled.
CREATE TABLE mpp_ws_users    (id bigserial PRIMARY KEY, name text, age int);
CREATE TABLE mpp_ws_products (id bigserial PRIMARY KEY, name text, age int);
CREATE INDEX mpp_ws_users_idx ON mpp_ws_users USING bm25 (id, name, age)
WITH (key_field='id', text_fields='{"name":{"tokenizer":{"type":"keyword"},"fast":true}}', numeric_fields='{"age":{"fast":true}}');
CREATE INDEX mpp_ws_products_idx ON mpp_ws_products USING bm25 (id, name, age)
WITH (key_field='id', text_fields='{"name":{"tokenizer":{"type":"keyword"},"fast":true}}', numeric_fields='{"age":{"fast":true}}');

SET paradedb.global_mutable_segment_rows = 0;
INSERT INTO mpp_ws_users (name, age)
SELECT (ARRAY['bob','alice'])[1 + (g % 2)], g % 50 FROM generate_series(1, 5000) g;
INSERT INTO mpp_ws_users (name, age)
SELECT (ARRAY['bob','alice'])[1 + (g % 2)], g % 50 FROM generate_series(5001, 10000) g;
INSERT INTO mpp_ws_products (name, age)
SELECT 'x', g % 50 FROM generate_series(1, 5000) g;
INSERT INTO mpp_ws_products (name, age)
SELECT 'x', g % 50 FROM generate_series(5001, 10000) g;
RESET paradedb.global_mutable_segment_rows;
ANALYZE mpp_ws_users;
ANALYZE mpp_ws_products;

-- One segment per index: a single insert each.
CREATE TABLE mpp_ws1_users    (id bigserial PRIMARY KEY, name text, age int);
CREATE TABLE mpp_ws1_products (id bigserial PRIMARY KEY, name text, age int);
CREATE INDEX mpp_ws1_users_idx ON mpp_ws1_users USING bm25 (id, name, age)
WITH (key_field='id', text_fields='{"name":{"tokenizer":{"type":"keyword"},"fast":true}}', numeric_fields='{"age":{"fast":true}}');
CREATE INDEX mpp_ws1_products_idx ON mpp_ws1_products USING bm25 (id, name, age)
WITH (key_field='id', text_fields='{"name":{"tokenizer":{"type":"keyword"},"fast":true}}', numeric_fields='{"age":{"fast":true}}');

SET paradedb.global_mutable_segment_rows = 0;
INSERT INTO mpp_ws1_users (name, age)
SELECT (ARRAY['bob','alice'])[1 + (g % 2)], g % 50 FROM generate_series(1, 10000) g;
INSERT INTO mpp_ws1_products (name, age)
SELECT 'x', g % 50 FROM generate_series(1, 10000) g;
RESET paradedb.global_mutable_segment_rows;
ANALYZE mpp_ws1_users;
ANALYZE mpp_ws1_products;

-- 2-segment tables, cap of 8 producers: the plan's widest stage has 2
-- tasks, so the launch must spawn exactly 2 workers, not the cap.
DO $$
DECLARE
    r record;
    launched int := -1;
BEGIN
    FOR r IN EXECUTE 'EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF)
        SELECT u.id FROM mpp_ws_users u JOIN mpp_ws_products p ON u.age = p.age
        WHERE u.name @@@ ''bob'' ORDER BY u.id LIMIT 10'
    LOOP
        IF r."QUERY PLAN" LIKE '%MPP Launch:%' THEN
            launched := (regexp_match(r."QUERY PLAN", 'workers=(\d+)'))[1]::int;
        END IF;
    END LOOP;
    IF launched = 2 THEN
        INSERT INTO mpp_ws_outcome VALUES ('2-segment join: launched exactly 2 producers');
    ELSIF launched = -1 THEN
        INSERT INTO mpp_ws_outcome VALUES ('2-segment join: UNEXPECTED serial run (no MPP Launch line)');
    ELSE
        INSERT INTO mpp_ws_outcome
        VALUES ('2-segment join: UNEXPECTED worker count ' || launched);
    END IF;
END$$;

-- 1-segment tables: nothing to distribute, so nothing is forked and no
-- launch is recorded.
DO $$
DECLARE
    r record;
    saw_launch boolean := false;
BEGIN
    FOR r IN EXECUTE 'EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF)
        SELECT u.id FROM mpp_ws1_users u JOIN mpp_ws1_products p ON u.age = p.age
        WHERE u.name @@@ ''bob'' ORDER BY u.id LIMIT 10'
    LOOP
        IF r."QUERY PLAN" LIKE '%MPP Launch:%' THEN
            saw_launch := true;
        END IF;
    END LOOP;
    IF saw_launch THEN
        INSERT INTO mpp_ws_outcome VALUES ('1-segment join: UNEXPECTED distributed launch');
    ELSE
        INSERT INTO mpp_ws_outcome VALUES ('1-segment join: ran serially, no workers forked');
    END IF;
END$$;

SELECT line FROM mpp_ws_outcome ORDER BY line;

DROP TABLE mpp_ws_outcome, mpp_ws_users, mpp_ws_products, mpp_ws1_users, mpp_ws1_products;
RESET paradedb.enable_join_custom_scan;
RESET max_parallel_workers_per_gather;
RESET max_parallel_workers;
RESET min_parallel_table_scan_size;
RESET parallel_setup_cost;
RESET parallel_tuple_cost;
