CREATE EXTENSION IF NOT EXISTS pg_prewarm;
SELECT pg_prewarm('cohere_wiki_emb_idx');
SELECT pg_prewarm('cohere_wiki_text_fts_idx');
SELECT pg_prewarm('cohere_wiki');
