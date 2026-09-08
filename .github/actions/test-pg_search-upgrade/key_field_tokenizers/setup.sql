CREATE TABLE text_key (id TEXT PRIMARY KEY, body TEXT);
INSERT INTO text_key VALUES ('Mixed Case-ID', 'example');
CREATE INDEX text_key_idx ON text_key USING bm25 (id, body) WITH (key_field = 'id');

CREATE TABLE uuid_key (id UUID PRIMARY KEY, body TEXT);
INSERT INTO uuid_key VALUES ('550e8400-e29b-41d4-a716-446655440000', 'example');
CREATE INDEX uuid_key_idx ON uuid_key USING bm25 (id, body) WITH (key_field = 'id');
