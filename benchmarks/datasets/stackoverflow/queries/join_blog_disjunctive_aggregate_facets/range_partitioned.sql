SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_range_partitioned_join TO on; SELECT
  p.tags,
  COUNT(*) AS post_count
FROM
  users u
  JOIN stackoverflow_posts p ON p.owner_user_id = u.id
WHERE
  u.about_me ||| 'python'
  OR p.title ||| 'python'
GROUP BY
  p.tags
ORDER BY
  post_count DESC
LIMIT 10;
