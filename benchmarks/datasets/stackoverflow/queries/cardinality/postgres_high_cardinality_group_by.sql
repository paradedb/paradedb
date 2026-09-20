SET work_mem TO '4GB'; SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score)
FROM stackoverflow_posts
WHERE to_tsvector('english', body) @@ plainto_tsquery('english', 'javascript')
GROUP BY tags
LIMIT 65000;
