SELECT post_type_id, COUNT(*)
FROM stackoverflow_posts
WHERE to_tsvector('english', body) @@ plainto_tsquery('english', 'javascript')
GROUP BY post_type_id
ORDER BY post_type_id;
