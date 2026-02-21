-- Create extension
CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;

-- Disable mutable segments to ensure multiple segments are created
SET paradedb.global_mutable_segment_rows = 0;

-- Set seed for determinism
SELECT setseed(0.5);

-- Create tables
CREATE TABLE pages (
    "id" INTEGER PRIMARY KEY,
    "fileId" INTEGER,
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

CREATE TABLE files (
    "id" INTEGER PRIMARY KEY,
    "documentId" INTEGER,
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

CREATE TABLE documents (
    "id" INTEGER PRIMARY KEY,
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

-- Create indexes BEFORE inserting data to force segment creation per batch
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
        "content": {
            "tokenizer": {"type": "default"}, "fast": true
        },
        "title": {
            "tokenizer": {"type": "default"}, "fast": true
        },
        "parents": {
            "tokenizer": {"type": "default"}, "fast": true
        }
    }',
    numeric_fields = '{
        "fileId": {"fast": true}
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
    sort_by = 'documentId ASC NULLS FIRST',
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
    }',
    numeric_fields = '{
        "documentId": {"fast": true}
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
    sort_by = 'id ASC NULLS FIRST',
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

-- Insert data in batches to create multiple segments
-- We'll insert 2000 rows in 4 batches of 500

-- Batch 1: 1-500
INSERT INTO documents (
    "id", "parents", "content", "title", "createdAt",
    "fill0", "fill1", "fill2", "fill3", "fill4", "fill5", "fill6", "fill7", "fill8", "fill9",
    "fill10", "fill11", "fill12", "fill13", "fill14", "fill15", "fill16", "fill17", "fill18", "fill19", "fill20", "fill21", "fill22", "fill23", "fill24", "fill25", "fill26", "fill27", "fill28"
)
SELECT
    s.id AS "id",
    CASE (random() * 10)::INT
        WHEN 0 THEN 'SFR ' || substring(md5(random()::text), 1, 20)
        WHEN 1 THEN 'PROJECT_ALPHA ' || substring(md5(random()::text), 1, 15)
        ELSE 'PARENT_GROUP_' || (random()*200)::INT || ' ' || substring(md5(random()::text), 1, 10)
    END AS "parents",
    'Content ' || md5(random()::text) AS "content",
    'Document Title ' || (random()*50000)::INT AS "title",
    '2023-01-01 00:00:00'::TIMESTAMP + (s.id * INTERVAL '1 second') AS "createdAt",
    (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT,
    (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT,
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text),
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text),
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text),
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text)
FROM generate_series(1, 500) s(id);

INSERT INTO files (
    "id", "documentId", "content", "title", "parents", "sizeInBytes", "createdAt",
    "fill0", "fill1", "fill2", "fill3", "fill4", "fill5", "fill6", "fill7", "fill8", "fill9",
    "fill10", "fill11", "fill12", "fill13", "fill14", "fill15", "fill16", "fill17", "fill18", "fill19", "fill20", "fill21", "fill22", "fill23", "fill24", "fill25", "fill26", "fill27", "fill28"
)
SELECT
    s.id AS "id",
    ceil(random() * 2000)::integer AS "documentId",
    'Content ' || md5(random()::text) AS "content",
    'File Title ' || (random()*50000)::INT AS "title",
    'Parent ' || substring(md5(random()::text), 1, 18) AS "parents",
    (random()*10000)::BIGINT AS "sizeInBytes",
    '2023-01-01 00:00:00'::TIMESTAMP + (s.id * INTERVAL '1 second') AS "createdAt",
    (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT,
    (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT,
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text),
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text),
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text),
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text)
FROM generate_series(1, 500) s(id);

-- Batch 2: 501-1000
INSERT INTO documents (
    "id", "parents", "content", "title", "createdAt",
    "fill0", "fill1", "fill2", "fill3", "fill4", "fill5", "fill6", "fill7", "fill8", "fill9",
    "fill10", "fill11", "fill12", "fill13", "fill14", "fill15", "fill16", "fill17", "fill18", "fill19", "fill20", "fill21", "fill22", "fill23", "fill24", "fill25", "fill26", "fill27", "fill28"
)
SELECT
    s.id AS "id",
    CASE (random() * 10)::INT
        WHEN 0 THEN 'SFR ' || substring(md5(random()::text), 1, 20)
        WHEN 1 THEN 'PROJECT_ALPHA ' || substring(md5(random()::text), 1, 15)
        ELSE 'PARENT_GROUP_' || (random()*200)::INT || ' ' || substring(md5(random()::text), 1, 10)
    END AS "parents",
    'Content ' || md5(random()::text) AS "content",
    'Document Title ' || (random()*50000)::INT AS "title",
    '2023-01-01 00:00:00'::TIMESTAMP + (s.id * INTERVAL '1 second') AS "createdAt",
    (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT,
    (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT,
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text),
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text),
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text),
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text)
FROM generate_series(501, 1000) s(id);

INSERT INTO files (
    "id", "documentId", "content", "title", "parents", "sizeInBytes", "createdAt",
    "fill0", "fill1", "fill2", "fill3", "fill4", "fill5", "fill6", "fill7", "fill8", "fill9",
    "fill10", "fill11", "fill12", "fill13", "fill14", "fill15", "fill16", "fill17", "fill18", "fill19", "fill20", "fill21", "fill22", "fill23", "fill24", "fill25", "fill26", "fill27", "fill28"
)
SELECT
    s.id AS "id",
    ceil(random() * 2000)::integer AS "documentId",
    'Content ' || md5(random()::text) AS "content",
    'File Title ' || (random()*50000)::INT AS "title",
    'Parent ' || substring(md5(random()::text), 1, 18) AS "parents",
    (random()*10000)::BIGINT AS "sizeInBytes",
    '2023-01-01 00:00:00'::TIMESTAMP + (s.id * INTERVAL '1 second') AS "createdAt",
    (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT,
    (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT,
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text),
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text),
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text),
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text)
FROM generate_series(501, 1000) s(id);

-- Batch 3: 1001-1500
INSERT INTO documents (
    "id", "parents", "content", "title", "createdAt",
    "fill0", "fill1", "fill2", "fill3", "fill4", "fill5", "fill6", "fill7", "fill8", "fill9",
    "fill10", "fill11", "fill12", "fill13", "fill14", "fill15", "fill16", "fill17", "fill18", "fill19", "fill20", "fill21", "fill22", "fill23", "fill24", "fill25", "fill26", "fill27", "fill28"
)
SELECT
    s.id AS "id",
    CASE (random() * 10)::INT
        WHEN 0 THEN 'SFR ' || substring(md5(random()::text), 1, 20)
        WHEN 1 THEN 'PROJECT_ALPHA ' || substring(md5(random()::text), 1, 15)
        ELSE 'PARENT_GROUP_' || (random()*200)::INT || ' ' || substring(md5(random()::text), 1, 10)
    END AS "parents",
    'Content ' || md5(random()::text) AS "content",
    'Document Title ' || (random()*50000)::INT AS "title",
    '2023-01-01 00:00:00'::TIMESTAMP + (s.id * INTERVAL '1 second') AS "createdAt",
    (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT,
    (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT,
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text),
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text),
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text),
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text)
FROM generate_series(1001, 1500) s(id);

INSERT INTO files (
    "id", "documentId", "content", "title", "parents", "sizeInBytes", "createdAt",
    "fill0", "fill1", "fill2", "fill3", "fill4", "fill5", "fill6", "fill7", "fill8", "fill9",
    "fill10", "fill11", "fill12", "fill13", "fill14", "fill15", "fill16", "fill17", "fill18", "fill19", "fill20", "fill21", "fill22", "fill23", "fill24", "fill25", "fill26", "fill27", "fill28"
)
SELECT
    s.id AS "id",
    ceil(random() * 2000)::integer AS "documentId",
    'Content ' || md5(random()::text) AS "content",
    'File Title ' || (random()*50000)::INT AS "title",
    'Parent ' || substring(md5(random()::text), 1, 18) AS "parents",
    (random()*10000)::BIGINT AS "sizeInBytes",
    '2023-01-01 00:00:00'::TIMESTAMP + (s.id * INTERVAL '1 second') AS "createdAt",
    (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT,
    (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT,
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text),
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text),
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text),
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text)
FROM generate_series(1001, 1500) s(id);

-- Batch 4: 1501-2000
INSERT INTO documents (
    "id", "parents", "content", "title", "createdAt",
    "fill0", "fill1", "fill2", "fill3", "fill4", "fill5", "fill6", "fill7", "fill8", "fill9",
    "fill10", "fill11", "fill12", "fill13", "fill14", "fill15", "fill16", "fill17", "fill18", "fill19", "fill20", "fill21", "fill22", "fill23", "fill24", "fill25", "fill26", "fill27", "fill28"
)
SELECT
    s.id AS "id",
    CASE (random() * 10)::INT
        WHEN 0 THEN 'SFR ' || substring(md5(random()::text), 1, 20)
        WHEN 1 THEN 'PROJECT_ALPHA ' || substring(md5(random()::text), 1, 15)
        ELSE 'PARENT_GROUP_' || (random()*200)::INT || ' ' || substring(md5(random()::text), 1, 10)
    END AS "parents",
    'Content ' || md5(random()::text) AS "content",
    'Document Title ' || (random()*50000)::INT AS "title",
    '2023-01-01 00:00:00'::TIMESTAMP + (s.id * INTERVAL '1 second') AS "createdAt",
    (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT,
    (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT,
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text),
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text),
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text),
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text)
FROM generate_series(1501, 2000) s(id);

INSERT INTO files (
    "id", "documentId", "content", "title", "parents", "sizeInBytes", "createdAt",
    "fill0", "fill1", "fill2", "fill3", "fill4", "fill5", "fill6", "fill7", "fill8", "fill9",
    "fill10", "fill11", "fill12", "fill13", "fill14", "fill15", "fill16", "fill17", "fill18", "fill19", "fill20", "fill21", "fill22", "fill23", "fill24", "fill25", "fill26", "fill27", "fill28"
)
SELECT
    s.id AS "id",
    ceil(random() * 2000)::integer AS "documentId",
    'Content ' || md5(random()::text) AS "content",
    'File Title ' || (random()*50000)::INT AS "title",
    'Parent ' || substring(md5(random()::text), 1, 18) AS "parents",
    (random()*10000)::BIGINT AS "sizeInBytes",
    '2023-01-01 00:00:00'::TIMESTAMP + (s.id * INTERVAL '1 second') AS "createdAt",
    (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT,
    (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT, (random()*20000000)::BIGINT,
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text),
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text),
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text),
    md5(random()::text), md5(random()::text), md5(random()::text), md5(random()::text)
FROM generate_series(1501, 2000) s(id);

-- Freeze tables to ensure visibility
VACUUM FREEZE documents;
VACUUM FREEZE files;
VACUUM FREEZE pages;

VACUUM ANALYZE;

-- Query
SET work_mem TO '4GB'; 
SET paradedb.enable_join_custom_scan TO on; 

EXPLAIN (COSTS OFF)
SELECT
    f.id,
    f.title,
    f."createdAt"
FROM files f
WHERE
    -- The "Join" is a filter against a list of IDs (Semi-Join)
    f."documentId" IN (
        SELECT id
        FROM documents
        WHERE parents @@@ 'PROJECT_ALPHA'
        AND title @@@ 'Document Title'
    )
ORDER BY
    f.title ASC
LIMIT 25;

SELECT
    f.id,
    f.title,
    f."createdAt"
FROM files f
WHERE
    -- The "Join" is a filter against a list of IDs (Semi-Join)
    f."documentId" IN (
        SELECT id
        FROM documents
        WHERE parents @@@ 'PROJECT_ALPHA'
        AND title @@@ 'Document Title'
    )
ORDER BY
    f.title ASC
LIMIT 25;

-- Reset configuration
RESET paradedb.global_mutable_segment_rows;

DROP TABLE IF EXISTS pages CASCADE;
DROP TABLE IF EXISTS files CASCADE;
DROP TABLE IF EXISTS documents CASCADE;
