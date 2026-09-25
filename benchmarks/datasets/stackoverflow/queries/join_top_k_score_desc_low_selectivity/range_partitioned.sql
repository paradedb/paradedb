-- version with range partitioned join scan on
SET paradedb.enable_join_custom_scan TO on; SET paradedb.enable_range_partitioned_join TO DEFAULT; SELECT
  p.id,
  pdb.score(p.id) AS score,
  p.title
FROM stackoverflow_posts p
JOIN users u ON p.owner_user_id = u.id
WHERE p.body ||| 'code developer' AND u.about_me ||| 'code developer' -- restricted on user.about_me contents
ORDER BY score DESC -- sort is driven by a single table
LIMIT 5;
