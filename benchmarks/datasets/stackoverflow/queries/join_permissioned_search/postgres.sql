SET work_mem TO '4GB'; SELECT
    p.id,
    p.title,
    ts_rank(to_tsvector('english', p.title), plainto_tsquery('english', 'how using get create')) as relevance
FROM stackoverflow_posts p
JOIN users u ON p.owner_user_id = u.id
WHERE
    to_tsvector('english', p.title) @@ plainto_tsquery('english', 'how using get create')
    AND u.reputation > 100
ORDER BY
    relevance DESC
LIMIT 10;
