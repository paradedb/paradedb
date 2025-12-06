CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS data_docstore CASCADE;
CREATE TABLE data_docstore (
    id SERIAL PRIMARY KEY,
    doc_text VARCHAR
);

CREATE INDEX data_docstore_text_search_idx ON data_docstore 
USING bm25 (id, doc_text) 
WITH (key_field=id);

INSERT INTO data_docstore (doc_text)
VALUES (repeat('BigData_ ', 200000));

-- Query immediately after insert.
SELECT id, doc_text
FROM data_docstore
WHERE doc_text @@@ 'BigData_'
LIMIT 10;

UPDATE data_docstore SET doc_text = repeat('BigData_ ', 200000) WHERE id = 1;

-- And again after an update.
SELECT id, doc_text
FROM data_docstore
WHERE doc_text @@@ 'BigData_'
LIMIT 10;

VACUUM data_docstore;

-- And again once the old tuple version has been VACUUM'd away.
SELECT id, doc_text
FROM data_docstore
WHERE doc_text @@@ 'BigData_'
LIMIT 10;
