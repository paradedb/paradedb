-- Shape: GROUP BY aggregate on JOIN
-- Join: stackoverflow_posts → comments
-- Description: Grouped aggregate (COUNT, SUM) with GROUP BY on the parent
-- table's title column. Exercises the DataFusion backend's grouped
-- aggregate pipeline including custom_scan_tlist for scanrelid=0.

-- Query Info (statistics from 100k dataset; larger datasets may have different values):
-- - 'code' selectivity on stackoverflow_posts.body: ~75%

-- Postgres default plan (custom scan off)
SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*), SUM(c.score)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code'
GROUP BY p.title
ORDER BY p.title;

-- DataFusion aggregate scan
SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*), SUM(c.score)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code'
GROUP BY p.title
ORDER BY p.title;
