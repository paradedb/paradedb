-- Shape: TopK Aggregate on JOIN (DataFusion)
-- Join: stackoverflow_posts -> comments
-- Description: GROUP BY a field on the search-side table with COUNT(*)
-- ordered DESC and LIMIT 10 on a join query. Tests the DataFusion
-- TopKAggregateExec optimization versus full aggregation + post-hoc sort.

-- Postgres default plan (aggregate custom scan off)
SET paradedb.enable_aggregate_custom_scan TO off; SELECT
    p.title,
    COUNT(*)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE
    p.body ||| 'the'
GROUP BY
    p.title
ORDER BY
    COUNT(*) DESC
LIMIT 10;

-- DataFusion TopK aggregate scan
SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT
    p.title,
    COUNT(*)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE
    p.body ||| 'the'
GROUP BY
    p.title
ORDER BY
    COUNT(*) DESC
LIMIT 10;
