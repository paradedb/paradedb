-- Count matching questions
SELECT COUNT(*)
FROM posts_questions
WHERE posts_questions @@@ paradedb.parse('title:python');
