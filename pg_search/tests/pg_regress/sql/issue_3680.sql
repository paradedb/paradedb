\i common/common_setup.sql

CREATE TABLE data_docstore (
    id SERIAL PRIMARY KEY,
    doc_text VARCHAR
);

CREATE INDEX data_docstore_text_search_idx ON data_docstore
USING bm25 (id, doc_text)
WITH (key_field=id);

INSERT INTO data_docstore (doc_text)
VALUES (repeat('BigData_ ', 200000));

SELECT id
FROM data_docstore
WHERE doc_text ||| 'BigData_';

UPDATE data_docstore SET doc_text = repeat('BigData_ ', 200000) WHERE id = 1;
SELECT mutable, num_docs FROM paradedb.index_info('data_docstore_text_search_idx');

SELECT id
FROM data_docstore
WHERE doc_text ||| 'BigData_';

DROP TABLE data_docstore;
