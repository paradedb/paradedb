-- Strategy: Canonical 3-branch UNION ALL + GROUP BY (100% exact recall).
-- See join_blog_disjunctive_score_sort_medium_selectivity/README.md for a full
-- explanation of the algorithm and its performance tradeoffs.
SET work_mem TO '4GB';
SET enable_indexonlyscan TO off;

SELECT
  u_id AS user_id,
  display_name,
  p_id AS post_id,
  title,
  c_id AS comment_id,
  comment_score,
  SUM(part_score) AS score
FROM (
  SELECT
    u.id AS u_id,
    u.display_name,
    p.id AS p_id,
    p.title,
    c.id AS c_id,
    c.score AS comment_score,
    u.u_score AS part_score
  FROM (
    WITH _ AS (SELECT 1)
    SELECT
      id,
      display_name,
      ts_rank(to_tsvector('english', about_me), plainto_tsquery('english', 'code')) AS u_score
    FROM users
    WHERE to_tsvector('english', about_me) @@ plainto_tsquery('english', 'code')
  ) u
  JOIN stackoverflow_posts p ON p.owner_user_id = u.id
  JOIN comments c ON c.post_id = p.id

  UNION ALL

  SELECT
    u.id AS u_id,
    u.display_name,
    p.id AS p_id,
    p.title,
    c.id AS c_id,
    c.score AS comment_score,
    p.p_score AS part_score
  FROM (
    WITH _ AS (SELECT 1)
    SELECT
      id,
      title,
      owner_user_id,
      ts_rank(to_tsvector('english', title), plainto_tsquery('english', 'code')) AS p_score
    FROM stackoverflow_posts
    WHERE to_tsvector('english', title) @@ plainto_tsquery('english', 'code')
  ) p
  JOIN users u ON u.id = p.owner_user_id
  JOIN comments c ON c.post_id = p.id

  UNION ALL

  SELECT
    u.id AS u_id,
    u.display_name,
    p.id AS p_id,
    p.title,
    c.id AS c_id,
    c.comment_score,
    c.c_score AS part_score
  FROM (
    WITH _ AS (SELECT 1)
    SELECT
      id,
      score AS comment_score,
      post_id,
      ts_rank(to_tsvector('english', text), plainto_tsquery('english', 'code')) AS c_score
    FROM comments
    WHERE to_tsvector('english', text) @@ plainto_tsquery('english', 'code')
  ) c
  JOIN stackoverflow_posts p ON p.id = c.post_id
  JOIN users u ON u.id = p.owner_user_id
) t
GROUP BY u_id, display_name, p_id, title, c_id, comment_score
ORDER BY score DESC, c_id DESC
LIMIT 20;
