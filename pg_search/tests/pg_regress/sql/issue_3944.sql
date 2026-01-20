\i common/common_setup.sql

DROP TABLE IF EXISTS alias_json;
CREATE TABLE alias_json (
    id SERIAL PRIMARY KEY,
    metadata JSONB
);

INSERT INTO alias_json (metadata) VALUES
    ('{"name": "red apple", "tags": ["test", "jsonb"]}'),
    ('{"name": "blueberry", "tags": ["test", "jsonb"]}');

CREATE INDEX alias_idx ON alias_json USING bm25 (id, metadata, (metadata::pdb.simple('alias=metadata_simple'))) WITH (key_field = 'id');

SELECT * FROM paradedb.schema('alias_idx');
SELECT * FROM alias_json WHERE id @@@ pdb.parse('metadata_simple.name:red');

DROP TABLE alias_json;
