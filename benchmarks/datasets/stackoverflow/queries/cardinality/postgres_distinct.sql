SET work_mem TO '4GB'; SELECT COUNT(DISTINCT post_type_id)
FROM stackoverflow_posts
WHERE to_tsvector('english', body) @@ plainto_tsquery('english', 'javascript');
