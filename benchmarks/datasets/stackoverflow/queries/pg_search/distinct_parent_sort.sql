-- Shape: Distinct Parent Sort (Join Explosion)
-- Join: Single Feature (Local Field with Deduplication)
-- Description: The user wants to find users that match criteria in the comments table (a "Many" side join), ordered by a field on the users table. Because joining to comments explodes the row count (1 User -> N Comments), the query requires DISTINCT. This tests the engine's ability to maintain the sort order while deduplicating the join results.

SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT
    u.id,
    u.display_name,
    u.about_me
FROM users u
JOIN stackoverflow_posts p ON u.id = p.owner_user_id
JOIN comments c ON p.id = c.post_id
WHERE
    c.score > 0                     -- Filter on the "Many" side
    AND u.about_me ||| 'and'
ORDER BY
    u.display_name ASC              -- Single Feature Sort (Parent Field)
LIMIT 50;

SET work_mem TO '64MB'; SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT
    u.id,
    u.display_name,
    u.about_me
FROM users u
JOIN stackoverflow_posts p ON u.id = p.owner_user_id
JOIN comments c ON p.id = c.post_id
WHERE
    c.score > 0                     -- Filter on the "Many" side
    AND u.about_me ||| 'and'
ORDER BY
    u.display_name ASC              -- Single Feature Sort (Parent Field)
LIMIT 50;
