-- Shape: Permissioned Search (Score Sort)
-- Join: Single Feature (BM25 Score)
-- Description: A Full Text Search (BM25) drives the ranking, but the result set must be restricted by a JOIN (e.g., checking permissions or document isolation). The score comes purely from the stackoverflow_posts table, but the validity of the row depends on the users table.

-- Query Info (statistics from 100k dataset; larger datasets may have different values):
-- - 'how using get create' selectivity on stackoverflow_posts.title: ~10%
-- - reputation > 100 selectivity on users.reputation: ~82% (active users are overrepresented at smaller sizes; likely lower for larger datasets)

SET paradedb.enable_join_custom_scan TO off; SELECT
    p.id,
    p.title,
    pdb.score(p.id) as relevance
FROM stackoverflow_posts p
JOIN users u ON p.owner_user_id = u.id
WHERE
    p.title ||| 'how using get create'  -- Driving the Sort (Single Feature)
    AND u.id @@@ pdb.all()
    AND u.reputation > 100
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
    p.title ||| 'how using get create'  -- Driving the Sort (Single Feature)
    AND u.id @@@ pdb.all()
    AND u.reputation > 100
ORDER BY
    relevance DESC
LIMIT 10;
