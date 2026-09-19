-- DataFusion TopK aggregate scan with the strings kept late-materialized
SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_aggregate_late_materialization TO on; SELECT
    b.name,
    COUNT(*)
FROM stackoverflow_posts p
JOIN badges b ON b.user_id = p.owner_user_id
WHERE
    p.body ||| 'javascript'
GROUP BY
    b.name
ORDER BY
    COUNT(*) DESC
LIMIT 10;
