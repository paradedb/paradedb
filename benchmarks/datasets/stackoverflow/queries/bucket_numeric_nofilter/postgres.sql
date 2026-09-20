SELECT post_type_id, COUNT(*)
FROM stackoverflow_posts
GROUP BY post_type_id
ORDER BY post_type_id;
