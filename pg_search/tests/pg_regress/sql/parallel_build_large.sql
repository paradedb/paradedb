\i common/parallel_build_large_setup.sql

SET max_parallel_workers = 8;

-- This should return a "not enough memory" error
SET maintenance_work_mem = '64MB';
SET max_parallel_maintenance_workers = 8;
SET paradedb.target_segment_count = 16;
CREATE INDEX parallel_build_large_idx ON parallel_build_large USING bm25 (id, name) WITH (key_field = 'id');

-- These should complete and create the target segment count
DO $$
DECLARE
    maintenance_work_mem text[] := ARRAY['2GB', '128MB'];
    maintenance_workers int[] := ARRAY[6, 2];
    leader_participation boolean[] := ARRAY[true, false];
    target_segments int[] := ARRAY[4, 32];
    mw int;
    lp boolean;
    ts int;
    mwm text;
    count_val int;
    num_docs_val int;
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

                    CREATE INDEX parallel_build_large_idx ON parallel_build_large USING bm25 (id, name) WITH (key_field = 'id');

                    SELECT COUNT(*) INTO count_val FROM paradedb.index_info('parallel_build_large_idx');
                    IF ts = 4 THEN
                        ASSERT count_val = 4, format('Expected index info count to be 4, but got %s', count_val);
                    ELSIF ts = 32 THEN
                        ASSERT count_val BETWEEN 28 AND 32, format('Expected index info count to be between 28 and 32, but got %s', count_val);
                    END IF;

                    SELECT SUM(num_docs) INTO num_docs_val FROM paradedb.index_info('parallel_build_large_idx');
                    ASSERT num_docs_val = 35000, format('Expected num_docs to be 35000, but got %s', num_docs_val);

                    DROP INDEX parallel_build_large_idx;
                END LOOP;
            END LOOP;
        END LOOP;
    END LOOP;
END $$;


\i common/parallel_build_large_cleanup.sql
