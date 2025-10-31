CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS snippet_test;
CREATE TABLE snippet_test (
    id SERIAL PRIMARY KEY,
    content TEXT
);

INSERT INTO snippet_test (content) VALUES
('This is a test test of the snippet function with multiple test words'),
('Another test of the snippet snippet function with repeated snippet words'),
('Yet another test test test of the function function function'),
('test Lorem ipsum dolor sit amet...test');

CREATE INDEX ON snippet_test USING bm25 (
    id,
    content
) WITH (
    key_field = 'id'
);

SELECT paradedb.snippet(content), paradedb.snippet_positions(content) FROM snippet_test WHERE content @@@ 'test';
SELECT paradedb.snippet(content, "limit" => 1), paradedb.snippet_positions(content, "limit" => 1) FROM snippet_test WHERE content @@@ 'test';
SELECT paradedb.snippet(content, "limit" => 1, "offset" => 1), paradedb.snippet_positions(content, "limit" => 1, "offset" => 1) FROM snippet_test WHERE content @@@ 'test';
SELECT paradedb.snippet(content, "limit" => 5, "offset" => 2), paradedb.snippet_positions(content, "limit" => 5, "offset" => 2) FROM snippet_test WHERE content @@@ 'test';

SELECT paradedb.snippet_positions(content) FROM snippet_test WHERE content @@@ 'test';
SELECT paradedb.snippets(content) FROM snippet_test WHERE content @@@ 'test';

DROP TABLE snippet_test;
