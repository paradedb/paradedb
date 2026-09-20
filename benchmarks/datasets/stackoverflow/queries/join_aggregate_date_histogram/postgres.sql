SET work_mem TO '4GB'; SELECT
    p.creation_date::date AS day,
    COUNT(*)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE
    to_tsvector('english', p.body) @@ plainto_tsquery('english', 'code')
GROUP BY
    day
ORDER BY
    day ASC
LIMIT 30;
