\i common/common_setup.sql

DROP TABLE IF EXISTS t;
CREATE TABLE t (
    id SERIAL PRIMARY KEY,
    description TEXT,
    org_id UUID
);

INSERT INTO t (description, org_id) VALUES
    ('banana', '123e4567-e89b-12d3-a456-426614174000'),
    ('banana', '123e4567-e89b-12d3-a456-426614174001'),
    ('banana', '123e4567-e89b-12d3-a456-426614174002'),
    ('banana', '123e4567-e89b-12d3-a456-426614174003'),
    ('banana', '123e4567-e89b-12d3-a456-426614174004');

CREATE INDEX t_idx ON t USING bm25
(id, description, org_id) WITH (key_field='id', text_fields='{"description": {"tokenizer": {"type": "keyword"}}}');

SELECT * FROM paradedb.schema('t_idx');
DROP TABLE t;
