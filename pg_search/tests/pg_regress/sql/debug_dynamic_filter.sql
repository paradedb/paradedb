\i common/common_setup.sql

SET max_parallel_workers_per_gather = 0;
SET enable_indexscan TO OFF;
SET paradedb.enable_join_custom_scan = on;

DROP TABLE IF EXISTS df_t1 CASCADE;
DROP TABLE IF EXISTS df_t2 CASCADE;

CREATE TABLE df_t1 (id INTEGER PRIMARY KEY, val TEXT);
CREATE TABLE df_t2 (id INTEGER PRIMARY KEY, t1_id INTEGER, val TEXT);
INSERT INTO df_t1 SELECT i, 'val ' || i FROM generate_series(1, 1000) i;
INSERT INTO df_t2 SELECT i, (i % 1000) + 1, 'val ' || i FROM generate_series(1, 1000) i;

CREATE INDEX df_t1_idx ON df_t1 USING bm25 (id, val)
WITH (key_field = 'id', sort_by = 'id ASC NULLS FIRST', text_fields = '{"val": {"fast": true}}');

CREATE INDEX df_t2_idx ON df_t2 USING bm25 (id, t1_id, val)
WITH (key_field = 'id', sort_by = 't1_id ASC NULLS FIRST', numeric_fields = '{"t1_id": {"fast": true}}');

ANALYZE df_t1;
ANALYZE df_t2;

-- EXPLAIN ANALYZE to see the final TopK filter after execution
EXPLAIN (ANALYZE, COSTS OFF, VERBOSE, TIMING OFF)
SELECT t1.val, t2.val
FROM df_t1 t1
JOIN df_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.id ASC NULLS FIRST
LIMIT 10;

DROP TABLE df_t1 CASCADE;
DROP TABLE df_t2 CASCADE;
