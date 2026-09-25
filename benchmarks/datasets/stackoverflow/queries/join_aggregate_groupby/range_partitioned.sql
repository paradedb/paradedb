-- DataFusion aggregate scan with range partitioned join
SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_range_partitioned_join TO DEFAULT; SELECT p.post_type_id, COUNT(*), SUM(c.score)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code'
GROUP BY p.post_type_id
ORDER BY SUM(c.score) DESC;
