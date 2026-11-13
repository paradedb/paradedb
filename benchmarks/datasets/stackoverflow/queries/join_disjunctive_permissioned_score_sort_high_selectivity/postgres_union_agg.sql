-- Strategy: Canonical 2-branch UNION ALL + GROUP BY (100% exact recall).
-- See ../join_disjunctive_score_sort_low_selectivity/README.md for details.
SET work_mem TO '4GB';
SET enable_indexonlyscan TO off;

SELECT
  p_id AS id,
  c_id AS id,
  SUM(part_score) AS pdb_score
FROM (
  SELECT p.id AS p_id, c.id AS c_id, p.p_score AS part_score
  FROM (
    WITH _ AS (SELECT 1)
    SELECT
      id,
      ts_rank(to_tsvector('english', title), plainto_tsquery('english', 'welcome')) AS p_score
    FROM stackoverflow_posts
    WHERE to_tsvector('english', title) @@ plainto_tsquery('english', 'welcome')
      AND score >= (SELECT 50) AND score < (SELECT 51)
  ) p
  JOIN comments c ON c.post_id = p.id

  UNION ALL

  SELECT p.id AS p_id, c.id AS c_id, c.c_score AS part_score
  FROM (
    WITH _ AS (SELECT 1)
    SELECT
      id,
      post_id,
      ts_rank(to_tsvector('english', text), plainto_tsquery('english', 'welcome')) AS c_score
    FROM comments
    WHERE to_tsvector('english', text) @@ plainto_tsquery('english', 'welcome')
  ) c
  JOIN stackoverflow_posts p ON p.id = c.post_id
  WHERE p.score >= (SELECT 50) AND p.score < (SELECT 51)
) sub
GROUP BY p_id, c_id
ORDER BY pdb_score DESC, c_id ASC
LIMIT 20;
