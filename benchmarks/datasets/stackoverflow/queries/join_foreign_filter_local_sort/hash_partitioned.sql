SET paradedb.enable_range_partitioned_join TO off;
SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT
    p.id,
    p.title,
    p.creation_date,
    u.display_name as user_display_name,
    u.about_me as user_about_me
FROM stackoverflow_posts p
JOIN users u ON p.owner_user_id = u.id
WHERE
    u.id @@@ pdb.all()
    AND u.reputation > 100
    AND p.title ||| 'error'
ORDER BY
    p.creation_date DESC              -- Single Feature Sort (Local Fast Field)
LIMIT 20;
