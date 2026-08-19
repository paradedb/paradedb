\set ON_ERROR_STOP on

SET work_mem = '1GB';
SET paradedb.enable_join_custom_scan = on;

-- MPP path: must contain both DistributedExec and MPP Launch.
SET max_parallel_workers_per_gather = 4;
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

-- Serial JoinScan control: must contain neither DistributedExec nor MPP Launch.
SET max_parallel_workers_per_gather = 0;
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

RESET max_parallel_workers_per_gather;
RESET work_mem;
RESET paradedb.enable_join_custom_scan;
