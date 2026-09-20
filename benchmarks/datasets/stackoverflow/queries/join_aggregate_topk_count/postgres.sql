SET work_mem TO '8GB'; SELECT
    b.name,
    COUNT(*)
FROM stackoverflow_posts p
JOIN badges b ON b.user_id = p.owner_user_id
WHERE
    to_tsvector('english', p.body) @@ plainto_tsquery('english', 'javascript')
GROUP BY
    b.name
ORDER BY
    COUNT(*) DESC
LIMIT 10;
