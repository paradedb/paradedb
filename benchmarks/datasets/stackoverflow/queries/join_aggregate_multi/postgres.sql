SET work_mem TO '4GB'; SELECT COUNT(*), MIN(c.score), MAX(c.score)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE to_tsvector('english', p.body) @@ plainto_tsquery('english', 'code');
