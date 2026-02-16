CREATE EXTENSION IF NOT EXISTS pg_search;

\set rows 1000

CREATE OR REPLACE FUNCTION uuid_text(p_integer INTEGER)
RETURNS TEXT AS $$
DECLARE
    int_text TEXT;
BEGIN
    int_text := LPAD(p_integer::TEXT, 10, '0');
    RETURN RPAD(int_text, 32, int_text)::uuid::text;
END;
$$ LANGUAGE plpgsql;

DROP TABLE IF EXISTS pages CASCADE;
CREATE TABLE pages (
    "id" TEXT PRIMARY KEY,
    "fileId" TEXT,
    "content" TEXT,
    "title" TEXT,
    "parents" TEXT,
    "sizeInBytes" BIGINT,
    "createdAt" TIMESTAMP,
    "fill0" BIGINT,
    "fill1" BIGINT,
    "fill2" BIGINT,
    "fill3" BIGINT,
    "fill4" BIGINT,
    "fill5" BIGINT,
    "fill6" BIGINT,
    "fill7" BIGINT,
    "fill8" BIGINT,
    "fill9" BIGINT,
    "fill10" VARCHAR,
    "fill11" VARCHAR,
    "fill12" VARCHAR,
    "fill13" VARCHAR,
    "fill14" VARCHAR,
    "fill15" VARCHAR,
    "fill16" VARCHAR,
    "fill17" VARCHAR,
    "fill18" VARCHAR,
    "fill19" VARCHAR,
    "fill20" VARCHAR,
    "fill21" VARCHAR,
    "fill22" VARCHAR,
    "fill23" VARCHAR,
    "fill24" VARCHAR,
    "fill25" VARCHAR,
    "fill26" VARCHAR,
    "fill27" VARCHAR,
    "fill28" VARCHAR
);

DROP TABLE IF EXISTS files CASCADE;
CREATE TABLE files (
    "id" TEXT PRIMARY KEY,
    "documentId" TEXT,
    "content" TEXT,
    "title" TEXT,
    "parents" TEXT,
    "sizeInBytes" BIGINT,
    "createdAt" TIMESTAMP,
    "fill0" BIGINT,
    "fill1" BIGINT,
    "fill2" BIGINT,
    "fill3" BIGINT,
    "fill4" BIGINT,
    "fill5" BIGINT,
    "fill6" BIGINT,
    "fill7" BIGINT,
    "fill8" BIGINT,
    "fill9" BIGINT,
    "fill10" VARCHAR,
    "fill11" VARCHAR,
    "fill12" VARCHAR,
    "fill13" VARCHAR,
    "fill14" VARCHAR,
    "fill15" VARCHAR,
    "fill16" VARCHAR,
    "fill17" VARCHAR,
    "fill18" VARCHAR,
    "fill19" VARCHAR,
    "fill20" VARCHAR,
    "fill21" VARCHAR,
    "fill22" VARCHAR,
    "fill23" VARCHAR,
    "fill24" VARCHAR,
    "fill25" VARCHAR,
    "fill26" VARCHAR,
    "fill27" VARCHAR,
    "fill28" VARCHAR
);

DROP TABLE IF EXISTS documents CASCADE;
CREATE TABLE documents (
    "id" TEXT PRIMARY KEY,
    "parents" TEXT,
    "content" TEXT,
    "title" TEXT,
    "createdAt" TIMESTAMP,
    "fill0" BIGINT,
    "fill1" BIGINT,
    "fill2" BIGINT,
    "fill3" BIGINT,
    "fill4" BIGINT,
    "fill5" BIGINT,
    "fill6" BIGINT,
    "fill7" BIGINT,
    "fill8" BIGINT,
    "fill9" BIGINT,
    "fill10" VARCHAR,
    "fill11" VARCHAR,
    "fill12" VARCHAR,
    "fill13" VARCHAR,
    "fill14" VARCHAR,
    "fill15" VARCHAR,
    "fill16" VARCHAR,
    "fill17" VARCHAR,
    "fill18" VARCHAR,
    "fill19" VARCHAR,
    "fill20" VARCHAR,
    "fill21" VARCHAR,
    "fill22" VARCHAR,
    "fill23" VARCHAR,
    "fill24" VARCHAR,
    "fill25" VARCHAR,
    "fill26" VARCHAR,
    "fill27" VARCHAR,
    "fill28" VARCHAR
);


-- Create a metadata table which queries can use to quickly determine interesting primary keys.
DROP TABLE IF EXISTS docs_schema_metadata CASCADE;
CREATE TABLE docs_schema_metadata (
  "name" TEXT PRIMARY KEY,
  "value" TEXT
);

INSERT INTO docs_schema_metadata
  ("name", "value")
VALUES
  -- The id of the minimum row in the pages table when sorted by id.
  ('pages-row-id-min', uuid_text(1)),
  -- The id of the middle row in the pages table when sorted by id.
  ('pages-row-id-median', uuid_text(floor(:rows / 2)::integer)),
  -- The id of the max row in the pages table when sorted by id.
  ('pages-row-id-max', uuid_text(:rows));


-- Insert data

INSERT INTO documents (
    "id",
    "parents",
    "content",
    "title",
    "createdAt",
    "fill0",
    "fill1",
    "fill2",
    "fill3",
    "fill4",
    "fill5",
    "fill6",
    "fill7",
    "fill8",
    "fill9",
    "fill10",
    "fill11",
    "fill12",
    "fill13",
    "fill14",
    "fill15",
    "fill16",
    "fill17",
    "fill18",
    "fill19",
    "fill20",
    "fill21",
    "fill22",
    "fill23",
    "fill24",
    "fill25",
    "fill26",
    "fill27",
    "fill28"
)
SELECT
    uuid_text(s.id) AS "id",
    CASE (s.id % 10) -- Deterministic distribution of parent prefixes
        WHEN 0 THEN 'SFR ' || substring(md5('documents:parents:sfr:' || s.id::text), 1, 20)
        WHEN 1 THEN 'PROJECT_ALPHA ' || substring(md5('documents:parents:alpha:' || s.id::text), 1, 15)
        ELSE 'PARENT_GROUP_' || ((s.id * 37) % 200)::INT || ' ' || substring(md5('documents:parents:group:' || s.id::text), 1, 10)
    END AS "parents",
    'Document Content Chunk 1: ' || md5('documents:content:a:' || s.id::text) || E'\nDocument Content Chunk 2: ' || md5('documents:content:b:' || s.id::text) || E'\nEnd of Document Content. ID: ' || ((s.id * 1543) % 100000)::INT AS "content",
    'Document Title ' || ((s.id * 97) % 50000)::INT || ' - ' || substring(md5('documents:title:' || s.id::text), 1, 25) AS "title",
    '2023-01-01 00:00:00'::TIMESTAMP + (((s.id * 7919) % 31535999)::text || ' seconds')::INTERVAL AS "createdAt",
    ((s.id * 101) % 20000000)::BIGINT AS "fill0",
    ((s.id * 103) % 20000000)::BIGINT AS "fill1",
    ((s.id * 107) % 20000000)::BIGINT AS "fill2",
    ((s.id * 109) % 20000000)::BIGINT AS "fill3",
    ((s.id * 113) % 20000000)::BIGINT AS "fill4",
    ((s.id * 127) % 20000000)::BIGINT AS "fill5",
    ((s.id * 131) % 20000000)::BIGINT AS "fill6",
    ((s.id * 137) % 20000000)::BIGINT AS "fill7",
    ((s.id * 139) % 20000000)::BIGINT AS "fill8",
    ((s.id * 149) % 20000000)::BIGINT AS "fill9",
    md5('documents:fill10:' || s.id::text) "fill10",
    md5('documents:fill11:' || s.id::text) "fill11",
    md5('documents:fill12:' || s.id::text) "fill12",
    md5('documents:fill13:' || s.id::text) "fill13",
    md5('documents:fill14:' || s.id::text) "fill14",
    md5('documents:fill15:' || s.id::text) "fill15",
    md5('documents:fill16:' || s.id::text) "fill16",
    md5('documents:fill17:' || s.id::text) "fill17",
    md5('documents:fill18:' || s.id::text) "fill18",
    md5('documents:fill19:' || s.id::text) "fill19",
    md5('documents:fill20:' || s.id::text) "fill20",
    md5('documents:fill21:' || s.id::text) "fill21",
    md5('documents:fill22:' || s.id::text) "fill22",
    md5('documents:fill23:' || s.id::text) "fill23",
    md5('documents:fill24:' || s.id::text) "fill24",
    md5('documents:fill25:' || s.id::text) "fill25",
    md5('documents:fill26:' || s.id::text) "fill26",
    md5('documents:fill27:' || s.id::text) "fill27",
    md5('documents:fill28:' || s.id::text) "fill28"
FROM generate_series(1, ceil(:rows / 8.0)::integer) s(id);


INSERT INTO files (
    "id",
    "documentId",
    "content",
    "title",
    "parents",
    "sizeInBytes",
    "createdAt",
    "fill0",
    "fill1",
    "fill2",
    "fill3",
    "fill4",
    "fill5",
    "fill6",
    "fill7",
    "fill8",
    "fill9",
    "fill10",
    "fill11",
    "fill12",
    "fill13",
    "fill14",
    "fill15",
    "fill16",
    "fill17",
    "fill18",
    "fill19",
    "fill20",
    "fill21",
    "fill22",
    "fill23",
    "fill24",
    "fill25",
    "fill26",
    "fill27",
    "fill28"
)
SELECT
    uuid_text(s.id) AS "id",
    uuid_text(1 + ((s.id * 17) % ceil(:rows / 8.0)::integer)) AS "documentId",
    'File Content Section A: ' || md5('files:content:a:' || s.id::text) || E'\nFile Content Section B (metadata): ' || md5('files:content:b:' || s.id::text) || E'\nAssociated ID: ' || ((s.id * 1877) % 100000)::INT AS "content",
    CASE (s.id % 10) -- Deterministic distribution of title prefixes
        WHEN 0 THEN 'collab12 ' || substring(md5('files:title:collab12:' || s.id::text), 1, 20)
        WHEN 1 THEN 'FY2025_BUDGET_DATA ' || substring(md5('files:title:budget:' || s.id::text), 1, 10)
        ELSE 'File Title ' || ((s.id * 89) % 50000)::INT || ' ' || substring(md5('files:title:default:' || s.id::text), 1, 22)
    END AS "title",
    'File Parent Identifier: ' || substring(md5('files:parents:' || s.id::text), 1, 18) AS "parents",
    ((s.id * 173) % 10000)::BIGINT AS "sizeInBytes",
    '2023-01-01 00:00:00'::TIMESTAMP + (((s.id * 7877) % 31535999)::text || ' seconds')::INTERVAL AS "createdAt",
    ((s.id * 151) % 20000000)::BIGINT AS "fill0",
    ((s.id * 157) % 20000000)::BIGINT AS "fill1",
    ((s.id * 163) % 20000000)::BIGINT AS "fill2",
    ((s.id * 167) % 20000000)::BIGINT AS "fill3",
    ((s.id * 173) % 20000000)::BIGINT AS "fill4",
    ((s.id * 179) % 20000000)::BIGINT AS "fill5",
    ((s.id * 181) % 20000000)::BIGINT AS "fill6",
    ((s.id * 191) % 20000000)::BIGINT AS "fill7",
    ((s.id * 193) % 20000000)::BIGINT AS "fill8",
    ((s.id * 197) % 20000000)::BIGINT AS "fill9",
    md5('files:fill10:' || s.id::text) "fill10",
    md5('files:fill11:' || s.id::text) "fill11",
    md5('files:fill12:' || s.id::text) "fill12",
    md5('files:fill13:' || s.id::text) "fill13",
    md5('files:fill14:' || s.id::text) "fill14",
    md5('files:fill15:' || s.id::text) "fill15",
    md5('files:fill16:' || s.id::text) "fill16",
    md5('files:fill17:' || s.id::text) "fill17",
    md5('files:fill18:' || s.id::text) "fill18",
    md5('files:fill19:' || s.id::text) "fill19",
    md5('files:fill20:' || s.id::text) "fill20",
    md5('files:fill21:' || s.id::text) "fill21",
    md5('files:fill22:' || s.id::text) "fill22",
    md5('files:fill23:' || s.id::text) "fill23",
    md5('files:fill24:' || s.id::text) "fill24",
    md5('files:fill25:' || s.id::text) "fill25",
    md5('files:fill26:' || s.id::text) "fill26",
    md5('files:fill27:' || s.id::text) "fill27",
    md5('files:fill28:' || s.id::text) "fill28"
FROM generate_series(1, ceil(:rows / 8.0)::integer) s(id);


INSERT INTO pages (
    "id",
    "fileId",
    "content",
    "title",
    "parents",
    "sizeInBytes",
    "createdAt",
    "fill0",
    "fill1",
    "fill2",
    "fill3",
    "fill4",
    "fill5",
    "fill6",
    "fill7",
    "fill8",
    "fill9",
    "fill10",
    "fill11",
    "fill12",
    "fill13",
    "fill14",
    "fill15",
    "fill16",
    "fill17",
    "fill18",
    "fill19",
    "fill20",
    "fill21",
    "fill22",
    "fill23",
    "fill24",
    "fill25",
    "fill26",
    "fill27",
    "fill28"
)
SELECT
    uuid_text(s.id) AS "id",
    uuid_text(1 + ((s.id * 19) % ceil(:rows / 8.0)::integer)) AS "fileId",
    CASE (s.id % 10) -- Deterministic distribution of content variants
        WHEN 0 THEN 'Single Number Reach configuration details: ' || md5('pages:content:snr:a:' || s.id::text) || E'\nInstructions for setup: ' || md5('pages:content:snr:b:' || s.id::text)
        WHEN 1 THEN 'Page Chapter 1: Introduction - ' || md5('pages:content:intro:a:' || s.id::text) || E'\nKey Points: ' || md5('pages:content:intro:b:' || s.id::text)
        ELSE 'Page Content block Alpha: ' || md5('pages:content:alpha:' || s.id::text) || E'\nPage Content block Beta: ' || md5('pages:content:beta:' || s.id::text) || E'\nPage Content block Gamma. Ref ID: ' || ((s.id * 1999) % 100000)::INT
    END AS "content",
    'Page Title ' || ((s.id * 83) % 50000)::INT || ' - Section ' || substring(md5('pages:title:' || s.id::text), 1, 15) AS "title",
    'Page Parent Reference: ' || substring(md5('pages:parents:' || s.id::text), 1, 20) AS "parents",
    ((s.id * 223) % 10000)::BIGINT AS "sizeInBytes",
    '2023-01-01 00:00:00'::TIMESTAMP + (((s.id * 7759) % 31535999)::text || ' seconds')::INTERVAL AS "createdAt",
    ((s.id * 211) % 20000000)::BIGINT AS "fill0",
    ((s.id * 223) % 20000000)::BIGINT AS "fill1",
    ((s.id * 227) % 20000000)::BIGINT AS "fill2",
    ((s.id * 229) % 20000000)::BIGINT AS "fill3",
    ((s.id * 233) % 20000000)::BIGINT AS "fill4",
    ((s.id * 239) % 20000000)::BIGINT AS "fill5",
    ((s.id * 241) % 20000000)::BIGINT AS "fill6",
    ((s.id * 251) % 20000000)::BIGINT AS "fill7",
    ((s.id * 257) % 20000000)::BIGINT AS "fill8",
    ((s.id * 263) % 20000000)::BIGINT AS "fill9",
    md5('pages:fill10:' || s.id::text) "fill10",
    md5('pages:fill11:' || s.id::text) "fill11",
    md5('pages:fill12:' || s.id::text) "fill12",
    md5('pages:fill13:' || s.id::text) "fill13",
    md5('pages:fill14:' || s.id::text) "fill14",
    md5('pages:fill15:' || s.id::text) "fill15",
    md5('pages:fill16:' || s.id::text) "fill16",
    md5('pages:fill17:' || s.id::text) "fill17",
    md5('pages:fill18:' || s.id::text) "fill18",
    md5('pages:fill19:' || s.id::text) "fill19",
    md5('pages:fill20:' || s.id::text) "fill20",
    md5('pages:fill21:' || s.id::text) "fill21",
    md5('pages:fill22:' || s.id::text) "fill22",
    md5('pages:fill23:' || s.id::text) "fill23",
    md5('pages:fill24:' || s.id::text) "fill24",
    md5('pages:fill25:' || s.id::text) "fill25",
    md5('pages:fill26:' || s.id::text) "fill26",
    md5('pages:fill27:' || s.id::text) "fill27",
    md5('pages:fill28:' || s.id::text) "fill28"
FROM generate_series(1, :rows) s(id);

CREATE INDEX pages_index ON pages
USING bm25 (
    "id",
    "content",
    "title",
    "parents",
    "fileId",
    "sizeInBytes",
    "createdAt"
)
WITH (
    key_field = 'id',
    text_fields = '{
        "fileId": {
            "tokenizer": {"type": "keyword"}, "fast": true
        },
        "content": {
            "tokenizer": {"type": "default"}, "fast": true
        },
        "title": {
            "tokenizer": {"type": "default"}, "fast": true
        },
        "parents": {
            "tokenizer": {"type": "default"}, "fast": true
        }
    }'
);

CREATE INDEX files_index ON files
USING bm25 (
    "id",
    "content",
    "documentId",
    "title",
    "parents",
    "sizeInBytes",
    "createdAt"
)
WITH (
    key_field = 'id',
    text_fields = '{
        "documentId": {
            "tokenizer": {"type": "keyword"}, "fast": true
        },
        "content": {
            "tokenizer": {"type": "default"}, "fast": true
        },
        "title": {
            "tokenizer": {"type": "default"}, "fast": true
        },
        "parents": {
            "tokenizer": {"type": "default"}, "fast": true
        }
    }'
);

CREATE INDEX documents_index ON documents
USING bm25 (
    "id",
    "content",
    "title",
    "parents",
    "createdAt"
)
WITH (
    key_field = 'id',
    text_fields = '{
        "content": {
            "tokenizer": {"type": "default"}, "fast": true
        },
        "title": {
            "tokenizer": {"type": "default"}, "fast": true
        },
        "parents": {
            "tokenizer": {"type": "default"}, "fast": true
        }
    }'
);
