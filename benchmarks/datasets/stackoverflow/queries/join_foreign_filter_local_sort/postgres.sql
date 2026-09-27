SET work_mem TO '4GB'; SELECT
    p.id,
    p.title,
    p.creation_date,
    u.display_name as user_display_name,
    u.about_me as user_about_me
FROM stackoverflow_posts p
JOIN users u ON p.owner_user_id = u.id
WHERE
    u.reputation > 100
    AND to_tsvector('english', p.title) @@ plainto_tsquery('english', 'error')
ORDER BY
    p.creation_date DESC
LIMIT 20;
