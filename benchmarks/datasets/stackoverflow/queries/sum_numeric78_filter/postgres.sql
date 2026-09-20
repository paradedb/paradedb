SELECT SUM(amount78)
FROM stackoverflow_posts
WHERE to_tsvector('english', body) @@ plainto_tsquery('english', 'error');
