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
-- `CREATE EXTENSION` runs the SQL script but doesn't always trigger the
-- `.so` dlopen + `_PG_init` (which registers our GUCs) before the next
-- statement. Other regress tests don't notice because their setup runs
-- a `SET paradedb.<guc>` that forces the load; this test goes straight
-- to `SHOW` and would race. `LOAD` is the canonical force-load.
LOAD 'pg_search';

-- GUCs must be visible after the extension loads.
SHOW paradedb.mpp_debug;
SHOW paradedb.mpp_queue_size;
SHOW paradedb.mpp_min_rows;

-- Defaults: the debug knob stays off.
SELECT current_setting('paradedb.mpp_debug')::bool AS mpp_debug_default_off;
SELECT current_setting('paradedb.mpp_queue_size') AS queue_size_default;
-- The regress database pins the size gate to 0 (setup.sql), so read the shipped
-- default from the GUC's boot value instead of the session.
SELECT boot_val::int AS mpp_min_rows_boot_default
FROM pg_settings WHERE name = 'paradedb.mpp_min_rows';

-- Toggle the boolean GUCs and verify they stick.
SET paradedb.mpp_debug TO on;
SELECT current_setting('paradedb.mpp_debug')::bool AS mpp_debug_after_set_on;
SET paradedb.mpp_debug TO off;

-- Queue size: accepts standard Postgres byte units (kB, MB, GB).
SET paradedb.mpp_queue_size TO '32MB';
SELECT current_setting('paradedb.mpp_queue_size') AS queue_size_after_32mb;
SET paradedb.mpp_queue_size TO '1GB';
SELECT current_setting('paradedb.mpp_queue_size') AS queue_size_after_1gb;
SET paradedb.mpp_queue_size TO '8MB';
SELECT current_setting('paradedb.mpp_queue_size') AS queue_size_back_to_default;

-- Size gate: zero must be accepted (the regress database relies on it to engage
-- MPP on tiny tables), and a session-level value must stick.
SET paradedb.mpp_min_rows TO 0;
SELECT current_setting('paradedb.mpp_min_rows')::int AS mpp_min_rows_zero;
SET paradedb.mpp_min_rows TO 750000;
SELECT current_setting('paradedb.mpp_min_rows')::int AS mpp_min_rows_set;

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

RESET paradedb.mpp_debug;
RESET paradedb.mpp_queue_size;
RESET paradedb.mpp_min_rows;
