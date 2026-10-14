-- DataFusion aggregate scan with range partitioned join
SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_range_partitioned_join TO on; SELECT COUNT(*)
FROM users u
JOIN stackoverflow_posts p ON u.id = p.owner_user_id
JOIN comments c ON p.id = c.post_id
WHERE u.about_me ||| 'python' OR p.title ||| 'python' OR c.text ||| 'python';
