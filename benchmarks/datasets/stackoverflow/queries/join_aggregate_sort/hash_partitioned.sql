SET paradedb.enable_range_partitioned_join TO off;
SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT
    p.id,
    p.title,
    MAX(c.creation_date) as last_activity
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE
    p.body ||| 'code'             -- Search Term
GROUP BY
    p.id, p.title
ORDER BY
    last_activity DESC            -- Single Feature Sort (Computed Aggregate)
LIMIT 10;
