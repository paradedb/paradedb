-- Join questions with answers and search
SELECT q.id, q.title, q.score AS question_score, a.id AS answer_id, a.score AS answer_score
FROM posts_questions q
JOIN posts_answers a ON a.parent_id = q.id
WHERE posts_questions @@@ paradedb.parse('title:react')
ORDER BY a.score DESC
LIMIT 100;
