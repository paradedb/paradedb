-- Shape: TopK Aggregate on JOIN (DataFusion)
-- Join: stackoverflow_posts -> badges
-- Description: GROUP BY badges.name, a string every badge row carries and that
-- repeats across most of them, with COUNT(*) ordered DESC and LIMIT 10 on a
-- join query. This realistically models an Elasticsearch Terms Aggregation on
-- a dense key: each matched post fans out to all of its owner's badges, so the
-- aggregate sees far more rows than distinct names, and every row carries a
-- real string.

-- Query Info (statistics from 20m dataset):
-- - 'javascript' selectivity on stackoverflow_posts.body: ~4%
-- - badges has no partition_by, so there is no range-partitioned variant

-- Postgres default plan (aggregate custom scan off)
SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO off; SELECT
    b.name,
    COUNT(*)
FROM stackoverflow_posts p
JOIN badges b ON b.user_id = p.owner_user_id
WHERE
    p.body ||| 'javascript'
GROUP BY
    b.name
ORDER BY
    COUNT(*) DESC
LIMIT 10;

-- DataFusion TopK aggregate scan
SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT
    b.name,
    COUNT(*)
FROM stackoverflow_posts p
JOIN badges b ON b.user_id = p.owner_user_id
WHERE
    p.body ||| 'javascript'
GROUP BY
    b.name
ORDER BY
    COUNT(*) DESC
LIMIT 10;

-- DataFusion TopK aggregate scan with the strings kept late-materialized
SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_aggregate_late_materialization TO on; SELECT
    b.name,
    COUNT(*)
FROM stackoverflow_posts p
JOIN badges b ON b.user_id = p.owner_user_id
WHERE
    p.body ||| 'javascript'
GROUP BY
    b.name
ORDER BY
    COUNT(*) DESC
LIMIT 10;
