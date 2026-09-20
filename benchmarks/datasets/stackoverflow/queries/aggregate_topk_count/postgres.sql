SET work_mem TO '8GB'; SELECT
    p.title,
    COUNT(*)
FROM stackoverflow_posts p
WHERE
    to_tsvector('english', p.body) @@ plainto_tsquery('english', 'code')
GROUP BY
    p.title
ORDER BY
    COUNT(*) DESC
LIMIT 10;
