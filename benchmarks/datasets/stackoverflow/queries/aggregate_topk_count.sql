-- Shape: TopK Aggregate (Single Table, Tantivy)
-- Join: None (single table)
-- Description: GROUP BY a high-cardinality field with COUNT(*) ordered DESC
-- and LIMIT 10. Tests the Tantivy TopK optimization (TermsAggregation.size=K)
-- versus full aggregation + post-hoc sort.

-- Query Info (statistics from 100k dataset; larger datasets may have different values):
-- - 'code' selectivity on stackoverflow_posts.body: ~75%

-- Postgres default plan (aggregate custom scan off)
SET paradedb.enable_aggregate_custom_scan TO off; SELECT
    p.title,
    COUNT(*)
FROM stackoverflow_posts p
WHERE
    p.body ||| 'code'
GROUP BY
    p.title
ORDER BY
    COUNT(*) DESC
LIMIT 10;

-- DataFusion aggregate scan
SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT
    p.title,
    COUNT(*)
FROM stackoverflow_posts p
WHERE
    p.body ||| 'code'
GROUP BY
    p.title
ORDER BY
    COUNT(*) DESC
LIMIT 10;
