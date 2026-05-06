-- Shape: Semi-Join Filter (Term Set)
-- Join: Single Feature (Fast Field)
-- Description: The sort is simple (e.g., by Title), but the filter involves a "List" logic where the valid IDs are derived from a complex subquery or list. This was previously implemented using a term_set, effectively pushing a semi-join down to the search index.

-- Query Info (statistics from 100k dataset; larger datasets may have different values):
-- - 'java' selectivity on users.about_me: ~5%
-- - 'David John Alex' selectivity on users.display_name: ~2%
-- - combined selectivity on users (about_me ||| 'java' AND display_name ||| 'David John Alex'): <1%

SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT
    p.id,
    p.title,
    p.creation_date
FROM stackoverflow_posts p
WHERE
    p.owner_user_id IN (
        SELECT id
        FROM users
        WHERE about_me ||| 'java'
        AND display_name ||| 'David John Alex'
    )
ORDER BY
    p.title ASC
LIMIT 25;

-- Sortedness disabled, with join scan.
SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT
    p.id,
    p.title,
    p.creation_date
FROM stackoverflow_posts p
WHERE
    p.owner_user_id IN (
        SELECT id
        FROM users
        WHERE about_me ||| 'java'
        AND display_name ||| 'David John Alex'
    )
ORDER BY
    p.title ASC
LIMIT 25;

-- Sortedness enabled, no join scan.
SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT
    p.id,
    p.title,
    p.creation_date
FROM stackoverflow_posts p
WHERE
    p.owner_user_id IN (
        SELECT id
        FROM users
        WHERE about_me ||| 'java'
        AND display_name ||| 'David John Alex'
    )
ORDER BY
    p.title ASC
LIMIT 25;

-- term_set workaround, no join
SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT
    p.id,
    p.title,
    p.creation_date
FROM stackoverflow_posts p
WHERE
    p.owner_user_id @@@ pdb.term_set((
        SELECT array_agg(id) FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex'
    ))
ORDER BY
    p.title ASC
LIMIT 25;

-- Sortedness enabled, with join scan.
SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; SELECT
    p.id,
    p.title,
    p.creation_date
FROM stackoverflow_posts p
WHERE
    p.owner_user_id IN (
        SELECT id
        FROM users
        WHERE about_me ||| 'java'
        AND display_name ||| 'David John Alex'
    )
ORDER BY
    p.title ASC
LIMIT 25;
