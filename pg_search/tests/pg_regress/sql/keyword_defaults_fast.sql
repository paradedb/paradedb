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

CREATE INDEX t_idx ON t USING paradedb
(id, (description::pdb.literal), org_id);

SELECT * FROM paradedb.schema('t_idx');
DROP TABLE t;

-- Verify JSON reloptions defaults for single-token tokenizers and range fields
CREATE TABLE t_json (
    id SERIAL PRIMARY KEY,
    kw_text TEXT,
    raw_text TEXT,
    lit_norm_text TEXT,
    kw_json JSONB,
    range_col INT4RANGE
);

CREATE INDEX t_json_idx ON t_json USING paradedb
(id, kw_text, raw_text, lit_norm_text, kw_json, range_col)
WITH (
    text_fields = '{
        "kw_text": {"tokenizer": {"type": "keyword"}},
        "raw_text": {"tokenizer": {"type": "raw"}},
        "lit_norm_text": {"tokenizer": {"type": "literal_normalized"}}
    }',
    json_fields = '{
        "kw_json": {"tokenizer": {"type": "keyword"}}
    }',
    range_fields = '{
        "range_col": {}
    }'
);

SELECT * FROM paradedb.schema('t_json_idx') ORDER BY name;
DROP TABLE t_json;

