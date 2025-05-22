CREATE EXTENSION IF NOT EXISTS pg_prewarm;
SELECT pg_prewarm('pages_index');
SELECT pg_prewarm('files_index');
SELECT pg_prewarm('documents_index');
