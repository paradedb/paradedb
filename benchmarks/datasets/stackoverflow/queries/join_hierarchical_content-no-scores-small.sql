-- Join with no scores, small target list.

-- Query Info (statistics from 100k dataset; larger datasets may have different values):
-- - reputation > 100 selectivity on users.reputation: ~82% (active users are overrepresented at smaller sizes; likely lower for larger datasets)
-- - 'error' selectivity on stackoverflow_posts.title: ~1%
-- - 'question' selectivity on comments.text: ~7%

SET paradedb.enable_join_custom_scan TO off; SELECT
  users.id,
  stackoverflow_posts.id,
  comments.id
FROM
  users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id
WHERE
  users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question'
LIMIT 5;

SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT
  users.id,
  stackoverflow_posts.id,
  comments.id
FROM
  users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id
WHERE
  users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question'
LIMIT 5;
