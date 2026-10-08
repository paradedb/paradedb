SET work_mem TO '8GB'; SELECT DISTINCT
    u.id,
    u.display_name,
    u.about_me
FROM users u
JOIN stackoverflow_posts p ON u.id = p.owner_user_id
JOIN comments c ON p.id = c.post_id
WHERE
    c.score > 0
    AND u.reputation > 100
ORDER BY
    u.display_name ASC
LIMIT 50;
