-- Shape: Scalar COUNT(*) on JOIN
-- Join: stackoverflow_posts → comments
-- Description: Count total joined rows matching a search predicate.
-- This is the simplest aggregate-on-join shape and exercises the
-- DataFusion backend's basic scan → join → aggregate pipeline.

-- Query Info (statistics from 100k dataset; larger datasets may have different values):
-- - 'code' selectivity on stackoverflow_posts.body: ~75%

-- Postgres default plan (custom scan off)
SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code';

-- DataFusion aggregate scan
SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code';

-- MPP aggregate scan (ScalarAggOnBinaryJoin shape; default mpp_worker_count=4)
SET statement_timeout TO '600s'; SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SELECT COUNT(*)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code';

-- N-sweep variants below capture the scaling curve for the linear-scaling
-- investigation. All variants pin paradedb.mpp_queue_size='4MB' so total DSM
-- (= num_meshes * N*(N-1) * queue_size) stays well under MPP_DSM_MAX_BYTES at
-- every N — the half-MPP fallback bug fires when the cap trips. Also enables
-- paradedb.mpp_trace=on so the per-query log captures mesh_init / worker_cpu /
-- shm_mq spin lines (see pg_search/src/postgres/customscan/mpp/{dsm,worker,exec_worker}.rs).

-- MPP N=4
SET statement_timeout TO '600s'; SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SET paradedb.mpp_worker_count TO 5; SET max_parallel_workers_per_gather TO 4; SET max_parallel_workers TO 32; SET paradedb.mpp_queue_size TO '4MB'; SET paradedb.mpp_trace TO on; SELECT COUNT(*)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code';

-- MPP N=8
SET statement_timeout TO '600s'; SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SET paradedb.mpp_worker_count TO 9; SET max_parallel_workers_per_gather TO 8; SET max_parallel_workers TO 32; SET paradedb.mpp_queue_size TO '4MB'; SET paradedb.mpp_trace TO on; SELECT COUNT(*)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code';

-- MPP N=12
SET statement_timeout TO '600s'; SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SET paradedb.mpp_worker_count TO 13; SET max_parallel_workers_per_gather TO 12; SET max_parallel_workers TO 32; SET paradedb.mpp_queue_size TO '4MB'; SET paradedb.mpp_trace TO on; SELECT COUNT(*)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code';

-- MPP N=16
SET statement_timeout TO '600s'; SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SET paradedb.mpp_worker_count TO 17; SET max_parallel_workers_per_gather TO 16; SET max_parallel_workers TO 32; SET paradedb.mpp_queue_size TO '4MB'; SET paradedb.mpp_trace TO on; SELECT COUNT(*)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code';

-- MPP N=24
SET statement_timeout TO '600s'; SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SET paradedb.mpp_worker_count TO 25; SET max_parallel_workers_per_gather TO 24; SET max_parallel_workers TO 48; SET paradedb.mpp_queue_size TO '4MB'; SET paradedb.mpp_trace TO on; SELECT COUNT(*)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code';
