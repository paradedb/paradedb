-- Shape: Multi-Facet Window Aggregates on JOIN
-- Join: stackoverflow_posts -> comments
-- Description: Uses window functions (OVER PARTITION BY) to retrieve Top K hits 
-- alongside global facet counts across multiple dimensions. This perfectly mimics 
-- Elasticsearch's faceting behavior, but currently prevents TopK optimization.

-- Query Info (statistics from 100k dataset; larger datasets may have different values):
-- - 'code' selectivity on stackoverflow_posts.body: ~75%

-- Postgres default plan (custom scan off)
SET paradedb.enable_aggregate_custom_scan TO off; SET paradedb.enable_join_custom_scan TO off; SELECT
    c.id,
    p.post_type_id,
    p.owner_user_id,
    COUNT(*) OVER (PARTITION BY p.post_type_id) as post_type_facet,
    COUNT(*) OVER (PARTITION BY p.owner_user_id) as user_facet
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE
    p.body ||| 'code'
ORDER BY
    c.score DESC
LIMIT 10;

-- TODO(https://github.com/paradedb/paradedb/issues/5637): Implement support for executing window functions with the DataFusion backend for aggregate scans on joins.
-- Custom scan enabled
SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_join_custom_scan TO on; SELECT
    c.id,
    p.post_type_id,
    p.owner_user_id,
    COUNT(*) OVER (PARTITION BY p.post_type_id) as post_type_facet,
    COUNT(*) OVER (PARTITION BY p.owner_user_id) as user_facet
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE
    p.body ||| 'code'
ORDER BY
    c.score DESC
LIMIT 10;
