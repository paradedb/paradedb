-- =====================================================================
-- #5667: the plan-first launch sizes the worker pool from the plan's
-- task fragments, with PG's parallelism GUCs as the ceiling.
--
--   - 2 segments per table under an oversized cap (per_gather = 8) must
--     launch exactly 2 producers. EXPLAIN ANALYZE's "MPP Launch:
--     workers=N" line is the launch's own report of what it spawned.
--   - 1 segment per table produces no distributable stages: the query
--     runs serially ("MPP Launch" line absent), and under
--     paradedb.mpp_debug no "spawning" trace appears — no launch is
--     even attempted. Plain EXPLAIN must also render the serial shape
--     (#5784), not a cap-sized distributed plan.
--   - 4 segments per table under a cap of 2 launch exactly 2 producers;
--     the extra tasks multiplex and results match the serial run.
--   - AggregateScan rides the same launch: a 2-segment GROUP BY join
--     also launches exactly 2 producers, and its 1-segment variant
--     attempts no launch.
--
-- Outcomes are collected in a table (not NOTICEs) so the expected
-- output stays stable under client_min_messages.
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_join_custom_scan TO on;
SET paradedb.enable_aggregate_custom_scan TO on;
SET max_parallel_workers_per_gather TO 8;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;

CREATE TABLE mpp_ws_outcome (line text);

-- Two segments per index: two bulk inserts with the mutable segment disabled.
CREATE TABLE mpp_ws_users    (id bigserial PRIMARY KEY, name text, age int);
CREATE TABLE mpp_ws_products (id bigserial PRIMARY KEY, name text, age int);
CREATE INDEX mpp_ws_users_idx ON mpp_ws_users USING paradedb (id, name, age)
WITH (key_field='id', text_fields='{"name":{"tokenizer":{"type":"keyword"},"fast":true}}', numeric_fields='{"age":{"fast":true}}');
CREATE INDEX mpp_ws_products_idx ON mpp_ws_products USING paradedb (id, name, age)
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
CREATE INDEX mpp_ws1_users_idx ON mpp_ws1_users USING paradedb (id, name, age)
WITH (key_field='id', text_fields='{"name":{"tokenizer":{"type":"keyword"},"fast":true}}', numeric_fields='{"age":{"fast":true}}');
CREATE INDEX mpp_ws1_products_idx ON mpp_ws1_products USING paradedb (id, name, age)
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
    FOR r IN EXECUTE 'EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, TIMING OFF)
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
    FOR r IN EXECUTE 'EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, TIMING OFF)
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
        INSERT INTO mpp_ws_outcome VALUES ('1-segment join: ran serially (no MPP launch recorded)');
    END IF;
END$$;

-- #5784: plain EXPLAIN must match the serial ANALYZE shape when launch
-- will not run — no cap-sized RoundRobin / SortPreservingMerge.
DO $$
DECLARE
    r record;
    plan text := '';
BEGIN
    FOR r IN EXECUTE 'EXPLAIN (VERBOSE, COSTS OFF)
        SELECT u.id FROM mpp_ws1_users u JOIN mpp_ws1_products p ON u.age = p.age
        WHERE u.name @@@ ''bob'' ORDER BY u.id LIMIT 10'
    LOOP
        plan := plan || E'\n' || r."QUERY PLAN";
    END LOOP;
    IF plan LIKE '%RoundRobinBatch%' OR plan LIKE '%SortPreservingMergeExec%' THEN
        INSERT INTO mpp_ws_outcome
        VALUES ('1-segment join plain EXPLAIN: UNEXPECTED distributed shape');
    ELSE
        INSERT INTO mpp_ws_outcome
        VALUES ('1-segment join plain EXPLAIN: serial shape (no distributed merge/repartition)');
    END IF;
END$$;

DO $$
DECLARE
    r record;
    plan text := '';
BEGIN
    FOR r IN EXECUTE 'EXPLAIN (VERBOSE, COSTS OFF)
        SELECT p.age, count(*) FROM mpp_ws1_users u JOIN mpp_ws1_products p ON u.age = p.age
        WHERE u.name @@@ ''bob'' GROUP BY p.age ORDER BY p.age LIMIT 5'
    LOOP
        plan := plan || E'\n' || r."QUERY PLAN";
    END LOOP;
    IF plan LIKE '%RoundRobinBatch%' OR plan LIKE '%SortPreservingMergeExec%'
       OR plan LIKE '%NetworkShuffle%' OR plan LIKE '%DistributedExec%' THEN
        INSERT INTO mpp_ws_outcome
        VALUES ('1-segment aggregate plain EXPLAIN: UNEXPECTED distributed shape');
    ELSE
        INSERT INTO mpp_ws_outcome
        VALUES ('1-segment aggregate plain EXPLAIN: serial shape (no distributed merge/repartition)');
    END IF;
END$$;

-- AggregateScan rides the same plan-first launch: same 2-segment tables under
-- a cap of 8 must also launch exactly 2 producers.
DO $$
DECLARE
    r record;
    launched int := -1;
BEGIN
    FOR r IN EXECUTE 'EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, TIMING OFF)
        SELECT p.age, count(*) FROM mpp_ws_users u JOIN mpp_ws_products p ON u.age = p.age
        WHERE u.name @@@ ''bob'' GROUP BY p.age ORDER BY p.age LIMIT 5'
    LOOP
        IF r."QUERY PLAN" LIKE '%MPP Launch:%' THEN
            launched := (regexp_match(r."QUERY PLAN", 'workers=(\d+)'))[1]::int;
        END IF;
    END LOOP;
    IF launched = 2 THEN
        INSERT INTO mpp_ws_outcome VALUES ('2-segment aggregate: launched exactly 2 producers');
    ELSIF launched = -1 THEN
        INSERT INTO mpp_ws_outcome VALUES ('2-segment aggregate: UNEXPECTED serial run (no MPP Launch line)');
    ELSE
        INSERT INTO mpp_ws_outcome
        VALUES ('2-segment aggregate: UNEXPECTED worker count ' || launched);
    END IF;
END$$;

-- #6157: a DataFusion scan below ModifyTable must honor PostgreSQL's query-wide
-- parallelModeOK=false decision. The equivalent standalone SELECT above proves
-- that this dataset and plan shape really do launch MPP; beneath INSERT the
-- custom scan must remain selected but build a serial DataFusion plan.
CREATE TABLE mpp_ws_agg_insert_out (age int, n bigint);

DO $$
DECLARE
    r record;
    plan text := '';
BEGIN
    FOR r IN EXECUTE 'EXPLAIN (VERBOSE, COSTS OFF)
        INSERT INTO mpp_ws_agg_insert_out
        SELECT p.age, count(*)
        FROM mpp_ws_users u JOIN mpp_ws_products p ON u.age = p.age
        WHERE u.name @@@ ''bob''
        GROUP BY p.age ORDER BY p.age LIMIT 5'
    LOOP
        plan := plan || E'\n' || r."QUERY PLAN";
    END LOOP;
    IF plan LIKE '%ParadeDB Aggregate Scan%'
       AND plan LIKE '%DataFusion Physical Plan:%'
       AND plan LIKE '%PgSearchScan:%'
       AND plan NOT LIKE '%DistributedExec%'
       AND plan NOT LIKE '%NetworkShuffle%' THEN
        INSERT INTO mpp_ws_outcome
        VALUES ('INSERT aggregate plan: DataFusion custom scan stayed serial');
    ELSE
        INSERT INTO mpp_ws_outcome
        VALUES ('INSERT aggregate plan: UNEXPECTED custom-scan or distributed shape');
    END IF;
END$$;

INSERT INTO mpp_ws_agg_insert_out
SELECT p.age, count(*)
FROM mpp_ws_users u JOIN mpp_ws_products p ON u.age = p.age
WHERE u.name @@@ 'bob'
GROUP BY p.age ORDER BY p.age LIMIT 5;

INSERT INTO mpp_ws_outcome
SELECT CASE
    WHEN count(*) = 5 AND min(n) = 40000 AND max(n) = 40000
    THEN 'INSERT aggregate execution: inserted correct rows without parallel-mode error'
    ELSE 'INSERT aggregate execution: UNEXPECTED result'
END
FROM mpp_ws_agg_insert_out;

-- JoinScan shares the same self-launching MPP machinery and must obey the same
-- statement-wide invariant.
CREATE TABLE mpp_ws_join_insert_out (id bigint);

DO $$
DECLARE
    r record;
    plan text := '';
BEGIN
    FOR r IN EXECUTE 'EXPLAIN (VERBOSE, COSTS OFF)
        INSERT INTO mpp_ws_join_insert_out
        SELECT u.id
        FROM mpp_ws_users u JOIN mpp_ws_products p ON u.age = p.age
        WHERE u.name @@@ ''bob'' ORDER BY u.id LIMIT 10'
    LOOP
        plan := plan || E'\n' || r."QUERY PLAN";
    END LOOP;
    IF plan LIKE '%ParadeDB Join Scan%'
       AND plan LIKE '%DataFusion Physical Plan:%'
       AND plan LIKE '%PgSearchScan:%'
       AND plan NOT LIKE '%DistributedExec%'
       AND plan NOT LIKE '%NetworkShuffle%' THEN
        INSERT INTO mpp_ws_outcome
        VALUES ('INSERT join plan: DataFusion custom scan stayed serial');
    ELSE
        INSERT INTO mpp_ws_outcome
        VALUES ('INSERT join plan: UNEXPECTED custom-scan or distributed shape');
    END IF;
END$$;

INSERT INTO mpp_ws_join_insert_out
SELECT u.id
FROM mpp_ws_users u JOIN mpp_ws_products p ON u.age = p.age
WHERE u.name @@@ 'bob' ORDER BY u.id LIMIT 10;

INSERT INTO mpp_ws_outcome
SELECT CASE
    WHEN count(*) = 10 AND min(id) = 2 AND max(id) = 2
    THEN 'INSERT join execution: inserted correct rows without parallel-mode error'
    ELSE 'INSERT join execution: UNEXPECTED result'
END
FROM mpp_ws_join_insert_out;

-- Force a generic prepared plan so the search predicate remains a Param until
-- executor startup. This exercises JoinScan's unsafe-statement serial rebake,
-- not just the static logical plan used by the INSERT above.
TRUNCATE mpp_ws_join_insert_out;
SET plan_cache_mode TO force_generic_plan;
PREPARE mpp_ws_prepared_join_insert(text) AS
INSERT INTO mpp_ws_join_insert_out
SELECT u.id
FROM mpp_ws_users u JOIN mpp_ws_products p ON u.age = p.age
WHERE u.name @@@ $1 ORDER BY u.id LIMIT 10;

DO $$
DECLARE
    r record;
    plan text := '';
BEGIN
    FOR r IN EXECUTE 'EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, TIMING OFF)
        EXECUTE mpp_ws_prepared_join_insert(''bob'')'
    LOOP
        plan := plan || E'\n' || r."QUERY PLAN";
    END LOOP;
    IF plan LIKE '%ParadeDB Join Scan%'
       AND plan LIKE '%DataFusion Physical Plan:%'
       AND plan LIKE '%PgSearchScan:%'
       AND plan NOT LIKE '%DistributedExec%'
       AND plan NOT LIKE '%NetworkShuffle%'
       AND plan NOT LIKE '%MPP Launch:%' THEN
        INSERT INTO mpp_ws_outcome
        VALUES ('prepared INSERT join plan: parameterized custom scan stayed serial');
    ELSE
        INSERT INTO mpp_ws_outcome
        VALUES ('prepared INSERT join plan: UNEXPECTED custom-scan or distributed shape');
    END IF;
END$$;

EXECUTE mpp_ws_prepared_join_insert('alice');
INSERT INTO mpp_ws_outcome
SELECT CASE
    WHEN count(*) FILTER (WHERE id = 1) = 10
         AND count(*) FILTER (WHERE id = 2) = 10
         AND count(*) = 20
    THEN 'prepared INSERT join execution: runtime parameter changed results'
    ELSE 'prepared INSERT join execution: UNEXPECTED result'
END
FROM mpp_ws_join_insert_out;
DEALLOCATE mpp_ws_prepared_join_insert;
RESET plan_cache_mode;

-- Four segments per index, cap of 2 (per_gather = 2): more tasks than workers.
-- The launch must clamp to the cap and the extra tasks multiplex onto the two
-- producers (dispatch::push_owned_tasks), returning the same results as serial.
CREATE TABLE mpp_ws4_users    (id bigserial PRIMARY KEY, name text, age int);
CREATE TABLE mpp_ws4_products (id bigserial PRIMARY KEY, name text, age int);
CREATE INDEX mpp_ws4_users_idx ON mpp_ws4_users USING paradedb (id, name, age)
WITH (key_field='id', text_fields='{"name":{"tokenizer":{"type":"keyword"},"fast":true}}', numeric_fields='{"age":{"fast":true}}');
CREATE INDEX mpp_ws4_products_idx ON mpp_ws4_products USING paradedb (id, name, age)
WITH (key_field='id', text_fields='{"name":{"tokenizer":{"type":"keyword"},"fast":true}}', numeric_fields='{"age":{"fast":true}}');

SET paradedb.global_mutable_segment_rows = 0;
INSERT INTO mpp_ws4_users (name, age)
SELECT (ARRAY['bob','alice'])[1 + (g % 2)], g % 50 FROM generate_series(1, 2500) g;
INSERT INTO mpp_ws4_users (name, age)
SELECT (ARRAY['bob','alice'])[1 + (g % 2)], g % 50 FROM generate_series(2501, 5000) g;
INSERT INTO mpp_ws4_users (name, age)
SELECT (ARRAY['bob','alice'])[1 + (g % 2)], g % 50 FROM generate_series(5001, 7500) g;
INSERT INTO mpp_ws4_users (name, age)
SELECT (ARRAY['bob','alice'])[1 + (g % 2)], g % 50 FROM generate_series(7501, 10000) g;
INSERT INTO mpp_ws4_products (name, age)
SELECT 'x', g % 50 FROM generate_series(1, 100) g;
INSERT INTO mpp_ws4_products (name, age)
SELECT 'x', g % 50 FROM generate_series(101, 200) g;
INSERT INTO mpp_ws4_products (name, age)
SELECT 'x', g % 50 FROM generate_series(201, 300) g;
INSERT INTO mpp_ws4_products (name, age)
SELECT 'x', g % 50 FROM generate_series(301, 400) g;
RESET paradedb.global_mutable_segment_rows;
ANALYZE mpp_ws4_users;
ANALYZE mpp_ws4_products;

SET work_mem TO '64MB';
DO $$
DECLARE
    r record;
    launched int := -1;
    serial_cnt bigint;
    serial_sum bigint;
    mpp_cnt bigint;
    mpp_sum bigint;
    q constant text := 'SELECT count(*), coalesce(sum(id), 0) FROM (
        SELECT u.id FROM mpp_ws4_users u JOIN mpp_ws4_products p ON u.age = p.age
        WHERE u.name @@@ ''bob'' ORDER BY u.id LIMIT 50000) t';
BEGIN
    PERFORM set_config('max_parallel_workers_per_gather', '0', false);
    EXECUTE q INTO serial_cnt, serial_sum;

    PERFORM set_config('max_parallel_workers_per_gather', '2', false);
    FOR r IN EXECUTE 'EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, TIMING OFF) ' || q LOOP
        IF r."QUERY PLAN" LIKE '%MPP Launch:%' THEN
            launched := (regexp_match(r."QUERY PLAN", 'workers=(\d+)'))[1]::int;
        END IF;
    END LOOP;
    EXECUTE q INTO mpp_cnt, mpp_sum;

    IF launched = 2 AND mpp_cnt = serial_cnt AND mpp_sum = serial_sum THEN
        INSERT INTO mpp_ws_outcome
        VALUES ('4-segment join at cap 2: launched 2 producers, results match serial');
    ELSE
        INSERT INTO mpp_ws_outcome
        VALUES ('4-segment join at cap 2: UNEXPECTED launched=' || launched
                || ' rows=' || mpp_cnt || '/' || serial_cnt
                || ' sum=' || mpp_sum || '/' || serial_sum);
    END IF;
END$$;
RESET work_mem;
SET max_parallel_workers_per_gather TO 8;

-- A launch attempt traces "spawning N producers" as a WARNING under
-- paradedb.mpp_debug. The 1-segment query must attempt none, distinguishing
-- "never launched" from "launched then aborted": expect no warnings below.
SET paradedb.mpp_debug TO on;
DO $$
DECLARE
    r record;
BEGIN
    FOR r IN EXECUTE 'EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, TIMING OFF)
        SELECT u.id FROM mpp_ws1_users u JOIN mpp_ws1_products p ON u.age = p.age
        WHERE u.name @@@ ''bob'' ORDER BY u.id LIMIT 10'
    LOOP
        NULL;
    END LOOP;
    FOR r IN EXECUTE 'EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, TIMING OFF)
        SELECT p.age, count(*) FROM mpp_ws1_users u JOIN mpp_ws1_products p ON u.age = p.age
        WHERE u.name @@@ ''bob'' GROUP BY p.age ORDER BY p.age LIMIT 5'
    LOOP
        NULL;
    END LOOP;
END$$;
SET paradedb.mpp_debug TO off;

SELECT line FROM mpp_ws_outcome ORDER BY line;

DROP TABLE mpp_ws_outcome, mpp_ws_users, mpp_ws_products, mpp_ws1_users, mpp_ws1_products,
           mpp_ws4_users, mpp_ws4_products, mpp_ws_agg_insert_out, mpp_ws_join_insert_out;
RESET paradedb.mpp_debug;
RESET paradedb.enable_aggregate_custom_scan;
RESET paradedb.enable_join_custom_scan;
RESET max_parallel_workers_per_gather;
RESET max_parallel_workers;
RESET min_parallel_table_scan_size;
RESET parallel_setup_cost;
RESET parallel_tuple_cost;
