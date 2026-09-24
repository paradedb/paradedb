-- DataFusion aggregate scan
-- @mvcc_sensitive
SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code';
