SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT
    p.creation_date::date AS day,
    COUNT(*)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE
    p.body ||| 'code'
GROUP BY
    day
ORDER BY
    day ASC
LIMIT 30;
