SET paradedb.enable_range_partitioned_join TO off;
SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT
  users.id,
  stackoverflow_posts.id,
  comments.id
FROM
  users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id
WHERE
  users.about_me ||| 'python' OR stackoverflow_posts.title ||| 'python' OR comments.text ||| 'python'
ORDER BY
  comments.creation_date DESC,
  comments.id DESC
LIMIT 20;
