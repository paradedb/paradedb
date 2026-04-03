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
    f.content @@@ 'Section'
GROUP BY
    f.title
ORDER BY
    COUNT(*) DESC
LIMIT 10;

-- Tantivy TopK aggregate scan
SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT
    f.title,
    COUNT(*)
FROM files f
WHERE
    f.content @@@ 'Section'
GROUP BY
    f.title
ORDER BY
    COUNT(*) DESC
LIMIT 10;
