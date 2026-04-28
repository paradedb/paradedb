-- Join with no scores, large target list.

SET paradedb.enable_join_custom_scan TO off; SELECT
  *
FROM
  users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id
WHERE
  users.about_me ||| 'and' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question'
LIMIT 5;

SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT
  *
FROM
  users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id
WHERE
  users.about_me ||| 'and' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question'
LIMIT 5;
