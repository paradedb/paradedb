SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO off; SELECT
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
