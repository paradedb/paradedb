-- Search answers for specific content
SELECT id, body, score, parent_id
FROM posts_answers
WHERE posts_answers @@@ paradedb.parse('body:optimization')
ORDER BY score DESC
LIMIT 100;
