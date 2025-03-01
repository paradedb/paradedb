CREATE EXTENSION IF NOT EXISTS pg_prewarm;
SELECT pg_prewarm('message_gin');
SELECT pg_prewarm('country_btree');
SELECT pg_prewarm('severity_btree');
SELECT pg_prewarm('timestamp_btree');
SELECT pg_prewarm('metadata_label_gin');
