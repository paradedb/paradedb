DROP TABLE IF EXISTS exists_json;
CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE exists_json (
    id SERIAL PRIMARY KEY,
    description TEXT,
    data JSONB
);

INSERT INTO exists_json (description, data) VALUES ('Marketing manager', '{"first_name": "John", "last_name": "Smith"}');
INSERT INTO exists_json (description, data) VALUES ('Sales manager', '{"first_name": "Jane"}');
INSERT INTO exists_json (description, data) VALUES ('Engineer', '{"last_name": "Wilson"}');
INSERT INTO exists_json (description, data) VALUES ('CEO', NULL);
INSERT INTO exists_json (description, data) VALUES ('CTO', '{"first_name": "Jim", "last_name": "Johnson"}');

CREATE INDEX idx_exists_json_data ON exists_json USING bm25 (id, description, data)
WITH (key_field = 'id', json_fields = '{"data": {"fast": true}}');

SELECT * FROM exists_json WHERE id @@@ paradedb.exists('data.first_name');
SELECT * FROM exists_json WHERE id @@@ paradedb.exists('data.last_name') OR description @@@ 'CEO';
SELECT * FROM exists_json WHERE id @@@ paradedb.exists('data');

DROP TABLE IF EXISTS exists_json;
