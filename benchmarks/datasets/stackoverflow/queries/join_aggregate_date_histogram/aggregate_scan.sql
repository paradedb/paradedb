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
