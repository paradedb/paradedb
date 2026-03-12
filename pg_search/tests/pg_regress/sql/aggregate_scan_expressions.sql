-- Repro for documents_view aggregation
CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE SCHEMA IF NOT EXISTS documents;

CREATE TABLE documents.core
(
    id          serial constraint pkey_core_id primary key,
    dwf_doid    bigint,
    record_type varchar,
    author      character varying[]
);

CREATE UNIQUE INDEX idx_core_dwf_doid
    ON documents.core (dwf_doid);

CREATE INDEX idx_parade_core ON documents.core USING bm25 (
    id,
    dwf_doid,
    (lower(record_type)::pdb.literal),
    (author::pdb.literal)
) WITH (
    key_field='id'
);

CREATE VIEW documents.documents_view
AS
SELECT core.id,
       core.dwf_doid,
       core.author
FROM documents.core;

-- Insert some dummy data so the query returns rows
INSERT INTO documents.core (dwf_doid, record_type, author) VALUES (1, 'Type1', ARRAY['author1']);
INSERT INTO documents.core (dwf_doid, record_type, author) VALUES (2, 'Type2', ARRAY['author2']);

-- Enable the aggregate custom scan.
SET paradedb.enable_aggregate_custom_scan TO on;

-- Explain and execute the repro query
EXPLAIN
SELECT
    lower(record_type),
    pdb.agg('{"value_count": {"field": "dwf_doid"}}') AS count
FROM documents.core
WHERE dwf_doid @@@ pdb.all()
GROUP BY lower(record_type)
ORDER BY lower(record_type) DESC
LIMIT 100
OFFSET 0;

SELECT
    lower(record_type),
    pdb.agg('{"value_count": {"field": "dwf_doid"}}') AS count
FROM documents.core
WHERE dwf_doid @@@ pdb.all()
GROUP BY lower(record_type)
ORDER BY lower(record_type) DESC
LIMIT 100
OFFSET 0;

-- Explain and execute the repro query without unnest.
EXPLAIN
SELECT author, pdb.agg('{"value_count": {"field": "dwf_doid"}}') FROM documents.documents_view
WHERE dwf_doid @@@ pdb.all()
GROUP BY author
LIMIT 5 OFFSET 0;

SELECT author, pdb.agg('{"value_count": {"field": "dwf_doid"}}') FROM documents.documents_view
WHERE dwf_doid @@@ pdb.all()
GROUP BY author
LIMIT 5 OFFSET 0;

-- Explain and execute the repro query with unnest.
EXPLAIN
SELECT UNNEST(author), pdb.agg('{"value_count": {"field": "dwf_doid"}}') FROM documents.documents_view
WHERE dwf_doid @@@ pdb.all()
GROUP BY UNNEST(author)
LIMIT 5 OFFSET 0;

SELECT UNNEST(author), pdb.agg('{"value_count": {"field": "dwf_doid"}}') FROM documents.documents_view
WHERE dwf_doid @@@ pdb.all()
GROUP BY UNNEST(author)
LIMIT 5 OFFSET 0;

-- Drop the schema and everything in it
DROP SCHEMA documents CASCADE;
RESET paradedb.enable_aggregate_custom_scan;
