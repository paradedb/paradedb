SET work_mem TO '4GB'; SELECT
  users.id AS user_id,
  users.display_name,
  stackoverflow_posts.id AS post_id,
  stackoverflow_posts.title,
  comments.id AS comment_id,
  comments.score AS comment_score,
  ts_rank(to_tsvector('english', users.about_me), plainto_tsquery('english', 'java'))
  + ts_rank(to_tsvector('english', stackoverflow_posts.title), plainto_tsquery('english', 'error'))
  + ts_rank(to_tsvector('english', comments.text), plainto_tsquery('english', 'question')) AS score
FROM
  users
  JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id
  JOIN comments ON comments.post_id = stackoverflow_posts.id
WHERE
  to_tsvector('english', users.about_me) @@ plainto_tsquery('english', 'java')
  AND to_tsvector('english', stackoverflow_posts.title) @@ plainto_tsquery('english', 'error')
  AND to_tsvector('english', comments.text) @@ plainto_tsquery('english', 'question')
ORDER BY
  score DESC,
  comments.id DESC
LIMIT 20;
