\set rows :rows
\echo 'Generating' :rows 'rows in a facts table, and' :rows '/ 8 rows in two dimension tables.'


-- Create tables

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
    gen_random_uuid()::text AS "id",
    CASE (random() * 10)::INT -- Introduce 'SFR' in roughly 10% of rows
        WHEN 0 THEN 'SFR ' || substring(md5(random()::text), 1, 20)
        WHEN 1 THEN 'PROJECT_ALPHA ' || substring(md5(random()::text), 1, 15)
        ELSE 'PARENT_GROUP_' || (random()*200)::INT || ' ' || substring(md5(random()::text), 1, 10)
    END AS "parents",
    'Document Content Chunk 1: ' || md5(random()::text) || E'\nDocument Content Chunk 2: ' || md5(random()::text) || E'\nEnd of Document Content. ID: ' || (random()*100000)::INT AS "content",
    'Document Title ' || (random()*50000)::INT || ' - ' || substring(md5(random()::text), 1, 25) AS "title",
    '2023-01-01 00:00:00'::TIMESTAMP + random() * ('2023-12-31 23:59:59'::TIMESTAMP - '2023-01-01 00:00:00'::TIMESTAMP) AS "createdAt",
    (random()*20000000)::BIGINT AS "fill0",
    (random()*20000000)::BIGINT AS "fill1",
    (random()*20000000)::BIGINT AS "fill2",
    (random()*20000000)::BIGINT AS "fill3",
    (random()*20000000)::BIGINT AS "fill4",
    (random()*20000000)::BIGINT AS "fill5",
    (random()*20000000)::BIGINT AS "fill6",
    (random()*20000000)::BIGINT AS "fill7",
    (random()*20000000)::BIGINT AS "fill8",
    (random()*20000000)::BIGINT AS "fill9",
    md5(random()::text) "fill10",
    md5(random()::text) "fill11",
    md5(random()::text) "fill12",
    md5(random()::text) "fill13",
    md5(random()::text) "fill14",
    md5(random()::text) "fill15",
    md5(random()::text) "fill16",
    md5(random()::text) "fill17",
    md5(random()::text) "fill18",
    md5(random()::text) "fill19",
    md5(random()::text) "fill20",
    md5(random()::text) "fill21",
    md5(random()::text) "fill22",
    md5(random()::text) "fill23",
    md5(random()::text) "fill24",
    md5(random()::text) "fill25",
    md5(random()::text) "fill26",
    md5(random()::text) "fill27",
    md5(random()::text) "fill28"
FROM generate_series(1, ceil(:rows / 8.0)::integer);


WITH document_id_list AS (
    -- Select all document IDs and assign a unique row number after randomizing their order
    SELECT "id", row_number() OVER (ORDER BY random()) AS rn
    FROM documents
),
document_id_count AS (
    -- Get the total count of available document IDs
    SELECT count(*) AS total_docs FROM document_id_list
),
file_generation_series AS (
    SELECT
        generate_series(1, ceil(:rows / 8.0)::integer) AS series_idx,
        gen_random_uuid()::text AS new_file_id,
        -- This will be a random number between 1 and total_docs (inclusive)
        floor(random() * (SELECT total_docs FROM document_id_count) + 1)::bigint AS random_doc_rn
)
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
    fgs.new_file_id AS "id",
    dil."id" AS "documentId", -- Join to get the actual document ID based on the random row number
    'File Content Section A: ' || md5(random()::text) || E'\nFile Content Section B (metadata): ' || md5(random()::text) || E'\nAssociated ID: ' || (random()*100000)::INT AS "content",
    CASE (random() * 10)::INT -- Introduce 'collab12' in roughly 10% of rows
        WHEN 0 THEN 'collab12 ' || substring(md5(random()::text), 1, 20)
        WHEN 1 THEN 'FY2025_BUDGET_DATA ' || substring(md5(random()::text), 1, 10)
        ELSE 'File Title ' || (random()*50000)::INT || ' ' || substring(md5(random()::text), 1, 22)
    END AS "title",
    'File Parent Identifier: ' || substring(md5(random()::text), 1, 18) AS "parents",
    (random()*10000)::BIGINT AS "sizeInBytes",
    '2023-01-01 00:00:00'::TIMESTAMP + random() * ('2023-12-31 23:59:59'::TIMESTAMP - '2023-01-01 00:00:00'::TIMESTAMP) AS "createdAt",
    (random()*20000000)::BIGINT AS "fill0",
    (random()*20000000)::BIGINT AS "fill1",
    (random()*20000000)::BIGINT AS "fill2",
    (random()*20000000)::BIGINT AS "fill3",
    (random()*20000000)::BIGINT AS "fill4",
    (random()*20000000)::BIGINT AS "fill5",
    (random()*20000000)::BIGINT AS "fill6",
    (random()*20000000)::BIGINT AS "fill7",
    (random()*20000000)::BIGINT AS "fill8",
    (random()*20000000)::BIGINT AS "fill9",
    md5(random()::text) "fill10",
    md5(random()::text) "fill11",
    md5(random()::text) "fill12",
    md5(random()::text) "fill13",
    md5(random()::text) "fill14",
    md5(random()::text) "fill15",
    md5(random()::text) "fill16",
    md5(random()::text) "fill17",
    md5(random()::text) "fill18",
    md5(random()::text) "fill19",
    md5(random()::text) "fill20",
    md5(random()::text) "fill21",
    md5(random()::text) "fill22",
    md5(random()::text) "fill23",
    md5(random()::text) "fill24",
    md5(random()::text) "fill25",
    md5(random()::text) "fill26",
    md5(random()::text) "fill27",
    md5(random()::text) "fill28"
FROM file_generation_series fgs
JOIN document_id_list dil ON fgs.random_doc_rn = dil.rn;


WITH file_id_list AS (
    -- Select all file IDs and assign a unique row number after randomizing their order
    SELECT "id", row_number() OVER (ORDER BY random()) AS rn
    FROM files
),
file_id_count AS (
    -- Get the total count of available file IDs
    SELECT count(*) AS total_files FROM file_id_list
),
page_generation_series AS (
    SELECT
        generate_series(1, :rows) AS series_idx,
        gen_random_uuid()::text AS new_page_id,
        -- This will be a random number between 1 and total_files (inclusive)
        floor(random() * (SELECT total_files FROM file_id_count) + 1)::bigint AS random_file_rn
)
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
    pgs.new_page_id AS "id",
    fil."id" AS "fileId", -- Join to get the actual file ID based on the random row number
    CASE (random() * 10)::INT -- Introduce 'Single Number Reach' in roughly 10% of rows
        WHEN 0 THEN 'Single Number Reach configuration details: ' || md5(random()::text) || E'\nInstructions for setup: ' || md5(random()::text)
        WHEN 1 THEN 'Page Chapter 1: Introduction - ' || md5(random()::text) || E'\nKey Points: ' || md5(random()::text)
        ELSE 'Page Content block Alpha: ' || md5(random()::text) || E'\nPage Content block Beta: ' || md5(random()::text) || E'\nPage Content block Gamma. Ref ID: ' || (random()*100000)::INT
    END AS "content",
    'Page Title ' || (random()*50000)::INT || ' - Section ' || substring(md5(random()::text), 1, 15) AS "title",
    'Page Parent Reference: ' || substring(md5(random()::text), 1, 20) AS "parents",
    (random()*10000)::BIGINT AS "sizeInBytes",
    '2023-01-01 00:00:00'::TIMESTAMP + random() * ('2023-12-31 23:59:59'::TIMESTAMP - '2023-01-01 00:00:00'::TIMESTAMP) AS "createdAt",
    (random()*20000000)::BIGINT AS "fill0",
    (random()*20000000)::BIGINT AS "fill1",
    (random()*20000000)::BIGINT AS "fill2",
    (random()*20000000)::BIGINT AS "fill3",
    (random()*20000000)::BIGINT AS "fill4",
    (random()*20000000)::BIGINT AS "fill5",
    (random()*20000000)::BIGINT AS "fill6",
    (random()*20000000)::BIGINT AS "fill7",
    (random()*20000000)::BIGINT AS "fill8",
    (random()*20000000)::BIGINT AS "fill9",
    md5(random()::text) "fill10",
    md5(random()::text) "fill11",
    md5(random()::text) "fill12",
    md5(random()::text) "fill13",
    md5(random()::text) "fill14",
    md5(random()::text) "fill15",
    md5(random()::text) "fill16",
    md5(random()::text) "fill17",
    md5(random()::text) "fill18",
    md5(random()::text) "fill19",
    md5(random()::text) "fill20",
    md5(random()::text) "fill21",
    md5(random()::text) "fill22",
    md5(random()::text) "fill23",
    md5(random()::text) "fill24",
    md5(random()::text) "fill25",
    md5(random()::text) "fill26",
    md5(random()::text) "fill27",
    md5(random()::text) "fill28"
FROM page_generation_series pgs
JOIN file_id_list fil ON pgs.random_file_rn = fil.rn;
