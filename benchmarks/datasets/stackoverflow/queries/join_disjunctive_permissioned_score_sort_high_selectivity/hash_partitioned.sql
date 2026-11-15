SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SET paradedb.enable_range_partitioned_join TO off; SELECT
  p.id,
  c.id,
  pdb.score(p.id) + pdb.score(c.id) AS pdb_score
FROM
  stackoverflow_posts p
  JOIN comments c ON c.post_id = p.id
WHERE
  (p.title ||| 'welcome' OR c.text ||| 'welcome')
  AND p.score >= (SELECT 50) AND p.score < (SELECT 51)
ORDER BY
  pdb_score DESC,
  c.id ASC
LIMIT 20;
