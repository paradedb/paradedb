-- Join with no scores, small target list.

-- Query Info (statistics from 100k dataset; larger datasets may have different values):
-- - 'and' selectivity on users.about_me: ~20%
-- - 'error' selectivity on stackoverflow_posts.title: ~1%
-- - 'question' selectivity on comments.text: ~7%

SET paradedb.enable_join_custom_scan TO off; SELECT
  users.id,
  stackoverflow_posts.id,
  comments.id
FROM
  users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id
WHERE
  users.about_me ||| 'and' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question'
LIMIT 5;

SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT
  users.id,
  stackoverflow_posts.id,
  comments.id
FROM
  users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id
WHERE
  users.about_me ||| 'and' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question'
LIMIT 5;
