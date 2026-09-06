-- Shape: GROUP BY a long string through a fan-out JOIN (DataFusion)
-- Join: users -> stackoverflow_posts -> comments
-- Description: GROUP BY users.about_me, a long text column, with COUNT(*) ordered
-- DESC and LIMIT 10. Each user fans out through their posts and the posts'
-- comments, so far more rows reach the aggregate than there are distinct
-- values. A string decoded in the scan is carried, hashed and compared once per
-- joined row; grouping on term ordinals and decoding one value per group is
-- the case this query measures.

-- Query Info (statistics from 20m dataset):
-- - 'code' selectivity on stackoverflow_posts.body: ~75%
-- - about_me is NULL for users who left it empty and a few hundred bytes otherwise

-- Postgres default plan (aggregate custom scan off)
SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO off; SELECT
    u.about_me,
    COUNT(*)
FROM users u
JOIN stackoverflow_posts p ON u.id = p.owner_user_id
JOIN comments c ON p.id = c.post_id
WHERE
    p.body ||| 'code'
GROUP BY
    u.about_me
ORDER BY
    COUNT(*) DESC
LIMIT 10;

-- DataFusion TopK aggregate scan
SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT
    u.about_me,
    COUNT(*)
FROM users u
JOIN stackoverflow_posts p ON u.id = p.owner_user_id
JOIN comments c ON p.id = c.post_id
WHERE
    p.body ||| 'code'
GROUP BY
    u.about_me
ORDER BY
    COUNT(*) DESC
LIMIT 10;

-- DataFusion TopK aggregate scan with range partitioned join
SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_range_partitioned_join TO on; SELECT
    u.about_me,
    COUNT(*)
FROM users u
JOIN stackoverflow_posts p ON u.id = p.owner_user_id
JOIN comments c ON p.id = c.post_id
WHERE
    p.body ||| 'code'
GROUP BY
    u.about_me
ORDER BY
    COUNT(*) DESC
LIMIT 10;

-- DataFusion TopK aggregate scan with the strings kept late-materialized
SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_aggregate_late_materialization TO on; SELECT
    u.about_me,
    COUNT(*)
FROM users u
JOIN stackoverflow_posts p ON u.id = p.owner_user_id
JOIN comments c ON p.id = c.post_id
WHERE
    p.body ||| 'code'
GROUP BY
    u.about_me
ORDER BY
    COUNT(*) DESC
LIMIT 10;
