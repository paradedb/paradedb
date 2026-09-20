SET work_mem TO '4GB'; SELECT
    date_trunc('month', p.creation_date) AS month,
    COUNT(*)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE
    to_tsvector('english', p.body) @@ plainto_tsquery('english', 'code')
GROUP BY
    month
ORDER BY
    month ASC;
