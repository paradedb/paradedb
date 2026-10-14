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
