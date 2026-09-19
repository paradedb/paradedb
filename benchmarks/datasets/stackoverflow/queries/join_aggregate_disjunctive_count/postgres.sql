-- Postgres default plan (custom scan off)
SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*)
FROM users u
JOIN stackoverflow_posts p ON u.id = p.owner_user_id
JOIN comments c ON p.id = c.post_id
WHERE u.about_me ||| 'python' OR p.title ||| 'python' OR c.text ||| 'python';
