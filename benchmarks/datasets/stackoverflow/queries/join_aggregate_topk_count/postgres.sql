-- Postgres default plan (aggregate custom scan off)
SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO off; SELECT
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
