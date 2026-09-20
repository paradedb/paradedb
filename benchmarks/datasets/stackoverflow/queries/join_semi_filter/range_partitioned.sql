-- Range-partitioned join scan.
SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SET paradedb.enable_range_partitioned_join TO on; SELECT
    p.id,
    p.title,
    p.creation_date
FROM stackoverflow_posts p
WHERE
    p.owner_user_id IN (
        SELECT id
        FROM users
        WHERE about_me ||| 'java'
        AND display_name ||| 'David John Alex'
    )
ORDER BY
    p.title ASC
LIMIT 25;
