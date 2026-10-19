-- Strategy: Canonical 3-branch UNION ALL + GROUP BY (100% exact recall).
--
-- 1. PostgreSQL cannot decompose a cross-table OR predicate across joins into
--    per-table GIN index scans. Rewriting the disjunction into a 3-way UNION ALL
--    allows each branch to drive from one table's GIN index and join the other
--    two tables via B-Tree foreign-key/primary-key indexes.
-- 2. Each branch computes ts_rank() only on its GIN-filtered driving table inside
--    a fenced subquery (`WITH _ AS (SELECT 1)`). This optimization fence prevents
--    pull_up_subqueries from flattening the branch, enabling Parallel Append and
--    avoiding redundant to_tsvector() evaluations across the 1:N join fanout.
-- 3. Because (users.id, stackoverflow_posts.id, comments.id) is a unique 3-tuple
--    per branch, GROUP BY u_id, p_id, c_id with SUM(part_score) sums the partial
--    scores of multi-table matches and avoids NULL score propagation from
--    nullable users.about_me (81.5% NULL) and stackoverflow_posts.title (61.5% NULL).
-- 4. Tradeoff: Achieves 100% recall across all match combinations, but must
--    materialize and hash-aggregate all qualifying joined triples before sorting.
-- 5. `enable_indexonlyscan = off` prevents ParadeDB's bm25 index-only scan cost
--    estimate from hijacking unpredicated B-Tree join branches on tables that
--    also have a bm25 index.
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
