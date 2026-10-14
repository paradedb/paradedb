SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_range_partitioned_join TO off; SELECT
  b.name,
  COUNT(*) AS badge_count
FROM
  stackoverflow_posts p
  JOIN badges b ON b.user_id = p.owner_user_id
WHERE
  p.title ||| 'python'
GROUP BY
  b.name
ORDER BY
  badge_count DESC
LIMIT 10;
