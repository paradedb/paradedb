CREATE EXTENSION IF NOT EXISTS pg_prewarm;
SELECT pg_prewarm('stackoverflow_posts_idx');
SELECT pg_prewarm('badges_idx');
