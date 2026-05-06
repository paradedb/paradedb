-- Shape: Aggregate Sort (Computed Property)
-- Join: Single Feature (Computed Aggregate)
-- Description: The feature driving the ORDER BY does not exist until the join is performed. The user is sorting posts by an aggregate of their related comments (e.g., "Most recently active" or "Most interactions"). This requires computing the aggregate (via GROUP BY or LATERAL) before the Top K can be resolved.

-- Query Info (statistics from 100k dataset; larger datasets may have different values):
-- - 'code' selectivity on stackoverflow_posts.body: ~75%

SET paradedb.enable_join_custom_scan TO off; SELECT
    p.id,
    p.title,
    MAX(c.creation_date) as last_activity
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE
    p.body ||| 'code'             -- Search Term
GROUP BY
    p.id, p.title
ORDER BY
    last_activity DESC            -- Single Feature Sort (Computed Aggregate)
LIMIT 10;

SET paradedb.enable_join_custom_scan TO on; SELECT
    p.id,
    p.title,
    MAX(c.creation_date) as last_activity
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE
    p.body ||| 'code'             -- Search Term
GROUP BY
    p.id, p.title
ORDER BY
    last_activity DESC            -- Single Feature Sort (Computed Aggregate)
LIMIT 10;
