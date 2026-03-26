-- Top-K BM25 scored questions
SELECT id, title, score, paradedb.score(id)
FROM posts_questions
WHERE posts_questions @@@ paradedb.parse('title:database')
ORDER BY paradedb.score(id) DESC
LIMIT 10;
