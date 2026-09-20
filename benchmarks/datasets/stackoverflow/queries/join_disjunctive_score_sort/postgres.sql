-- NOTE: It is not possible to execute this query without the joinscan today, because
-- Postgres takes over execution of the entire score-sum expression, which triggers an
-- "unsupported query shape". We leave it here as a duplicate of the query below it, as
-- having our own queries starting from the second position is the convention, and it would
-- be confusing to do otherwise here.
SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT
  users.id,
  stackoverflow_posts.id,
  comments.id,
  pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score
FROM
  users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id
WHERE
  users.about_me ||| 'python' OR stackoverflow_posts.title ||| 'python' OR comments.text ||| 'python'
ORDER BY
  pdb_score DESC,
  comments.id DESC
LIMIT 20;
