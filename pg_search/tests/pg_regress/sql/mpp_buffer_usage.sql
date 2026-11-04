-- =====================================================================
-- Regression test for parallel/MPP worker buffer accounting (#6698).
--
-- Parallel workers launched through ParallelProcessBuilder (DataFusion MPP
-- JoinScan and AggregateScan, as well as Tantivy ParallelAggregation) run
-- outside PostgreSQL's Gather node. ParallelProcessBuilder allocates DSM
-- BufferUsage and WalUsage arrays, brackets worker execution with
-- InstrStartParallelQuery / InstrEndParallelQuery, and accumulates worker
-- buffer/WAL usage into the leader via InstrAccumParallelQuery after each
-- worker detaches or finishes.
--
-- This test verifies:
--   1. MPP JoinScan accumulates worker buffer usage into the scan node.
--   2. MPP AggregateScan (DataFusion) accumulates worker buffer usage.
--   3. Tantivy parallel AggregateScan (ParallelAggregation message-queue
--      workers with parallel_leader_participation = off) accumulates worker
--      buffer usage after worker completion.
--   4. Multi-loop LATERAL rescans of MPP JoinScan and MPP AggregateScan
--      cleanly tear down and relaunch MPP workers while accumulating prior
--      loops' worker buffer usage in rescan_custom_scan.
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS mpp_buf_categories CASCADE;
DROP TABLE IF EXISTS mpp_buf_items CASCADE;

CREATE TABLE mpp_buf_categories (
    id int PRIMARY KEY,
    name text NOT NULL
);

CREATE TABLE mpp_buf_items (
    id bigint PRIMARY KEY,
    category_id int NOT NULL,
    description text NOT NULL
);

-- Force multiple immutable segments by setting layer_sizes and mutable segment rows to 0.
CREATE INDEX mpp_buf_items_idx ON mpp_buf_items USING paradedb (id, category_id, description)
WITH (layer_sizes = '0', background_layer_sizes = '0');

CREATE INDEX mpp_buf_categories_idx ON mpp_buf_categories USING paradedb (id, (name::pdb.literal))
WITH (layer_sizes = '0', background_layer_sizes = '0');

SET paradedb.global_mutable_segment_rows = 0;

INSERT INTO mpp_buf_categories (id, name)
SELECT i, 'category_' || lpad(i::text, 3, '0') FROM generate_series(1, 50) i;

INSERT INTO mpp_buf_categories (id, name)
SELECT i, 'category_' || lpad(i::text, 3, '0') FROM generate_series(51, 100) i;

INSERT INTO mpp_buf_items (id, category_id, description)
SELECT i, (i % 100) + 1, 'item ' || i || ' query target' || ((i % 3) + 1)
FROM generate_series(1, 1000) i;

INSERT INTO mpp_buf_items (id, category_id, description)
SELECT i, (i % 100) + 1, 'item ' || i || ' query target' || ((i % 3) + 1)
FROM generate_series(1001, 2000) i;

INSERT INTO mpp_buf_items (id, category_id, description)
SELECT i, (i % 100) + 1, 'item ' || i || ' query target' || ((i % 3) + 1)
FROM generate_series(2001, 3000) i;

RESET paradedb.global_mutable_segment_rows;

ANALYZE mpp_buf_items;
ANALYZE mpp_buf_categories;

SET paradedb.enable_join_custom_scan = on;
SET paradedb.enable_aggregate_custom_scan = on;
SET paradedb.mpp_min_rows = 0;
SET max_parallel_workers = 8;
SET min_parallel_table_scan_size = 0;
SET parallel_setup_cost = 0;
SET parallel_tuple_cost = 0;

-- Extract the Custom Scan node's provider, loop count, launched worker count,
-- and total shared buffer accesses (hits + reads) from EXPLAIN ANALYZE JSON.
CREATE OR REPLACE FUNCTION mpp_buffer_usage_stats(
    q text,
    OUT provider text,
    OUT actual_loops int,
    OUT workers_launched int,
    OUT shared_blocks bigint
) LANGUAGE plpgsql AS $$
DECLARE
    plan jsonb;
    cscan jsonb;
BEGIN
    EXECUTE 'EXPLAIN (ANALYZE, VERBOSE, BUFFERS ON, COSTS OFF, TIMING OFF, FORMAT JSON) ' || q INTO plan;
    cscan := jsonb_path_query_first(plan, '$.** ? (@."Node Type" == "Custom Scan")');
    provider := cscan->>'Custom Plan Provider';
    actual_loops := (cscan->>'Actual Loops')::int;
    workers_launched := COALESCE(
        substring(cscan->>'MPP Launch' from 'workers=([0-9]+)')::int,
        (cscan->'Parallelism'->>'Workers Launched')::int,
        0
    );
    shared_blocks := COALESCE((cscan->>'Shared Hit Blocks')::bigint, 0)
                   + COALESCE((cscan->>'Shared Read Blocks')::bigint, 0);
END;
$$;

-- =====================================================================
-- 1. MPP JoinScan: single-loop worker buffer accumulation
-- =====================================================================

SET max_parallel_workers_per_gather = 0;
SELECT * INTO TEMP mpp_buf_join_serial FROM mpp_buffer_usage_stats($$
    SELECT t.id, t.category_id, c.name
    FROM mpp_buf_items t
    JOIN mpp_buf_categories c ON t.category_id = c.id
    WHERE t.description @@@ 'query'
    ORDER BY t.id
    LIMIT 25
$$);

SET max_parallel_workers_per_gather = 2;
SELECT * INTO TEMP mpp_buf_join_parallel FROM mpp_buffer_usage_stats($$
    SELECT t.id, t.category_id, c.name
    FROM mpp_buf_items t
    JOIN mpp_buf_categories c ON t.category_id = c.id
    WHERE t.description @@@ 'query'
    ORDER BY t.id
    LIMIT 25
$$);

SELECT
    s.provider AS serial_provider,
    p.provider AS mpp_provider,
    s.workers_launched = 0 AS serial_no_workers,
    p.workers_launched > 0 AS mpp_workers_launched,
    s.shared_blocks > 50 AS serial_buffers_non_trivial,
    p.shared_blocks >= s.shared_blocks AS mpp_buffers_accumulated
FROM mpp_buf_join_serial s, mpp_buf_join_parallel p;

DROP TABLE mpp_buf_join_serial;
DROP TABLE mpp_buf_join_parallel;

-- =====================================================================
-- 2. MPP AggregateScan (DataFusion join + GROUP BY): worker buffer accumulation
-- =====================================================================

SET max_parallel_workers_per_gather = 0;
SELECT * INTO TEMP mpp_buf_df_agg_serial FROM mpp_buffer_usage_stats($$
    SELECT c.name, COUNT(*), SUM(t.id)
    FROM mpp_buf_items t
    JOIN mpp_buf_categories c ON t.category_id = c.id
    WHERE t.description @@@ 'query'
    GROUP BY c.name
    ORDER BY c.name
    LIMIT 10
$$);

SET max_parallel_workers_per_gather = 2;
SELECT * INTO TEMP mpp_buf_df_agg_parallel FROM mpp_buffer_usage_stats($$
    SELECT c.name, COUNT(*), SUM(t.id)
    FROM mpp_buf_items t
    JOIN mpp_buf_categories c ON t.category_id = c.id
    WHERE t.description @@@ 'query'
    GROUP BY c.name
    ORDER BY c.name
    LIMIT 10
$$);

SELECT
    s.provider AS serial_provider,
    p.provider AS mpp_provider,
    s.workers_launched = 0 AS serial_no_workers,
    p.workers_launched > 0 AS mpp_workers_launched,
    s.shared_blocks > 50 AS serial_buffers_non_trivial,
    p.shared_blocks >= s.shared_blocks AS mpp_buffers_accumulated
FROM mpp_buf_df_agg_serial s, mpp_buf_df_agg_parallel p;

DROP TABLE mpp_buf_df_agg_serial;
DROP TABLE mpp_buf_df_agg_parallel;

-- =====================================================================
-- 3. Tantivy parallel AggregateScan (ParallelAggregation message-queue workers)
--
-- With parallel_leader_participation = off, the leader performs no segment
-- aggregation itself; all segment reads happen in background workers that send
-- intermediate results over shm_mq before WorkerStatsGuard drops.
-- =====================================================================

SET parallel_leader_participation = off;

SET max_parallel_workers_per_gather = 0;
SELECT * INTO TEMP mpp_buf_tantivy_agg_serial FROM mpp_buffer_usage_stats($$
    SELECT category_id, COUNT(*)
    FROM mpp_buf_items
    WHERE description @@@ 'query'
    GROUP BY category_id
$$);

SET max_parallel_workers_per_gather = 2;
SELECT * INTO TEMP mpp_buf_tantivy_agg_parallel FROM mpp_buffer_usage_stats($$
    SELECT category_id, COUNT(*)
    FROM mpp_buf_items
    WHERE description @@@ 'query'
    GROUP BY category_id
$$);

SELECT
    s.provider AS serial_provider,
    p.provider AS parallel_provider,
    s.workers_launched = 0 AS serial_no_workers,
    p.workers_launched > 0 AS parallel_workers_launched,
    s.shared_blocks > 50 AS serial_buffers_non_trivial,
    p.shared_blocks >= s.shared_blocks AS parallel_buffers_accumulated
FROM mpp_buf_tantivy_agg_serial s, mpp_buf_tantivy_agg_parallel p;

DROP TABLE mpp_buf_tantivy_agg_serial;
DROP TABLE mpp_buf_tantivy_agg_parallel;
RESET parallel_leader_participation;

-- =====================================================================
-- 4. Rescanned LATERAL MPP JoinScan: multi-loop rescan buffer accumulation
--
-- ExecReScanCustomScan runs between loops after PostgreSQL's InstrEndLoop,
-- while the CustomScanState's Instrumentation is not running. Finishing the
-- prior loop's MPP workers during rescan_custom_scan must still accumulate
-- their buffer usage into the CustomScanState node.
-- =====================================================================

SET max_parallel_workers_per_gather = 2;
SET plan_cache_mode = force_generic_plan;

CREATE TEMP TABLE mpp_buf_terms_1(term text);
INSERT INTO mpp_buf_terms_1 VALUES ('target1');

CREATE TEMP TABLE mpp_buf_terms_3(term text);
INSERT INTO mpp_buf_terms_3 VALUES ('target1'), ('target2'), ('target3');

SELECT t.term, sub.id
FROM mpp_buf_terms_3 t,
LATERAL (
    SELECT i.id
    FROM mpp_buf_items i
    JOIN mpp_buf_categories c ON i.category_id = c.id
    WHERE i.description @@@ t.term
    ORDER BY i.id
    LIMIT 2
) sub
ORDER BY t.term, sub.id;

SELECT * INTO TEMP mpp_buf_join_rescan_1 FROM mpp_buffer_usage_stats($$
    SELECT t.term, sub.id
    FROM mpp_buf_terms_1 t,
    LATERAL (
        SELECT i.id
        FROM mpp_buf_items i
        JOIN mpp_buf_categories c ON i.category_id = c.id
        WHERE i.description @@@ t.term
        ORDER BY i.id
        LIMIT 2
    ) sub
    ORDER BY t.term, sub.id
$$);

SELECT * INTO TEMP mpp_buf_join_rescan_3 FROM mpp_buffer_usage_stats($$
    SELECT t.term, sub.id
    FROM mpp_buf_terms_3 t,
    LATERAL (
        SELECT i.id
        FROM mpp_buf_items i
        JOIN mpp_buf_categories c ON i.category_id = c.id
        WHERE i.description @@@ t.term
        ORDER BY i.id
        LIMIT 2
    ) sub
    ORDER BY t.term, sub.id
$$);

SELECT
    r1.provider,
    r1.actual_loops AS loops_1,
    r3.actual_loops AS loops_3,
    r1.workers_launched > 0 AS loop1_launched_workers,
    r3.workers_launched > 0 AS loop3_launched_workers,
    r1.shared_blocks > 50 AS loop1_buffers_non_trivial,
    r3.shared_blocks >= r1.shared_blocks * 2 AS multi_loop_buffers_accumulated
FROM mpp_buf_join_rescan_1 r1, mpp_buf_join_rescan_3 r3;

DROP TABLE mpp_buf_join_rescan_1;
DROP TABLE mpp_buf_join_rescan_3;

-- =====================================================================
-- 5. Rescanned LATERAL MPP AggregateScan: multi-loop worker teardown,
--    relaunch, and rescan buffer accumulation
-- =====================================================================

SELECT t.term, sub.name, sub.cnt
FROM mpp_buf_terms_3 t,
LATERAL (
    SELECT c.name, COUNT(*) AS cnt
    FROM mpp_buf_items i
    JOIN mpp_buf_categories c ON i.category_id = c.id
    WHERE i.description @@@ t.term
    GROUP BY c.name
    ORDER BY c.name
    LIMIT 2
) sub
ORDER BY t.term, sub.name;

SELECT * INTO TEMP mpp_buf_agg_rescan_1 FROM mpp_buffer_usage_stats($$
    SELECT t.term, sub.name, sub.cnt
    FROM mpp_buf_terms_1 t,
    LATERAL (
        SELECT c.name, COUNT(*) AS cnt
        FROM mpp_buf_items i
        JOIN mpp_buf_categories c ON i.category_id = c.id
        WHERE i.description @@@ t.term
        GROUP BY c.name
        ORDER BY c.name
        LIMIT 2
    ) sub
    ORDER BY t.term, sub.name
$$);

SELECT * INTO TEMP mpp_buf_agg_rescan_3 FROM mpp_buffer_usage_stats($$
    SELECT t.term, sub.name, sub.cnt
    FROM mpp_buf_terms_3 t,
    LATERAL (
        SELECT c.name, COUNT(*) AS cnt
        FROM mpp_buf_items i
        JOIN mpp_buf_categories c ON i.category_id = c.id
        WHERE i.description @@@ t.term
        GROUP BY c.name
        ORDER BY c.name
        LIMIT 2
    ) sub
    ORDER BY t.term, sub.name
$$);

SELECT
    r1.provider,
    r1.actual_loops AS loops_1,
    r3.actual_loops AS loops_3,
    r1.workers_launched > 0 AS loop1_launched_workers,
    r3.workers_launched > 0 AS loop3_launched_workers,
    r1.shared_blocks > 50 AS loop1_buffers_non_trivial,
    r3.shared_blocks >= r1.shared_blocks * 2 AS multi_loop_buffers_accumulated
FROM mpp_buf_agg_rescan_1 r1, mpp_buf_agg_rescan_3 r3;

DROP TABLE mpp_buf_agg_rescan_1;
DROP TABLE mpp_buf_agg_rescan_3;
DROP TABLE mpp_buf_terms_1;
DROP TABLE mpp_buf_terms_3;

-- Cleanup
DROP FUNCTION mpp_buffer_usage_stats(text);
DROP TABLE mpp_buf_items CASCADE;
DROP TABLE mpp_buf_categories CASCADE;
RESET plan_cache_mode;
RESET paradedb.enable_join_custom_scan;
RESET paradedb.enable_aggregate_custom_scan;
RESET paradedb.mpp_min_rows;
RESET max_parallel_workers_per_gather;
RESET max_parallel_workers;
RESET min_parallel_table_scan_size;
RESET parallel_setup_cost;
RESET parallel_tuple_cost;
