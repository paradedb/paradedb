CREATE EXTENSION IF NOT EXISTS pg_prewarm;
-- Prewarm every index on cohere_wiki (plus the heap), whichever the run built: hnsw/ivfflat/vchord
-- ship an `emb` ANN index + a `text` GIN/tsvector index, while pg_search ships a single bm25 index
-- that serves both. Resolving indexes from the catalog keeps this index-agnostic across variants.
SELECT pg_prewarm(indexrelid::regclass::text) FROM pg_index WHERE indrelid = 'cohere_wiki'::regclass;
SELECT pg_prewarm('cohere_wiki');
