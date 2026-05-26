-- Shape: GROUP BY aggregate on JOIN
-- Join: stackoverflow_posts → comments
-- Description: Grouped aggregate (COUNT, SUM) with GROUP BY on the parent
-- table's title column. Exercises the DataFusion backend's grouped
-- aggregate pipeline including custom_scan_tlist for scanrelid=0.

-- Query Info (statistics from 100k dataset; larger datasets may have different values):
-- - 'code' selectivity on stackoverflow_posts.body: ~75%

-- Postgres default plan (custom scan off)
SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*), SUM(c.score)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code'
GROUP BY p.title
ORDER BY p.title;

-- DataFusion aggregate scan (temporarily increased from 4GB to 8GB)
SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*), SUM(c.score)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code'
GROUP BY p.title
ORDER BY p.title;

-- MPP aggregate scan (GroupByAggOnBinaryJoin shape; default mpp_worker_count=4)
SET statement_timeout TO '600s'; SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SELECT p.title, COUNT(*), SUM(c.score)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code'
GROUP BY p.title
ORDER BY p.title;

-- N-sweep variants for the linear-scaling investigation. Pins three knobs so
-- the only varying axis is N:
--   mpp_queue_size='4MB'        keeps DSM under MPP_DSM_MAX_BYTES at every N
--                               (the half-MPP fallback bug fires when the cap trips)
--   mpp_target_partitions=2     fixes per-producer fanout so all variants pick
--                               the same plan shape; otherwise different N's get
--                               different RepartitionExec/HashJoinExec layouts
--                               and the curve confounds plan-shape with N
--   mpp_trace=on                emits per-query mesh_init / worker_cpu / shm_mq
--                               spin lines for the three instruments

-- MPP N=4
SET statement_timeout TO '600s'; SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SET paradedb.mpp_worker_count TO 5; SET max_parallel_workers_per_gather TO 4; SET max_parallel_workers TO 32; SET paradedb.mpp_queue_size TO '4MB'; SET paradedb.mpp_target_partitions TO 2; SET paradedb.mpp_trace TO on; SELECT p.title, COUNT(*), SUM(c.score)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code'
GROUP BY p.title
ORDER BY p.title;

-- MPP N=8
SET statement_timeout TO '600s'; SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SET paradedb.mpp_worker_count TO 9; SET max_parallel_workers_per_gather TO 8; SET max_parallel_workers TO 32; SET paradedb.mpp_queue_size TO '4MB'; SET paradedb.mpp_target_partitions TO 2; SET paradedb.mpp_trace TO on; SELECT p.title, COUNT(*), SUM(c.score)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code'
GROUP BY p.title
ORDER BY p.title;

-- MPP N=12
SET statement_timeout TO '600s'; SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SET paradedb.mpp_worker_count TO 13; SET max_parallel_workers_per_gather TO 12; SET max_parallel_workers TO 32; SET paradedb.mpp_queue_size TO '4MB'; SET paradedb.mpp_target_partitions TO 2; SET paradedb.mpp_trace TO on; SELECT p.title, COUNT(*), SUM(c.score)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code'
GROUP BY p.title
ORDER BY p.title;

-- MPP N=16
SET statement_timeout TO '600s'; SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SET paradedb.mpp_worker_count TO 17; SET max_parallel_workers_per_gather TO 16; SET max_parallel_workers TO 32; SET paradedb.mpp_queue_size TO '4MB'; SET paradedb.mpp_target_partitions TO 2; SET paradedb.mpp_trace TO on; SELECT p.title, COUNT(*), SUM(c.score)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code'
GROUP BY p.title
ORDER BY p.title;

-- MPP N=24
SET statement_timeout TO '600s'; SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SET paradedb.mpp_worker_count TO 25; SET max_parallel_workers_per_gather TO 24; SET max_parallel_workers TO 48; SET paradedb.mpp_queue_size TO '4MB'; SET paradedb.mpp_target_partitions TO 2; SET paradedb.mpp_trace TO on; SELECT p.title, COUNT(*), SUM(c.score)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code'
GROUP BY p.title
ORDER BY p.title;
