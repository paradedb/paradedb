CREATE EXTENSION IF NOT EXISTS pg_search;
SET max_parallel_workers_per_gather = 0;
SET paradedb.global_mutable_segment_rows = 0;
CREATE TABLE documents (id integer PRIMARY KEY, body text);
INSERT INTO documents
SELECT id, 'alpha beta ' || repeat('gamma ', id % 200) || CASE WHEN id < 7 THEN ' rare' ELSE '' END
FROM generate_series(1, 1000) AS id;
CREATE INDEX documents_body_bm25_idx ON documents
USING bm25 ((body::pdb.unicode_words('pnorms=true')))
WITH (target_segment_count=1);
SET enable_seqscan = off;
SELECT id, pdb.score(id) AS score FROM documents
WHERE body @@@ pdb.parse('alpha AND beta', lenient => true)
ORDER BY score DESC LIMIT 10;
SELECT id, pdb.score(id) AS score FROM documents
WHERE body @@@ pdb.parse('alpha AND rare', lenient => true)
ORDER BY score DESC LIMIT 10;
