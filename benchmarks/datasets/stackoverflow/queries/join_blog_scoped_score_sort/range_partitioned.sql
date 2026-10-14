SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SET paradedb.enable_range_partitioned_join TO on; SET paradedb.enable_rank_join TO off; SELECT
  p.id,
  p.title,
  u.display_name,
  u.reputation,
  pdb.score(p.id) AS score
FROM
  stackoverflow_posts p
  JOIN users u ON p.owner_user_id = u.id
WHERE
  p.title ||| 'python error'
  AND u.about_me ||| 'software developer'
ORDER BY
  score DESC,
  p.id DESC
LIMIT 20;
