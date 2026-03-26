CREATE EXTENSION IF NOT EXISTS pg_prewarm;
SELECT pg_prewarm('stackoverflow_posts_idx');
SELECT pg_prewarm('posts_questions_idx');
SELECT pg_prewarm('posts_answers_idx');
SELECT pg_prewarm('comments_idx');
SELECT pg_prewarm('users_idx');
