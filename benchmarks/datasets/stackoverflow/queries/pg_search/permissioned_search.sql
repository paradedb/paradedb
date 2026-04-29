-- Shape: Permissioned Search (Score Sort)
-- Join: Single Feature (BM25 Score)
-- Description: A Full Text Search (BM25) drives the ranking, but the result set must be restricted by a JOIN (e.g., checking permissions or document isolation). The score comes purely from the stackoverflow_posts table, but the validity of the row depends on the users table.

-- Query Info (statistics from 100k dataset; larger datasets may have different values):
-- - 'to' selectivity on stackoverflow_posts.title: ~11%
-- - 'and' selectivity on users.about_me: ~20%

SET paradedb.enable_join_custom_scan TO off; SELECT
    p.id,
    p.title,
    pdb.score(p.id) as relevance
FROM stackoverflow_posts p
JOIN users u ON p.owner_user_id = u.id
WHERE
    p.title ||| 'to'                -- Driving the Sort (Single Feature)
    AND u.about_me ||| 'and'
ORDER BY
    relevance DESC
LIMIT 10;

SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT
    p.id,
    p.title,
    pdb.score(p.id) as relevance
FROM stackoverflow_posts p
JOIN users u ON p.owner_user_id = u.id
WHERE
    p.title ||| 'to'                -- Driving the Sort (Single Feature)
    AND u.about_me ||| 'and'
ORDER BY
    relevance DESC
LIMIT 10;
