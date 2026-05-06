-- Join with scores/order-by/limit, large target list.

-- Query Info (statistics from 100k dataset; larger datasets may have different values):
-- - 'java' selectivity on users.about_me: ~5%
-- - 'error' selectivity on stackoverflow_posts.title: ~1%
-- - 'question' selectivity on comments.text: ~7%

-- Directly, without a CTE.
SET paradedb.enable_join_custom_scan TO off; SELECT
  *,
  pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score
FROM
  users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id
WHERE
  users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question'
ORDER BY pdb_score DESC
LIMIT 1000;

-- CTE to execute a smaller join before Top K and then fetch the rest of the content after Top K.
WITH topk AS (
  SELECT
    users.id AS user_id,
    stackoverflow_posts.id AS post_id,
    comments.id AS comment_id,
    pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score
  FROM
    users
    JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id
    JOIN comments ON comments.post_id = stackoverflow_posts.id
  WHERE
    users.about_me ||| 'java'
    AND stackoverflow_posts.title ||| 'error'
    AND comments.text ||| 'question'
  ORDER BY
    pdb_score DESC
  LIMIT 1000
)
SELECT
  u.*,
  p.*,
  c.*,
  topk.pdb_score
FROM
  topk
  JOIN users u ON topk.user_id = u.id
  JOIN stackoverflow_posts p ON topk.post_id = p.id
  JOIN comments c ON topk.comment_id = c.id
WHERE
  topk.user_id = u.id AND topk.post_id = p.id AND topk.comment_id = c.id
ORDER BY
  topk.pdb_score DESC;

-- Directly, without a CTE.
SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT
  *,
  pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score
FROM
  users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id
WHERE
  users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question'
ORDER BY pdb_score DESC
LIMIT 1000;
