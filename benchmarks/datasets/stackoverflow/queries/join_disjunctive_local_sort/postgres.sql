SET work_mem TO '4GB'; SELECT
  users.id,
  stackoverflow_posts.id,
  comments.id
FROM
  users
  JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id
  JOIN comments ON comments.post_id = stackoverflow_posts.id
WHERE
  to_tsvector('english', users.about_me) @@ plainto_tsquery('english', 'python')
  OR to_tsvector('english', stackoverflow_posts.title) @@ plainto_tsquery('english', 'python')
  OR to_tsvector('english', comments.text) @@ plainto_tsquery('english', 'python')
ORDER BY
  comments.creation_date DESC,
  comments.id DESC
LIMIT 20;
