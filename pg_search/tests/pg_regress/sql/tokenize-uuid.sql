\i common/common_setup.sql

CREATE TABLE test_table (
    id SERIAL PRIMARY KEY,
    uuid UUID
);

INSERT INTO test_table (uuid) VALUES
    ('123e4567-e89b-12d3-a456-426614174000'),
    ('987fcdeb-51a2-43e8-b567-890123456789'),
    ('a1b2c3d4-e5f6-47a8-89b0-123456789abc'),
    ('b2c3d4e5-f6a7-48b9-90c1-23456789abcd'),
    ('c3d4e5f6-a7b8-49c0-01d2-3456789abcde');


-- verify default is literal
CREATE INDEX idx_test_table_uuid ON test_table USING bm25 (id, uuid) WITH (key_field='id');
SELECT * FROM paradedb.schema('idx_test_table_uuid');
DROP INDEX idx_test_table_uuid;

-- use unicode
CREATE INDEX idx_test_table_uuid ON test_table USING bm25 (id, (uuid::pdb.unicode_words)) WITH (key_field='id');
SELECT * FROM paradedb.schema('idx_test_table_uuid');
DROP INDEX idx_test_table_uuid;

-- use alias
CREATE INDEX idx_test_table_uuid ON test_table USING bm25 (id, (uuid::pdb.unicode_words('alias=uuid_words'))) WITH (key_field='id');
SELECT * FROM paradedb.schema('idx_test_table_uuid');
DROP INDEX idx_test_table_uuid;

DROP TABLE test_table;
