-- Shape: Multiple Aggregates on JOIN
-- Join: stackoverflow_posts → comments
-- Description: Multiple aggregate functions (COUNT, SUM, MIN, MAX) on a join.
-- Exercises the DataFusion backend's ability to compute multiple aggregates
-- in a single pass over the joined data.

-- Query Info (statistics from 100k dataset; larger datasets may have different values):
-- - 'code' selectivity on stackoverflow_posts.body: ~75%

-- Postgres default plan (custom scan off)
SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*), MIN(c.score), MAX(c.score)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code';

-- DataFusion aggregate scan
SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code';

-- MPP aggregate scan (ScalarAggOnBinaryJoin shape with multiple aggregates;
-- default mpp_worker_count=4)
SET statement_timeout TO '600s'; SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code';
