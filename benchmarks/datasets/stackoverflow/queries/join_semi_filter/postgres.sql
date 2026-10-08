SET work_mem TO '4GB'; SELECT
    p.id,
    p.title,
    p.creation_date
FROM stackoverflow_posts p
WHERE
    p.owner_user_id IN (
        SELECT id
        FROM users
        WHERE to_tsvector('english', about_me) @@ plainto_tsquery('english', 'java')
        AND to_tsvector('english', display_name) @@ plainto_tsquery('english', 'David John Alex')
    )
ORDER BY
    p.title ASC
LIMIT 25;
