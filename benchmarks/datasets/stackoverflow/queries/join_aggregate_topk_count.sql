-- Shape: TopK Aggregate on JOIN (DataFusion)
-- Join: stackoverflow_posts -> comments
-- Description: GROUP BY a medium-cardinality dimension (owner_display_name)
-- on the search-side table with COUNT(*) ordered DESC and LIMIT 10 on a
-- join query. This realistically models an Elasticsearch Terms Aggregation
-- and tests the DataFusion TopKAggregateExec optimization versus full
-- aggregation + post-hoc sort.

-- Query Info (statistics from 20m dataset):
-- - 'code' selectivity on stackoverflow_posts.body: ~75%
-- - owner_display_name distinct count: ~70k, null count: ~19.5m (highly sparse)

-- Postgres default plan (aggregate custom scan off)
SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO off; SELECT
    p.owner_display_name,
    COUNT(*)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE
    p.body ||| 'code'
GROUP BY
    p.owner_display_name
ORDER BY
    COUNT(*) DESC
LIMIT 10;

-- DataFusion TopK aggregate scan
SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT
    p.owner_display_name,
    COUNT(*)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE
    p.body ||| 'code'
GROUP BY
    p.owner_display_name
ORDER BY
    COUNT(*) DESC
LIMIT 10;

-- DataFusion TopK aggregate scan with range partitioned join
SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_range_partitioned_join TO on; SELECT
    p.owner_display_name,
    COUNT(*)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE
    p.body ||| 'code'
GROUP BY
    p.owner_display_name
ORDER BY
    COUNT(*) DESC
LIMIT 10;

