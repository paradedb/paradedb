SET work_mem TO '8GB'; SELECT
  b.name,
  COUNT(*) AS badge_count
FROM
  stackoverflow_posts p
  JOIN badges b ON b.user_id = p.owner_user_id
WHERE
  to_tsvector('english', p.title) @@ to_tsquery('english', 'python')
GROUP BY
  b.name
ORDER BY
  badge_count DESC
LIMIT 10;
