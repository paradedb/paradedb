SET work_mem TO '4GB'; SELECT COUNT(*) FROM (
    SELECT post_type_id
    FROM stackoverflow_posts
    WHERE to_tsvector('english', body) @@ plainto_tsquery('english', 'javascript')
    GROUP BY post_type_id
    ORDER BY post_type_id
);
