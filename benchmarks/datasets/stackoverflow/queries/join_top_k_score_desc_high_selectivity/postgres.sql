SELECT 
  p.id, 
  ts_rank(to_tsvector('english', p.body), websearch_to_tsquery('english', 'code or developer')) AS score, 
  p.title 
FROM stackoverflow_posts p 
JOIN users u ON p.owner_user_id = u.id 
WHERE to_tsvector('english', p.body) @@ websearch_to_tsquery('english', 'code or developer')
  AND to_tsvector('english', u.about_me) @@ websearch_to_tsquery('english', 'code or developer')
ORDER BY score DESC
LIMIT 5;
