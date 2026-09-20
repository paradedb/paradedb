SET work_mem TO '4GB'; SELECT
    p.id,
    p.title,
    MAX(c.creation_date) as last_activity
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE
    to_tsvector('english', p.body) @@ plainto_tsquery('english', 'code')
GROUP BY
    p.id, p.title
ORDER BY
    last_activity DESC
LIMIT 10;
