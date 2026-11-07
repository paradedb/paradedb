-- Strategy: Canonical 3-branch UNION ALL + GROUP BY (100% exact recall).
-- See ../join_disjunctive_score_sort_low_selectivity/README.md for details.
SET work_mem TO '4GB';
SET enable_indexonlyscan TO off;

SELECT
  u_id AS id,
  p_id AS id,
  c_id AS id,
  SUM(part_score) AS score
FROM (
  SELECT u.id AS u_id, p.id AS p_id, c.id AS c_id, u.u_score AS part_score
  FROM (
    WITH _ AS (SELECT 1)
    SELECT
      id,
      ts_rank(to_tsvector('english', about_me), plainto_tsquery('english', 'ocaml')) AS u_score
    FROM users
    WHERE to_tsvector('english', about_me) @@ plainto_tsquery('english', 'ocaml')
  ) u
  JOIN stackoverflow_posts p ON p.owner_user_id = u.id
  JOIN comments c ON c.post_id = p.id

  UNION ALL

  SELECT u.id AS u_id, p.id AS p_id, c.id AS c_id, p.p_score AS part_score
  FROM (
    WITH _ AS (SELECT 1)
    SELECT
      id,
      owner_user_id,
      ts_rank(to_tsvector('english', title), plainto_tsquery('english', 'ocaml')) AS p_score
    FROM stackoverflow_posts
    WHERE to_tsvector('english', title) @@ plainto_tsquery('english', 'ocaml')
  ) p
  JOIN users u ON u.id = p.owner_user_id
  JOIN comments c ON c.post_id = p.id

  UNION ALL

  SELECT u.id AS u_id, p.id AS p_id, c.id AS c_id, c.c_score AS part_score
  FROM (
    WITH _ AS (SELECT 1)
    SELECT
      id,
      post_id,
      ts_rank(to_tsvector('english', text), plainto_tsquery('english', 'ocaml')) AS c_score
    FROM comments
    WHERE to_tsvector('english', text) @@ plainto_tsquery('english', 'ocaml')
  ) c
  JOIN stackoverflow_posts p ON p.id = c.post_id
  JOIN users u ON u.id = p.owner_user_id
) t
GROUP BY u_id, p_id, c_id
ORDER BY score DESC, c_id DESC
LIMIT 20;
