SET work_mem TO '4GB'; SELECT
  p.id,
  p.title,
  u.display_name,
  u.reputation,
  ts_rank(
    to_tsvector('english', p.title),
    to_tsquery('english', 'python | error')
  ) AS score
FROM
  stackoverflow_posts p
  JOIN users u ON p.owner_user_id = u.id
WHERE
  to_tsvector('english', p.title) @@ to_tsquery('english', 'python | error')
  AND to_tsvector('english', u.about_me) @@ to_tsquery('english', 'software | developer')
ORDER BY
  score DESC,
  p.id DESC
LIMIT 20;
