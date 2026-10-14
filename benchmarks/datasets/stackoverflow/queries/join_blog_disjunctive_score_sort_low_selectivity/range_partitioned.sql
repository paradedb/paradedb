SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SET paradedb.enable_range_partitioned_join TO on; SET paradedb.enable_rank_join TO off; SELECT
  users.id AS user_id,
  users.display_name,
  stackoverflow_posts.id AS post_id,
  stackoverflow_posts.title,
  comments.id AS comment_id,
  comments.score AS comment_score,
  pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score
FROM
  users
  JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id
  JOIN comments ON comments.post_id = stackoverflow_posts.id
WHERE
  users.about_me ||| 'code'
  OR stackoverflow_posts.title ||| 'code'
  OR comments.text ||| 'code'
ORDER BY
  pdb_score DESC,
  comments.id DESC
LIMIT 20;
