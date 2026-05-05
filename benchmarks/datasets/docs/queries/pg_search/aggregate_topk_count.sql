-- Shape: TopK Aggregate (Single Table, Tantivy)
-- Join: None (single table)
-- Description: GROUP BY a high-cardinality field with COUNT(*) ordered DESC
-- and LIMIT 10. Tests the Tantivy TopK optimization (TermsAggregation.size=K)
-- versus full aggregation + post-hoc sort.

-- Postgres default plan (aggregate custom scan off)
SET paradedb.enable_aggregate_custom_scan TO off; SELECT
    f.title,
    COUNT(*)
FROM files f
WHERE
    f.content ||| 'Section'
GROUP BY
    f.title
ORDER BY
    COUNT(*) DESC
LIMIT 10;

<<<<<<< HEAD:benchmarks/datasets/docs/queries/pg_search/aggregate_topk_count.sql
-- Tantivy TopK aggregate scan
SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT
    f.title,
=======
-- DataFusion aggregate scan
SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT
    p.title,
>>>>>>> aae60d675 (chore: Increase memory limit for `aggregate_topk_count.sql`. (#4992)):benchmarks/datasets/stackoverflow/queries/pg_search/aggregate_topk_count.sql
    COUNT(*)
FROM files f
WHERE
    f.content ||| 'Section'
GROUP BY
    f.title
ORDER BY
    COUNT(*) DESC
LIMIT 10;
