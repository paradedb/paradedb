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
SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code';
