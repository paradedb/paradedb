-- Repro for documents_view aggregation
CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE SCHEMA IF NOT EXISTS documents;

CREATE TABLE documents.core
(
    id          serial constraint pkey_core_id primary key,
    dwf_doid    bigint,
    record_type varchar
);

CREATE INDEX idx_parade_core ON documents.core USING bm25 (
    id,
    dwf_doid,
    (lower(record_type)::pdb.literal)
) WITH (
    key_field='id'
);

-- Insert some dummy data so the query returns rows
INSERT INTO documents.core (dwf_doid, record_type) VALUES (1, 'Type1');
INSERT INTO documents.core (dwf_doid, record_type) VALUES (2, 'Type2');

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

-- Drop the schema and everything in it
DROP SCHEMA documents CASCADE;
RESET paradedb.enable_aggregate_custom_scan;
