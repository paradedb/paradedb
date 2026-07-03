-- Shape: Foreign Filter, Local Sort
-- Join: Single Feature (Fast Field)
-- Description: A standard join where the user filters by a property of the parent table (users), but sorts by a deterministic "fast field" on the child table (stackoverflow_posts). The challenge is balancing the selectivity of the foreign filter against the sort order of the local table.

-- Query Info (statistics from 100k dataset; larger datasets may have different values):
-- - reputation > 100 selectivity on users.reputation: ~82% (active users are overrepresented at smaller sizes; likely lower for larger datasets)
-- - 'error' selectivity on stackoverflow_posts.title: ~1%

SET paradedb.enable_join_custom_scan TO off; SELECT
    p.id,
    p.title,
    p.creation_date,
    u.display_name as user_display_name,
    u.about_me as user_about_me
FROM stackoverflow_posts p
JOIN users u ON p.owner_user_id = u.id
WHERE
    u.id @@@ pdb.all()
    AND u.reputation > 100
    AND p.title ||| 'error'
ORDER BY
    p.creation_date DESC              -- Single Feature Sort (Local Fast Field)
LIMIT 20;

SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT
    p.id,
    p.title,
    p.creation_date,
    u.display_name as user_display_name,
    u.about_me as user_about_me
FROM stackoverflow_posts p
JOIN users u ON p.owner_user_id = u.id
WHERE
    u.id @@@ pdb.all()
    AND u.reputation > 100
    AND p.title ||| 'error'
ORDER BY
    p.creation_date DESC              -- Single Feature Sort (Local Fast Field)
LIMIT 20;
