SET work_mem TO '8GB'; SET paradedb.enable_join_custom_scan TO on; SET paradedb.enable_range_partitioned_join TO DEFAULT; SELECT DISTINCT
    u.id,
    u.display_name,
    u.about_me
FROM users u
JOIN stackoverflow_posts p ON u.id = p.owner_user_id
JOIN comments c ON p.id = c.post_id
WHERE
    c.score > 0                     -- Filter on the "Many" side
    AND u.id @@@ pdb.all()
    AND u.reputation > 100
ORDER BY
    u.display_name ASC              -- Single Feature Sort (Parent Field)
LIMIT 50;
