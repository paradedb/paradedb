-- Strategy: Hierarchy-aware early-terminating 3-branch Top-K.
-- See join_blog_disjunctive_score_sort_medium_selectivity/README.md for a full
-- explanation of the 1:N:M hierarchy algorithm and its recall/performance tradeoffs.
SET work_mem TO '4GB';
SET enable_indexonlyscan TO off;

SELECT
  u_id AS user_id,
  display_name,
  p_id AS post_id,
  title,
  c_id AS comment_id,
  comment_score,
  MAX(score) AS score
FROM (
  (
    SELECT
      u.id AS u_id,
      u.display_name,
      p_c.p_id,
      p_c.title,
      p_c.c_id,
      p_c.comment_score,
      u.u_score AS score
    FROM (
      SELECT
        u.id,
        u.display_name,
        ts_rank(to_tsvector('english', u.about_me), plainto_tsquery('english', 'ocaml')) AS u_score
      FROM users u
      WHERE to_tsvector('english', u.about_me) @@ plainto_tsquery('english', 'ocaml')
      ORDER BY u_score DESC
      LIMIT 100
    ) u
    CROSS JOIN LATERAL (
      SELECT p.id AS p_id, p.title, c.id AS c_id, c.score AS comment_score
      FROM stackoverflow_posts p
      JOIN comments c ON c.post_id = p.id
      WHERE p.owner_user_id = u.id
    ) p_c
    ORDER BY u.u_score DESC, p_c.c_id DESC
    LIMIT 20
  )
  UNION ALL
  (
    SELECT
      p.owner_user_id AS u_id,
      uc.display_name,
      p.id AS p_id,
      p.title,
      uc.c_id,
      uc.comment_score,
      p.up_score AS score
    FROM (
      SELECT
        p.id,
        p.title,
        p.owner_user_id,
        COALESCE(u_m.u_score, 0.0)
          + ts_rank(to_tsvector('english', p.title), plainto_tsquery('english', 'ocaml')) AS up_score
      FROM stackoverflow_posts p
      LEFT JOIN (
        SELECT
          u.id,
          ts_rank(to_tsvector('english', u.about_me), plainto_tsquery('english', 'ocaml')) AS u_score
        FROM users u
        WHERE to_tsvector('english', u.about_me) @@ plainto_tsquery('english', 'ocaml')
      ) u_m ON u_m.id = p.owner_user_id
      WHERE to_tsvector('english', p.title) @@ plainto_tsquery('english', 'ocaml')
        AND p.owner_user_id IS NOT NULL
      ORDER BY up_score DESC
      LIMIT 100
    ) p
    CROSS JOIN LATERAL (
      SELECT u.display_name, c.id AS c_id, c.score AS comment_score
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
    SELECT
      pu.u_id,
      pu.display_name,
      c.post_id AS p_id,
      pu.title,
      c.id AS c_id,
      c.comment_score,
      c.upc_score AS score
    FROM (
      SELECT
        c.id,
        c.score AS comment_score,
        c.post_id,
        COALESCE(p_m.up_score, 0.0)
          + ts_rank(to_tsvector('english', c.text), plainto_tsquery('english', 'ocaml')) AS upc_score
      FROM comments c
      LEFT JOIN (
        SELECT
          p.id,
          COALESCE(u_m.u_score, 0.0)
            + ts_rank(to_tsvector('english', p.title), plainto_tsquery('english', 'ocaml')) AS up_score
        FROM stackoverflow_posts p
        LEFT JOIN (
          SELECT
            u.id,
            ts_rank(to_tsvector('english', u.about_me), plainto_tsquery('english', 'ocaml')) AS u_score
          FROM users u
          WHERE to_tsvector('english', u.about_me) @@ plainto_tsquery('english', 'ocaml')
        ) u_m ON u_m.id = p.owner_user_id
        WHERE to_tsvector('english', p.title) @@ plainto_tsquery('english', 'ocaml')
          AND p.owner_user_id IS NOT NULL
      ) p_m ON p_m.id = c.post_id
      WHERE to_tsvector('english', c.text) @@ plainto_tsquery('english', 'ocaml')
      ORDER BY upc_score DESC, c.id DESC
      LIMIT 100
    ) c
    CROSS JOIN LATERAL (
      SELECT u.id AS u_id, u.display_name, p.title
      FROM stackoverflow_posts p
      JOIN users u ON u.id = p.owner_user_id
      WHERE p.id = c.post_id
    ) pu
    ORDER BY c.upc_score DESC, c.id DESC
    LIMIT 20
  )
) candidates
GROUP BY u_id, display_name, p_id, title, c_id, comment_score
ORDER BY score DESC, c_id DESC
LIMIT 20;
