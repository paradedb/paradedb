SET work_mem TO '8GB'; SELECT
  p.tags,
  COUNT(*) AS post_count
FROM
  users u
  JOIN stackoverflow_posts p ON p.owner_user_id = u.id
WHERE
  to_tsvector('english', u.about_me) @@ to_tsquery('english', 'python')
  OR to_tsvector('english', p.title) @@ to_tsquery('english', 'python')
GROUP BY
  p.tags
ORDER BY
  post_count DESC
LIMIT 10;
