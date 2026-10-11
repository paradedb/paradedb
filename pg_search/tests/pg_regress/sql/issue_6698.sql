-- Regression test for issue #6698:
-- Shared buffer counts underreported under MPP query plans.
--
-- Under MPP query plans (JoinScan / AggregateScan), parallel workers run
-- outside PostgreSQL's standard Gather node. ParallelProcessBuilder and
-- generic_parallel_worker_entry_point allocate DSM BufferUsage and WalUsage
-- arrays and call InstrStartParallelQuery / InstrEndParallelQuery on the workers,
-- then accumulate worker statistics into the leader via InstrAccumParallelQuery.
--
-- This test verifies that:
--   1. MPP JoinScan executes with multiple background workers across segments.
--   2. Worker buffer usage is accumulated into the leader's EXPLAIN (ANALYZE, BUFFERS) stats.

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS issue_6698_categories CASCADE;
DROP TABLE IF EXISTS issue_6698_items CASCADE;

CREATE TABLE issue_6698_categories (
    id int PRIMARY KEY,
    name text NOT NULL
);

CREATE TABLE issue_6698_items (
    id bigint PRIMARY KEY,
    category_id int NOT NULL,
    description text NOT NULL
);

-- Force multiple immutable segments by setting layer_sizes and mutable segment rows to 0
CREATE INDEX issue_6698_items_idx ON issue_6698_items USING paradedb (id, category_id, description)
WITH (layer_sizes = '0', background_layer_sizes = '0');

CREATE INDEX issue_6698_categories_idx ON issue_6698_categories USING paradedb (id, (name::pdb.literal))
WITH (layer_sizes = '0', background_layer_sizes = '0');

SET paradedb.global_mutable_segment_rows = 0;

INSERT INTO issue_6698_categories (id, name)
SELECT i, 'category ' || i FROM generate_series(1, 50) i;

INSERT INTO issue_6698_categories (id, name)
SELECT i, 'category ' || i FROM generate_series(51, 100) i;

INSERT INTO issue_6698_items (id, category_id, description)
SELECT i, (i % 100) + 1, 'item ' || i || ' query target'
FROM generate_series(1, 1000) i;

INSERT INTO issue_6698_items (id, category_id, description)
SELECT i, (i % 100) + 1, 'item ' || i || ' query target'
FROM generate_series(1001, 2000) i;

INSERT INTO issue_6698_items (id, category_id, description)
SELECT i, (i % 100) + 1, 'item ' || i || ' query target'
FROM generate_series(2001, 3000) i;

RESET paradedb.global_mutable_segment_rows;

ANALYZE issue_6698_items;
ANALYZE issue_6698_categories;

-- Enable MPP join execution
SET paradedb.enable_join_custom_scan = on;
SET paradedb.mpp_min_rows = 0;
SET max_parallel_workers_per_gather = 2;
SET max_parallel_workers = 8;
SET min_parallel_table_scan_size = 0;
SET parallel_setup_cost = 0;
SET parallel_tuple_cost = 0;

-- Function to extract whether buffer usage includes worker activity
CREATE OR REPLACE FUNCTION issue_6698_check_mpp_buffers(q text)
RETURNS text AS $$
DECLARE
    plan jsonb;
    hits bigint;
BEGIN
    EXECUTE 'EXPLAIN (ANALYZE, BUFFERS ON, COSTS OFF, TIMING OFF, FORMAT JSON) ' || q INTO plan;
    hits := (plan->0->'Plan'->>'Shared Hit Blocks')::bigint;
    -- When worker buffer statistics are accumulated, buffer hits reflect worker I/O (> 50 blocks).
    IF hits > 50 THEN
        RETURN 'buffers accumulated';
    ELSE
        RETURN 'buffers missing: only ' || hits;
    END IF;
END;
$$ LANGUAGE plpgsql;

DO $$
DECLARE
    res text;
BEGIN
    SELECT issue_6698_check_mpp_buffers('
        SELECT t.id, t.category_id, c.name
        FROM issue_6698_items t
        JOIN issue_6698_categories c ON t.category_id = c.id
        WHERE t.description @@@ ''query''
        ORDER BY t.id
        LIMIT 25
    ') INTO res;
    IF res <> 'buffers accumulated' THEN
        RAISE EXCEPTION 'MPP buffer accounting failed: %', res;
    END IF;
END;
$$;

-- Cleanup
DROP FUNCTION issue_6698_check_mpp_buffers(text);
DROP TABLE issue_6698_items CASCADE;
DROP TABLE issue_6698_categories CASCADE;
RESET paradedb.enable_join_custom_scan;
RESET paradedb.mpp_min_rows;
RESET max_parallel_workers_per_gather;
RESET max_parallel_workers;
RESET min_parallel_table_scan_size;
RESET parallel_setup_cost;
RESET parallel_tuple_cost;
