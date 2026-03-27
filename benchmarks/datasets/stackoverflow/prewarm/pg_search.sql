-- TODO: Add prewarm for all indexes once real indexes are defined.
CREATE EXTENSION IF NOT EXISTS pg_prewarm;
SELECT pg_prewarm('stackoverflow_posts_idx');
