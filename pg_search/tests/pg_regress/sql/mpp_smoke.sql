-- =====================================================================
-- MPP smoke test: verify GUCs exist and accept sensible values.
--
-- This is the Phase 0 / 4b gate — if any of these SHOW/SET fails, either
-- the extension didn't load or the MPP GUCs weren't registered in init().
-- Nothing here exercises the actual MPP data path yet (that lands when
-- AggregateScan's ParallelQueryCapable hooks are wired); this test locks
-- in the GUC surface so a future accidental rename breaks loudly.
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pg_search;

-- GUCs must be visible after the extension loads.
SHOW paradedb.enable_mpp;
SHOW paradedb.mpp_debug;
SHOW paradedb.mpp_worker_count;
SHOW paradedb.mpp_queue_size;

-- Defaults: MPP is off until explicitly enabled.
SELECT current_setting('paradedb.enable_mpp')::bool AS enable_mpp_default_off;
SELECT current_setting('paradedb.mpp_debug')::bool AS mpp_debug_default_off;
SELECT current_setting('paradedb.mpp_worker_count')::int AS worker_count_default;
SELECT current_setting('paradedb.mpp_queue_size') AS queue_size_default;

-- Toggle the boolean GUCs and verify they stick.
SET paradedb.enable_mpp TO on;
SELECT current_setting('paradedb.enable_mpp')::bool AS enable_mpp_after_set_on;
SET paradedb.enable_mpp TO off;
SELECT current_setting('paradedb.enable_mpp')::bool AS enable_mpp_after_set_off;

SET paradedb.mpp_debug TO on;
SELECT current_setting('paradedb.mpp_debug')::bool AS mpp_debug_after_set_on;
SET paradedb.mpp_debug TO off;

-- Worker count: accepts 1..64 per the GUC definition.
SET paradedb.mpp_worker_count TO 2;
SELECT current_setting('paradedb.mpp_worker_count')::int AS worker_count_two;
SET paradedb.mpp_worker_count TO 4;
SELECT current_setting('paradedb.mpp_worker_count')::int AS worker_count_four;

-- Out-of-range worker count must fail (GUC min=1, max=64).
DO $$
BEGIN
    BEGIN
        PERFORM set_config('paradedb.mpp_worker_count', '0', true);
        RAISE EXCEPTION 'expected worker_count=0 to be rejected';
    EXCEPTION WHEN invalid_parameter_value THEN
        RAISE NOTICE 'worker_count=0 correctly rejected';
    END;
    BEGIN
        PERFORM set_config('paradedb.mpp_worker_count', '65', true);
        RAISE EXCEPTION 'expected worker_count=65 to be rejected';
    EXCEPTION WHEN invalid_parameter_value THEN
        RAISE NOTICE 'worker_count=65 correctly rejected';
    END;
END$$;

-- Queue size: accepts standard Postgres byte units (kB, MB, GB).
SET paradedb.mpp_queue_size TO '32MB';
SELECT current_setting('paradedb.mpp_queue_size') AS queue_size_after_32mb;
SET paradedb.mpp_queue_size TO '1GB';
SELECT current_setting('paradedb.mpp_queue_size') AS queue_size_after_1gb;
SET paradedb.mpp_queue_size TO '64MB';
SELECT current_setting('paradedb.mpp_queue_size') AS queue_size_back_to_default;

-- Out-of-range queue size must fail (GUC min=64kB, max=1GB).
DO $$
BEGIN
    BEGIN
        PERFORM set_config('paradedb.mpp_queue_size', '4kB', true);
        RAISE EXCEPTION 'expected mpp_queue_size=4kB to be rejected';
    EXCEPTION WHEN invalid_parameter_value THEN
        RAISE NOTICE 'mpp_queue_size=4kB correctly rejected';
    END;
    BEGIN
        PERFORM set_config('paradedb.mpp_queue_size', '2GB', true);
        RAISE EXCEPTION 'expected mpp_queue_size=2GB to be rejected';
    EXCEPTION WHEN invalid_parameter_value THEN
        RAISE NOTICE 'mpp_queue_size=2GB correctly rejected';
    END;
END$$;

-- MPP GUCs must not affect non-MPP queries at all. Run a trivial query
-- with mpp_debug on to confirm it's a no-op (no warnings except the
-- expected ones) and results are correct.
SET paradedb.mpp_debug TO on;
SELECT 1 AS trivial_query_still_works;
SET paradedb.mpp_debug TO off;

RESET paradedb.enable_mpp;
RESET paradedb.mpp_debug;
RESET paradedb.mpp_worker_count;
RESET paradedb.mpp_queue_size;
