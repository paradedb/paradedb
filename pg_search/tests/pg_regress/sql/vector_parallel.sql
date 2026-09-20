SET client_min_messages = warning;
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.vector_min_training_rows = 1;
SET paradedb.vector_cluster_max_probe = 1;
SET min_parallel_table_scan_size = 0;
SET max_parallel_workers_per_gather = 2;
SET max_parallel_maintenance_workers = 2;

CREATE TABLE vp (id integer PRIMARY KEY, bucket text, vec vector(3));
INSERT INTO vp SELECT i, CASE WHEN i % 7 = 0 THEN 'keep' ELSE 'other' END,
    ARRAY[(i % 17)::real, (i % 23)::real, (i % 29)::real]::vector
FROM generate_series(1, 500) i;
CREATE INDEX vp_idx ON vp USING bm25 (id, bucket, vec vector_l2_ops)
WITH (target_segment_count = 8, centroid_ratio = 0.05,
      cluster_replication = 2, mutable_segment_rows = 0, background_layer_sizes = '0');
INSERT INTO vp SELECT i, CASE WHEN i % 7 = 0 THEN 'keep' ELSE 'other' END,
    ARRAY[(i % 17)::real, (i % 23)::real, (i % 29)::real]::vector
FROM generate_series(501, 1000) i;
INSERT INTO vp SELECT i, CASE WHEN i % 7 = 0 THEN 'keep' ELSE 'other' END,
    ARRAY[(i % 17)::real, (i % 23)::real, (i % 29)::real]::vector
FROM generate_series(1001, 1500) i;
INSERT INTO vp SELECT i, CASE WHEN i % 7 = 0 THEN 'keep' ELSE 'other' END,
    ARRAY[(i % 17)::real, (i % 23)::real, (i % 29)::real]::vector
FROM generate_series(1501, 2000) i;
ALTER INDEX vp_idx RESET (mutable_segment_rows);
ANALYZE vp;

EXPLAIN (COSTS OFF)
SELECT id FROM vp WHERE id @@@ pdb.all() ORDER BY vec <-> '[3,5,7]', id LIMIT 10;

DO $$
DECLARE
    expected integer[];
    actual integer[];
    predicate text;
    workers integer;
BEGIN
    FOREACH predicate IN ARRAY ARRAY['true', 'bucket = ''keep''', 'bucket = ''missing'''] LOOP
        EXECUTE format('SELECT array_agg(id) FROM (SELECT id FROM vp WHERE %s ORDER BY (vec <-> ''[3,5,7]'') + 0.0, id LIMIT 10 OFFSET 3) q', predicate) INTO expected;
        FOREACH workers IN ARRAY ARRAY[0, 1, 2, 7] LOOP
            PERFORM set_config('max_parallel_workers_per_gather', workers::text, true);
            EXECUTE format('SELECT array_agg(id) FROM (SELECT id FROM vp WHERE id @@@ pdb.all() AND %s ORDER BY vec <-> ''[3,5,7]'', id LIMIT 10 OFFSET 3) q', predicate) INTO actual;
            IF actual IS DISTINCT FROM expected THEN
                RAISE EXCEPTION 'workers %, filter %: got %, expected %', workers, predicate, actual, expected;
            END IF;
        END LOOP;
    END LOOP;
END $$;

DELETE FROM vp WHERE id % 3 = 0;
UPDATE vp SET vec = '[3,5,7]' WHERE id % 11 = 0;
INSERT INTO vp VALUES (2001, 'keep', '[3,5,7]');

DO $$
DECLARE expected integer[]; actual integer[]; workers integer;
BEGIN
    SELECT array_agg(id) INTO expected FROM
        (SELECT id FROM vp WHERE bucket = 'keep' ORDER BY (vec <-> '[3,5,7]') + 0.0, id LIMIT 20) q;
    FOREACH workers IN ARRAY ARRAY[0, 1, 2, 7] LOOP
        PERFORM set_config('max_parallel_workers_per_gather', workers::text, true);
        SELECT array_agg(id) INTO actual FROM
            (SELECT id FROM vp WHERE id @@@ pdb.all() AND bucket = 'keep' ORDER BY vec <-> '[3,5,7]', id LIMIT 20) q;
        IF actual IS DISTINCT FROM expected THEN
            RAISE EXCEPTION 'visibility mismatch with % workers: got %, expected %', workers, actual, expected;
        END IF;
    END LOOP;
END $$;

SET plan_cache_mode = force_generic_plan;
PREPARE vp_query(vector, integer, integer) AS
SELECT id FROM vp WHERE id @@@ pdb.all() ORDER BY vec <-> $1, id LIMIT $2 OFFSET $3;
EXECUTE vp_query('[3,5,7]', 3, 0);
EXECUTE vp_query('[0,0,0]', 5, 2);
EXECUTE vp_query('[3,5,7]', 3, 0);
SET paradedb.vector_cluster_max_probe = 0.02;
EXECUTE vp_query('[3,5,7]', 3, 0);
DEALLOCATE vp_query;
RESET plan_cache_mode;

DO $$
DECLARE
    expected integer[];
    actual integer[];
    predicate text;
    workers integer;
    budget double precision;
    query text;
BEGIN
    FOREACH budget IN ARRAY ARRAY[0.001, 0.05, 0.25, 1.0] LOOP
        PERFORM set_config('paradedb.vector_cluster_max_probe', budget::text, true);
        FOREACH predicate IN ARRAY ARRAY['true', 'bucket @@@ pdb.term(''keep'')', 'bucket = ''keep'''] LOOP
            query := format('SELECT array_agg(id) FROM (SELECT id FROM vp WHERE id @@@ pdb.all() AND %s ORDER BY vec <-> ''[3,5,7]'', id LIMIT 20 OFFSET 3) q', predicate);
            PERFORM set_config('max_parallel_workers_per_gather', '0', true);
            EXECUTE query INTO expected;
            FOREACH workers IN ARRAY ARRAY[1, 2, 7] LOOP
                PERFORM set_config('max_parallel_workers_per_gather', workers::text, true);
                EXECUTE query INTO actual;
                IF actual IS DISTINCT FROM expected THEN
                    RAISE EXCEPTION 'workers %, budget %, filter %: got %, expected %', workers, budget, predicate, actual, expected;
                END IF;
            END LOOP;
        END LOOP;
    END LOOP;
END $$;
DROP TABLE vp;
RESET paradedb.vector_cluster_max_probe;
RESET paradedb.vector_min_training_rows;
RESET min_parallel_table_scan_size;
RESET max_parallel_workers_per_gather;
RESET max_parallel_maintenance_workers;
RESET client_min_messages;
