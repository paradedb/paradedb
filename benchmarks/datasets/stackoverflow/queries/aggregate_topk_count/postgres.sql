-- Postgres default plan (aggregate custom scan off)
SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO off; SELECT
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
