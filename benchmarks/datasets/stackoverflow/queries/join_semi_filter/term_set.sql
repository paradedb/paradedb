-- term_set workaround, no join
SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO off; SELECT
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
