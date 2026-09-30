DROP INDEX IF EXISTS documents_body_bm25_idx;
\timing on
CREATE INDEX documents_body_bm25_idx ON documents
USING bm25 (id, (body::pdb.unicode_words('pnorms=true')))
WITH (key_field=id, target_segment_count=8);
\timing off
SELECT pg_relation_size('documents_body_bm25_idx') AS index_bytes;
VACUUM (ANALYZE) documents;
CHECKPOINT;
