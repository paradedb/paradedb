-- FIXME: 'the' is a stopword in the Postgres 'english' dictionary and yields an empty tsquery
-- with plainto_tsquery, matching 0 rows. Consider using the 'simple' text search configuration
-- or substituting a high-frequency non-stopword token.
SELECT 
  p.id, 
  ts_rank(to_tsvector('english', p.body), plainto_tsquery('english', 'the')) AS score, 
  p.title 
FROM stackoverflow_posts p 
JOIN users u ON p.owner_user_id = u.id 
WHERE to_tsvector('english', p.body) @@ plainto_tsquery('english', 'the')
  AND to_tsvector('english', u.about_me) @@ plainto_tsquery('english', 'the')
ORDER BY score DESC
LIMIT 5;
