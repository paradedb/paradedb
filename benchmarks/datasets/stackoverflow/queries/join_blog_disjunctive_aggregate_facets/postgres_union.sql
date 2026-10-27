SET work_mem TO '8GB';
SET enable_indexonlyscan TO off;

SELECT
  tags,
  COUNT(*) AS post_count
FROM (
  SELECT
    p.id,
    p.tags
  FROM
    users u
    JOIN stackoverflow_posts p ON p.owner_user_id = u.id
  WHERE
    to_tsvector('english', u.about_me) @@ to_tsquery('english', 'python')
  UNION
  SELECT
    p.id,
    p.tags
  FROM
    stackoverflow_posts p
    JOIN users u ON u.id = p.owner_user_id
  WHERE
    to_tsvector('english', p.title) @@ to_tsquery('english', 'python')
) matched
GROUP BY
  tags
ORDER BY
  post_count DESC
LIMIT 10;
