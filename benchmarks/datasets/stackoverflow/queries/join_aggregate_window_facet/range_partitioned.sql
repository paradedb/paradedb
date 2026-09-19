SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_join_custom_scan TO on; SET paradedb.enable_range_partitioned_join TO on; SELECT
    c.id,
    p.post_type_id,
    p.owner_user_id,
    COUNT(*) OVER (PARTITION BY p.post_type_id) as post_type_facet,
    COUNT(*) OVER (PARTITION BY p.owner_user_id) as user_facet
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE
    p.body ||| 'code'
ORDER BY
    c.score DESC
LIMIT 10;
