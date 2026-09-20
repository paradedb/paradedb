CREATE EXTENSION IF NOT EXISTS pg_prewarm;
SELECT pg_prewarm('stackoverflow_posts_idx');
SELECT pg_prewarm('badges_idx');
SELECT pg_prewarm('comments_idx');
SELECT pg_prewarm('users_idx');

-- Prewarm standard Postgres companion indexes
SELECT pg_prewarm('stackoverflow_posts_owner_user_id_idx');
SELECT pg_prewarm('comments_post_id_idx');
SELECT pg_prewarm('badges_user_id_idx');
SELECT pg_prewarm('stackoverflow_posts_body_fts_idx');
SELECT pg_prewarm('stackoverflow_posts_title_fts_idx');
SELECT pg_prewarm('users_about_me_fts_idx');
SELECT pg_prewarm('users_display_name_fts_idx');
SELECT pg_prewarm('comments_text_fts_idx');
SELECT pg_prewarm('badges_name_fts_idx');
SELECT pg_prewarm('stackoverflow_posts_post_type_id_idx');
SELECT pg_prewarm('stackoverflow_posts_creation_date_idx');
SELECT pg_prewarm('users_reputation_idx');
SELECT pg_prewarm('comments_score_idx');
SELECT pg_prewarm('comments_creation_date_id_idx');
SELECT pg_prewarm('badges_name_idx');
