-- Strategy: Hierarchy-aware early-terminating 3-branch Top-K.
-- See ../join_disjunctive_score_sort_low_selectivity/README.md for details.
SET work_mem TO '4GB';
SET enable_indexonlyscan TO off;

SELECT u_id AS id, p_id AS id, c_id AS id, MAX(score) AS score
FROM (
  (
    SELECT u.id AS u_id, p_c.p_id, p_c.c_id, u.u_score AS score
    FROM (
      SELECT
        u.id,
        ts_rank(to_tsvector('english', u.about_me), plainto_tsquery('english', 'python')) AS u_score
      FROM users u
      WHERE to_tsvector('english', u.about_me) @@ plainto_tsquery('english', 'python')
      ORDER BY u_score DESC
      LIMIT 100
    ) u
    CROSS JOIN LATERAL (
      SELECT p.id AS p_id, c.id AS c_id
      FROM stackoverflow_posts p
      JOIN comments c ON c.post_id = p.id
      WHERE p.owner_user_id = u.id
    ) p_c
    ORDER BY u.u_score DESC, p_c.c_id DESC
    LIMIT 20
  )
  UNION ALL
  (
    SELECT p.owner_user_id AS u_id, p.id AS p_id, uc.c_id, p.up_score AS score
    FROM (
      SELECT
        p.id,
        p.owner_user_id,
        COALESCE(u_m.u_score, 0.0)
          + ts_rank(to_tsvector('english', p.title), plainto_tsquery('english', 'python')) AS up_score
      FROM stackoverflow_posts p
      LEFT JOIN (
        SELECT
          u.id,
          ts_rank(to_tsvector('english', u.about_me), plainto_tsquery('english', 'python')) AS u_score
        FROM users u
        WHERE to_tsvector('english', u.about_me) @@ plainto_tsquery('english', 'python')
      ) u_m ON u_m.id = p.owner_user_id
      WHERE to_tsvector('english', p.title) @@ plainto_tsquery('english', 'python')
        AND p.owner_user_id IS NOT NULL
      ORDER BY up_score DESC
      LIMIT 100
    ) p
    CROSS JOIN LATERAL (
      SELECT c.id AS c_id
      FROM users u
      CROSS JOIN comments c
      WHERE u.id = p.owner_user_id
        AND c.post_id = p.id
    ) uc
    ORDER BY p.up_score DESC, uc.c_id DESC
    LIMIT 20
  )
  UNION ALL
  (
    SELECT pu.u_id, c.post_id AS p_id, c.id AS c_id, c.upc_score AS score
    FROM (
      SELECT
        c.id,
        c.post_id,
        COALESCE(p_m.up_score, 0.0)
          + ts_rank(to_tsvector('english', c.text), plainto_tsquery('english', 'python')) AS upc_score
      FROM comments c
      LEFT JOIN (
        SELECT
          p.id,
          COALESCE(u_m.u_score, 0.0)
            + ts_rank(to_tsvector('english', p.title), plainto_tsquery('english', 'python')) AS up_score
        FROM stackoverflow_posts p
        LEFT JOIN (
          SELECT
            u.id,
            ts_rank(to_tsvector('english', u.about_me), plainto_tsquery('english', 'python')) AS u_score
          FROM users u
          WHERE to_tsvector('english', u.about_me) @@ plainto_tsquery('english', 'python')
        ) u_m ON u_m.id = p.owner_user_id
        WHERE to_tsvector('english', p.title) @@ plainto_tsquery('english', 'python')
          AND p.owner_user_id IS NOT NULL
      ) p_m ON p_m.id = c.post_id
      WHERE to_tsvector('english', c.text) @@ plainto_tsquery('english', 'python')
      ORDER BY upc_score DESC, c.id DESC
      LIMIT 100
    ) c
    CROSS JOIN LATERAL (
      SELECT u.id AS u_id
      FROM stackoverflow_posts p
      JOIN users u ON u.id = p.owner_user_id
      WHERE p.id = c.post_id
    ) pu
    ORDER BY c.upc_score DESC, c.id DESC
    LIMIT 20
  )
) candidates
GROUP BY u_id, p_id, c_id
ORDER BY score DESC, c_id DESC
LIMIT 20;
