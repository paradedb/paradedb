CREATE EXTENSION IF NOT EXISTS pg_prewarm;
-- Prewarm every index on cohere_wiki
SELECT pg_prewarm(indexrelid::regclass::text) FROM pg_index WHERE indrelid = 'cohere_wiki'::regclass;
SELECT pg_prewarm('cohere_wiki');
