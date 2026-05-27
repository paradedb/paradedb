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
-- investigation. Pins two knobs so the only varying axis is N:
--   mpp_queue_size='32MB'       gives the mesh-mux MPSC ring ~4MB per slot
--                               (RING_SLOTS=8) which comfortably holds Arrow
--                               IPC frames from this query's batches. Under
--                               mesh-mux total DSM scales as N×queue_bytes
--                               (not the old N×(N-1)) so 32MB at N=24 stays
--                               well under MPP_DSM_MAX_BYTES.
--   mpp_target_partitions=2     fixes the per-producer logical fanout so all
--                               variants pick the same plan shape; otherwise the
--                               default (scale with n_workers) gives each N a
--                               different RepartitionExec/HashJoinExec layout
--                               and the curve confounds plan-shape with N
-- mpp_trace is intentionally NOT set: each WARNING line it emits is a server
-- log write the bench is timing, and the cost grows with stage × partition.
-- Flip it on by hand when iterating locally, not in CI.

-- MPP N=4
SET statement_timeout TO '600s'; SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SET paradedb.mpp_worker_count TO 5; SET max_parallel_workers_per_gather TO 4; SET max_parallel_workers TO 32; SET paradedb.mpp_queue_size TO '32MB'; SET paradedb.mpp_target_partitions TO 2; SELECT COUNT(*)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code';

-- MPP N=8
SET statement_timeout TO '600s'; SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SET paradedb.mpp_worker_count TO 9; SET max_parallel_workers_per_gather TO 8; SET max_parallel_workers TO 32; SET paradedb.mpp_queue_size TO '32MB'; SET paradedb.mpp_target_partitions TO 2; SELECT COUNT(*)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code';

-- MPP N=12
SET statement_timeout TO '600s'; SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SET paradedb.mpp_worker_count TO 13; SET max_parallel_workers_per_gather TO 12; SET max_parallel_workers TO 32; SET paradedb.mpp_queue_size TO '32MB'; SET paradedb.mpp_target_partitions TO 2; SELECT COUNT(*)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code';

-- MPP N=16
SET statement_timeout TO '600s'; SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SET paradedb.mpp_worker_count TO 17; SET max_parallel_workers_per_gather TO 16; SET max_parallel_workers TO 32; SET paradedb.mpp_queue_size TO '32MB'; SET paradedb.mpp_target_partitions TO 2; SELECT COUNT(*)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code';

-- MPP N=24
SET statement_timeout TO '600s'; SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SET paradedb.mpp_worker_count TO 25; SET max_parallel_workers_per_gather TO 24; SET max_parallel_workers TO 48; SET paradedb.mpp_queue_size TO '32MB'; SET paradedb.mpp_target_partitions TO 2; SELECT COUNT(*)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code';
