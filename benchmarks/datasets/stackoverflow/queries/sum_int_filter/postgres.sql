SELECT SUM(view_count)
FROM stackoverflow_posts
WHERE to_tsvector('english', body) @@ plainto_tsquery('english', 'error');
