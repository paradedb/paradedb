-- version with scan off (Postgres-driven join)
SET paradedb.enable_join_custom_scan TO off; SELECT 
  p.id, 
  pdb.score(p.id) AS score, 
  p.title 
FROM stackoverflow_posts p 
JOIN users u ON p.owner_user_id = u.id 
WHERE p.body ||| 'the' AND u.about_me ||| 'the' -- restricted on user.about_me contents
ORDER BY score DESC -- sort is driven by a single table
LIMIT 5;
