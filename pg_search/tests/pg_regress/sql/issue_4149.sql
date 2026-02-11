CREATE EXTENSION IF NOT EXISTS pg_search;

\set rows 1000
SELECT setseed(0.1);

DROP TABLE IF EXISTS documents CASCADE;
CREATE TABLE documents (
  row_id     int PRIMARY KEY,
  file_id    int,
  file_title text,
  doc_title  text,
  doc_parents text
);

INSERT INTO documents (row_id, file_id, file_title, doc_title, doc_parents)
SELECT
  i AS row_id,
  i AS file_id,
  CASE
    WHEN (i % 10) = 0 THEN 'collab12 ' || i
    WHEN (i % 10) = 1 THEN 'FY2025_BUDGET_DATA ' || i
    ELSE 'File Title ' || i
  END AS file_title,
  'Document Title ' || i AS doc_title,
  CASE
    WHEN (i % 10) = 0 THEN 'SFR ' || i
    WHEN (i % 10) = 1 THEN 'PROJECT_ALPHA ' || i
    ELSE 'PARENT_GROUP_' || (i % 200) || ' ' || i
  END AS doc_parents
FROM generate_series(1, ceil(:rows / 8.0)::int) AS s(i);

CREATE INDEX documents_bm25
ON documents
USING bm25 (row_id, file_id, file_title, doc_title, doc_parents)
WITH (
  key_field = 'row_id',
  text_fields = '{
    "file_title":  { "tokenizer": { "type": "default" }, "fast": true },
    "doc_title":   { "tokenizer": { "type": "default" }, "fast": true },
    "doc_parents": { "tokenizer": { "type": "default" }, "fast": true }
  }'
);

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT
  t.file_id,
  t.file_title,
  paradedb.score(t.row_id) AS score
FROM documents t
WHERE
  t.doc_parents @@@ 'PARENT_GROUP'
  AND (t.file_title @@@ 'Title' OR t.doc_title @@@ 'Title')
ORDER BY score DESC
LIMIT 10;

SELECT
  t.file_id,
  t.file_title,
  paradedb.score(t.row_id) AS score
FROM documents t
WHERE
  t.doc_parents @@@ 'PARENT_GROUP'
  AND (t.file_title @@@ 'Title' OR t.doc_title @@@ 'Title')
ORDER BY score DESC
LIMIT 10;
