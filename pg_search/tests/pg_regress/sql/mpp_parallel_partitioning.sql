-- Test MPP parallel partitioning behavior based on segment count
SET client_min_messages TO WARNING;
CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS mpp_parts, mpp_ref CASCADE;

CREATE TABLE mpp_parts (
    id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    name TEXT NOT NULL,
    value INTEGER
);

CREATE TABLE mpp_ref (
    id INTEGER PRIMARY KEY
);

-- Create BM25 index BEFORE inserting data to create multiple segments
CREATE INDEX mpp_parts_idx ON mpp_parts
USING bm25 (id, name, value)
WITH (key_field = 'id');

CREATE INDEX mpp_ref_idx ON mpp_ref
USING bm25 (id)
WITH (key_field = 'id');

-- Insert 4 batches of data (creates 4 segments)
INSERT INTO mpp_parts (name, value) SELECT 'item ' || g, g % 10 FROM generate_series(1, 2000) g;
INSERT INTO mpp_parts (name, value) SELECT 'item ' || g, g % 10 FROM generate_series(2001, 4000) g;
INSERT INTO mpp_parts (name, value) SELECT 'item ' || g, g % 10 FROM generate_series(4001, 6000) g;
INSERT INTO mpp_parts (name, value) SELECT 'item ' || g, g % 10 FROM generate_series(6001, 8000) g;

INSERT INTO mpp_ref SELECT generate_series(1, 100);

ANALYZE mpp_parts;
ANALYZE mpp_ref;

-- Force postgres to consider parallel plans
SET max_parallel_workers_per_gather = 3;
SET max_parallel_workers = 8;
SET parallel_tuple_cost = 0;
SET parallel_setup_cost = 0;
SET min_parallel_table_scan_size = 0;
SET min_parallel_index_scan_size = 0;
SET work_mem = '1GB';
SET debug_parallel_query = on;

-- Paradedb parallel settings
SET paradedb.enable_join_custom_scan TO on;

-- 4 segments, target_partitions = 3.
-- min(4, 3) = 3 partitions. So MPP SHOULD trigger!
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name 
FROM mpp_parts p 
WHERE p.name @@@ 'item' 
AND EXISTS (SELECT 1 FROM mpp_ref r WHERE r.id = p.value)
ORDER BY p.id LIMIT 5;

SELECT p.id, p.name 
FROM mpp_parts p 
WHERE p.name @@@ 'item' 
AND EXISTS (SELECT 1 FROM mpp_ref r WHERE r.id = p.value)
ORDER BY p.id LIMIT 5;
