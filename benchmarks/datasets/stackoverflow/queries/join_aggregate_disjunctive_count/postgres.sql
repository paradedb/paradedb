SET work_mem TO '4GB'; SELECT COUNT(*)
FROM users u
JOIN stackoverflow_posts p ON u.id = p.owner_user_id
JOIN comments c ON p.id = c.post_id
WHERE to_tsvector('english', u.about_me) @@ plainto_tsquery('english', 'python')
   OR to_tsvector('english', p.title) @@ plainto_tsquery('english', 'python')
   OR to_tsvector('english', c.text) @@ plainto_tsquery('english', 'python');
