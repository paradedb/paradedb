-- Multi-field search across title and body
SELECT id, title, score
FROM posts_questions
WHERE posts_questions @@@ paradedb.parse('title:indexing AND body:performance')
ORDER BY score DESC
LIMIT 100;
