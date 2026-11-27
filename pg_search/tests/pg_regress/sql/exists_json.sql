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

CREATE INDEX idx_exists_json_data ON exists_json USING paradedb (id, description, (data::pdb.unicode_words('columnar=true')));

SELECT * FROM exists_json WHERE (data -> 'first_name') @@@ pdb.exists();
SELECT * FROM exists_json WHERE (data -> 'last_name') @@@ pdb.exists() OR description ||| 'CEO';
SELECT * FROM exists_json WHERE data @@@ pdb.exists();

DROP TABLE IF EXISTS exists_json;
