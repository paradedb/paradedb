CREATE EXTENSION IF NOT EXISTS pg_prewarm;
DO $$
DECLARE
    child_idx regclass;
BEGIN
    FOR child_idx IN
        SELECT indexrelid::regclass
        FROM pg_index
        WHERE indrelid IN (
            SELECT inhrelid FROM pg_inherits
            WHERE inhparent = 'benchmark_logs_partitioned'::regclass
        )
    LOOP
        BEGIN
            PERFORM pg_prewarm(child_idx);
        EXCEPTION WHEN OTHERS THEN
            -- skip indexes without storage (e.g. pkey on partitioned)
            NULL;
        END;
    END LOOP;
END;
$$;
