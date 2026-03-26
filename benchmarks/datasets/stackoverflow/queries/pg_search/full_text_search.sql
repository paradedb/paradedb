-- Full-text search on questions
SELECT id, title, score, view_count
FROM posts_questions
WHERE posts_questions @@@ paradedb.parse('title:postgres OR title:postgresql')
ORDER BY score DESC
LIMIT 100;
