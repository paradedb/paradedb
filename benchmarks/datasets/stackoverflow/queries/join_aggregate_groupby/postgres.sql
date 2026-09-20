SET work_mem TO '8GB'; SELECT p.post_type_id, COUNT(*), SUM(c.score)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE to_tsvector('english', p.body) @@ plainto_tsquery('english', 'code')
GROUP BY p.post_type_id
ORDER BY SUM(c.score) DESC;
