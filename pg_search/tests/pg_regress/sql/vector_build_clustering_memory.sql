-- The projected clustering memory for a vector index build (workers × per-merge
-- training set) must fit in maintenance_work_mem; the build refuses to start
-- otherwise.
SET client_min_messages = WARNING;
CREATE EXTENSION IF NOT EXISTS vector;
\i common/common_setup.sql

DROP TABLE IF EXISTS clustering_mem;
CREATE TABLE clustering_mem (id SERIAL PRIMARY KEY, embedding vector(128));
INSERT INTO clustering_mem (embedding)
SELECT ('[' || (SELECT string_agg('0.5', ',') FROM generate_series(1, 128)) || ']')::vector
FROM generate_series(1, 20000);

SET max_parallel_maintenance_workers = 0;
SET maintenance_work_mem = '16MB';

-- ~20k docs in one merged segment at centroid_ratio 1.0 needs ~20MB of training
-- set, exceeding the 16MB maintenance_work_mem
CREATE INDEX clustering_mem_idx ON clustering_mem
    USING bm25 (id, embedding vector_l2_ops)
    WITH (key_field = 'id', centroid_ratio = 1.0);

-- at the default centroid_ratio the training set fits and the build succeeds
CREATE INDEX clustering_mem_idx ON clustering_mem
    USING bm25 (id, embedding vector_l2_ops)
    WITH (key_field = 'id');
SELECT relname FROM pg_class WHERE relname = 'clustering_mem_idx';

DROP TABLE clustering_mem;
