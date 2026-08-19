\set ON_ERROR_STOP on
SET max_parallel_workers_per_gather = 4;
SET work_mem = '1GB';
SET paradedb.enable_join_custom_scan = on;

EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, BUFFERS, SETTINGS, TIMING OFF, SUMMARY ON)
SELECT l.id, l.title
FROM anti_bench_library AS l
WHERE l.id @@@ pdb.all()
  AND NOT EXISTS (
      SELECT 1 FROM anti_bench_owned AS o
      WHERE o.user_id = 42 AND o.item_id = l.id
  )
ORDER BY l.id
LIMIT 25;

EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, BUFFERS, SETTINGS, TIMING OFF, SUMMARY ON)
SELECT l.id, l.title
FROM anti_bench_library AS l
WHERE l.id @@@ 'title:dragon'
  AND NOT EXISTS (
      SELECT 1 FROM anti_bench_owned AS o
      WHERE o.user_id = 42 AND o.item_id = l.id
  )
ORDER BY l.id
LIMIT 25;

EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, BUFFERS, SETTINGS, TIMING OFF, SUMMARY ON)
SELECT l.id, l.title
FROM anti_bench_library AS l
WHERE l.id @@@ 'title:love'
  AND NOT EXISTS (
      SELECT 1 FROM anti_bench_owned AS o
      WHERE o.user_id = 42 AND o.item_id = l.id
  )
ORDER BY l.id
LIMIT 25;
