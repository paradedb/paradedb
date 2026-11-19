-- Severity applies to planning, execution, EXPLAIN ANALYZE, and saved plans.
\i common/common_setup.sql

CREATE TABLE warning_policy_docs (
    id int PRIMARY KEY,
    body text COLLATE "C",
    padding text
) WITH (autovacuum_enabled = false, fillfactor = 70);
INSERT INTO warning_policy_docs
SELECT id, 'database ' || id, repeat('x', 600)
FROM generate_series(1, 20000) id;
CREATE INDEX warning_policy_idx ON warning_policy_docs USING paradedb(id, body)
WITH (sort_by = 'ctid ASC NULLS FIRST', mutable_segment_rows = 0,
      target_segment_count = 2, layer_sizes = '10TB', background_layer_sizes = '0');
VACUUM warning_policy_docs;

-- Inspecting a plan remains possible; executing it must raise, including after
-- an EXPLAIN error unwinds its guard.
SET paradedb.planner_warnings = 'error';
EXPLAIN (COSTS OFF)
SELECT id FROM warning_policy_docs WHERE body ||| 'database' ORDER BY body LIMIT 1;
EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF)
SELECT id FROM warning_policy_docs WHERE body ||| 'database' ORDER BY body LIMIT 1;
SELECT id FROM warning_policy_docs WHERE body ||| 'database' ORDER BY body LIMIT 1;
EXPLAIN (COSTS OFF)
SELECT id FROM warning_policy_docs WHERE body ||| 'database' ORDER BY body LIMIT 1;

-- A generic plan compiled with warnings off must be replanned after SET.
SET plan_cache_mode = force_generic_plan;
SET paradedb.planner_warnings = 'off';
PREPARE warning_policy_query AS
SELECT id FROM warning_policy_docs WHERE body ||| 'database' ORDER BY body LIMIT 1;
EXECUTE warning_policy_query;
SET paradedb.planner_warnings = 'error';
EXECUTE warning_policy_query;

-- Restoring SET LOCAL on commit or rollback must invalidate a permissive plan.
BEGIN;
SET LOCAL paradedb.planner_warnings = 'off';
EXECUTE warning_policy_query;
COMMIT;
EXECUTE warning_policy_query;
BEGIN;
SET LOCAL paradedb.planner_warnings = 'off';
EXECUTE warning_policy_query;
ROLLBACK;
EXECUTE warning_policy_query;

-- Warning mode also rechecks a plan previously executed with warnings off.
SET paradedb.planner_warnings = 'off';
EXECUTE warning_policy_query;
SET paradedb.planner_warnings = 'warning';
EXECUTE warning_policy_query;
DEALLOCATE warning_policy_query;
RESET plan_cache_mode;

-- EXPLAIN EXECUTE must allow inspection without caching a permissive plan.
SET plan_cache_mode = force_generic_plan;
SET paradedb.planner_warnings = 'off';
PREPARE warning_policy_explain AS
SELECT id FROM warning_policy_docs WHERE body ||| 'database' ORDER BY body LIMIT 1;
EXECUTE warning_policy_explain;
SET paradedb.planner_warnings = 'error';
SET client_min_messages = error;
DO $$
DECLARE
    errors int := 0;
BEGIN
    -- Both implicit and explicit false ANALYZE options must allow inspection.
    EXECUTE 'EXPLAIN EXECUTE warning_policy_explain';
    EXECUTE 'EXPLAIN (ANALYZE FALSE) EXECUTE warning_policy_explain';
    BEGIN
        EXECUTE 'EXECUTE warning_policy_explain';
    EXCEPTION WHEN OTHERS THEN
        IF SQLERRM NOT LIKE 'Query has LIMIT 1 but is not using Top K scan%' THEN
            RAISE;
        END IF;
        errors := errors + 1;
    END;
    BEGIN
        EXECUTE 'EXPLAIN ANALYZE EXECUTE warning_policy_explain';
    EXCEPTION WHEN OTHERS THEN
        IF SQLERRM NOT LIKE 'Query has LIMIT 1 but is not using Top K scan%' THEN
            RAISE;
        END IF;
        errors := errors + 1;
    END;
    ASSERT errors = 2, 'execution must still enforce error mode after EXPLAIN';
END
$$;
RESET client_min_messages;
DEALLOCATE warning_policy_explain;
RESET plan_cache_mode;

-- Runtime scan and spill warnings honor off, and are errors under error mode.
SET paradedb.enable_custom_scan = off;
SET enable_bitmapscan = off;
SET work_mem = '64kB';
SET paradedb.planner_warnings = 'off';
SELECT id FROM warning_policy_docs WHERE body ||| 'database' AND id IN (1, 20000) ORDER BY id;
SET paradedb.planner_warnings = 'warning';
SELECT id FROM warning_policy_docs WHERE body ||| 'database' AND id IN (1, 20000) ORDER BY id;
SET paradedb.planner_warnings = 'error';
SELECT id FROM warning_policy_docs WHERE body ||| 'database' AND id IN (1, 20000) ORDER BY id;

RESET paradedb.enable_custom_scan;
RESET enable_bitmapscan;
RESET work_mem;

-- Stale-visibility-map diagnostics follow the same severity policy.
SET paradedb.visibility_map_warning_threshold = 100;
UPDATE warning_policy_docs SET padding = 'dirty' WHERE id % 5 = 0;
SET paradedb.planner_warnings = 'off';
SELECT count(*) FROM warning_policy_docs WHERE body ||| 'database';
SET paradedb.planner_warnings = 'warning';
SELECT count(*) FROM warning_policy_docs WHERE body ||| 'database';
SET paradedb.planner_warnings = 'error';
SELECT count(*) FROM warning_policy_docs WHERE body ||| 'database';
EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF)
SELECT count(*) FROM warning_policy_docs WHERE body ||| 'database';

-- Catching a runtime error must not suppress another attempt in the same
-- statement (including the same statement timestamp used by warning dedup).
DO $$
DECLARE
    errors int := 0;
BEGIN
    FOR attempt IN 1..2 LOOP
        BEGIN
            PERFORM count(*) FROM warning_policy_docs WHERE body ||| 'database';
        EXCEPTION WHEN OTHERS THEN
            IF SQLERRM NOT LIKE 'visibility map for table % is significantly out of date%' THEN
                RAISE;
            END IF;
            errors := errors + 1;
        END;
    END LOOP;
    ASSERT errors = 2, 'both attempts must raise the visibility-map error';
END
$$;

-- Workers must enforce runtime errors when the leader does not scan.
-- Use one worker so its sample covers enough of the table to detect staleness.
SET max_parallel_workers_per_gather = 1;
SET parallel_leader_participation = off;
SET parallel_setup_cost = 0;
SET parallel_tuple_cost = 0;
SELECT count(*) FROM warning_policy_docs WHERE body ||| 'database';
RESET parallel_setup_cost;
RESET parallel_tuple_cost;
RESET parallel_leader_participation;
SET max_parallel_workers_per_gather = 0;

VACUUM warning_policy_docs;
SELECT count(*) FROM warning_policy_docs WHERE body ||| 'database';

RESET paradedb.visibility_map_warning_threshold;
RESET paradedb.planner_warnings;
DROP TABLE warning_policy_docs;
\i common/common_cleanup.sql
