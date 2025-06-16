\i common/parallel_build_small_setup.sql

SET max_parallel_workers = 8;
SET client_min_messages TO INFO;

DO $$
DECLARE
    maintenance_work_mem text[] := ARRAY['2GB', '64MB'];
    maintenance_workers int[] := ARRAY[8, 1, 0];
    leader_participation boolean[] := ARRAY[true, false];
    target_segments int[] := ARRAY[4, 1];
    mw int;
    lp boolean;
    ts int;
    mwm text;
    count_val int;
BEGIN
    FOREACH mw IN ARRAY maintenance_workers LOOP
        FOREACH lp IN ARRAY leader_participation LOOP
            FOREACH ts IN ARRAY target_segments LOOP
                FOREACH mwm IN ARRAY maintenance_work_mem LOOP
                    -- Set configuration
                    EXECUTE format('SET max_parallel_maintenance_workers = %s', mw);
                    EXECUTE format('SET parallel_leader_participation = %s', lp);
                    EXECUTE format('SET paradedb.target_segment_count = %s', ts);
                    EXECUTE format('SET maintenance_work_mem = %L', mwm);

                    CREATE INDEX parallel_build_small_idx ON parallel_build_small USING bm25 (id, name, age) WITH (key_field = 'id');

                    -- Check index info and display results
                    SELECT COUNT(*) INTO count_val FROM paradedb.index_info('parallel_build_small_idx');
                    RAISE INFO 'Config: workers=%, leader_participation=%, segments=%, work_mem=% -> Count: %',
                        mw, lp, ts, mwm, count_val;

                    DROP INDEX parallel_build_small_idx;
                END LOOP;
            END LOOP;
        END LOOP;
    END LOOP;
END $$;


\i common/parallel_build_small_cleanup.sql
