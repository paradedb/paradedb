SET client_min_messages = WARNING;
\i common/common_setup.sql
SET max_parallel_maintenance_workers = 0;

CREATE TABLE build_temp_limit (id bigint, body text, value double precision, metadata jsonb);
INSERT INTO build_temp_limit
SELECT g, md5(g::text), g::double precision, jsonb_build_object('category', g % 10)
FROM generate_series(1, 20000) g;

-- Project a larger corpus from a small sample of all indexed fields.
UPDATE pg_class SET reltuples = 1000000 WHERE oid = 'build_temp_limit'::regclass;
SET temp_file_limit = '2MB';
\set VERBOSITY terse
CREATE INDEX build_temp_limit_idx ON build_temp_limit
USING paradedb (id, body, value, metadata) WITH (target_segment_count = 1);

-- The warning is advisory: the small build still completes.
SELECT count(*) FROM build_temp_limit WHERE id @@@ pdb.all();

UPDATE pg_class SET reltuples = 1000000 WHERE oid = 'build_temp_limit'::regclass;
SET temp_file_limit = '1GB';
REINDEX INDEX build_temp_limit_idx;
UPDATE pg_class SET reltuples = 1000000 WHERE oid = 'build_temp_limit'::regclass;
SET temp_file_limit = '-1';
REINDEX INDEX build_temp_limit_idx;

-- All parallel participants share one warning flag, including across partition writers.
DROP INDEX build_temp_limit_idx;
SET max_parallel_maintenance_workers = 3;
SET maintenance_work_mem = '128MB';
SET temp_file_limit = '2MB';
UPDATE pg_class SET reltuples = 1000000 WHERE oid = 'build_temp_limit'::regclass;
CREATE INDEX build_temp_limit_idx ON build_temp_limit
USING paradedb (id, body, value, metadata)
WITH (target_segment_count = 4, partition_by = 'id');

-- An early sample must preserve the segment target on the regular build path.
DROP INDEX build_temp_limit_idx;
SET temp_file_limit = '1GB';
INSERT INTO build_temp_limit
SELECT g, md5(g::text), g::double precision, jsonb_build_object('category', g % 10)
FROM generate_series(20001, 200000) g;
ANALYZE build_temp_limit;
SET max_parallel_maintenance_workers = 0;
CREATE INDEX build_temp_limit_idx ON build_temp_limit
USING paradedb (id, body, value, metadata) WITH (target_segment_count = 4);
SELECT count(*) AS segments, sum(num_docs) AS documents
FROM paradedb.index_info('build_temp_limit_idx');
\set VERBOSITY default
RESET temp_file_limit;
RESET max_parallel_maintenance_workers;
RESET maintenance_work_mem;
DROP TABLE build_temp_limit;
