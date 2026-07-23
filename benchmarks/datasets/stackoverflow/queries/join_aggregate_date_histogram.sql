-- Shape: Date Histogram on JOIN
-- Join: stackoverflow_posts -> comments
-- Description: Group by a time bucket (month) using date_trunc. This models the
-- extremely common Elasticsearch date_histogram aggregation used for time-series
-- analytics. Sorted chronologically.

-- Query Info (statistics from 100k dataset; larger datasets may have different values):
-- - 'code' selectivity on stackoverflow_posts.body: ~75%

-- Postgres default plan (custom scan off)
SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO off; SELECT
    date_trunc('month', p.creation_date) AS month,
    COUNT(*)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE
    p.body ||| 'code'
GROUP BY
    month
ORDER BY
    month ASC;

-- TODO(https://github.com/paradedb/paradedb/issues/4082): Aggregate scan currently falls back to Postgres for date_trunc.
SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT
    date_trunc('month', p.creation_date) AS month,
    COUNT(*)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE
    p.body ||| 'code'
GROUP BY
    month
ORDER BY
    month ASC;
