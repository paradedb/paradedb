-- Shape: Top-k by score, restricted by a join
-- Join: stackoverflow_posts -> users
-- Description: This is a join that is blockmax-wand-eligible and uses the scores 
-- from a single table to drive the sort


-- Query Info (statistics from 100k dataset; larger datasets may have different values):
-- - 'the' chosen deliberately to introduce a very large pool of candidates for the 
--    topk (12.5K in the 100K dataset), to ensure we see the effect of the threshold 
--    tightening.

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

-- version with scan on (pg-search-driven join)
SET paradedb.enable_join_custom_scan TO on; SELECT 
  p.id, 
  pdb.score(p.id) AS score, 
  p.title 
FROM stackoverflow_posts p 
JOIN users u ON p.owner_user_id = u.id 
WHERE p.body ||| 'the' AND u.about_me ||| 'the' -- restricted on user.about_me contents
ORDER BY score DESC -- sort is driven by a single table
LIMIT 5;
